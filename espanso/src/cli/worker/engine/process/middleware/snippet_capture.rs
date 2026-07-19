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

use std::{path::Path, thread, time::Duration};

use espanso_clipboard::{Clipboard, ClipboardOperationOptions};
use espanso_engine::process::SnippetCapturer;
use espanso_inject::{keys::Key, InjectionOptions, Injector};
use log::{error, info, warn};

pub struct SnippetCapturerAdapter<'a> {
    injector: &'a dyn Injector,
    clipboard: &'a dyn Clipboard,
    config_root: &'a Path,
    injection_options: InjectionOptions,
    clipboard_options: ClipboardOperationOptions,
}

impl<'a> SnippetCapturerAdapter<'a> {
    pub fn new(
        injector: &'a dyn Injector,
        clipboard: &'a dyn Clipboard,
        config_root: &'a Path,
        injection_options: InjectionOptions,
        clipboard_options: ClipboardOperationOptions,
    ) -> Self {
        Self {
            injector,
            clipboard,
            config_root,
            injection_options,
            clipboard_options,
        }
    }
}

impl SnippetCapturer for SnippetCapturerAdapter<'_> {
    fn capture_selection(&self) {
        let previous = self.clipboard.get_text(&self.clipboard_options);
        let modifier = if cfg!(target_os = "macos") {
            Key::Meta
        } else {
            Key::Control
        };
        if let Err(error) = self
            .injector
            .send_key_combination(&[modifier, Key::C], self.injection_options)
        {
            error!("unable to copy selected text for snippet capture: {error}");
            return;
        }

        let captured = (0..10).find_map(|_| {
            thread::sleep(Duration::from_millis(50));
            let current = self.clipboard.get_text(&self.clipboard_options);
            (current != previous).then_some(current).flatten()
        });
        let Some(captured) = captured else {
            warn!("snippet capture ignored because the clipboard did not change");
            return;
        };

        match crate::cli::settings::append_captured_snippet(self.config_root, &captured) {
            Ok(true) => info!("captured selected text as a search-only snippet"),
            Ok(false) => info!("snippet capture ignored empty or duplicate text"),
            Err(error) => error!("unable to save captured snippet: {error:#}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        sync::{Arc, Mutex},
    };

    use anyhow::Result;
    use espanso_inject::keys::Key;

    use super::*;

    struct FakeClipboard {
        content: Arc<Mutex<Option<String>>>,
    }

    impl Clipboard for FakeClipboard {
        fn get_text(&self, _: &ClipboardOperationOptions) -> Option<String> {
            self.content.lock().unwrap().clone()
        }

        fn set_text(&self, text: &str, _: &ClipboardOperationOptions) -> Result<()> {
            *self.content.lock().unwrap() = Some(text.to_owned());
            Ok(())
        }

        fn set_image(&self, _: &Path, _: &ClipboardOperationOptions) -> Result<()> {
            Ok(())
        }

        fn set_html(&self, _: &str, _: Option<&str>, _: &ClipboardOperationOptions) -> Result<()> {
            Ok(())
        }
    }

    struct FakeInjector {
        clipboard: Arc<Mutex<Option<String>>>,
        selection: String,
    }

    impl Injector for FakeInjector {
        fn send_string(&self, _: &str, _: InjectionOptions) -> Result<()> {
            Ok(())
        }

        fn send_keys(&self, _: &[Key], _: InjectionOptions) -> Result<()> {
            Ok(())
        }

        fn send_key_combination(&self, _: &[Key], _: InjectionOptions) -> Result<()> {
            *self.clipboard.lock().unwrap() = Some(self.selection.clone());
            Ok(())
        }
    }

    #[test]
    fn captures_selection_without_opening_ui() {
        let directory = tempfile::tempdir().unwrap();
        let clipboard_state = Arc::new(Mutex::new(Some("previous".to_owned())));
        let clipboard = FakeClipboard {
            content: Arc::clone(&clipboard_state),
        };
        let injector = FakeInjector {
            clipboard: clipboard_state,
            selection: "Captured selection".to_owned(),
        };
        let capturer = SnippetCapturerAdapter::new(
            &injector,
            &clipboard,
            directory.path(),
            InjectionOptions::default(),
            ClipboardOperationOptions::default(),
        );

        capturer.capture_selection();

        let yaml =
            std::fs::read_to_string(directory.path().join("match").join("base.yml")).unwrap();
        assert!(yaml.contains("label: Captured selection"));
        assert!(yaml.contains("replace: Captured selection"));
        let captured_section = yaml.rsplit("- label: Captured selection").next().unwrap();
        assert!(!captured_section.contains("trigger:"));
    }
}
