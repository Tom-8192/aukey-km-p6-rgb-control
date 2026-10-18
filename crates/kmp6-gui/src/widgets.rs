//! Small building blocks of the window: cards, segmented buttons, colour swatches.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Response, RichText, Sense, Stroke, Ui};

use crate::theme::{self, ACCENT, CARD_RADIUS, DANGER};

/// Framed card that fills the available width.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.card_stroke))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Card title with icon and an optional line of explanation.
pub fn card_title(ui: &mut Ui, icon: &str, title: &str, subtitle: Option<&str>) {
    card_title_in(ui, icon, ACCENT, title, subtitle);
}

/// `card_title` with another icon colour (e.g. red for actions that delete data).
pub fn card_title_in(ui: &mut Ui, icon: &str, color: Color32, title: &str, subtitle: Option<&str>) {
    let p = theme::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(18.0).color(color));
        ui.label(RichText::new(title).size(16.0).strong().color(p.strong));
    });
    if let Some(s) = subtitle {
        ui.label(RichText::new(s).color(p.muted));
    }
    ui.add_space(4.0);
}

pub fn muted(ui: &mut Ui, text: impl Into<String>) -> Response {
    let p = theme::palette(ui.ctx());
    ui.label(RichText::new(text.into()).color(p.muted))
}

/// Row of connected toggle buttons; returns true when the user picked another option.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, options: &[(T, String)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let last = options.len().saturating_sub(1);
        for (i, (value, label)) in options.iter().enumerate() {
            let selected = *current == *value;
            let r = theme::RADIUS;
            let radius = CornerRadius {
                nw: if i == 0 { r } else { 0 },
                sw: if i == 0 { r } else { 0 },
                ne: if i == last { r } else { 0 },
                se: if i == last { r } else { 0 },
            };
            let text = if selected { RichText::new(label).color(Color32::WHITE) } else { RichText::new(label) };
            let mut button = egui::Button::new(text).corner_radius(radius).min_size(egui::vec2(0.0, 28.0));
            if selected {
                button = button.fill(ACCENT).stroke(Stroke::new(1.0, ACCENT));
            }
            if ui.add(button).clicked() && !selected {
                *current = *value;
                changed = true;
            }
        }
    });
    changed
}

/// Round colour field; a ring marks the selected one.
pub fn swatch(ui: &mut Ui, color: Color32, selected: bool, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    let painter = ui.painter();
    let c = rect.center();
    let radius = size / 2.0 - 3.0;
    let ring = if selected {
        Stroke::new(2.0, ACCENT)
    } else if response.hovered() {
        Stroke::new(1.5, ui.visuals().widgets.hovered.bg_stroke.color)
    } else {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    painter.circle(c, radius, color, Stroke::NONE);
    painter.circle_stroke(c, radius + 2.0, ring);
    if color == Color32::BLACK {
        // "off": draw a slash so it does not look like an empty hole
        let d = radius * 0.6;
        painter.line_segment([c + egui::vec2(-d, d), c + egui::vec2(d, -d)], Stroke::new(1.5, DANGER));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Button with an icon in front of the text.
pub fn icon_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    ui.button(format!("{icon}  {text}"))
}

/// Large selectable tile with an icon above the label (effect choice).
pub fn mode_tile(ui: &mut Ui, glyph: &str, label: &str, selected: bool, width: f32) -> Response {
    let pal = theme::palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 68.0), Sense::click());
    let (fill, stroke) = if selected {
        (pal.accent_soft, Stroke::new(1.5, ACCENT))
    } else if resp.hovered() {
        (ui.visuals().widgets.hovered.weak_bg_fill, Stroke::new(1.0, pal.card_stroke))
    } else {
        (ui.visuals().widgets.inactive.weak_bg_fill, Stroke::new(1.0, pal.card_stroke))
    };
    ui.painter().rect(rect, CornerRadius::same(8), fill, stroke, egui::StrokeKind::Inside);
    let color = if selected { ACCENT } else { pal.strong };
    let painter = ui.painter();
    painter.text(
        rect.center() - egui::vec2(0.0, 11.0),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(22.0),
        color,
    );
    painter.text(
        rect.center() + egui::vec2(0.0, 16.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.5),
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// "warm-white" -> "Warm white" for names that come from command-line words.
pub fn nice(name: &str) -> String {
    let name = name.replace('-', " ");
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// Left column of fixed width next to a right column that takes the rest; stacked when the window is narrow.
pub fn two_columns<T>(
    ui: &mut Ui,
    state: &mut T,
    left_width: f32,
    left: impl FnOnce(&mut Ui, &mut T),
    right: impl FnOnce(&mut Ui, &mut T),
) {
    if ui.available_width() < left_width + 320.0 {
        left(ui, state);
        ui.add_space(4.0);
        right(ui, state);
        return;
    }
    // separate child areas instead of a horizontal layout, which would limit the height of both columns
    let gap = ui.spacing().item_spacing.x;
    let area = ui.available_rect_before_wrap();
    let height = area.height().max(400.0);
    let layout = egui::Layout::top_down(egui::Align::Min);
    let left_rect = egui::Rect::from_min_size(area.min, egui::vec2(left_width, height));
    let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_rect).layout(layout));
    left(&mut left_ui, state);
    let right_min = egui::pos2(area.min.x + left_width + gap, area.min.y);
    let right_rect = egui::Rect::from_min_size(right_min, egui::vec2(area.max.x - right_min.x, height));
    let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect).layout(layout));
    right(&mut right_ui, state);
    ui.advance_cursor_after_rect(left_ui.min_rect().union(right_ui.min_rect()));
}
