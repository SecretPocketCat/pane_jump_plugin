use ansi_term::{Colour::Fixed, Colour::RGB, Style};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use zellij_tile::prelude::*;
use zellij_tile_utils::palette_match;

#[derive(Default)]
enum PluginStatus {
    #[default]
    Init,
    Hidden,
    Showing,
    Focused,
    Hiding,
}

#[derive(Default, Clone)]
struct JumpPane {
    title: String,
    id: u32,
    is_plugin: bool,
}

impl JumpPane {
    fn focus(&self) {
        if self.is_plugin {
            focus_plugin_pane(self.id, false);
        } else {
            focus_terminal_pane(self.id, false);
        }
    }
}

#[derive(Default)]
struct PluginState {
    status: PluginStatus,
    tab: usize,
    focus_floating: bool,
    prev_focus: Option<JumpPane>,
    panes: HashMap<String, JumpPane>,
    label_len: u8,
    label_input: String,
    label_alphabet: Vec<char>,
    dash_pane_id: u32,
    palette: Palette,
}

register_plugin!(PluginState);

impl PluginState {
    fn clear(&mut self) {
        self.label_input.clear();
        self.panes.clear();
        self.prev_focus.take();
    }
}

impl ZellijPlugin for PluginState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.dash_pane_id = get_plugin_ids().plugin_id;
        self.label_alphabet = configuration
            .get("label_alphabet")
            .map(|alphabet| alphabet.trim().to_lowercase())
            .unwrap_or("tnseriaoplfuwyzkbvm".to_string())
            // qwerty
            // .unwrap_or("fjdkslarueiwoqpcmx".to_string())
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
            EventType::TabUpdate,
            EventType::ModeUpdate,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(Key::Esc) => {
                if let Some(pane) = self.prev_focus.take() {
                    pane.focus();
                }
                self.status = PluginStatus::Hiding;
                self.clear();
                return true;
            }
            Event::Key(Key::Backspace) | Event::Key(Key::Delete) => {
                self.label_input.pop();
                return true;
            }
            Event::Key(Key::Char(c)) => {
                if self.label_len == 1 {
                    self.label_input = c.to_string();
                } else {
                    self.label_input.push(c);
                    self.label_input = self.label_input.trim().to_string();
                }

                if let Some(pane) = self.panes.get(&self.label_input) {
                    pane.focus();
                    self.prev_focus.take();
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
                self.status = if visible {
                    PluginStatus::Showing
                } else {
                    PluginStatus::Hidden
                };
                return true;
            }
            Event::TabUpdate(tabs) => {
                if let Some(tab) = tabs.get(self.tab) {
                    self.focus_floating = tab.are_floating_panes_visible;
                }
            }
            Event::PaneUpdate(PaneManifest { panes }) => {
                if let PluginStatus::Init = self.status {
                    match panes
                        .iter()
                        .find(|(_, panes)| panes.iter().any(|p| p.id == self.dash_pane_id))
                        .map(|(tab, _)| *tab)
                    {
                        Some(tab) => {
                            self.tab = tab;
                        }
                        None => {
                            return false;
                        }
                    }
                }

                if let Some(tab_panes) = panes.get(&self.tab) {
                    if let Some(focused) = tab_panes.iter().find(|p| {
                        p.is_focused
                            && p.is_floating == self.focus_floating
                            && p.id != self.dash_pane_id
                    }) {
                        self.prev_focus = Some(JumpPane {
                            id: focused.id,
                            title: focused.title.clone(),
                            is_plugin: focused.is_plugin,
                        });
                    }

                    match self.status {
                        PluginStatus::Hidden => return false,
                        PluginStatus::Showing => {
                            show_self(true);
                            self.status = PluginStatus::Focused;
                        }
                        _ => {}
                    }

                    self.panes.clear();

                    let visible_panes: Vec<_> = tab_panes
                        .iter()
                        .filter(|p| p.is_selectable && p.id != self.dash_pane_id)
                        .collect();
                    let label_len = if visible_panes.len() <= self.label_alphabet.len() {
                        1
                    } else {
                        2
                    };

                    let mut unprocessed_panes = Vec::new();
                    for pane in visible_panes {
                        let preferred_label = pane
                            .title
                            .chars()
                            .take(label_len)
                            .collect::<String>()
                            .to_lowercase();
                        if !self.panes.contains_key(&preferred_label)
                            && preferred_label
                                .chars()
                                .all(|c| self.label_alphabet.contains(&c))
                        {
                            self.panes.insert(
                                preferred_label,
                                JumpPane {
                                    id: pane.id,
                                    title: pane.title.clone(),
                                    is_plugin: pane.is_plugin,
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
                                            is_plugin: pane.is_plugin,
                                        },
                                    );
                                    break;
                                }
                            }
                        }
                    }

                    self.label_len = label_len as u8;
                    return true;
                }

                return false;
            }
            _ => unimplemented!("{event:?}"),
        };

        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        let padding = "   ";

        // input
        println!(
            "{padding}{}|",
            color_bold(self.palette.red, &self.label_input)
        );

        // title
        println!("{padding}{}\n", color_bold(self.palette.fg, "Editor"));

        // list
        for (label, pane) in self.panes.iter() {
            if self.prev_focus.as_ref().is_some_and(|p| p.id == pane.id) {
                continue;
            }

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
                    color_bold(self.palette.cyan, label)
                };
            println!("{padding}[{label}] {}", pane.title);
        }

        if let Some(pane) = &self.prev_focus {
            println!(
                "\n{padding}{} {}",
                color_bold(self.palette.red, "[ESC]"),
                pane.title
            );
        }
    }
}

fn color_bold(color: PaletteColor, text: &str) -> String {
    format!(
        "{}",
        Style::new().fg(palette_match!(color)).bold().paint(text)
    )
}
