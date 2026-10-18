//! State of the window and its single page: effect, levels, colours. Every change is sent shortly after the
//! last input (like the M6980X tool); the pad cannot be read back, so the window starts from the stored setting.
//! The button on the pad changes the effect without the PC noticing, so every click sends - also on the effect or
//! colour slot that is already selected, and also when the setting is the same as the one sent before.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, ThemePreference, Ui};
use egui_phosphor::regular as icon;
use kmp6_core::names::{self, COLORS, format_hex, mode_name};
use kmp6_core::protocol::{MAX_BRIGHTNESS, MAX_SPEED};
use kmp6_core::{Mode, Settings, config, i18n, tr};

use crate::theme::{self, ACCENT};
use crate::widgets;
use crate::worker::{Device, Job};

pub const LANG_KEY: &str = "lang";
pub const THEME_KEY: &str = "theme";
/// quiet time after the last change before it is sent
const SETTLE: Duration = Duration::from_millis(120);
/// while a slider is dragged, still send at least this often
const MAX_DELAY: Duration = Duration::from_millis(400);
const LOG_LINES: usize = 200;

pub struct App {
    dev: Device,
    pub fake: bool,
    pub settings: Settings,
    first_change: Option<Instant>,
    last_change: Option<Instant>,
    pub connected: Option<bool>,
    pub status: String,
    pub log: Vec<String>,
    pub show_log: bool,
    pub show_settings: bool,
    pub theme: ThemePreference,
    start: Instant,
    /// text field of the selected slot and the colour it was made from
    hex_text: String,
    hex_color: [u8; 3],
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, device: Option<String>, fake: bool, lang: Option<String>) -> Self {
        let stored = |key: &str| cc.storage.and_then(|s| s.get_string(key));
        // --lang before the stored choice before environment/system language
        let stored_lang = stored(LANG_KEY).as_deref().and_then(i18n::resolve);
        i18n::set_lang(&lang.or(stored_lang).unwrap_or_else(i18n::detect));
        let theme = stored(THEME_KEY).as_deref().map_or(ThemePreference::Dark, theme::theme_from_id);
        theme::install(&cc.egui_ctx, theme);
        let mut log = Vec::new();
        let settings = config::load().unwrap_or_else(|e| {
            log.push(e.to_string());
            Settings::default()
        });
        let mut dev = Device::spawn(cc.egui_ctx.clone(), device, fake);
        dev.submit(Job::Connect);
        let hex_color = settings.colors[usize::from(settings.slot)];
        Self {
            dev,
            fake,
            settings,
            first_change: None,
            last_change: None,
            connected: None,
            status: tr!("gui.connecting"),
            log,
            show_log: false,
            show_settings: false,
            theme,
            start: Instant::now(),
            hex_text: format_hex(hex_color),
            hex_color,
        }
    }

    fn log_line(&mut self, line: String) {
        self.log.push(line);
        if self.log.len() > LOG_LINES {
            self.log.drain(..self.log.len() - LOG_LINES);
        }
    }

    /// Something was changed in the window: send it once the input settles.
    pub fn changed(&mut self) {
        let now = Instant::now();
        self.first_change.get_or_insert(now);
        self.last_change = Some(now);
    }

    pub fn send_now(&mut self) {
        self.changed();
        self.first_change = Some(Instant::now() - MAX_DELAY);
    }

    pub fn reconnect(&mut self) {
        self.connected = None;
        self.status = tr!("gui.connecting");
        self.dev.submit(Job::Connect);
    }

    pub fn finish_jobs(&mut self) {
        while let Ok(done) = self.dev.rx.try_recv() {
            self.dev.pending -= 1;
            self.connected = Some(done.connected);
            match (done.job, done.result) {
                (Job::Apply(s), Ok(n)) => {
                    self.status = tr!("gui.sent", n = n);
                    self.log_line(format!("{} - {}", mode_name(s.mode), self.status));
                }
                (Job::Connect, Ok(_)) => self.status = tr!(if self.fake { "gui.ready_fake" } else { "gui.ready" }),
                (_, Err(e)) => {
                    self.status = e.clone();
                    self.log_line(e);
                }
            }
        }
    }

    /// Sends the setting once the input has settled (only one job at a time).
    pub fn poll_send(&mut self) {
        let (Some(first), Some(last)) = (self.first_change, self.last_change) else { return };
        if !self.dev.idle() || (last.elapsed() < SETTLE && first.elapsed() < MAX_DELAY) {
            return;
        }
        self.first_change = None;
        self.last_change = None;
        match self.settings.validate() {
            Ok(()) => self.dev.submit(Job::Apply(self.settings.clone())),
            Err(e) => self.status = e.to_string(),
        }
    }

    pub fn busy(&self) -> bool {
        !self.dev.idle() || self.last_change.is_some()
    }

    pub fn time(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    // ---------------------------------------------------------------- page

    pub fn ui_page(&mut self, ui: &mut Ui) {
        let mode = self.settings.mode;
        widgets::card(ui, |ui| {
            widgets::card_title(ui, icon::LIGHTBULB, &tr!("gui.effect"), Some(&tr!("gui.effect_hint")));
            ui.add_space(4.0);
            self.ui_modes(ui);
            ui.add_space(4.0);
            // the button on the pad changes the effect without telling the PC: follow it by hand
            let button = widgets::icon_button(ui, icon::HAND_POINTING, &tr!("gui.pad_button"))
                .on_hover_text(tr!("gui.pad_button_hint"));
            if button.clicked() {
                self.settings = self.settings.after_button();
                self.sync_hex();
                self.send_now();
            }
            ui.add_space(6.0);
            crate::preview::show(ui, &self.settings, self.time());
        });
        ui.add_space(6.0);
        widgets::card(ui, |ui| {
            widgets::card_title(ui, icon::SLIDERS_HORIZONTAL, &tr!("gui.adjust"), None);
            self.ui_levels(ui, mode);
        });
        ui.add_space(6.0);
        widgets::card(ui, |ui| {
            widgets::card_title(ui, icon::PALETTE, &tr!("gui.colors"), Some(&tr!("gui.colors_hint")));
            if !mode.uses_colors() {
                widgets::muted(ui, tr!("gui.colors_unused", mode = mode_name(mode)));
                ui.add_space(4.0);
            }
            self.ui_colors(ui);
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, icon::ARROW_COUNTER_CLOCKWISE, &tr!("gui.factory")).clicked() {
                self.settings = Settings::default();
                self.sync_hex();
                self.changed();
            }
            if widgets::icon_button(ui, icon::PAPER_PLANE_TILT, &tr!("gui.resend")).clicked() {
                self.send_now();
            }
        });
    }

    fn ui_modes(&mut self, ui: &mut Ui) {
        let tiles = [
            (Mode::Static, icon::SUN),
            (Mode::Breathe, icon::WIND),
            (Mode::Neon, icon::LIGHTNING),
            (Mode::Wave, icon::WAVES),
            (Mode::Off, icon::POWER),
        ];
        let gap = ui.spacing().item_spacing.x;
        let n = tiles.len() as f32;
        // one row while the tiles stay readable, otherwise they wrap
        let width = (((ui.available_width() - gap * (n - 1.0)) / n).floor() - 1.0).max(84.0);
        ui.horizontal_wrapped(|ui| {
            for (mode, glyph) in tiles {
                let selected = self.settings.mode == mode;
                // also when already selected: the button on the pad may have switched to something else
                if widgets::mode_tile(ui, glyph, &mode_name(mode), selected, width).clicked() {
                    self.settings.mode = mode;
                    self.send_now();
                }
            }
        });
    }

    fn ui_levels(&mut self, ui: &mut Ui, mode: Mode) {
        egui::Grid::new("levels").num_columns(3).spacing([16.0, 12.0]).show(ui, |ui| {
            let mut changed = false;
            let rows = [
                (tr!("gui.brightness"), mode.uses_brightness(), MAX_BRIGHTNESS, &mut self.settings.brightness),
                (tr!("gui.speed"), mode.uses_speed(), MAX_SPEED, &mut self.settings.speed),
            ];
            for (label, used, max, value) in rows {
                ui.label(label);
                ui.add_enabled_ui(used, |ui| {
                    let slider = egui::Slider::new(value, 0..=max).show_value(false);
                    changed |= ui.add(slider).changed();
                });
                let text = if used { tr!("gui.level_of", v = *value, max = max) } else { tr!("gui.not_used") };
                widgets::muted(ui, text);
                ui.end_row();
            }
            // The wave direction is still sent like G-aim does, but left and right look the same on the pad
            // (two zones only), so it is not offered here; the command line keeps --direction.
            if changed {
                self.changed();
            }
        });
    }

    fn sync_hex(&mut self) {
        self.hex_color = self.settings.colors[usize::from(self.settings.slot)];
        self.hex_text = format_hex(self.hex_color);
    }

    fn ui_colors(&mut self, ui: &mut Ui) {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for i in 0..self.settings.colors.len() {
                let [r, g, b] = self.settings.colors[i];
                let cell = egui::Layout::top_down(egui::Align::Center);
                ui.allocate_ui_with_layout(egui::vec2(48.0, 66.0), cell, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let selected = usize::from(self.settings.slot) == i;
                    let resp = widgets::swatch(ui, Color32::from_rgb(r, g, b), selected, 44.0)
                        .on_hover_text(tr!("gui.slot", slot = i + 1));
                    if resp.clicked() {
                        self.settings.slot = i as u8;
                        changed = true;
                    }
                    ui.label(RichText::new((i + 1).to_string()).small());
                });
            }
        });
        if changed {
            self.sync_hex();
        }
        ui.add_space(6.0);
        let slot = usize::from(self.settings.slot);
        ui.horizontal(|ui| {
            ui.label(tr!("gui.slot_color", slot = slot + 1));
            let color = &mut self.settings.colors[slot];
            if egui::color_picker::color_edit_button_srgb(ui, color).changed() {
                changed = true;
            }
            if *color != self.hex_color {
                self.hex_color = *color;
                self.hex_text = format_hex(*color);
            }
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.hex_text).desired_width(76.0).font(egui::TextStyle::Monospace),
            );
            if edit.changed()
                && let Some(c) = names::parse_hex(&self.hex_text)
                && self.hex_text.trim_start_matches('#').len() == 6
            {
                self.settings.colors[slot] = c;
                self.hex_color = c;
                changed = true;
            }
            if edit.lost_focus() {
                self.hex_text = format_hex(self.settings.colors[slot]);
            }
        });
        ui.horizontal_wrapped(|ui| {
            widgets::muted(ui, tr!("gui.quick_colors"));
            for (id, rgb) in COLORS {
                let name = i18n::name(&format!("color.{id}"));
                let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                if widgets::swatch(ui, c, false, 24.0).on_hover_text(widgets::nice(&name)).clicked() {
                    self.settings.colors[slot] = rgb;
                    changed = true;
                }
            }
        });
        ui.add_space(4.0);
        let cycle_used = self.settings.mode.uses_colors();
        ui.add_enabled_ui(cycle_used, |ui| {
            changed |= ui.checkbox(&mut self.settings.cycle, tr!("gui.cycle")).changed();
        });
        if changed {
            self.changed();
        }
    }

    // ---------------------------------------------------------------- frame parts

    pub fn ui_header(&mut self, ui: &mut Ui) {
        let pal = theme::palette(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon::RECTANGLE).size(26.0).color(ACCENT));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("KM-P6 RGB Tool").size(16.0).strong().color(pal.strong));
                let (color, text) = match self.connected {
                    _ if self.fake => (ACCENT, tr!("gui.status.simulation")),
                    None => (theme::WARNING, tr!("gui.connecting")),
                    Some(true) => (theme::SUCCESS, tr!("gui.status.connected")),
                    Some(false) => (theme::DANGER, tr!("gui.status.disconnected")),
                };
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, color);
                    ui.label(RichText::new(text).small().color(pal.muted));
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new(icon::GEAR).size(18.0)).on_hover_text(tr!("gui.settings")).clicked() {
                    self.show_settings = true;
                }
                if self.connected == Some(false)
                    && !self.fake
                    && widgets::icon_button(ui, icon::PLUGS_CONNECTED, &tr!("gui.reconnect")).clicked()
                {
                    self.reconnect();
                }
            });
        });
    }

    pub fn ui_status(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.busy() {
                ui.spinner();
            }
            ui.add(egui::Label::new(RichText::new(&self.status).small()).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let arrow = if self.show_log { icon::CARET_DOWN } else { icon::CARET_RIGHT };
                let text = RichText::new(format!("{}  {}  {arrow}", icon::LIST, tr!("gui.log"))).small();
                if ui.add(egui::Button::new(text).frame(false)).clicked() {
                    self.show_log = !self.show_log;
                }
            });
        });
    }

    pub fn ui_settings(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("settings")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon::GEAR).size(20.0).color(ACCENT));
                ui.heading(tr!("gui.settings"));
            });
            ui.add_space(6.0);
            egui::Grid::new("settings").num_columns(2).spacing([16.0, 12.0]).show(ui, |ui| {
                ui.label(tr!("gui.language"));
                let mut lang = i18n::lang();
                egui::ComboBox::from_id_salt("lang").selected_text(i18n::native_name(&lang)).show_ui(ui, |ui| {
                    for l in i18n::available() {
                        let name = i18n::native_name(&l);
                        ui.selectable_value(&mut lang, l, name);
                    }
                });
                if lang != i18n::lang() {
                    i18n::set_lang(&lang);
                }
                ui.end_row();
                ui.label(tr!("gui.theme"));
                let options = [
                    (ThemePreference::System, tr!("gui.theme.system")),
                    (ThemePreference::Light, tr!("gui.theme.light")),
                    (ThemePreference::Dark, tr!("gui.theme.dark")),
                ];
                if widgets::segmented(ui, &mut self.theme, &options) {
                    ctx.set_theme(self.theme);
                }
                ui.end_row();
                ui.label(tr!("gui.settings_file"));
                let file = config::path().map(|p| p.display().to_string()).unwrap_or_else(|| "-".into());
                ui.add(egui::Label::new(RichText::new(file).small()).wrap());
                ui.end_row();
            });
            ui.add_space(8.0);
            close = ui.button(tr!("gui.close")).clicked();
        });
        if close || modal.should_close() {
            self.show_settings = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_colours_have_names_in_every_language() {
        for lang in i18n::available() {
            for (id, _) in COLORS {
                assert!(!i18n::text_in(&lang, &format!("color.{id}")).is_empty());
            }
        }
    }
}
