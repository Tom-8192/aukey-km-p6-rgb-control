//! Development aid (feature `dev-screenshot`): `KMP6_SHOT=out.bmp` saves the window as a BMP after a few frames
//! and closes it (`KMP6_SHOT_HEIGHT` makes the window taller first). Used to check the layout when no screen capture is possible; not part of release builds.

use eframe::egui::{self, ColorImage, Event, UserData, ViewportCommand};

/// frames before the picture is taken, so that fonts and the animation have settled
const FRAMES: u64 = 30;

pub fn frame(ctx: &egui::Context) {
    let Some(out) = std::env::var_os("KMP6_SHOT") else { return };
    // KMP6_SHOT_HEIGHT: taller window than the screen allows, to see the whole page
    if ctx.cumulative_frame_nr() == 1
        && let Some(h) = std::env::var("KMP6_SHOT_HEIGHT").ok().and_then(|h| h.parse::<f32>().ok())
    {
        let w = std::env::var("KMP6_SHOT_WIDTH").ok().and_then(|w| w.parse::<f32>().ok()).unwrap_or(720.0);
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(w, h)));
    }
    if ctx.cumulative_frame_nr() == FRAMES {
        ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
    }
    let image = ctx.input(|i| {
        i.raw.events.iter().find_map(|e| match e {
            Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(image) = image {
        if let Err(e) = std::fs::write(&out, bmp(&image)) {
            eprintln!("screenshot: {e}");
        }
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}

/// 24-bit bottom-up BMP.
fn bmp(img: &ColorImage) -> Vec<u8> {
    let [w, h] = img.size;
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut out = Vec::with_capacity(size);
    let le32 = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
    out.extend_from_slice(b"BM");
    le32(&mut out, size as u32);
    le32(&mut out, 0);
    le32(&mut out, 54);
    le32(&mut out, 40);
    le32(&mut out, w as u32);
    le32(&mut out, h as u32);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    for v in [0u32, (row * h) as u32, 2835, 2835, 0, 0] {
        le32(&mut out, v);
    }
    for y in (0..h).rev() {
        let start = out.len();
        for p in &img.pixels[y * w..(y + 1) * w] {
            out.extend_from_slice(&[p.b(), p.g(), p.r()]);
        }
        out.resize(start + row, 0);
    }
    out
}
