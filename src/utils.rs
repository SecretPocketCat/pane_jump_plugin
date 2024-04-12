use ansi_term::{Color::Fixed, Color::RGB, Style};
use zellij_tile::prelude::PaletteColor;
use zellij_tile_utils::palette_match;

pub(crate) fn color_bold(color: PaletteColor, text: &str) -> String {
    format!(
        "{}",
        Style::new().fg(palette_match!(color)).bold().paint(text)
    )
}
