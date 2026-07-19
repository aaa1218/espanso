/*
 * This file is part of espanso.
 *
 * Copyright (C) 2026 Espanso contributors
 *
 * espanso is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

#define _UNICODE

#include "../common/common.h"
#include "../interop/interop.h"

#include <string>
#include <vector>

#include <wx/listctrl.h>
#include <wx/notebook.h>
#include <wx/choice.h>

const wxColour SETTINGS_DARK_BG = wxColour(32, 33, 36);
const wxColour SETTINGS_DARK_CONTROL_BG = wxColour(43, 45, 48);
const wxColour SETTINGS_DARK_TEXT = wxColour(245, 245, 245);

typedef void (*SettingsResultCallback)(const SnippetMetadata *snippets,
                                       int snippets_count,
                                       const char *search_shortcut,
                                       const char *snippet_capture_shortcut,
                                       const char *double_tap_key,
                                       const char *double_tap_action,
                                       int show_icon,
                                       int show_notifications,
                                       int auto_restart, void *result);

SettingsMetadata *settings_metadata = nullptr;
SettingsResultCallback settings_result_callback = nullptr;
void *settings_result_data = nullptr;

struct SnippetData {
    int source_index;
    std::string label;
    std::string trigger;
    std::string replace;
    bool editable;
};

std::string ToUtf8(const wxString &value) {
    const wxScopedCharBuffer buffer = value.ToUTF8();
    return buffer.data() ? std::string(buffer.data()) : std::string();
}

void ApplyDarkControl(wxWindow *window) {
    window->SetBackgroundColour(SETTINGS_DARK_CONTROL_BG);
    window->SetForegroundColour(SETTINGS_DARK_TEXT);
}

class SnippetDialog : public wxDialog {
  public:
    SnippetDialog(wxWindow *parent, const SnippetData *snippet, bool isDark)
        : wxDialog(parent, wxID_ANY,
                   snippet ? wxT("Edit snippet") : wxT("Add snippet"),
                   wxDefaultPosition, wxSize(560, 430),
                   wxDEFAULT_DIALOG_STYLE | wxRESIZE_BORDER) {
        wxBoxSizer *root = new wxBoxSizer(wxVERTICAL);
        wxFlexGridSizer *fields = new wxFlexGridSizer(2, 8, 8);
        fields->AddGrowableCol(1, 1);
        fields->AddGrowableRow(2, 1);

        wxStaticText *labelText = new wxStaticText(this, wxID_ANY, wxT("Name"));
        label = new wxTextCtrl(this, wxID_ANY);
        wxStaticText *triggerText =
            new wxStaticText(this, wxID_ANY, wxT("Trigger"));
        trigger = new wxTextCtrl(this, wxID_ANY);
        wxStaticText *replaceText =
            new wxStaticText(this, wxID_ANY, wxT("Content"));
        replace = new wxTextCtrl(this, wxID_ANY, wxEmptyString,
                                 wxDefaultPosition, wxDefaultSize,
                                 wxTE_MULTILINE | wxTE_RICH2);

        fields->Add(labelText, 0, wxALIGN_CENTER_VERTICAL);
        fields->Add(label, 1, wxEXPAND);
        fields->Add(triggerText, 0, wxALIGN_CENTER_VERTICAL);
        fields->Add(trigger, 1, wxEXPAND);
        fields->Add(replaceText, 0, wxALIGN_TOP);
        fields->Add(replace, 1, wxEXPAND);

        root->Add(fields, 1, wxEXPAND | wxALL, 16);
        root->Add(CreateSeparatedButtonSizer(wxOK | wxCANCEL), 0,
                  wxEXPAND | wxLEFT | wxRIGHT | wxBOTTOM, 16);
        SetSizer(root);

        if (snippet) {
            label->SetValue(wxString::FromUTF8(snippet->label));
            trigger->SetValue(wxString::FromUTF8(snippet->trigger));
            replace->SetValue(wxString::FromUTF8(snippet->replace));
        }

        if (isDark) {
            SetBackgroundColour(SETTINGS_DARK_BG);
            SetForegroundColour(SETTINGS_DARK_TEXT);
            labelText->SetForegroundColour(SETTINGS_DARK_TEXT);
            triggerText->SetForegroundColour(SETTINGS_DARK_TEXT);
            replaceText->SetForegroundColour(SETTINGS_DARK_TEXT);
            ApplyDarkControl(label);
            ApplyDarkControl(trigger);
            ApplyDarkControl(replace);
            ApplyDarkTitleBar(this, true);
        }

        CentreOnParent();
        label->SetFocus();
    }

    std::string GetSnippetLabel() const { return ToUtf8(label->GetValue()); }
    std::string GetSnippetTrigger() const { return ToUtf8(trigger->GetValue()); }
    std::string GetSnippetReplacement() const {
        return ToUtf8(replace->GetValue());
    }

  private:
    wxTextCtrl *label = nullptr;
    wxTextCtrl *trigger = nullptr;
    wxTextCtrl *replace = nullptr;
};

class SettingsFrame : public wxFrame {
  public:
    SettingsFrame();

  private:
    std::vector<SnippetData> snippets;
    std::vector<size_t> visible_snippets;
    bool isDark = false;

    wxTextCtrl *filter = nullptr;
    wxListCtrl *snippetList = nullptr;
    wxButton *editButton = nullptr;
    wxButton *deleteButton = nullptr;
    wxTextCtrl *searchShortcut = nullptr;
    wxTextCtrl *snippetCaptureShortcut = nullptr;
    wxTextCtrl *doubleTapKey = nullptr;
    wxChoice *doubleTapAction = nullptr;
    wxCheckBox *showIcon = nullptr;
    wxCheckBox *showNotifications = nullptr;
    wxCheckBox *autoRestart = nullptr;

    void RefreshSnippets();
    int SelectedSnippetIndex() const;
    void UpdateActionState();
    void OnFilterChanged(wxCommandEvent &event);
    void OnSelectionChanged(wxListEvent &event);
    void OnAdd(wxCommandEvent &event);
    void OnEdit(wxCommandEvent &event);
    void OnDelete(wxCommandEvent &event);
    void OnSave(wxCommandEvent &event);
    void OnCancel(wxCommandEvent &event);
};

SettingsFrame::SettingsFrame()
    : wxFrame(nullptr, wxID_ANY, wxT("Espanso Settings"), wxDefaultPosition,
              wxSize(860, 620), wxDEFAULT_FRAME_STYLE) {
    isDark = IsSystemDarkMode();
    if (settings_metadata->window_icon_path) {
        setFrameIcon(wxString::FromUTF8(settings_metadata->window_icon_path),
                     this);
    }

    for (int index = 0; index < settings_metadata->snippets_count; index++) {
        const SnippetMetadata &snippet = settings_metadata->snippets[index];
        snippets.push_back(SnippetData{
            snippet.source_index, snippet.label ? snippet.label : "",
            snippet.trigger ? snippet.trigger : "",
            snippet.replace ? snippet.replace : "", snippet.editable == 1});
    }

    wxPanel *rootPanel = new wxPanel(this);
    wxBoxSizer *root = new wxBoxSizer(wxVERTICAL);
    wxNotebook *notebook = new wxNotebook(rootPanel, wxID_ANY);

    wxPanel *snippetsPage = new wxPanel(notebook);
    wxBoxSizer *snippetsRoot = new wxBoxSizer(wxVERTICAL);
    wxBoxSizer *toolbar = new wxBoxSizer(wxHORIZONTAL);
    filter = new wxTextCtrl(snippetsPage, wxID_ANY);
    filter->SetHint(wxT("Search snippets"));
    wxButton *addButton = new wxButton(snippetsPage, wxID_ANY, wxT("Add"));
    editButton = new wxButton(snippetsPage, wxID_ANY, wxT("Edit"));
    deleteButton = new wxButton(snippetsPage, wxID_ANY, wxT("Delete"));
    toolbar->Add(filter, 1, wxRIGHT, 8);
    toolbar->Add(addButton, 0, wxRIGHT, 8);
    toolbar->Add(editButton, 0, wxRIGHT, 8);
    toolbar->Add(deleteButton, 0);
    snippetsRoot->Add(toolbar, 0, wxEXPAND | wxALL, 12);

    snippetList = new wxListCtrl(snippetsPage, wxID_ANY, wxDefaultPosition,
                                 wxDefaultSize,
                                 wxLC_REPORT | wxLC_SINGLE_SEL);
    snippetList->AppendColumn(wxT("Name"), wxLIST_FORMAT_LEFT, 220);
    snippetList->AppendColumn(wxT("Trigger"), wxLIST_FORMAT_LEFT, 160);
    snippetList->AppendColumn(wxT("Content"), wxLIST_FORMAT_LEFT, 410);
    snippetsRoot->Add(snippetList, 1, wxEXPAND | wxLEFT | wxRIGHT | wxBOTTOM,
                      12);
    snippetsPage->SetSizer(snippetsRoot);
    notebook->AddPage(snippetsPage, wxT("Snippets"), true);

    wxPanel *settingsPage = new wxPanel(notebook);
    wxBoxSizer *settingsRoot = new wxBoxSizer(wxVERTICAL);
    wxFlexGridSizer *settingsFields = new wxFlexGridSizer(2, 12, 12);
    settingsFields->AddGrowableCol(1, 1);
    wxStaticText *shortcutLabel =
        new wxStaticText(settingsPage, wxID_ANY, wxT("Search shortcut"));
    searchShortcut = new wxTextCtrl(
        settingsPage, wxID_ANY,
        wxString::FromUTF8(settings_metadata->search_shortcut));
    settingsFields->Add(shortcutLabel, 0, wxALIGN_CENTER_VERTICAL);
    settingsFields->Add(searchShortcut, 1, wxEXPAND);

    wxStaticText *captureShortcutLabel = new wxStaticText(
        settingsPage, wxID_ANY, wxT("Capture selection shortcut"));
    snippetCaptureShortcut = new wxTextCtrl(
        settingsPage, wxID_ANY,
        wxString::FromUTF8(settings_metadata->snippet_capture_shortcut));
    settingsFields->Add(captureShortcutLabel, 0, wxALIGN_CENTER_VERTICAL);
    settingsFields->Add(snippetCaptureShortcut, 1, wxEXPAND);

    wxStaticText *doubleTapKeyLabel = new wxStaticText(
        settingsPage, wxID_ANY,
        wxT("Double-tap key (NONCONVERT, CapsLock, F1-F12)"));
    doubleTapKey = new wxTextCtrl(
        settingsPage, wxID_ANY,
        wxString::FromUTF8(settings_metadata->double_tap_key));
    settingsFields->Add(doubleTapKeyLabel, 0, wxALIGN_CENTER_VERTICAL);
    settingsFields->Add(doubleTapKey, 1, wxEXPAND);

    wxStaticText *doubleTapActionLabel = new wxStaticText(
        settingsPage, wxID_ANY, wxT("Double-tap action"));
    wxArrayString doubleTapActions;
    doubleTapActions.Add(wxT("Off"));
    doubleTapActions.Add(wxT("Search snippets"));
    doubleTapActions.Add(wxT("Capture selection"));
    doubleTapAction = new wxChoice(settingsPage, wxID_ANY, wxDefaultPosition,
                                   wxDefaultSize, doubleTapActions);
    const wxString configuredDoubleTapAction =
        wxString::FromUTF8(settings_metadata->double_tap_action).Upper();
    if (configuredDoubleTapAction == wxT("SEARCH")) {
        doubleTapAction->SetSelection(1);
    } else if (configuredDoubleTapAction == wxT("CAPTURE_SELECTION")) {
        doubleTapAction->SetSelection(2);
    } else {
        doubleTapAction->SetSelection(0);
    }
    settingsFields->Add(doubleTapActionLabel, 0, wxALIGN_CENTER_VERTICAL);
    settingsFields->Add(doubleTapAction, 1, wxEXPAND);
    settingsRoot->Add(settingsFields, 0, wxEXPAND | wxALL, 20);

    showIcon = new wxCheckBox(settingsPage, wxID_ANY, wxT("Show tray icon"));
    showIcon->SetValue(settings_metadata->show_icon == 1);
    showNotifications = new wxCheckBox(settingsPage, wxID_ANY,
                                       wxT("Show notifications"));
    showNotifications->SetValue(settings_metadata->show_notifications == 1);
    autoRestart = new wxCheckBox(
        settingsPage, wxID_ANY,
        wxT("Automatically reload after configuration changes"));
    autoRestart->SetValue(settings_metadata->auto_restart == 1);
    settingsRoot->Add(showIcon, 0, wxLEFT | wxRIGHT | wxBOTTOM, 20);
    settingsRoot->Add(showNotifications, 0, wxLEFT | wxRIGHT | wxBOTTOM, 20);
    settingsRoot->Add(autoRestart, 0, wxLEFT | wxRIGHT | wxBOTTOM, 20);
    settingsPage->SetSizer(settingsRoot);
    notebook->AddPage(settingsPage, wxT("Settings"), false);

    root->Add(notebook, 1, wxEXPAND | wxALL, 12);
    wxBoxSizer *actions = new wxBoxSizer(wxHORIZONTAL);
    actions->AddStretchSpacer();
    wxButton *cancelButton =
        new wxButton(rootPanel, wxID_CANCEL, wxT("Cancel"));
    wxButton *saveButton = new wxButton(rootPanel, wxID_SAVE, wxT("Save"));
    actions->Add(cancelButton, 0, wxRIGHT, 8);
    actions->Add(saveButton, 0);
    root->Add(actions, 0, wxEXPAND | wxLEFT | wxRIGHT | wxBOTTOM, 12);
    rootPanel->SetSizer(root);

    if (isDark) {
        SetBackgroundColour(SETTINGS_DARK_BG);
        SetForegroundColour(SETTINGS_DARK_TEXT);
        rootPanel->SetBackgroundColour(SETTINGS_DARK_BG);
        snippetsPage->SetBackgroundColour(SETTINGS_DARK_BG);
        settingsPage->SetBackgroundColour(SETTINGS_DARK_BG);
        notebook->SetBackgroundColour(SETTINGS_DARK_BG);
        notebook->SetForegroundColour(SETTINGS_DARK_TEXT);
        ApplyDarkControl(filter);
        ApplyDarkControl(snippetList);
        ApplyDarkControl(searchShortcut);
        ApplyDarkControl(snippetCaptureShortcut);
        ApplyDarkControl(doubleTapKey);
        ApplyDarkControl(doubleTapAction);
        shortcutLabel->SetForegroundColour(SETTINGS_DARK_TEXT);
        captureShortcutLabel->SetForegroundColour(SETTINGS_DARK_TEXT);
        doubleTapKeyLabel->SetForegroundColour(SETTINGS_DARK_TEXT);
        doubleTapActionLabel->SetForegroundColour(SETTINGS_DARK_TEXT);
        showIcon->SetForegroundColour(SETTINGS_DARK_TEXT);
        showNotifications->SetForegroundColour(SETTINGS_DARK_TEXT);
        autoRestart->SetForegroundColour(SETTINGS_DARK_TEXT);
        ApplyDarkTitleBar(this, true);
    }

    filter->Bind(wxEVT_TEXT, &SettingsFrame::OnFilterChanged, this);
    snippetList->Bind(wxEVT_LIST_ITEM_SELECTED,
                      &SettingsFrame::OnSelectionChanged, this);
    snippetList->Bind(wxEVT_LIST_ITEM_DESELECTED,
                      &SettingsFrame::OnSelectionChanged, this);
    snippetList->Bind(wxEVT_LIST_ITEM_ACTIVATED,
                      &SettingsFrame::OnSelectionChanged, this);
    addButton->Bind(wxEVT_BUTTON, &SettingsFrame::OnAdd, this);
    editButton->Bind(wxEVT_BUTTON, &SettingsFrame::OnEdit, this);
    deleteButton->Bind(wxEVT_BUTTON, &SettingsFrame::OnDelete, this);
    saveButton->Bind(wxEVT_BUTTON, &SettingsFrame::OnSave, this);
    cancelButton->Bind(wxEVT_BUTTON, &SettingsFrame::OnCancel, this);

    RefreshSnippets();
    UpdateActionState();
    CentreOnScreen();
}

void SettingsFrame::RefreshSnippets() {
    const wxString query = filter->GetValue().Lower();
    snippetList->DeleteAllItems();
    visible_snippets.clear();

    for (size_t index = 0; index < snippets.size(); index++) {
        const SnippetData &snippet = snippets[index];
        const wxString searchable =
            (wxString::FromUTF8(snippet.label) + wxT(" ") +
             wxString::FromUTF8(snippet.trigger) + wxT(" ") +
             wxString::FromUTF8(snippet.replace))
                .Lower();
        if (!query.IsEmpty() && searchable.Find(query) == wxNOT_FOUND) {
            continue;
        }

        const long row = snippetList->InsertItem(
            snippetList->GetItemCount(), wxString::FromUTF8(snippet.label));
        snippetList->SetItem(row, 1, wxString::FromUTF8(snippet.trigger));
        wxString preview = wxString::FromUTF8(snippet.replace);
        preview.Replace(wxT("\r"), wxT(" "));
        preview.Replace(wxT("\n"), wxT(" "));
        if (!snippet.editable) {
            preview = wxT("[Advanced] ") + preview;
        }
        snippetList->SetItem(row, 2, preview);
        visible_snippets.push_back(index);
    }

    UpdateActionState();
}

int SettingsFrame::SelectedSnippetIndex() const {
    const long selected = snippetList->GetNextItem(
        -1, wxLIST_NEXT_ALL, wxLIST_STATE_SELECTED);
    if (selected == -1 || static_cast<size_t>(selected) >= visible_snippets.size()) {
        return -1;
    }
    return static_cast<int>(visible_snippets[static_cast<size_t>(selected)]);
}

void SettingsFrame::UpdateActionState() {
    const int index = SelectedSnippetIndex();
    editButton->Enable(index >= 0 && snippets[static_cast<size_t>(index)].editable);
    deleteButton->Enable(index >= 0);
}

void SettingsFrame::OnFilterChanged(wxCommandEvent &event) {
    RefreshSnippets();
    event.Skip();
}

void SettingsFrame::OnSelectionChanged(wxListEvent &event) {
    UpdateActionState();
    if (event.GetEventType() == wxEVT_LIST_ITEM_ACTIVATED && editButton->IsEnabled()) {
        wxCommandEvent editEvent;
        OnEdit(editEvent);
    }
}

void SettingsFrame::OnAdd(wxCommandEvent &) {
    SnippetDialog dialog(this, nullptr, isDark);
    if (dialog.ShowModal() != wxID_OK) {
        return;
    }
    const std::string triggerValue = dialog.GetSnippetTrigger();
    const std::string labelValue =
        dialog.GetSnippetLabel().empty() ? triggerValue
                                         : dialog.GetSnippetLabel();
    snippets.push_back(SnippetData{-1, labelValue, triggerValue,
                                   dialog.GetSnippetReplacement(), true});
    RefreshSnippets();
}

void SettingsFrame::OnEdit(wxCommandEvent &) {
    const int index = SelectedSnippetIndex();
    if (index < 0 || !snippets[static_cast<size_t>(index)].editable) {
        return;
    }

    SnippetData &snippet = snippets[static_cast<size_t>(index)];
    SnippetDialog dialog(this, &snippet, isDark);
    if (dialog.ShowModal() != wxID_OK) {
        return;
    }
    snippet.label = dialog.GetSnippetLabel().empty()
                        ? dialog.GetSnippetTrigger()
                        : dialog.GetSnippetLabel();
    snippet.trigger = dialog.GetSnippetTrigger();
    snippet.replace = dialog.GetSnippetReplacement();
    RefreshSnippets();
}

void SettingsFrame::OnDelete(wxCommandEvent &) {
    const int index = SelectedSnippetIndex();
    if (index < 0) {
        return;
    }
    if (wxMessageBox(wxT("Delete the selected snippet?"),
                     wxT("Espanso Settings"), wxYES_NO | wxICON_QUESTION,
                     this) != wxYES) {
        return;
    }

    snippets.erase(snippets.begin() + index);
    RefreshSnippets();
}

void SettingsFrame::OnSave(wxCommandEvent &) {
    std::vector<SnippetMetadata> metadata;
    metadata.reserve(snippets.size());
    for (const SnippetData &snippet : snippets) {
        metadata.push_back(SnippetMetadata{
            snippet.source_index, snippet.label.c_str(), snippet.trigger.c_str(),
            snippet.replace.c_str(), snippet.editable ? 1 : 0});
    }

    const std::string shortcut = ToUtf8(searchShortcut->GetValue());
    const std::string captureShortcut =
        ToUtf8(snippetCaptureShortcut->GetValue());
    const std::string configuredDoubleTapKey = ToUtf8(doubleTapKey->GetValue());
    const char *configuredDoubleTapAction = "OFF";
    if (doubleTapAction->GetSelection() == 1) {
        configuredDoubleTapAction = "SEARCH";
    } else if (doubleTapAction->GetSelection() == 2) {
        configuredDoubleTapAction = "CAPTURE_SELECTION";
    }
    settings_result_callback(metadata.data(), static_cast<int>(metadata.size()),
                             shortcut.c_str(), captureShortcut.c_str(),
                             configuredDoubleTapKey.c_str(),
                             configuredDoubleTapAction,
                             showIcon->GetValue() ? 1 : 0,
                             showNotifications->GetValue() ? 1 : 0,
                             autoRestart->GetValue() ? 1 : 0,
                             settings_result_data);
    Close(true);
}

void SettingsFrame::OnCancel(wxCommandEvent &) { Close(true); }

class SettingsApp : public wxApp {
  public:
    bool OnInit() override {
        SettingsFrame *frame = new SettingsFrame();
        frame->Show(true);
        Activate(frame);
        return true;
    }
};

extern "C" void interop_show_settings(SettingsMetadata *metadata,
                                      SettingsResultCallback callback,
                                      void *result) {
#ifdef __WXMSW__
    SetProcessDPIAware();
#endif

    settings_metadata = metadata;
    settings_result_callback = callback;
    settings_result_data = result;

    wxApp::SetInstance(new SettingsApp());
    int argc = 0;
    wxEntry(argc, static_cast<char **>(nullptr));
}
