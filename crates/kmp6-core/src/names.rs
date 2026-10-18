//! Names that are shown and typed: effects, directions, colours. Display names come from `locales/`, input is
//! recognised in every language (with and without umlauts).

use crate::i18n;
use crate::protocol::{Direction, Mode, Rgb, SLOTS};
use crate::{Result, invalid, tr};

/// Named colours (`color.<id>` in the language files); the first seven are the factory slots.
pub const COLORS: [(&str, Rgb); 12] = [
    ("red", [255, 0, 0]),
    ("green", [0, 255, 0]),
    ("blue", [0, 0, 255]),
    ("yellow", [255, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("white", [255, 255, 255]),
    ("orange", [255, 128, 0]),
    ("purple", [128, 0, 255]),
    ("pink", [255, 64, 160]),
    ("warm-white", [255, 180, 100]),
    ("black", [0, 0, 0]),
];

pub fn mode_name(mode: Mode) -> String {
    i18n::name(&format!("mode.{}", mode.id()))
}

pub fn direction_name(dir: Direction) -> String {
    i18n::name(&format!("direction.{}", dir.id()))
}

pub fn parse_mode(text: &str) -> Result<Mode> {
    Mode::ALL
        .into_iter()
        .find(|m| i18n::matches(&format!("mode.{}", m.id()), text))
        .map_or_else(|| invalid(tr!("error.expected_one_of", names = Mode::ALL.map(mode_name).join(", "))), Ok)
}

pub fn parse_direction(text: &str) -> Result<Direction> {
    Direction::ALL.into_iter().find(|d| i18n::matches(&format!("direction.{}", d.id()), text)).map_or_else(
        || invalid(tr!("error.expected_one_of", names = Direction::ALL.map(direction_name).join(", "))),
        Ok,
    )
}

/// Colour slot as typed: "1"-"7", or the word for "cycle" -> `None`.
pub fn parse_slot(text: &str) -> Result<Option<u8>> {
    if i18n::matches("word.cycle", text) {
        return Ok(None);
    }
    match text.trim().parse::<u8>() {
        Ok(n) if (1..=SLOTS as u8).contains(&n) => Ok(Some(n - 1)),
        _ => invalid(tr!("error.slot_input", max = SLOTS, cycle = i18n::name("word.cycle"))),
    }
}

/// "#RRGGBB" / "RRGGBB" / "#RGB" -> colour.
pub fn parse_hex(text: &str) -> Option<Rgb> {
    let t = text.trim().trim_start_matches('#');
    let digits: Vec<u8> = t.chars().map(|c| c.to_digit(16).map(|d| d as u8)).collect::<Option<_>>()?;
    match digits.as_slice() {
        [r, g, b] => Some([r * 17, g * 17, b * 17]),
        [r1, r2, g1, g2, b1, b2] => Some([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2]),
        _ => None,
    }
}

pub fn format_hex(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// Colour as typed: a name ("rot", "red"), hex ("#FF8000", "f80") or "r,g,b" ("255,128,0").
pub fn parse_color(text: &str) -> Result<Rgb> {
    if let Some((_, rgb)) = COLORS.iter().find(|(id, _)| i18n::matches(&format!("color.{id}"), text)) {
        return Ok(*rgb);
    }
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    if parts.len() == 3 {
        if let [Ok(r), Ok(g), Ok(b)] = [0, 1, 2].map(|i| parts[i].parse::<u8>()) {
            return Ok([r, g, b]);
        }
    } else if let Some(rgb) = parse_hex(text) {
        return Ok(rgb);
    }
    invalid(tr!("error.color_input", text = text.trim(), names = color_names().join(", ")))
}

/// Display names of all named colours.
pub fn color_names() -> Vec<String> {
    COLORS.iter().map(|(id, _)| i18n::name(&format!("color.{id}"))).collect()
}

/// "#FF0000 (red)" if the colour has a name, otherwise only the hex value.
pub fn format_color(rgb: Rgb) -> String {
    match COLORS.iter().find(|(_, c)| *c == rgb) {
        Some((id, _)) => format!("{} ({})", format_hex(rgb), i18n::name(&format!("color.{id}"))),
        None => format_hex(rgb),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_in_all_forms() {
        assert_eq!(parse_color("red").unwrap(), [255, 0, 0]);
        assert_eq!(parse_color("Grün").unwrap(), [0, 255, 0]);
        assert_eq!(parse_color("gruen").unwrap(), [0, 255, 0]);
        assert_eq!(parse_color("#ff8000").unwrap(), [255, 128, 0]);
        assert_eq!(parse_color("F80").unwrap(), [255, 136, 0]);
        assert_eq!(parse_color("10, 20,30").unwrap(), [10, 20, 30]);
        assert!(parse_color("300,0,0").is_err());
        assert!(parse_color("banane").is_err());
        assert_eq!(format_hex([1, 171, 255]), "#01ABFF");
    }

    #[test]
    fn modes_directions_slots() {
        assert_eq!(parse_mode("atmen").unwrap(), Mode::Breathe);
        assert_eq!(parse_mode("Wave").unwrap(), Mode::Wave);
        assert_eq!(parse_mode("aus").unwrap(), Mode::Off);
        // the prototype called the wave "Regenbogen"
        assert_eq!(parse_mode("regenbogen").unwrap(), Mode::Wave);
        assert!(parse_mode("disco").is_err());
        assert_eq!(parse_direction("links").unwrap(), Direction::Left);
        assert_eq!(parse_direction("right").unwrap(), Direction::Right);
        assert_eq!(parse_slot("1").unwrap(), Some(0));
        assert_eq!(parse_slot("7").unwrap(), Some(6));
        assert_eq!(parse_slot("zyklus").unwrap(), None);
        assert!(parse_slot("8").is_err());
    }
}
