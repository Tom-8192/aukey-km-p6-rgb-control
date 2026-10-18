//! Drawn mouse pad after Tom's product photo: long dark mat (about 2.6:1, rounded corners) with a thin lit edge
//! and a soft glow, the connector box on the top edge at the left with the lit logo and the cable going up.
//! Shapes only, no images. The animation uses the times measured on the pad (docs/PROTOCOL.md "Timing").

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, ecolor::Hsva};
use kmp6_core::protocol::BRIGHTNESS_CODES;
use kmp6_core::{Direction, Mode, Settings};

/// width : height of the mat
const ASPECT: f32 = 2.63;
const MAX_PAD_HEIGHT: f32 = 230.0;
/// seconds per breath, per speed level 0-4; levels 0, 2, 4 measured, 1 and 3 from the fitted curve
const BREATHE: [f64; 5] = [7.9, 4.6, 2.4, 2.05, 1.73];
/// seconds per round through the rainbow; levels 0 and 4 measured
const NEON: [f64; 5] = [24.0, 12.0, 6.0, 4.8, 4.0];
const WAVE: [f64; 5] = [8.0, 3.3, 1.5, 1.2, 1.0];
/// static + cycle: the colour changes about four times a second, the speed has no effect
const CYCLE_STEP: f64 = 0.25;
/// how far the logo runs ahead of the edge in the wave (a share of the rainbow; only seen as "out of step")
const WAVE_OFFSET: f64 = 0.18;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Zone {
    Edge,
    Logo,
}

/// Duration of one breath or rainbow round in seconds; `None` for effects without a speed.
fn period(mode: Mode, speed: u8) -> Option<f64> {
    let i = usize::from(speed).min(BREATHE.len() - 1);
    match mode {
        Mode::Breathe => Some(BREATHE[i]),
        Mode::Neon => Some(NEON[i]),
        Mode::Wave => Some(WAVE[i]),
        Mode::Static | Mode::Off => None,
    }
}

pub fn show(ui: &mut Ui, s: &Settings, time: f64) {
    let avail = ui.available_width();
    let pad_w = avail.min(MAX_PAD_HEIGHT * ASPECT) - 24.0;
    let pad_h = pad_w / ASPECT;
    // room above the mat for the box sticking out and the cable
    let top_room = 34.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail, pad_h + top_room + 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let pad =
        Rect::from_min_size(egui::pos2(rect.center().x - pad_w / 2.0, rect.top() + top_room), egui::vec2(pad_w, pad_h));
    let level = f32::from(BRIGHTNESS_CODES[usize::from(s.brightness)]) / 32.0;
    let lit = |zone| color_at(s, zone, time).map(|c| scale(c, level));
    let radius = CornerRadius::same((pad_w * 0.024).round().clamp(4.0, 30.0) as u8);

    // glow outside the edge, the mat, the thin lit edge
    if let Some(c) = lit(Zone::Edge) {
        // many thin rings with falling alpha, so the glow fades out smoothly
        for i in 1..=12 {
            let alpha = 0.16 * (1.0 - i as f32 / 13.0).powi(2);
            painter.rect_stroke(
                pad.expand(i as f32 * 1.4),
                radius,
                Stroke::new(1.6, c.gamma_multiply(alpha)),
                StrokeKind::Outside,
            );
        }
    }
    painter.rect_filled(pad, radius, Color32::from_rgb(0x15, 0x16, 0x18));
    let edge = lit(Zone::Edge).unwrap_or(Color32::from_rgb(0x2A, 0x2C, 0x30));
    painter.rect_stroke(pad, radius, Stroke::new(2.0, edge), StrokeKind::Inside);

    // connector box: about 5 % from the left, 6.4 % of the width wide, sticking out a little above the mat
    let box_w = (pad_w * 0.064).max(30.0);
    let box_h = box_w * 0.58;
    let box_rect =
        Rect::from_min_size(egui::pos2(pad.left() + pad_w * 0.052, pad.top() - box_h * 0.25), egui::vec2(box_w, box_h));
    let cable_x = box_rect.left() + box_w * 0.6;
    painter.line_segment(
        [egui::pos2(cable_x, rect.top()), egui::pos2(cable_x, box_rect.top())],
        Stroke::new(2.5, Color32::from_rgb(0x30, 0x32, 0x36)),
    );
    painter.rect(
        box_rect,
        CornerRadius::same(3),
        Color32::from_rgb(0x0B, 0x0C, 0x0D),
        Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3C, 0x40)),
        StrokeKind::Inside,
    );
    // logo LED: a small triangle in the middle of the box
    let c = box_rect.center();
    let r = box_h * 0.28;
    let tri: Vec<Pos2> = [-90.0f32, 30.0, 150.0]
        .iter()
        .map(|a| c + egui::vec2(a.to_radians().cos(), a.to_radians().sin()) * r)
        .collect();
    match lit(Zone::Logo) {
        Some(col) => {
            painter.circle_filled(c, r * 1.6, col.gamma_multiply(0.12));
            painter.add(Shape::closed_line(tri, Stroke::new(2.0, col)));
        }
        None => {
            painter.add(Shape::closed_line(tri, Stroke::new(1.5, Color32::from_rgb(0x30, 0x32, 0x36))));
        }
    }
}

/// Colour of a zone at time `t` (seconds); `None` = dark. Both zones share the colour, except the wave where
/// they run out of step.
fn color_at(s: &Settings, zone: Zone, t: f64) -> Option<Color32> {
    let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    let slot = |step: f64| {
        if s.cycle { s.colors[(step as usize) % s.colors.len()] } else { s.colors[usize::from(s.slot)] }
    };
    let period = period(s.mode, s.speed).unwrap_or(1.0);
    match s.mode {
        Mode::Off => None,
        Mode::Static => Some(rgb(slot(t / CYCLE_STEP))),
        Mode::Breathe => {
            // one colour per breath when cycling
            let k = 0.5 - 0.5 * ((t / period).fract() * std::f64::consts::TAU).cos();
            Some(scale(rgb(slot(t / period)), k as f32))
        }
        Mode::Neon => Some(hue(t / period)),
        Mode::Wave => {
            let sign = if s.direction == Direction::Left { 1.0 } else { -1.0 };
            let offset = if zone == Zone::Logo { WAVE_OFFSET * sign } else { 0.0 };
            Some(hue(t / period + offset))
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
        assert!(color_at(&off, Zone::Edge, 1.0).is_none());
        let s = Settings { cycle: false, slot: 2, ..Settings::default() };
        assert_eq!(color_at(&s, Zone::Logo, 5.0), Some(Color32::from_rgb(0, 0, 255)));
    }

    #[test]
    fn only_the_wave_puts_the_zones_out_of_step() {
        for mode in [Mode::Static, Mode::Breathe, Mode::Neon] {
            let s = Settings { mode, ..Settings::default() };
            assert_eq!(color_at(&s, Zone::Edge, 2.3), color_at(&s, Zone::Logo, 2.3), "{mode:?}");
        }
        let wave = Settings { mode: Mode::Wave, ..Settings::default() };
        assert_ne!(color_at(&wave, Zone::Edge, 2.3), color_at(&wave, Zone::Logo, 2.3));
    }

    #[test]
    fn faster_levels_are_shorter() {
        for mode in [Mode::Breathe, Mode::Neon, Mode::Wave] {
            let p: Vec<f64> = (0..5).map(|l| period(mode, l).unwrap()).collect();
            assert!(p.windows(2).all(|w| w[0] > w[1]), "{mode:?} {p:?}");
        }
        assert_eq!(period(Mode::Static, 3), None);
    }

    #[test]
    fn static_cycle_steps_four_times_a_second() {
        let s = Settings::default();
        assert_eq!(color_at(&s, Zone::Edge, 0.1), Some(Color32::from_rgb(255, 0, 0)));
        assert_eq!(color_at(&s, Zone::Edge, 0.3), Some(Color32::from_rgb(0, 255, 0)));
    }
}
