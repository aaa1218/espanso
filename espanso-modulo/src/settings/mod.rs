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

pub use crate::sys::settings::show;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditableSnippet {
    pub source_index: i32,
    pub label: String,
    pub trigger: String,
    pub replace: String,
    pub editable: bool,
}

#[derive(Debug)]
pub struct SettingsOptions {
    pub window_icon_path: Option<String>,
    pub snippets: Vec<EditableSnippet>,
    pub search_shortcut: String,
    pub snippet_capture_shortcut: String,
    pub double_tap_key: String,
    pub double_tap_action: String,
    pub show_icon: bool,
    pub show_notifications: bool,
    pub auto_restart: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SettingsResult {
    pub snippets: Vec<EditableSnippet>,
    pub search_shortcut: String,
    pub snippet_capture_shortcut: String,
    pub double_tap_key: String,
    pub double_tap_action: String,
    pub show_icon: bool,
    pub show_notifications: bool,
    pub auto_restart: bool,
}
