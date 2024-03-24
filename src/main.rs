use ansi_term::{Colour::Fixed, Colour::RGB, Style};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use zellij_tile::prelude::*;
use zellij_tile_utils::palette_match;

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
    pane_id: u32,
    refresh_panes: bool,
    palette: Palette,
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
        self.refresh_panes = true;
        self.pane_id = get_plugin_ids().plugin_id;
        self.label_alphabet = configuration
            .get("label_alphabet")
            .map(|alphabet| alphabet.trim().to_lowercase())
            .unwrap_or("fjdkslarueiwoqpcmx".to_string())
            .chars()
            .collect();

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::Visible,
            EventType::Key,
            EventType::PaneUpdate,
            EventType::ModeUpdate,
        ]);
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
                self.label_input = self.label_input.trim().to_string();

                if let Some(pane) = self.panes.get(&self.label_input) {
                    // pane selected
                    focus_terminal_pane(pane.id, false);
                    hide_self();
                    self.clear();
                }

                return true;
            }
            Event::Key(_) => {}
            Event::ModeUpdate(ModeInfo { style, .. }) => {
                self.palette = style.colors;
                return true;
            }
            Event::Visible(visible) => {
                self.refresh_panes = visible;
                return true;
            }
            Event::PaneUpdate(PaneManifest { panes }) => {
                if !self.refresh_panes {
                    return false;
                }

                self.panes.clear();
                let visible_panes: Vec<_> = panes
                    .values()
                    .flatten()
                    .filter(|p| p.is_selectable && p.id != self.pane_id)
                    .collect();
                let label_len = if visible_panes.len() <= self.label_alphabet.len() {
                    1
                } else {
                    2
                };

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

    fn render(&mut self, _rows: usize, _cols: usize) {
        // title
        println!("{}", color_bold(self.palette.green, "Jump panes"));

        // list
        for (label, pane) in self.panes.iter() {
            let label =
                if !self.label_input.trim().is_empty() && label.starts_with(&self.label_input) {
                    format!(
                        "{}{}",
                        color_bold(self.palette.red, &self.label_input),
                        color_bold(
                            self.palette.green,
                            &label
                                .chars()
                                .skip(self.label_input.len())
                                .collect::<String>()
                        )
                    )
                } else {
                    color_bold(self.palette.green, label)
                };
            println!("[{label}] {}", pane.title);
        }
    }
}

fn color_bold(color: PaletteColor, text: &str) -> String {
    format!(
        "{}",
        Style::new().fg(palette_match!(color)).bold().paint(text)
    )
}
