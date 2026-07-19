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

#[cfg(feature = "modulo")]
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(feature = "modulo")]
use anyhow::{Context, Result};
#[cfg(feature = "modulo")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "modulo")]
use serde_norway::Value;
#[cfg(feature = "modulo")]
use tempfile::NamedTempFile;

use super::{CliModule, CliModuleArgs};

#[cfg(feature = "modulo")]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MatchDocument {
    #[serde(default)]
    matches: Vec<MatchEntry>,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[cfg(feature = "modulo")]
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

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[cfg(feature = "modulo")]
impl MatchEntry {
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

    fn replacement(&self) -> String {
        self.replace
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    fn is_editable(&self) -> bool {
        let has_single_trigger = self.trigger.is_some()
            || self
                .triggers
                .as_ref()
                .is_some_and(|triggers| triggers.len() == 1);
        has_single_trigger && self.replace.as_ref().is_some_and(Value::is_string)
    }

    fn update_from(&mut self, snippet: &espanso_modulo::settings::EditableSnippet) {
        self.label = Some(snippet.label.clone());
        if let Some(trigger) = self.trigger.as_mut() {
            *trigger = snippet.trigger.clone();
        } else if let Some(triggers) = self.triggers.as_mut() {
            if triggers.len() == 1 {
                triggers[0] = snippet.trigger.clone();
            }
        }
        self.replace = Some(Value::String(snippet.replace.clone()));
    }
}

#[cfg(feature = "modulo")]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ConfigDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    search_shortcut: Option<String>,

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
                    trigger: Some(snippet.trigger),
                    replace: Some(Value::String(snippet.replace)),
                    ..Default::default()
                })
            }
        })
        .collect::<Result<Vec<_>>>()?;

    config_document.search_shortcut = Some(result.search_shortcut);
    config_document.show_icon = Some(result.show_icon);
    config_document.show_notifications = Some(result.show_notifications);
    config_document.auto_restart = Some(result.auto_restart);

    write_yaml_atomically(&match_path, &match_document)?;
    write_yaml_atomically(&config_path, &config_document)?;
    Ok(())
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

#[cfg(feature = "modulo")]
fn read_yaml<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("unable to read {}", path.display()))?;
    serde_norway::from_str(&content).with_context(|| format!("unable to parse {}", path.display()))
}

#[cfg(feature = "modulo")]
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
}
