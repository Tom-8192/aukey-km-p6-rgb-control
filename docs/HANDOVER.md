# Handover / project status – KM-P6 RGB Tool (as of 2026-10-18, version 0.1.0)

Everything a new session needs to continue. Protocol details: `docs/PROTOCOL.md`, usage: `README.md`.

## Goal

Replacement for the lighting part of **G-aim Control Center** for the **Aukey KM-P6** RGB mouse pad (USB `04D9:A1FA`,
Holtek): small, no installation, **Windows first, Linux kept in mind**, LEDs off at shutdown. Built like the sister
project [m6980x-driver](https://github.com/Tom-8192/m6980x-driver) (same workspace layout, transport, i18n, GUI style).

## History

- Python/Tkinter prototype (`prototype/python/Mauspad_GUI.py`) and a PowerShell shutdown script with C# P/Invoke
  (`prototype/powershell/`), both from before the Rust port. The prototype wrote blindly to every interface and
  ignored all errors; its packet order and mode list were guessed.
- 2026-10-18: G-aim Control Center 1.1.5 analysed (installer unpacked with innoextract, not installed; Ghidra via REA;
  the user interface is QML inside the Qt resources of the exe). The result is `docs/PROTOCOL.md`. Then the Rust port
  0.1.0 and tests on the pad (Windows).

## Layout

| Path | Content |
|---|---|
| `crates/kmp6-core` | `protocol.rs` (settings → reports), `device.rs` (sending, lock, retries), `transport/` (hidapi on Windows, hidraw on Linux), `config.rs` (stored setting), `names.rs`, `i18n.rs`, `fake.rs` |
| `crates/kmp6-cli` | `kmp6_rgbtool` (clap; help texts from `locales/`, `help.rs`) |
| `crates/kmp6-gui` | `kmp6_rgbtool_gui` (eframe/egui 0.36), one page; `preview.rs` draws the pad; `dev_shot.rs` = feature `dev-screenshot` |
| `locales/` | all texts, `en.yml` and `de.yml`; `cargo test` checks both have the same keys and placeholders |
| `scripts/windows/` | shutdown install/uninstall (local group policy `scripts.ini`, optional logon task) |
| `linux/` | udev rule, systemd unit for "off at shutdown" |

## Verified on the pad (Windows, 2026-10-18)

Static red; breathe with slot 3 (blue) - the slot nibble works; neon; wave left/right; off (G-aim: static with
brightness 0) and mode `04` (also off, even at full brightness); a single set of reports is enough (G-aim's double
send is kept anyway); static + cycle jumps through the colours; the setting survives unplugging - **"off" too**.
The pad has two zones (edge, logo on the connector box): same colour except in the wave, where they run out of step;
left and right look the same. The button on the connector box steps static ×7 → wave → breathe → neon on
the pad itself; it sends nothing over USB (USBPcap capture, see PROTOCOL.md "Button"). Twice in a comparison run Windows refused a SET_FEATURE (`0x1F`); since then every
report is retried up to 4 times.

`install_shutdown.ps1` ran on Tom's PC (2026-10-18): `scripts.ini` has `kmp6_rgbtool.exe off`, the old
`mauspad_off.ps1` entry is gone from `psscripts.ini`, `gpt.ini` version 3 → 4.

A real shutdown with the new entry switched the LEDs off (Tom, 2026-10-18).

Not yet: Linux on the pad. The GUI was checked by hand on the pad only for one case: after the button on the pad changed the
effect, a click on the already selected effect tile sets it again (every click sends, see `app.rs`); the rest of the
GUI only with `--fake` via `dev-screenshot` pictures.

GUI after Tom's wish "show the behaviour of the pad better" (2026-10-18): drawing after his product photo
(`preview.rs`, shapes only), speeds measured on the pad (PROTOCOL.md "Timing"), the button modelled
(`Settings::after_button`, GUI button "Pressed the button on the pad", CLI `next`), wave direction hidden in the GUI.
Then: two columns from 1040 px content width (no scrolling on 16:9), effects ordered like the button (static, wave,
breathe, neon, off), rainbow computed on sRGB values and dark saturated colours lifted for drawing (blue looked
black on screen). Tom does not want seconds shown in the GUI.

Release v0.1.0: `rust` merged into `main` (fast-forward), tag `v0.1.0` (2026-10-18).

## Guidelines

- Code, comments, docs and commit messages in English; `README.de.md` and `locales/de.yml` German with real umlauts.
- Every claim about the device in the code needs evidence in `docs/PROTOCOL.md`.
- No vendor files in the repository (G-aim program, images, extracted QML).
- Commit/push and releases only when Tom agrees. A tag `v*` builds the release (`.github/workflows/release.yml`,
  notes in `docs/releases/<tag>.md`); the version in `Cargo.toml` must match.

## Checking the GUI without a screen

`CopyFromScreen`/`PrintWindow` gave white windows in this session (screen probably locked). Instead:
```
cargo build -p kmp6-gui --features dev-screenshot
KMP6_CONFIG=<dir>/settings.json KMP6_SHOT=out.bmp KMP6_SHOT_HEIGHT=1100 target/debug/kmp6_rgbtool_gui --fake
```
The window saves itself as BMP after 30 frames and closes; the settings file decides what is shown.

## Open

1. Test on Linux (hidraw path, udev rule, systemd unit).
3. OpenRGB: Tom switches the pad on at startup with OpenRGB - check whether OpenRGB supports the KM-P6 and how it
   relates to this tool.
4. Unknown: what the wave direction does; the levels 1 and 3 of neon/wave are interpolated, not measured.
