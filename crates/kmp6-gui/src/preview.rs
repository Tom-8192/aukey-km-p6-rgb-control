//! Drawn mouse pad with its two light zones - the lit edge and the logo on the connector box (top left) -
//! animated roughly like the effect. Only an impression: the timing of the real pad is not measured.

use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, ecolor::Hsva};
use kmp6_core::protocol::{BRIGHTNESS_CODES, SPEED_CODES};
use kmp6_core::{Direction, Mode, Settings};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Zone {
    Edge,
    Logo,
}

pub fn show(ui: &mut Ui, s: &Settings, time: f64) {
    let width = ui.available_width();
    let height = (width * 0.26).clamp(90.0, 170.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect.expand(2.0));
    let pad = Rect::from_min_max(rect.min + egui::vec2(12.0, 22.0), rect.max - egui::vec2(12.0, 10.0));
    let dark = ui.visuals().dark_mode;
    let surface = if dark { Color32::from_rgb(0x0E, 0x10, 0x14) } else { Color32::from_rgb(0x26, 0x29, 0x30) };
    let level = f32::from(BRIGHTNESS_CODES[usize::from(s.brightness)]) / 32.0;
    let speed = f64::from(SPEED_CODES[usize::from(s.speed)]);
    let lit = |zone| color_at(s, zone, time, speed).map(|c| scale(c, level));

    // pad with the glowing edge
    painter.rect_filled(pad, CornerRadius::same(14), surface);
    if let Some(c) = lit(Zone::Edge) {
        painter.rect_stroke(pad, CornerRadius::same(14), Stroke::new(10.0, c.gamma_multiply(0.16)), StrokeKind::Middle);
        painter.rect_stroke(pad, CornerRadius::same(14), Stroke::new(3.5, c), StrokeKind::Middle);
    }
    // connector box with the logo, sitting on the top left corner
    let box_rect = Rect::from_min_size(pad.min + egui::vec2(18.0, -14.0), egui::vec2(64.0, 26.0));
    let box_fill = if dark { Color32::from_rgb(0x1C, 0x1F, 0x26) } else { Color32::from_rgb(0x33, 0x37, 0x40) };
    painter.rect_filled(box_rect, CornerRadius::same(6), box_fill);
    let logo = Rect::from_center_size(box_rect.center(), egui::vec2(30.0, 8.0));
    match lit(Zone::Logo) {
        Some(c) => {
            painter.rect_filled(logo.expand(4.0), CornerRadius::same(6), c.gamma_multiply(0.2));
            painter.rect_filled(logo, CornerRadius::same(3), c);
        }
        None => {
            painter.rect_filled(logo, CornerRadius::same(3), surface);
        }
    }
}

/// Colour of a zone at time `t` (seconds); `speed` is the device value 0-5. `None` = dark.
/// Seen on the pad: both zones share the colour, except the wave where they run out of step;
/// the wave is faster than neon.
fn color_at(s: &Settings, zone: Zone, t: f64, speed: f64) -> Option<Color32> {
    let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    let cycled = |step: f64| {
        if s.cycle { s.colors[(step as usize) % s.colors.len()] } else { s.colors[usize::from(s.slot)] }
    };
    match s.mode {
        Mode::Off => None,
        Mode::Static => Some(rgb(cycled(t / 1.5))),
        Mode::Breathe => {
            let period = 4.5 / (0.6 + speed * 0.5);
            let k = 0.5 - 0.5 * ((t / period).fract() * std::f64::consts::TAU).cos();
            Some(scale(rgb(cycled(t / period)), k as f32))
        }
        Mode::Neon => Some(hue(t * (0.02 + speed * 0.02))),
        Mode::Wave => {
            let sign = if s.direction == Direction::Left { 1.0 } else { -1.0 };
            let offset = if zone == Zone::Logo { 0.18 * sign } else { 0.0 };
            Some(hue(t * (0.06 + speed * 0.06) + offset))
        }
    }
}

fn hue(h: f64) -> Color32 {
    Hsva::new(h.rem_euclid(1.0) as f32, 1.0, 1.0, 1.0).into()
}

fn scale(c: Color32, k: f32) -> Color32 {
    let f = |v: u8| (f32::from(v) * k.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_is_dark_and_static_shows_the_slot() {
        let off = Settings { mode: Mode::Off, ..Settings::default() };
        assert!(color_at(&off, Zone::Edge, 1.0, 5.0).is_none());
        let s = Settings { cycle: false, slot: 2, ..Settings::default() };
        assert_eq!(color_at(&s, Zone::Logo, 5.0, 0.0), Some(Color32::from_rgb(0, 0, 255)));
    }

    #[test]
    fn only_the_wave_puts_the_zones_out_of_step() {
        for mode in [Mode::Static, Mode::Breathe, Mode::Neon] {
            let s = Settings { mode, ..Settings::default() };
            assert_eq!(color_at(&s, Zone::Edge, 2.3, 3.0), color_at(&s, Zone::Logo, 2.3, 3.0), "{mode:?}");
        }
        let wave = Settings { mode: Mode::Wave, ..Settings::default() };
        assert_ne!(color_at(&wave, Zone::Edge, 2.3, 3.0), color_at(&wave, Zone::Logo, 2.3, 3.0));
    }
}
