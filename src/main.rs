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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PaneId {
    Terminal(u32),
    Plugin(u32),
}

impl PaneId {
    fn new(id: u32, plugin: bool) -> Self {
        if plugin {
            Self::Plugin(id)
        } else {
            Self::Terminal(id)
        }
    }

    fn focus(&self) {
        match self {
            PaneId::Terminal(id) => focus_terminal_pane(*id, false),
            PaneId::Plugin(id) => focus_plugin_pane(*id, false),
        }
    }

    fn hide(&self) {
        match self {
            PaneId::Terminal(id) => hide_terminal_pane(*id),
            PaneId::Plugin(id) => hide_plugin_pane(*id),
        }
    }
}

impl From<&PaneInfo> for PaneId {
    fn from(pane: &PaneInfo) -> Self {
        Self::new(pane.id, pane.is_plugin)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PaneFocus {
    Tiled(PaneId),
    Floating(PaneId),
}

impl PaneFocus {
    fn new(id: impl Into<PaneId>, floating: bool) -> Self {
        if floating {
            Self::Floating(id.into())
        } else {
            Self::Tiled(id.into())
        }
    }

    fn id(&self) -> PaneId {
        match self {
            PaneFocus::Tiled(id) => id.clone(),
            PaneFocus::Floating(id) => id.clone(),
        }
    }

    fn floating(&self) -> bool {
        matches!(self, PaneFocus::Floating(_))
    }
}

impl From<&PaneInfo> for PaneFocus {
    fn from(pane: &PaneInfo) -> Self {
        Self::new(pane, pane.is_floating)
    }
}

#[derive(Debug, Clone)]
struct DashPane {
    title: String,
    id: PaneId,
    editor: bool,
}

struct PluginState {
    status: PluginStatus,
    tab: usize,
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: PaneFocus,
    prev_focus: Option<PaneFocus>,
    last_focused_editor: Option<PaneFocus>,
    all_focused_panes: Vec<PaneInfo>,
    dash_panes: HashMap<PaneId, DashPane>,
    dash_pane_labels: HashMap<String, PaneId>,
    label_len: u8,
    label_input: String,
    label_alphabet: Vec<char>,
    dash_pane_id: PaneId,
    palette: Palette,
    columns: usize,
    rows: usize,
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: PluginStatus::Init,
            tab: 0,
            floating: false,
            current_focus: PaneFocus::Tiled(PaneId::Terminal(0)),
            prev_focus: None,
            last_focused_editor: None,
            all_focused_panes: Default::default(),
            dash_panes: Default::default(),
            dash_pane_labels: Default::default(),
            label_len: 1,
            label_input: Default::default(),
            label_alphabet: Default::default(),
            dash_pane_id: PaneId::Plugin(0),
            palette: Default::default(),
            columns: 0,
            rows: 0,
        }
    }
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
                .find(|(_, panes)| panes.iter().any(|p| &PaneId::from(p) == &self.dash_pane_id))
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

    fn dash_pane_label_pairs(&self) -> Vec<(&DashPane, &str)> {
        self.dash_pane_labels
            .iter()
            .filter_map(|(label, id)| self.dash_panes.get(id).map(|p| (p, label.as_str())))
            .collect()
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
            id: pane.into(),
            title: pane.title.clone(),
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
                        self.current_focus != PaneFocus::from(*p) || self.last_focused_editor.is_none())
            })
            .cloned()
    }

    fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        // eprintln!("Focus change: {}", focused_pane.title);
        self.prev_focus = Some(std::mem::replace(
            &mut self.current_focus,
            focused_pane.into(),
        ));

        if let Some(last_focused_editor) = &self.last_focused_editor {
            if let Some(current_dash_pane) = self.dash_panes.get(&self.current_focus.id()) {
                if current_dash_pane.editor && last_focused_editor != &self.current_focus {
                    // eprintln!(
                    //     "Hide prev editor pane: {:?}, current_focus: {:?}, focused: {:?}",
                    //     current_dash_pane.title, self.current_focus, focused_pane
                    // );
                    last_focused_editor.id().hide();
                }
            } else {
                // eprintln!("Dash pane [{:?}] not found", last_focused_editor.id());
            }
        }

        if let Some(current_pane) = self.dash_panes.get(&self.current_focus.id()) {
            if current_pane.editor {
                self.last_focused_editor = Some(self.current_focus.clone());
            }
        } else {
            // eprintln!(
            //     "Currently focused Dash pane [{:?}] not found",
            //     self.current_focus
            // );
        }

        if self.status != PluginStatus::Editing
            && self.current_focus.floating()
            && self.current_focus.id() == self.dash_pane_id
        {
            // eprintln!("switching to Dashing state: {:?}", self.current_focus);
            self.status = PluginStatus::Dashing;
        }
    }
}

impl ZellijPlugin for PluginState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.dash_pane_id = PaneId::new(get_plugin_ids().plugin_id, true);
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
            PermissionType::RunCommands,
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
                    pane.id().focus();
                }
                self.status = PluginStatus::Editing;
                // eprintln!("switching editing on dash cancel");
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

                if let Some(pane) = self.dash_pane_labels.get(&self.label_input) {
                    pane.focus();
                    self.current_focus = PaneFocus::new(pane.clone(), false);
                    self.clear();
                    self.status = PluginStatus::Editing;
                    // eprintln!("switching to editing on dash");
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
                        // eprintln!(
                        //     "TabUpdate - floating changed: {}",
                        //     tab.are_floating_panes_visible
                        // );
                    }
                }
            }
            Event::PaneUpdate(PaneManifest { panes }) => {
                if !self.check_itialised(&panes) {
                    return false;
                }

                if let Some(tab_panes) = panes.get(&self.tab) {
                    // todo: maybe exclude floating?
                    let visible_panes: Vec<_> = tab_panes
                        .iter()
                        .filter(|p| {
                            p.is_selectable
                                && PaneId::from(*p) != self.dash_pane_id
                                && !p.title.ends_with("-bar")
                        })
                        .collect();
                    let label_len = if visible_panes.len() <= self.label_alphabet.len() {
                        1
                    } else {
                        2
                    };

                    let dash_pane_ids: HashSet<_> =
                        self.dash_panes.values().map(|p| p.id.clone()).collect();

                    let mut unlabeled_panes = Vec::new();
                    for pane in &visible_panes {
                        if dash_pane_ids.contains(&PaneId::from(*pane)) {
                            continue;
                        }

                        let preferred_label = pane
                            .title
                            .chars()
                            .take(label_len)
                            .collect::<String>()
                            .to_lowercase();
                        if !self.dash_pane_labels.contains_key(&preferred_label)
                            && preferred_label
                                .chars()
                                .all(|c| self.label_alphabet.contains(&c))
                        {
                            let dash_pane = self.map_pane(pane);
                            // eprintln!("new dash pane: {dash_pane:?}");
                            self.dash_pane_labels
                                .insert(preferred_label, dash_pane.id.clone());
                            self.dash_panes.insert(dash_pane.id.clone(), dash_pane);
                        } else {
                            unlabeled_panes.push(pane);
                        }
                    }

                    if !unlabeled_panes.is_empty() {
                        let mut alpha = self.label_alphabet.iter().permutations(label_len);
                        for pane in unlabeled_panes {
                            while let Some(label) = alpha.next() {
                                let label = label.iter().cloned().collect();
                                let dash_pane = self.map_pane(pane);
                                if !self.dash_pane_labels.contains_key(&label) {
                                    self.dash_pane_labels.insert(label, dash_pane.id.clone());
                                    self.dash_panes.insert(dash_pane.id.clone(), dash_pane);
                                    break;
                                }
                            }
                        }
                    }

                    self.label_len = label_len as u8;

                    // cleanup closed panes
                    let dash_panes_len = self.dash_panes.len();
                    if self.dash_panes.len() > visible_panes.len() {
                        let visible_ids: HashSet<_> =
                            visible_panes.iter().map(|p| PaneId::from(*p)).collect();
                        self.dash_panes.retain(|_, p| visible_ids.contains(&p.id));
                        let remaining_pane_ids: HashSet<_> = self.dash_panes.keys().collect();
                        self.dash_pane_labels
                            .retain(|_, id| remaining_pane_ids.get(id).is_some());
                    }

                    let new_dash_panes_len = self.dash_panes.len();
                    if new_dash_panes_len < dash_panes_len && new_dash_panes_len > 0 {
                        if !self.dash_panes.contains_key(&self.current_focus.id()) {
                            // focus editor pane if the focused pane was closed
                            if let Some(editor_pane) =
                                self.dash_panes.values().filter(|p| p.editor).next()
                            {
                                editor_pane.id.focus();
                            }
                        }
                    } else if self.dash_panes.values().filter(|p| p.editor).count() == 0 {
                        // open a new editor pane if all editor panes were closed
                        eprintln!("No more editors");
                        open_command_pane(CommandToRun {
                            path: "hx".into(),
                            args: vec![".".to_string()],
                            cwd: None,
                        })
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
        for (pane, label) in self.dash_pane_label_pairs().iter().filter(|(pane, _)| {
            pane.editor && (self.current_focus.floating() || self.current_focus.id() == pane.id)
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

        if let Some(focus) = &self.prev_focus {
            if let Some(pane) = self.dash_panes.get(&focus.id()) {
                println!(
                    "\n{padding}{} {}",
                    color_bold(self.palette.red, "[ESC]"),
                    pane.title
                );
            } else {
                // eprintln!("Prev focus pane [{:?}] not found", focus);
            }
        }
    }
}

fn color_bold(color: PaletteColor, text: &str) -> String {
    format!(
        "{}",
        Style::new().fg(palette_match!(color)).bold().paint(text)
    )
}
