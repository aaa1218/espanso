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

use std::ffi::{c_char, c_int, c_void, CStr, CString};

use crate::{
    settings::{EditableSnippet, SettingsOptions, SettingsResult},
    sys::{
        interop::{SettingsMetadata, SnippetMetadata},
        util::convert_to_cstring_or_null,
    },
};

struct OwnedSnippet {
    source_index: i32,
    label: CString,
    trigger: CString,
    replace: CString,
    editable: bool,
}

impl OwnedSnippet {
    fn new(snippet: EditableSnippet) -> Self {
        Self {
            source_index: snippet.source_index,
            label: CString::new(snippet.label).expect("unable to convert label to CString"),
            trigger: CString::new(snippet.trigger).expect("unable to convert trigger to CString"),
            replace: CString::new(snippet.replace)
                .expect("unable to convert replacement to CString"),
            editable: snippet.editable,
        }
    }

    fn metadata(&self) -> SnippetMetadata {
        SnippetMetadata {
            source_index: self.source_index,
            label: self.label.as_ptr(),
            trigger: self.trigger.as_ptr(),
            replace: self.replace.as_ptr(),
            editable: i32::from(self.editable),
        }
    }
}

pub fn show(options: SettingsOptions) -> Option<SettingsResult> {
    let (_window_icon_path, window_icon_path_ptr) =
        convert_to_cstring_or_null(options.window_icon_path);
    let search_shortcut = CString::new(options.search_shortcut)
        .expect("unable to convert search shortcut to CString");
    let snippet_capture_shortcut = CString::new(options.snippet_capture_shortcut)
        .expect("unable to convert snippet capture shortcut to CString");
    let double_tap_key =
        CString::new(options.double_tap_key).expect("unable to convert double-tap key to CString");
    let double_tap_action = CString::new(options.double_tap_action)
        .expect("unable to convert double-tap action to CString");
    let owned_snippets: Vec<OwnedSnippet> = options
        .snippets
        .into_iter()
        .map(OwnedSnippet::new)
        .collect();
    let snippets: Vec<SnippetMetadata> =
        owned_snippets.iter().map(OwnedSnippet::metadata).collect();

    let metadata = SettingsMetadata {
        window_icon_path: window_icon_path_ptr,
        snippets: snippets.as_ptr(),
        snippets_count: snippets.len() as c_int,
        search_shortcut: search_shortcut.as_ptr(),
        snippet_capture_shortcut: snippet_capture_shortcut.as_ptr(),
        double_tap_key: double_tap_key.as_ptr(),
        double_tap_action: double_tap_action.as_ptr(),
        show_icon: i32::from(options.show_icon),
        show_notifications: i32::from(options.show_notifications),
        auto_restart: i32::from(options.auto_restart),
    };

    let mut result: Option<SettingsResult> = None;

    extern "C" fn result_callback(
        snippets: *const SnippetMetadata,
        snippets_count: c_int,
        search_shortcut: *const c_char,
        snippet_capture_shortcut: *const c_char,
        double_tap_key: *const c_char,
        double_tap_action: *const c_char,
        show_icon: c_int,
        show_notifications: c_int,
        auto_restart: c_int,
        result: *mut c_void,
    ) {
        let snippets = if snippets_count <= 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(snippets, snippets_count as usize) }
        }
        .iter()
        .map(|snippet| EditableSnippet {
            source_index: snippet.source_index,
            label: unsafe { CStr::from_ptr(snippet.label) }
                .to_string_lossy()
                .into_owned(),
            trigger: unsafe { CStr::from_ptr(snippet.trigger) }
                .to_string_lossy()
                .into_owned(),
            replace: unsafe { CStr::from_ptr(snippet.replace) }
                .to_string_lossy()
                .into_owned(),
            editable: snippet.editable == 1,
        })
        .collect();

        let settings_result = SettingsResult {
            snippets,
            search_shortcut: unsafe { CStr::from_ptr(search_shortcut) }
                .to_string_lossy()
                .into_owned(),
            snippet_capture_shortcut: unsafe { CStr::from_ptr(snippet_capture_shortcut) }
                .to_string_lossy()
                .into_owned(),
            double_tap_key: unsafe { CStr::from_ptr(double_tap_key) }
                .to_string_lossy()
                .into_owned(),
            double_tap_action: unsafe { CStr::from_ptr(double_tap_action) }
                .to_string_lossy()
                .into_owned(),
            show_icon: show_icon == 1,
            show_notifications: show_notifications == 1,
            auto_restart: auto_restart == 1,
        };

        let result = result.cast::<Option<SettingsResult>>();
        unsafe {
            *result = Some(settings_result);
        }
    }

    unsafe {
        super::interop::interop_show_settings(
            &metadata,
            result_callback,
            std::ptr::from_mut(&mut result).cast::<c_void>(),
        );
    }

    result
}
