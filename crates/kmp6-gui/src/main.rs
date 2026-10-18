//! KM-P6 RGB Tool - GUI (egui) for the lighting of the Aukey KM-P6 mouse pad (Windows + Linux).
//!
//!     kmp6_rgbtool_gui [--device PATH] [--fake] [--lang en|de]
//!
//! `--fake` runs against a simulated pad (dry run without a device, nothing is stored). The language can be
//! switched in the window and is remembered; without a stored choice, `--lang`, `KMP6_LANG` or the system
//! language applies.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
#[cfg(feature = "dev-screenshot")]
mod dev_shot;
mod preview;
mod theme;
mod widgets;
mod worker;

use std::time::Duration;

use eframe::egui::{self, RichText, Stroke};
use kmp6_core::i18n;

use app::{App, LANG_KEY, THEME_KEY};

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(LANG_KEY, i18n::lang());
        storage.set_string(THEME_KEY, theme::theme_id(self.theme).into());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.finish_jobs();
        self.poll_send();
        // the preview is animated
        ctx.request_repaint_after(Duration::from_millis(33));

        let pal = theme::palette(&ctx);
        let bar = egui::Frame::new().fill(pal.sidebar).inner_margin(egui::Margin::symmetric(16, 4));
        egui::Panel::bottom("status").frame(bar).exact_size(28.0).show(ui, |ui| self.ui_status(ui));
        if self.show_log {
            let frame = egui::Frame::new().fill(ui.visuals().extreme_bg_color).inner_margin(10);
            egui::Panel::bottom("log").frame(frame).resizable(true).default_size(120.0).show(ui, |ui| {
                egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                    for line in &self.log {
                        ui.label(RichText::new(line).monospace().small());
                    }
                });
            });
        }
        let head = egui::Frame::new()
            .fill(pal.sidebar)
            .inner_margin(egui::Margin::symmetric(20, 10))
            .stroke(Stroke::new(1.0, pal.card_stroke));
        egui::Panel::top("header").frame(head).show_separator_line(false).show(ui, |ui| self.ui_header(ui));

        let central = egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(egui::Margin::symmetric(20, 16));
        egui::CentralPanel::no_frame().frame(central).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(760.0));
                self.ui_page(ui);
            });
        });
        self.ui_settings(&ctx);
        #[cfg(feature = "dev-screenshot")]
        dev_shot::frame(&ctx);
    }
}

fn main() -> eframe::Result {
    let mut device = None;
    let mut fake = false;
    let mut lang = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--fake" => fake = true,
            "--device" => device = args.next(),
            "--lang" => lang = args.next().as_deref().and_then(i18n::resolve),
            _ => {}
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KM-P6 RGB Tool")
            .with_inner_size([720.0, 860.0])
            .with_min_inner_size([520.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("KM-P6 RGB Tool", options, Box::new(move |cc| Ok(Box::new(App::new(cc, device, fake, lang)))))
}
