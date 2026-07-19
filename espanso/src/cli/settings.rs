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

use std::{collections::HashMap, io::Write, path::Path};

#[cfg(feature = "modulo")]
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_norway::Value;
use tempfile::NamedTempFile;

use super::{CliModule, CliModuleArgs};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MatchDocument {
    #[serde(default)]
    matches: Vec<MatchEntry>,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MatchEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    triggers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    replace: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    search_terms: Option<Vec<String>>,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl MatchEntry {
    #[cfg(feature = "modulo")]
    fn primary_trigger(&self) -> String {
        self.trigger
            .clone()
            .or_else(|| {
                self.triggers
                    .as_ref()
                    .and_then(|triggers| triggers.first().cloned())
            })
            .or_else(|| {
                self.extra
                    .get("regex")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    }

    #[cfg(feature = "modulo")]
    fn replacement(&self) -> String {
        self.replace
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    #[cfg(feature = "modulo")]
    fn is_editable(&self) -> bool {
        let has_single_trigger = self.trigger.is_some()
            || self
                .triggers
                .as_ref()
                .is_some_and(|triggers| triggers.len() == 1);
        let has_no_cause =
            self.trigger.is_none() && self.triggers.is_none() && !self.extra.contains_key("regex");
        (has_single_trigger || has_no_cause) && self.replace.as_ref().is_some_and(Value::is_string)
    }

    #[cfg(feature = "modulo")]
    fn update_from(&mut self, snippet: &espanso_modulo::settings::EditableSnippet) {
        self.label = Some(snippet.label.clone());
        if self.trigger.is_some() {
            self.trigger = (!snippet.trigger.is_empty()).then(|| snippet.trigger.clone());
        } else if let Some(triggers) = self.triggers.as_mut() {
            if triggers.len() == 1 {
                if snippet.trigger.is_empty() {
                    self.triggers = None;
                } else {
                    triggers[0] = snippet.trigger.clone();
                }
            }
        } else if !snippet.trigger.is_empty() {
            self.trigger = Some(snippet.trigger.clone());
        }
        self.replace = Some(Value::String(snippet.replace.clone()));
        self.search_terms = Some(vec![snippet.replace.clone()]);
    }
}

#[cfg(feature = "modulo")]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ConfigDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    search_shortcut: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    snippet_capture_shortcut: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    double_tap_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    double_tap_action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    show_icon: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    show_notifications: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    auto_restart: Option<bool>,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

pub fn new() -> CliModule {
    CliModule {
        requires_paths: true,
        show_in_dock: true,
        subcommand: "settings".to_owned(),
        entry: settings_main,
        ..Default::default()
    }
}

#[cfg(feature = "modulo")]
fn settings_main(args: CliModuleArgs) -> i32 {
    let paths = args.paths.expect("missing paths in settings main");
    match run_settings(&paths.config, &paths.runtime) {
        Ok(()) => 0,
        Err(error) => {
            crate::error_eprintln!("unable to open settings: {error:#}");
            1
        }
    }
}

#[cfg(not(feature = "modulo"))]
fn settings_main(_: CliModuleArgs) -> i32 {
    crate::error_eprintln!("this version of espanso does not include GUI support");
    1
}

#[cfg(feature = "modulo")]
fn run_settings(config_root: &Path, runtime_dir: &Path) -> Result<()> {
    crate::config::populate_default_config(config_root)?;

    let match_path = config_root.join("match").join("base.yml");
    let config_path = resolve_default_config_path(config_root);
    let mut match_document: MatchDocument = read_yaml(&match_path)?;
    let mut config_document: ConfigDocument = read_yaml(&config_path)?;

    let snippets = match_document
        .matches
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let trigger = entry.primary_trigger();
            espanso_modulo::settings::EditableSnippet {
                source_index: index as i32,
                label: entry.label.clone().unwrap_or_else(|| {
                    if trigger.is_empty() {
                        "Advanced match".to_owned()
                    } else {
                        trigger.clone()
                    }
                }),
                trigger,
                replace: entry.replacement(),
                editable: entry.is_editable(),
            }
        })
        .collect();

    let icon_paths = crate::icon::load_icon_paths(runtime_dir)?;
    let result = espanso_modulo::settings::show(espanso_modulo::settings::SettingsOptions {
        window_icon_path: icon_paths
            .wizard_icon
            .map(|path| path.to_string_lossy().into_owned()),
        snippets,
        search_shortcut: config_document
            .search_shortcut
            .clone()
            .unwrap_or_else(|| "ALT+SPACE".to_owned()),
        snippet_capture_shortcut: config_document
            .snippet_capture_shortcut
            .clone()
            .unwrap_or_else(|| "CTRL+ALT+S".to_owned()),
        double_tap_key: config_document.double_tap_key.clone().unwrap_or_default(),
        double_tap_action: config_document
            .double_tap_action
            .clone()
            .unwrap_or_else(|| "OFF".to_owned()),
        show_icon: config_document.show_icon.unwrap_or(true),
        show_notifications: config_document.show_notifications.unwrap_or(true),
        auto_restart: config_document.auto_restart.unwrap_or(true),
    });

    let Some(result) = result else {
        return Ok(());
    };

    let original_matches = match_document.matches;
    match_document.matches = result
        .snippets
        .into_iter()
        .map(|snippet| {
            if snippet.source_index >= 0 {
                let mut entry = original_matches
                    .get(snippet.source_index as usize)
                    .cloned()
                    .context("settings returned an invalid snippet index")?;
                if snippet.editable {
                    entry.update_from(&snippet);
                }
                Ok(entry)
            } else {
                Ok(MatchEntry {
                    label: Some(snippet.label),
                    trigger: (!snippet.trigger.is_empty()).then_some(snippet.trigger),
                    replace: Some(Value::String(snippet.replace)),
                    ..Default::default()
                })
            }
        })
        .collect::<Result<Vec<_>>>()?;

    config_document.search_shortcut = Some(result.search_shortcut);
    config_document.snippet_capture_shortcut = Some(result.snippet_capture_shortcut);
    config_document.double_tap_key = Some(result.double_tap_key);
    config_document.double_tap_action = Some(result.double_tap_action);
    config_document.show_icon = Some(result.show_icon);
    config_document.show_notifications = Some(result.show_notifications);
    config_document.auto_restart = Some(result.auto_restart);

    write_yaml_atomically(&match_path, &match_document)?;
    write_yaml_atomically(&config_path, &config_document)?;
    Ok(())
}

pub(crate) fn append_captured_snippet(config_root: &Path, text: &str) -> Result<bool> {
    if text.trim().is_empty() {
        return Ok(false);
    }

    crate::config::populate_default_config(config_root)?;
    let match_path = config_root.join("match").join("base.yml");
    let mut document: MatchDocument = read_yaml(&match_path)?;
    if document.matches.iter().any(|entry| {
        entry.trigger.is_none()
            && entry.triggers.is_none()
            && entry.replace.as_ref().and_then(Value::as_str) == Some(text)
    }) {
        return Ok(false);
    }

    document.matches.push(MatchEntry {
        label: Some(captured_snippet_label(text)),
        replace: Some(Value::String(text.to_owned())),
        search_terms: Some(vec![text.to_owned()]),
        ..Default::default()
    });
    write_yaml_atomically(&match_path, &document)?;
    Ok(true)
}

fn captured_snippet_label(text: &str) -> String {
    const MAX_LABEL_CHARS: usize = 60;
    let first_line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut label: String = first_line.chars().take(MAX_LABEL_CHARS).collect();
    if first_line.chars().count() > MAX_LABEL_CHARS {
        label.push('…');
    }
    label
}

#[cfg(feature = "modulo")]
fn resolve_default_config_path(config_root: &Path) -> PathBuf {
    let yml_path = config_root.join("config").join("default.yml");
    if yml_path.is_file() {
        yml_path
    } else {
        config_root.join("config").join("default.yaml")
    }
}

fn read_yaml<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("unable to read {}", path.display()))?;
    serde_norway::from_str(&content).with_context(|| format!("unable to parse {}", path.display()))
}

fn write_yaml_atomically<T>(path: &Path, document: &T) -> Result<()>
where
    T: Serialize,
{
    let parent = path.parent().context("YAML path has no parent")?;
    let content = serde_norway::to_string(document)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("unable to replace {}", path.display()))?;
    Ok(())
}

#[cfg(all(test, feature = "modulo"))]
mod tests {
    use super::*;

    #[test]
    fn match_document_preserves_unknown_fields() {
        let input = r#"
imports:
  - "../shared.yml"
matches:
  - label: Greeting
    trigger: ":hello"
    replace: Hello
    propagate_case: true
"#;
        let mut document: MatchDocument = serde_norway::from_str(input).unwrap();
        let snippet = espanso_modulo::settings::EditableSnippet {
            source_index: 0,
            label: "Welcome".to_owned(),
            trigger: ":welcome".to_owned(),
            replace: "Welcome!".to_owned(),
            editable: true,
        };
        document.matches[0].update_from(&snippet);

        let output = serde_norway::to_string(&document).unwrap();
        assert!(output.contains("imports:"));
        assert!(output.contains("propagate_case: true"));
        assert!(output.contains("trigger: :welcome"));
    }

    #[test]
    fn advanced_matches_are_not_editable() {
        let input = r#"
matches:
  - regex: ":number\\d+"
    replace: number
  - triggers: [":one", ":two"]
    replace: shared
"#;
        let document: MatchDocument = serde_norway::from_str(input).unwrap();
        assert!(!document.matches[0].is_editable());
        assert!(!document.matches[1].is_editable());
    }

    #[test]
    fn captured_snippet_label_uses_first_non_empty_line_and_truncates_safely() {
        let text = format!("\n  {}\nsecond", "あ".repeat(70));
        let label = captured_snippet_label(&text);
        assert_eq!(label.chars().count(), 61);
        assert!(label.ends_with('…'));
    }

    #[test]
    fn captured_snippet_is_search_only_and_duplicates_are_ignored() {
        let directory = tempfile::tempdir().unwrap();
        assert!(append_captured_snippet(directory.path(), "First line\nSecond line").unwrap());
        assert!(!append_captured_snippet(directory.path(), "First line\nSecond line").unwrap());

        let document: MatchDocument =
            read_yaml(&directory.path().join("match").join("base.yml")).unwrap();
        let captured = document.matches.last().unwrap();
        assert_eq!(captured.label.as_deref(), Some("First line"));
        assert!(captured.trigger.is_none());
        assert!(captured.triggers.is_none());
        assert_eq!(
            captured.replace.as_ref().and_then(Value::as_str),
            Some("First line\nSecond line")
        );
        assert_eq!(
            captured.search_terms.as_deref(),
            Some(["First line\nSecond line".to_owned()].as_slice())
        );
    }
}
