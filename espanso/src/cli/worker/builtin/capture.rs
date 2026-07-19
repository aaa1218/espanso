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

use espanso_engine::event::EventType;

use super::{generate_next_builtin_id, BuiltInMatch};

pub fn create_capture_selection_match(hotkey: String) -> BuiltInMatch {
    BuiltInMatch {
        id: generate_next_builtin_id(),
        label: "Capture selected text",
        hotkey: Some(hotkey),
        action: |_| EventType::CaptureSelection,
        ..Default::default()
    }
}
