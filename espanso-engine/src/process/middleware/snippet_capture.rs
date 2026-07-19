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

use super::super::Middleware;
use crate::event::{Event, EventType};

pub trait SnippetCapturer {
    fn capture_selection(&self);
}

pub struct SnippetCaptureMiddleware<'a> {
    capturer: &'a dyn SnippetCapturer,
}

impl<'a> SnippetCaptureMiddleware<'a> {
    pub fn new(capturer: &'a dyn SnippetCapturer) -> Self {
        Self { capturer }
    }
}

impl Middleware for SnippetCaptureMiddleware<'_> {
    fn name(&self) -> &'static str {
        "snippet_capture"
    }

    fn next(&self, event: Event, _: &mut dyn FnMut(Event)) -> Event {
        if matches!(event.etype, EventType::CaptureSelection) {
            self.capturer.capture_selection();
            return Event::caused_by(event.source_id, EventType::NOOP);
        }
        event
    }
}
