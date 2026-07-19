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

use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use super::super::Middleware;
use crate::event::{
    input::{Key, Status},
    Event, EventType,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoubleTapAction {
    Search,
    CaptureSelection,
}

#[derive(Clone, Debug)]
pub struct DoubleTapOptions {
    pub key: Option<Key>,
    pub action: Option<DoubleTapAction>,
    pub interval: Duration,
}

impl Default for DoubleTapOptions {
    fn default() -> Self {
        Self {
            key: None,
            action: None,
            interval: Duration::from_millis(300),
        }
    }
}

pub struct DoubleTapMiddleware {
    options: DoubleTapOptions,
    last_release: RefCell<Option<Instant>>,
}

impl DoubleTapMiddleware {
    pub fn new(options: DoubleTapOptions) -> Self {
        Self {
            options,
            last_release: RefCell::new(None),
        }
    }
}

impl Middleware for DoubleTapMiddleware {
    fn name(&self) -> &'static str {
        "double_tap"
    }

    fn next(&self, event: Event, _: &mut dyn FnMut(Event)) -> Event {
        let (Some(configured_key), Some(action)) = (&self.options.key, self.options.action) else {
            return event;
        };
        let EventType::Keyboard(keyboard) = &event.etype else {
            return event;
        };
        if keyboard.status != Status::Released || keyboard.key != *configured_key {
            return event;
        }

        let now = Instant::now();
        let mut last_release = self.last_release.borrow_mut();
        let is_double_tap = last_release
            .is_some_and(|previous| now.duration_since(previous) <= self.options.interval);
        *last_release = if is_double_tap { None } else { Some(now) };
        if !is_double_tap {
            return event;
        }

        Event::caused_by(
            event.source_id,
            match action {
                DoubleTapAction::Search => EventType::ShowSearchBar,
                DoubleTapAction::CaptureSelection => EventType::CaptureSelection,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::input::KeyboardEvent;

    fn released(source_id: u32) -> Event {
        Event {
            source_id,
            etype: EventType::Keyboard(KeyboardEvent {
                key: Key::Other(0x1d),
                value: None,
                status: Status::Released,
                variant: None,
            }),
        }
    }

    #[test]
    fn triggers_configured_action_on_second_release() {
        let middleware = DoubleTapMiddleware::new(DoubleTapOptions {
            key: Some(Key::Other(0x1d)),
            action: Some(DoubleTapAction::Search),
            interval: Duration::from_secs(1),
        });
        let mut dispatch = |_| {};
        assert!(matches!(
            middleware.next(released(1), &mut dispatch).etype,
            EventType::Keyboard(_)
        ));
        assert!(matches!(
            middleware.next(released(2), &mut dispatch).etype,
            EventType::ShowSearchBar
        ));
    }
}
