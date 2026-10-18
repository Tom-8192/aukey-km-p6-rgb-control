//! Lighting settings and the feature reports that carry them (see `docs/PROTOCOL.md`).
//!
//! The pad only listens: every report is 8 data bytes on the vendor interface, and it cannot be read back.
//! The packet order is the one of G-aim Control Center 1.1.5 (`LightControl_mousePad.qml`, `saveData`).

use serde::{Deserialize, Serialize};

use crate::transport::REPORT_LEN;
use crate::{Result, invalid, tr};

pub type Rgb = [u8; 3];
pub type Packet = [u8; REPORT_LEN];

/// colour slots of the pad
pub const SLOTS: usize = 7;
/// effect, brightness, speed, colour slot / direction, colour of slot 1 (G-aim `hidMsPad_08H_SetLedType`)
pub const CMD_EFFECT: u8 = 0x08;
/// colours of slots 2+3, 4+5, 6+7 (G-aim `hidMsPad_SetLedColor`)
pub const CMD_COLORS: [u8; 3] = [0x09, 0x0A, 0x0B];
/// colour byte "cycle through all slots" instead of `slot << 4`
pub const CYCLE: u8 = 0x70;
/// device values of the 10 brightness levels of G-aim (0-9)
pub const BRIGHTNESS_CODES: [u8; 10] = [0, 3, 7, 10, 14, 17, 21, 24, 28, 32];
/// device values of the 5 speed levels of G-aim (0-4)
pub const SPEED_CODES: [u8; 5] = [0, 1, 3, 4, 5];
pub const MAX_BRIGHTNESS: u8 = BRIGHTNESS_CODES.len() as u8 - 1;
pub const MAX_SPEED: u8 = SPEED_CODES.len() as u8 - 1;
/// speed level the button on the pad uses for wave, breathe and neon, whatever was sent before (docs/PROTOCOL.md "Button")
pub const BUTTON_SPEED: u8 = 2;
/// factory colours of G-aim (`setting/pD.bin`): red, green, blue, yellow, cyan, magenta, white
pub const FACTORY_COLORS: [Rgb; SLOTS] =
    [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0], [0, 255, 255], [255, 0, 255], [255, 255, 255]];

/// Effects in the order of the G-aim buttons; the device code is the index (off is sent as static with brightness 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Static,
    Breathe,
    Neon,
    Wave,
    Off,
}

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Static, Mode::Breathe, Mode::Neon, Mode::Wave, Mode::Off];

    /// key under `mode.*` in the language files
    pub fn id(self) -> &'static str {
        match self {
            Mode::Static => "static",
            Mode::Breathe => "breathe",
            Mode::Neon => "neon",
            Mode::Wave => "wave",
            Mode::Off => "off",
        }
    }

    pub fn uses_brightness(self) -> bool {
        self != Mode::Off
    }

    /// G-aim sends speed 0 for static
    pub fn uses_speed(self) -> bool {
        matches!(self, Mode::Breathe | Mode::Neon | Mode::Wave)
    }

    pub fn uses_direction(self) -> bool {
        self == Mode::Wave
    }

    /// neon and wave run through their own colours; G-aim greys out the palette for them
    pub fn uses_colors(self) -> bool {
        matches!(self, Mode::Static | Mode::Breathe)
    }
}

/// Direction of the wave; the device code is the value (G-aim: left button = 1, right button = 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Right = 0,
    Left = 1,
}

impl Direction {
    pub const ALL: [Direction; 2] = [Direction::Left, Direction::Right];

    pub fn id(self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }
}

/// Everything the pad shows. Levels are those of G-aim (brightness 0-9, speed 0-4), slots count from 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub brightness: u8,
    pub speed: u8,
    pub direction: Direction,
    /// cycle through all seven colours instead of showing `slot`
    pub cycle: bool,
    pub slot: u8,
    #[serde(with = "hex_colors")]
    pub colors: [Rgb; SLOTS],
}

/// Factory state of G-aim (`setting/pD.bin`, section `[mousePad]`: `lightValue=0,9,4,1`, `isCycle=true`).
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Static,
            brightness: MAX_BRIGHTNESS,
            speed: MAX_SPEED,
            direction: Direction::Left,
            cycle: true,
            slot: 0,
            colors: FACTORY_COLORS,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.brightness > MAX_BRIGHTNESS {
            return invalid(tr!("error.brightness_range", max = MAX_BRIGHTNESS));
        }
        if self.speed > MAX_SPEED {
            return invalid(tr!("error.speed_range", max = MAX_SPEED));
        }
        if usize::from(self.slot) >= SLOTS {
            return invalid(tr!("error.slot_range", max = SLOTS));
        }
        Ok(())
    }

    /// colour byte of the effect packet: slot in the high nibble or "cycle"
    pub fn color_byte(&self) -> u8 {
        if self.cycle { CYCLE } else { self.slot << 4 }
    }

    /// Reports in the order G-aim sends them. Static and breathe are sent twice: G-aim notes that the pad only
    /// takes the colours over "on the next time" (`//再下一次才會改變`).
    pub fn packets(&self) -> Result<Vec<Packet>> {
        self.validate()?;
        let bright = BRIGHTNESS_CODES[usize::from(self.brightness)];
        let speed = SPEED_CODES[usize::from(self.speed)];
        let dir = self.direction as u8;
        let [r, g, b] = self.colors[0];
        let colors = || {
            CMD_COLORS.iter().enumerate().map(|(i, &cmd)| {
                let [a, c] = [self.colors[1 + 2 * i], self.colors[2 + 2 * i]];
                [cmd, a[0], a[1], a[2], c[0], c[1], c[2], 0]
            })
        };
        Ok(match self.mode {
            Mode::Static | Mode::Breathe => {
                let (mode, speed) = if self.mode == Mode::Static { (0, 0) } else { (1, speed) };
                let effect = [CMD_EFFECT, mode, bright, speed, self.color_byte(), r, g, b];
                let once: Vec<Packet> = colors().chain([effect]).collect();
                once.repeat(2)
            }
            Mode::Neon => vec![[CMD_EFFECT, 2, bright, speed, 0, 0, 0, 0]],
            Mode::Wave => vec![[CMD_EFFECT, 3, bright, speed, dir, 0, 0, 0]],
            Mode::Off => vec![[CMD_EFFECT, 0, 0, speed, dir, 0, 0, 0]],
        })
    }

    /// colour the pad shows (first slot when cycling)
    pub fn shown_color(&self) -> Rgb {
        if self.cycle { self.colors[0] } else { self.colors[usize::from(self.slot)] }
    }

    /// What the pad shows after one press of its button, starting from this setting (docs/PROTOCOL.md "Button"):
    /// static slot n -> slot n+1, after slot 7 or "cycle" the wave, then breathe through all colours, neon and
    /// static slot 1 again. "Off" is static slot 1 with brightness 0 for the pad, so it goes on with slot 2.
    /// The button always sets full brightness and its own speed; colours and direction stay.
    pub fn after_button(&self) -> Settings {
        let next = |mode, slot, cycle| Settings {
            mode,
            slot,
            cycle,
            brightness: MAX_BRIGHTNESS,
            speed: if mode == Mode::Static { self.speed } else { BUTTON_SPEED },
            ..self.clone()
        };
        let last = SLOTS as u8 - 1;
        match self.mode {
            Mode::Off => next(Mode::Static, 1, false),
            Mode::Static if self.cycle || self.slot >= last => next(Mode::Wave, self.slot, self.cycle),
            Mode::Static => next(Mode::Static, self.slot + 1, false),
            Mode::Wave => next(Mode::Breathe, self.slot, true),
            Mode::Breathe => next(Mode::Neon, self.slot, self.cycle),
            Mode::Neon => next(Mode::Static, 0, false),
        }
    }
}

/// Colours as "#RRGGBB" in the settings file, so it stays readable.
mod hex_colors {
    use serde::{Deserialize, Deserializer, Serializer, de::Error, ser::SerializeSeq};

    use super::{Rgb, SLOTS};

    pub fn serialize<S: Serializer>(colors: &[Rgb; SLOTS], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(SLOTS))?;
        for c in colors {
            seq.serialize_element(&crate::names::format_hex(*c))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[Rgb; SLOTS], D::Error> {
        let texts = Vec::<String>::deserialize(d)?;
        if texts.len() != SLOTS {
            return Err(D::Error::custom(format!("{SLOTS} colours expected, found {}", texts.len())));
        }
        let mut out = [[0; 3]; SLOTS];
        for (slot, text) in out.iter_mut().zip(&texts) {
            *slot = crate::names::parse_hex(text).ok_or_else(|| D::Error::custom(format!("invalid colour {text}")))?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;

    fn lines(s: &Settings) -> Vec<String> {
        s.packets().unwrap().iter().map(|p| hex(p)).collect()
    }

    /// G-aim factory state: static, brightness 9 (= 32), cycle -> 0x70, speed 0 for static, everything twice.
    #[test]
    fn factory_static() {
        let once = [
            "09 00 ff 00 00 00 ff 00",
            "0a ff ff 00 00 ff ff 00",
            "0b ff 00 ff ff ff ff 00",
            "08 00 20 00 70 ff 00 00",
        ];
        assert_eq!(lines(&Settings::default()), [once, once].concat());
    }

    #[test]
    fn breathe_uses_speed_and_slot() {
        let s = Settings { mode: Mode::Breathe, brightness: 4, speed: 2, cycle: false, slot: 3, ..Settings::default() };
        let p = lines(&s);
        assert_eq!(p.len(), 8);
        assert_eq!(p[3], "08 01 0e 03 30 ff 00 00");
        assert_eq!(p[3], p[7]);
    }

    #[test]
    fn neon_wave_off() {
        let base = Settings { brightness: 9, speed: 4, ..Settings::default() };
        let neon = Settings { mode: Mode::Neon, ..base.clone() };
        assert_eq!(lines(&neon), ["08 02 20 05 00 00 00 00"]);
        let wave = Settings { mode: Mode::Wave, direction: Direction::Right, speed: 1, ..base.clone() };
        assert_eq!(lines(&wave), ["08 03 20 01 00 00 00 00"]);
        // G-aim "off": static with brightness 0, speed and direction passed through
        let off = Settings { mode: Mode::Off, ..base };
        assert_eq!(lines(&off), ["08 00 00 05 01 00 00 00"]);
    }

    /// The sequence Tom watched on the pad (2026-10-18).
    #[test]
    fn button_sequence() {
        let dim_blue = Settings { brightness: 3, cycle: false, slot: 2, ..Settings::default() };
        let s = dim_blue.after_button();
        assert_eq!((s.mode, s.slot, s.cycle, s.brightness), (Mode::Static, 3, false, MAX_BRIGHTNESS));
        let mut s = s;
        for _ in 0..3 {
            s = s.after_button();
        }
        assert_eq!((s.mode, s.slot), (Mode::Static, 6));
        let wave = s.after_button();
        assert_eq!((wave.mode, wave.speed), (Mode::Wave, BUTTON_SPEED));
        let breathe = wave.after_button();
        assert_eq!((breathe.mode, breathe.cycle, breathe.speed), (Mode::Breathe, true, BUTTON_SPEED));
        let neon = breathe.after_button();
        assert_eq!((neon.mode, neon.speed), (Mode::Neon, BUTTON_SPEED));
        let first = neon.after_button();
        assert_eq!((first.mode, first.slot, first.cycle), (Mode::Static, 0, false));
        // static cycling counts as the last static step; off goes on with slot 2
        assert_eq!(Settings::default().after_button().mode, Mode::Wave);
        let off = Settings { mode: Mode::Off, brightness: 0, ..Settings::default() }.after_button();
        assert_eq!((off.mode, off.slot, off.brightness), (Mode::Static, 1, MAX_BRIGHTNESS));
    }

    #[test]
    fn ranges_are_checked() {
        assert!(Settings { brightness: 10, ..Settings::default() }.packets().is_err());
        assert!(Settings { speed: 5, ..Settings::default() }.packets().is_err());
        assert!(Settings { slot: 7, ..Settings::default() }.packets().is_err());
    }

    #[test]
    fn json_round_trip() {
        let s = Settings { mode: Mode::Wave, colors: [[1, 2, 3]; SLOTS], ..Settings::default() };
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains("\"#010203\"") && text.contains("\"wave\""));
        assert_eq!(serde_json::from_str::<Settings>(&text).unwrap(), s);
        // missing fields fall back to the factory values
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap(), Settings::default());
    }
}
