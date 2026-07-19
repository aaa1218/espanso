/*
 * This file is part of modulo.
 *
 * Copyright (C) 2020-2021 Federico Terzi
 *
 * modulo is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * modulo is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with modulo.  If not, see <https://www.gnu.org/licenses/>.
 */

#include "common.h"

#ifdef __WXMSW__
#include <dwmapi.h>
#include <windows.h>
#pragma comment(lib, "dwmapi.lib")
#endif
#ifdef __WXOSX__
#include "mac.h"
#endif

void setFrameIcon(wxString iconPath, wxFrame *frame) {
    if (!iconPath.IsEmpty()) {
        wxBitmapType imgType = wxICON_DEFAULT_TYPE;

#ifdef __WXMSW__
        imgType = wxBITMAP_TYPE_ICO;
#endif

        wxIcon icon;
        icon.LoadFile(iconPath, imgType);
        if (icon.IsOk()) {
            frame->SetIcon(icon);
        }
    }
}

void Activate(wxFrame *frame) {
#ifdef __WXMSW__

    HWND handle = frame->GetHandle();
    if (handle == GetForegroundWindow()) {
        return;
    }

    if (IsIconic(handle)) {
        ShowWindow(handle, 9);
    }

    INPUT ip;
    ip.type = INPUT_KEYBOARD;
    ip.ki.wScan = 0;
    ip.ki.time = 0;
    ip.ki.dwExtraInfo = 0;
    ip.ki.wVk = VK_MENU;
    ip.ki.dwFlags = 0;

    SendInput(1, &ip, sizeof(INPUT));
    ip.ki.dwFlags = KEYEVENTF_KEYUP;

    SendInput(1, &ip, sizeof(INPUT));

    SetForegroundWindow(handle);

#endif
#ifdef __WXOSX__
    ActivateApp();
#endif
}

void SetupWindowStyle(wxFrame *frame) {
#ifdef __WXOSX__
    SetWindowStyles((NSWindow *)frame->MacGetTopLevelWindowRef());
#endif
}

bool IsSystemDarkMode() {
#ifdef __WXMSW__
    DWORD appsUseLightTheme = 1;
    DWORD valueSize = sizeof(appsUseLightTheme);
    const LSTATUS status = RegGetValueW(
        HKEY_CURRENT_USER,
        L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        L"AppsUseLightTheme", RRF_RT_REG_DWORD, nullptr, &appsUseLightTheme,
        &valueSize);

    return status == ERROR_SUCCESS && appsUseLightTheme == 0;
#elif wxCHECK_VERSION(3, 1, 3)
    return wxSystemSettings::GetAppearance().IsDark();
#else
    const wxColour bg = wxSystemSettings::GetColour(wxSYS_COLOUR_WINDOW);
    const wxColour fg = wxSystemSettings::GetColour(wxSYS_COLOUR_WINDOWTEXT);
    const unsigned int bgSum = bg.Red() + bg.Blue() + bg.Green();
    const unsigned int fgSum = fg.Red() + fg.Blue() + fg.Green();
    return fgSum > bgSum;
#endif
}

void ApplyDarkTitleBar(wxWindow *window, bool isDark) {
#ifdef __WXMSW__
    BOOL enabled = isDark ? TRUE : FALSE;
    HWND handle = static_cast<HWND>(window->GetHandle());

    if (FAILED(DwmSetWindowAttribute(handle, 20, &enabled, sizeof(enabled)))) {
        DwmSetWindowAttribute(handle, 19, &enabled, sizeof(enabled));
    }
#else
    (void)window;
    (void)isDark;
#endif
}
