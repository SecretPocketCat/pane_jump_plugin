use crate::{utils::color_bold, PluginState, PluginStatus};

impl PluginState {
    pub(crate) fn should_render(&self) -> bool {
        matches!(self.status, PluginStatus::Dash { .. })
    }

    pub(crate) fn render_pane(&mut self, rows: usize, cols: usize) {
        self.set_rows(rows);
        self.set_columns(cols);

        if let PluginStatus::Dash { input } = &self.status {
            let padding = "   ";

            // input
            println!("{padding}{}|", color_bold(self.palette.red, &input));

            // title
            println!("{padding}{}\n", color_bold(self.palette.fg, "Editor"));

            // list
            for (pane, label) in self.dash_pane_label_pairs().iter().filter(|(pane, _)| {
                pane.editor && (self.current_focus.floating() || self.current_focus.id() == pane.id)
            }) {
                let label = if !input.trim().is_empty() && label.starts_with(input) {
                    format!(
                        "{}{}",
                        color_bold(self.palette.red, &input),
                        color_bold(
                            self.palette.green,
                            &label.chars().skip(input.len()).collect::<String>()
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
        } else {
            println!("Not in dash status!");
        }
    }
}
