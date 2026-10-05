//! Light or dark, and the seed color, from the app's settings, the
//! Omarchy theme and the system.

use iced::Color;
use serde::{Deserialize, Serialize};

use crate::omarchy;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// What the theme is chosen from.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    pub appearance: Appearance,
    /// Follow the desktop's accent (Omarchy's, else the system's).
    pub system_accent: bool,
    pub omarchy: Option<&'a omarchy::Palette>,
    pub system_accent_color: Option<Color>,
    pub system_dark: bool,
    /// The app's own color, or the one picked in its settings.
    pub chosen_accent: Color,
}

/// The seed color and whether the scheme is dark. With the system accent
/// on, the Omarchy theme's accent is the seed, else the system's own, else
/// the chosen one; in "Follow system" the Omarchy theme's own mode wins
/// over the system's.
pub fn choose(inputs: Inputs<'_>) -> (Color, bool) {
    let omarchy = inputs.omarchy.filter(|_| inputs.system_accent);
    let dark = match (inputs.appearance, omarchy) {
        (Appearance::System, Some(palette)) => palette.mode == omarchy::Mode::Dark,
        (Appearance::System, None) => inputs.system_dark,
        (Appearance::Light, _) => false,
        (Appearance::Dark, _) => true,
    };
    let seed = match omarchy {
        Some(palette) => {
            let accent = palette.accent;
            Color::from_rgb8(accent.red, accent.green, accent.blue)
        }
        None => inputs
            .system_accent_color
            .filter(|_| inputs.system_accent)
            .unwrap_or(inputs.chosen_accent),
    };
    (seed, dark)
}

/// "#RRGGBB" (or "RRGGBB") as a color.
pub fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::from_rgb8(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}

pub fn color_to_hex(color: Color) -> String {
    let [red, green, blue, _] = color.into_rgba8();
    format!("#{red:02X}{green:02X}{blue:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let color = hex_to_color("#2F6FDE").unwrap();
        assert_eq!(color_to_hex(color), "#2F6FDE");
        assert_eq!(hex_to_color("12345"), None);
    }

    #[test]
    fn fixed_appearance_ignores_the_system() {
        let inputs = Inputs {
            appearance: Appearance::Dark,
            system_accent: false,
            omarchy: None,
            system_accent_color: Some(Color::WHITE),
            system_dark: false,
            chosen_accent: Color::BLACK,
        };
        assert_eq!(choose(inputs), (Color::BLACK, true));
    }
}
