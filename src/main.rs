use ansi_term::{Colour::Fixed, Colour::RGB, Style};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap, HashSet};
use zellij_tile::prelude::*;
use zellij_tile_utils::palette_match;

#[derive(Default)]
enum PluginStatus {
    #[default]
    Init,
    Hidden,
    Hiding,
    Focused,
}

#[derive(Default, Clone)]
struct DashPane {
    title: String,
    id: u32,
    is_plugin: bool,
    is_editor: bool,
}

impl DashPane {
    fn focus(&self) {
        if self.is_plugin {
            focus_plugin_pane(self.id, false);
        } else {
            focus_terminal_pane(self.id, false);
        }
    }

    fn hide(&self) {
        if self.is_plugin {
            hide_plugin_pane(self.id);
        } else {
            hide_terminal_pane(self.id);
        }
    }
}

#[derive(Default)]
struct PluginState {
    status: PluginStatus,
    tab: usize,
    focus_floating: bool,
    prev_focus: Option<DashPane>,
    last_editor_focused_pane_id: Option<u32>,
    dash_panes: HashMap<String, DashPane>,
    label_len: u8,
    label_input: String,
    label_alphabet: Vec<char>,
    dash_pane_id: u32,
    palette: Palette,
    columns: usize,
    rows: usize,
}

register_plugin!(PluginState);

impl PluginState {
    fn clear(&mut self) {
        self.label_input.clear();
    }

    fn map_pane(&self, pane: &PaneInfo) -> DashPane {
        DashPane {
            id: pane.id,
            title: pane.title.clone(),
            is_plugin: pane.is_plugin,
            is_editor: !pane.is_floating
                && pane.is_selectable
                && (pane.pane_x == 0 && pane.pane_columns > (self.columns / 2)
                    || pane.pane_y <= 2 && pane.pane_rows > (self.rows / 2)),
        }
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
            EventType::Key,
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ModeUpdate,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(Key::Esc) => {
                if let Some(pane) = &self.prev_focus {
                    pane.focus();
                }
                self.status = PluginStatus::Hiding;
                eprintln!("switching to Hiding state");
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

                if let Some(pane) = self.dash_panes.get(&self.label_input) {
                    pane.focus();
                    self.prev_focus = Some(pane.clone());
                    self.clear();
                    self.status = PluginStatus::Hiding;
                    eprintln!("switching to Hiding state");
                }

                return true;
            }
            Event::Key(_) => {}
            Event::ModeUpdate(ModeInfo { mode, style, .. }) => {
                self.palette = style.colors;
                eprintln!("mode update: {:?}", mode);
                return true;
            }
            Event::TabUpdate(tabs) => {
                if let Some(tab) = tabs.get(self.tab) {
                    self.focus_floating = tab.are_floating_panes_visible;
                    eprintln!("tab update: floating: {}", self.focus_floating);
                }
            }
            Event::PaneUpdate(PaneManifest { panes }) => {
                eprintln!("pane update - floating: {}", self.focus_floating);
                if self.columns == 0 {
                    return true;
                }

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
                        let dash_pane = self.map_pane(focused);
                        if dash_pane.is_editor {
                            self.last_editor_focused_pane_id = Some(dash_pane.id);
                        }
                        self.prev_focus = Some(dash_pane);
                    }

                    match self.status {
                        PluginStatus::Hidden => {
                            if let Some(dash_pane) = tab_panes
                                .iter()
                                .find(|p| p.id == self.dash_pane_id && p.is_plugin)
                            {
                                if dash_pane.is_focused {
                                    eprintln!("switching to focused state: {}, focused: {}, suppressed: {}", dash_pane.title, dash_pane.is_focused, dash_pane.is_suppressed);
                                    self.status = PluginStatus::Focused;
                                } else {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        PluginStatus::Hiding => {
                            let visible_editor_id = self
                                .prev_focus
                                .as_ref()
                                .and_then(|p| if p.is_editor { Some(p.id) } else { None })
                                .or_else(|| self.last_editor_focused_pane_id);
                            for p in self.dash_panes.values().filter(|p| {
                                p.is_editor
                                    && visible_editor_id.is_some_and(|editor_id| p.id != editor_id)
                            }) {
                                p.hide();
                            }
                            eprintln!("switching to hidden state");
                            self.status = PluginStatus::Hidden;
                            return false;
                        }
                        _ => {}
                    }

                    // todo: maybe exclude floating?
                    let visible_panes: Vec<_> = tab_panes
                        .iter()
                        .filter(|p| {
                            p.is_selectable
                                && p.id != self.dash_pane_id
                                && !p.title.ends_with("-bar")
                        })
                        .collect();
                    let label_len = if visible_panes.len() <= self.label_alphabet.len() {
                        1
                    } else {
                        2
                    };

                    let dash_pane_ids: HashSet<_> =
                        self.dash_panes.values().map(|p| p.id).collect();

                    let mut unlabeled_panes = Vec::new();
                    for pane in &visible_panes {
                        if dash_pane_ids.contains(&pane.id) {
                            continue;
                        }

                        eprintln!("Adding a dash pane: {}", pane.title);

                        let preferred_label = pane
                            .title
                            .chars()
                            .take(label_len)
                            .collect::<String>()
                            .to_lowercase();
                        if !self.dash_panes.contains_key(&preferred_label)
                            && preferred_label
                                .chars()
                                .all(|c| self.label_alphabet.contains(&c))
                        {
                            self.dash_panes.insert(preferred_label, self.map_pane(pane));
                        } else {
                            unlabeled_panes.push(pane);
                        }
                    }

                    if !unlabeled_panes.is_empty() {
                        let mut alpha = self.label_alphabet.iter().permutations(label_len);
                        for pane in unlabeled_panes {
                            while let Some(label) = alpha.next() {
                                let label = label.iter().cloned().collect();
                                if !self.dash_panes.contains_key(&label) {
                                    self.dash_panes.insert(label, self.map_pane(pane));
                                    break;
                                }
                            }
                        }
                    }

                    self.label_len = label_len as u8;

                    // cleanup closed panes
                    if self.dash_panes.len() > visible_panes.len() {
                        let visible_ids: HashSet<_> = visible_panes.iter().map(|p| p.id).collect();
                        self.dash_panes.retain(|_, p| visible_ids.contains(&p.id));
                    }

                    return true;
                }

                return false;
            }
            _ => unimplemented!("{event:?}"),
        };

        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.columns = cols;

        let padding = "   ";

        // input
        println!(
            "{padding}{}|",
            color_bold(self.palette.red, &self.label_input)
        );

        // title
        println!("{padding}{}\n", color_bold(self.palette.fg, "Editor"));

        // list
        for (label, pane) in self.dash_panes.iter() {
            if !pane.is_editor || self.prev_focus.as_ref().is_some_and(|p| p.id == pane.id) {
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
