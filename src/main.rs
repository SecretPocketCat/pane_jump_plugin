use ansi_term::{Colour::Fixed, Style};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use zellij_tile::prelude::*;

#[derive(Default)]
struct JumpPane {
    title: String,
    id: u32,
}

#[derive(Default)]
struct PluginState {
    panes: HashMap<String, JumpPane>,
    label_input: String,
    label_alphabet: Vec<char>,
}

register_plugin!(PluginState);

impl PluginState {
    fn clear(&mut self) {
        self.label_input.clear();
        self.panes.clear();
    }
}

impl ZellijPlugin for PluginState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.label_alphabet = configuration
            .get("label_alphabet")
            .map(|alphabet| alphabet.trim().to_lowercase())
            // todo: probly qwerty default
            .unwrap_or("tnseriaoplfuwyzkbvm".to_string())
            .chars()
            .collect();

        // todo: tidy-up permissions
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[EventType::Key, EventType::PaneUpdate]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(Key::Esc) => {
                hide_self();
                self.clear();
                return true;
            }
            Event::Key(Key::Backspace) | Event::Key(Key::Delete) => {
                self.label_input.pop();
                return true;
            }
            Event::Key(Key::Char(c)) => {
                self.label_input.push(c);

                if let Some(pane) = self.panes.get(&self.label_input) {
                    // pane selected
                    focus_terminal_pane(pane.id, false);
                    hide_self();
                    self.clear();
                }

                return true;
            }
            Event::Key(_) => {}
            Event::PaneUpdate(PaneManifest { panes }) => {
                self.panes.clear();
                let visible_panes: Vec<_> = panes
                    .values()
                    .flatten()
                    .filter(|p| p.is_selectable)
                    .collect();
                let label_len = if visible_panes.len() <= self.label_alphabet.len() {
                    1
                } else {
                    2
                };

                // todo: exclude self && skip when the plugin pane is hidden
                let mut unprocessed_panes = Vec::new();
                for pane in visible_panes {
                    let preffered_label = pane
                        .title
                        .chars()
                        .take(label_len)
                        .collect::<String>()
                        .to_lowercase();
                    if !self.panes.contains_key(&preffered_label)
                        && preffered_label
                            .chars()
                            .all(|c| self.label_alphabet.contains(&c))
                    {
                        self.panes.insert(
                            preffered_label,
                            JumpPane {
                                id: pane.id,
                                title: pane.title.clone(),
                            },
                        );
                    } else {
                        unprocessed_panes.push(pane);
                    }
                }

                if !unprocessed_panes.is_empty() {
                    let mut alpha = self.label_alphabet.iter().permutations(label_len);
                    for pane in unprocessed_panes {
                        while let Some(label) = alpha.next() {
                            let label = label.iter().cloned().collect();
                            if !self.panes.contains_key(&label) {
                                self.panes.insert(
                                    label,
                                    JumpPane {
                                        id: pane.id,
                                        title: pane.title.clone(),
                                    },
                                );
                                break;
                            }
                        }
                    }
                }

                return true;
            }
            _ => unimplemented!("{event:?}"),
        };

        false
    }

    // todo: use theme colours
    fn render(&mut self, _rows: usize, _cols: usize) {
        // title
        println!(
            "{}",
            Style::new().fg(Fixed(GREEN)).bold().paint("Jump List\n")
        );

        // list
        for (label, pane) in self.panes.iter() {
            let label =
                if !self.label_input.trim().is_empty() && label.starts_with(&self.label_input) {
                    format!("{}", color_bold(GREEN, &self.label_input))
                } else {
                    color_bold(RED, label)
                };
            println!("[{label}] {}", pane.title);
        }
    }
}

pub const CYAN: u8 = 51;
pub const GRAY_LIGHT: u8 = 238;
pub const GRAY_DARK: u8 = 245;
pub const WHITE: u8 = 15;
pub const BLACK: u8 = 16;
pub const RED: u8 = 124;
pub const GREEN: u8 = 154;
pub const ORANGE: u8 = 166;

fn color_bold(color: u8, text: &str) -> String {
    format!("{}", Style::new().fg(Fixed(color)).bold().paint(text))
}
