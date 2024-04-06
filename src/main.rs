use ansi_term::{Colour::Fixed, Colour::RGB, Style};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap, HashSet};
use zellij_tile::prelude::*;
use zellij_tile_utils::palette_match;

// todo: move some PluginState fields into the variants
#[derive(Debug, Default, PartialEq)]
enum PluginStatus {
    #[default]
    Init,
    Editing,
    Dashing,
}

#[derive(Debug, Default, Clone)]
struct PaneFocus {
    id: u32,
    floating: bool,
    plugin: bool,
    editor: bool,
}

impl PaneFocus {
    fn focus(&self) {
        if self.plugin {
            focus_plugin_pane(self.id, false);
        } else {
            focus_terminal_pane(self.id, false);
        }
    }

    fn hide(&self) {
        if self.plugin {
            hide_plugin_pane(self.id);
        } else {
            hide_terminal_pane(self.id);
        }
    }
}

#[derive(Debug, Default, Clone)]
struct DashPane {
    title: String,
    id: u32,
    // todo: pane type enum
    plugin: bool,
    editor: bool,
}

impl DashPane {
    fn focus(&self) {
        if self.plugin {
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
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: PaneFocus,
    prev_focus: Option<PaneFocus>,
    last_focused_editor: Option<PaneFocus>,
    all_focused_panes: Vec<PaneInfo>,
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
    fn check_itialised(&mut self, panes: &HashMap<usize, Vec<PaneInfo>>) -> bool {
        if self.columns == 0 {
            return false;
        }

        if let PluginStatus::Init = self.status {
            match panes
                .iter()
                .find(|(_, panes)| panes.iter().any(|p| p.id == self.dash_pane_id))
                .map(|(tab, _)| *tab)
            {
                Some(tab) => {
                    self.tab = tab;
                    self.status = PluginStatus::Editing;
                }
                None => {
                    return false;
                }
            }
        }

        true
    }

    fn clear(&mut self) {
        self.label_input.clear();
    }

    fn is_editor_pane(&self, pane: &PaneInfo) -> bool {
        !pane.is_floating
            && pane.is_selectable
            && (pane.pane_x == 0 && pane.pane_columns > (self.columns / 2)
                || pane.pane_y <= 2 && pane.pane_rows > (self.rows / 2))
    }

    fn map_pane(&self, pane: &PaneInfo) -> DashPane {
        DashPane {
            id: pane.id,
            title: pane.title.clone(),
            plugin: pane.is_plugin,
            editor: self.is_editor_pane(pane),
        }
    }

    fn check_focus_change(&mut self) {
        if let Some(focused_pane) = self.has_focus_changed(&self.all_focused_panes) {
            self.on_focus_change(&focused_pane);
        }
    }

    fn has_focus_changed(&self, tab_panes: &[PaneInfo]) -> Option<PaneInfo> {
        tab_panes
            .iter()
            .find(|p| {
                p.is_focused
                    // both a tiled and a floating pane can be focused (but only the top one is relevant here)
                    && p.is_floating == self.floating
                    && (
                        // not the current focused pane or `last_focused_editor` has not been set yet
                        (self.current_focus.id != p.id
                            || self.current_focus.floating != p.is_floating)
                            || self.last_focused_editor.is_none())
            })
            .cloned()
    }

    fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        eprintln!("Focus change: {}", focused_pane.title);
        let editor = self.is_editor_pane(focused_pane);
        self.prev_focus = Some(std::mem::replace(
            &mut self.current_focus,
            PaneFocus {
                id: focused_pane.id,
                floating: self.floating,
                plugin: focused_pane.is_plugin,
                editor,
            },
        ));

        if let Some(focus) = &self.last_focused_editor {
            if self.current_focus.editor && focus.id != self.current_focus.id {
                eprintln!(
                    "Hide prev editor pane: {:?}, current_focus: {:?}, focused: {:?}",
                    focus, self.current_focus, focused_pane
                );
                focus.hide();
            } else {
                eprintln!(
                    "Keeping prev editor pane: {}, {:?}",
                    self.current_focus.editor, self.last_focused_editor
                );
            }
        }

        if self.current_focus.editor {
            self.last_focused_editor = Some(self.current_focus.clone());
        }

        if self.status != PluginStatus::Editing
            && self.current_focus.floating
            && self.current_focus.id == self.dash_pane_id
        {
            eprintln!("switching to Dashing state: {}", self.current_focus.id);
            self.status = PluginStatus::Dashing;
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
                self.status = PluginStatus::Editing;
                eprintln!("switching editing on dash cancel");
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
                    self.current_focus = PaneFocus {
                        id: pane.id,
                        floating: false,
                        plugin: pane.plugin,
                        editor: pane.editor,
                    };
                    self.clear();
                    self.status = PluginStatus::Editing;
                    eprintln!("switching to editing on dash");
                }

                return true;
            }
            Event::Key(_) => {}
            Event::ModeUpdate(ModeInfo { style, .. }) => {
                self.palette = style.colors;
                return true;
            }
            Event::TabUpdate(tabs) => {
                if let Some(tab) = tabs.get(self.tab) {
                    let floating = tab.are_floating_panes_visible;
                    if self.floating != floating {
                        self.floating = floating;
                        self.check_focus_change();
                        eprintln!(
                            "TabUpdate - floating changed: {}",
                            tab.are_floating_panes_visible
                        );
                    }
                }
            }
            Event::PaneUpdate(PaneManifest { panes }) => {
                if !self.check_itialised(&panes) {
                    eprintln!("Not init: bailing");
                    return false;
                }

                if let Some(tab_panes) = panes.get(&self.tab) {
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
                            let dash_pane = self.map_pane(pane);
                            eprintln!("new dash pane: {dash_pane:?}");
                            self.dash_panes.insert(preferred_label, dash_pane);
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

                    // collect all focused panes
                    // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top
                    self.all_focused_panes =
                        tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
                    self.check_focus_change();

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
        for (label, pane) in self.dash_panes.iter().filter(|(_, pane)| {
            pane.editor && (self.current_focus.floating || self.current_focus.id == pane.id)
        }) {
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
            // todo: fix
            // println!(
            //     "\n{padding}{} {}",
            //     color_bold(self.palette.red, "[ESC]"),
            //     pane.title
            // );
        }
    }
}

fn color_bold(color: PaletteColor, text: &str) -> String {
    format!(
        "{}",
        Style::new().fg(palette_match!(color)).bold().paint(text)
    )
}
