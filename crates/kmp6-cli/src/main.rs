//! kmp6_rgbtool: command line of the KM-P6 RGB Tool (Aukey KM-P6 RGB mouse pad).
//!
//! The pad cannot be read back, so every change starts from the stored setting (`kmp6_core::config`) and is
//! stored again after it was sent. `off` leaves the stored setting alone, so `on` brings it back (shutdown hook).
//! All texts (including the help) come from locales/*.yml in the language of `--lang`, `KMP6_LANG` or the system.

mod help;

use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use kmp6_core::names::{self, direction_name, format_color, mode_name};
use kmp6_core::protocol::{MAX_BRIGHTNESS, MAX_SPEED, SLOTS};
use kmp6_core::{Error, KmP6, Mode, Result, Settings, config, hex, i18n, open_transport, tr};
use serde_json::json;

/// Help texts: locales/*.yml under cli.about, cli.cmd.*, cli.arg.*, cli.opt.* (see help.rs).
#[derive(Parser)]
#[command(name = "kmp6_rgbtool", version)]
struct Cli {
    #[arg(long, global = true)]
    device: Option<String>,
    #[arg(long, global = true, value_parser = parse_lang)]
    lang: Option<String>,
    #[command(subcommand)]
    command: Cmd,
}

/// Options of every command that sends something.
#[derive(clap::Args, Clone, Copy)]
struct Send {
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    no_save: bool,
}

#[derive(Subcommand)]
enum Cmd {
    Gui,
    Info {
        #[arg(long)]
        json: bool,
    },
    Show {
        #[arg(long)]
        json: bool,
    },
    Set {
        #[arg(long, value_parser = parse_mode)]
        mode: Option<Mode>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=i64::from(MAX_BRIGHTNESS)))]
        brightness: Option<u8>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=i64::from(MAX_SPEED)))]
        speed: Option<u8>,
        #[arg(long, value_parser = parse_direction)]
        direction: Option<kmp6_core::Direction>,
        #[arg(long, value_parser = parse_slot)]
        slot: Option<Shown>,
        #[arg(long = "color", value_name = "SLOT=COLOR", value_parser = parse_slot_color)]
        colors: Vec<(u8, kmp6_core::Rgb)>,
        #[command(flatten)]
        send: Send,
    },
    Color {
        #[arg(value_parser = clap::value_parser!(u8).range(1..=SLOTS as i64))]
        slot: u8,
        #[arg(value_parser = parse_color)]
        color: kmp6_core::Rgb,
        #[command(flatten)]
        send: Send,
    },
    On {
        #[arg(long)]
        dry_run: bool,
    },
    Off {
        #[arg(long)]
        dry_run: bool,
    },
    Reset {
        #[command(flatten)]
        send: Send,
    },
    Next {
        #[command(flatten)]
        send: Send,
    },
}

fn parse_lang(text: &str) -> std::result::Result<String, String> {
    i18n::resolve(text).ok_or_else(|| tr!("error.expected_one_of", names = i18n::available().join(", ")))
}

fn parse_mode(text: &str) -> std::result::Result<Mode, String> {
    names::parse_mode(text).map_err(|e| e.to_string())
}

fn parse_direction(text: &str) -> std::result::Result<kmp6_core::Direction, String> {
    names::parse_direction(text).map_err(|e| e.to_string())
}

/// Colour the pad shows: one slot (index) or all in turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Slot(u8),
    Cycle,
}

fn parse_slot(text: &str) -> std::result::Result<Shown, String> {
    match names::parse_slot(text).map_err(|e| e.to_string())? {
        Some(i) => Ok(Shown::Slot(i)),
        None => Ok(Shown::Cycle),
    }
}

fn parse_color(text: &str) -> std::result::Result<kmp6_core::Rgb, String> {
    names::parse_color(text).map_err(|e| e.to_string())
}

/// "2=blue" -> (slot index 1, colour)
fn parse_slot_color(text: &str) -> std::result::Result<(u8, kmp6_core::Rgb), String> {
    let (slot, color) = text.split_once('=').ok_or_else(|| tr!("cli.color_needs_slot"))?;
    match names::parse_slot(slot).map_err(|e| e.to_string())? {
        Some(i) => Ok((i, parse_color(color)?)),
        None => Err(tr!("cli.color_needs_slot")),
    }
}

/// `--lang` must be known before parsing so that help and error messages already come in the right language.
fn lang_from_args() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--" {
            break;
        }
        if a == "--lang" {
            return args.next().as_deref().and_then(i18n::resolve);
        }
        if let Some(v) = a.strip_prefix("--lang=") {
            return i18n::resolve(v);
        }
    }
    None
}

fn parse_cli() -> Cli {
    let matches = help::localize(Cli::command()).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
}

fn main() -> ExitCode {
    i18n::set_lang(&lang_from_args().unwrap_or_else(i18n::detect));
    let cli = parse_cli();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e}", tr!("cli.error"));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    let device = cli.device.as_deref();
    match &cli.command {
        Cmd::Gui => launch_gui(device, cli.lang.as_deref()),
        Cmd::Info { json } => cmd_info(device, *json),
        Cmd::Show { json } => {
            let s = config::load()?;
            if *json {
                print_json(&json!({ "file": config_file(), "settings": s }));
            } else {
                print_settings(&s);
            }
            Ok(())
        }
        Cmd::Set { mode, brightness, speed, direction, slot, colors, send } => {
            let mut s = config::load()?;
            if let Some(m) = mode {
                s.mode = *m;
            }
            if let Some(b) = brightness {
                s.brightness = *b;
            }
            if let Some(v) = speed {
                s.speed = *v;
            }
            if let Some(d) = direction {
                s.direction = *d;
            }
            match slot {
                Some(Shown::Slot(i)) => (s.cycle, s.slot) = (false, *i),
                Some(Shown::Cycle) => s.cycle = true,
                None => {}
            }
            for (i, c) in colors {
                s.colors[usize::from(*i)] = *c;
            }
            send_settings(device, &s, *send)
        }
        Cmd::Color { slot, color, send } => {
            let mut s = config::load()?;
            s.colors[usize::from(slot - 1)] = *color;
            send_settings(device, &s, *send)?;
            if !s.mode.uses_colors() {
                println!("{}", tr!("cli.colors_unused", mode = mode_name(s.mode)));
            }
            Ok(())
        }
        Cmd::On { dry_run } => {
            let mut s = config::load()?;
            if s.mode == Mode::Off {
                s.mode = Mode::Static;
            }
            send_settings(device, &s, Send { dry_run: *dry_run, no_save: false })
        }
        Cmd::Off { dry_run } => {
            // the stored setting stays, so `on` can bring it back; a damaged file must not stop the shutdown hook
            let s = config::load().unwrap_or_default();
            let off = Settings { mode: Mode::Off, ..s };
            send_settings(device, &off, Send { dry_run: *dry_run, no_save: true })
        }
        Cmd::Reset { send } => send_settings(device, &Settings::default(), *send),
        Cmd::Next { send } => {
            let s = config::load()?.after_button();
            send_settings(device, &s, *send)?;
            print_settings(&s);
            Ok(())
        }
    }
}

fn send_settings(device: Option<&str>, s: &Settings, opts: Send) -> Result<()> {
    let packets = s.packets()?;
    if opts.dry_run {
        println!("{}", tr!("cli.dry_run"));
        for p in &packets {
            println!("  {}", hex(p));
        }
        return Ok(());
    }
    let mut pad = KmP6::new(open_transport(device)?);
    pad.send_all(&packets)?;
    if opts.no_save {
        println!("{}", tr!("cli.sent", n = packets.len()));
    } else {
        config::save(s)?;
        println!("{}", tr!("cli.sent_saved", n = packets.len(), file = config_file()));
    }
    if s.mode == Mode::Off {
        println!("{}", tr!("cli.off_hint"));
    }
    Ok(())
}

fn config_file() -> String {
    config::path().map(|p| p.display().to_string()).unwrap_or_else(|| "-".into())
}

fn cmd_info(device: Option<&str>, as_json: bool) -> Result<()> {
    // opening only looks for the device; nothing is sent
    let found = open_transport(device).map(|t| t.path());
    let stored = config::path().map(|p| config::load_from(&p)).transpose()?.flatten();
    if as_json {
        print_json(&json!({
            "device": found.as_ref().ok(),
            "error": found.as_ref().err().map(Error::to_string),
            "file": config_file(),
            "settings": stored,
        }));
        return Ok(());
    }
    match &found {
        Ok(path) => println!("{:<16}{path}", tr!("cli.device")),
        Err(e) => println!("{:<16}{e}", tr!("cli.device")),
    }
    println!("{:<16}{}", tr!("cli.settings_file"), config_file());
    match &stored {
        Some(s) => {
            println!("{}", tr!("cli.stored_setting"));
            print_settings(s);
        }
        None => println!("{}", tr!("cli.nothing_stored")),
    }
    Ok(())
}

fn print_json(value: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(value).expect("JSON value"));
}

fn print_settings(s: &Settings) {
    let unused = |used: bool| if used { String::new() } else { format!("  {}", tr!("cli.not_used")) };
    let row = |label: String, value: String| println!("  {label:<14}{value}");
    row(tr!("cli.mode"), mode_name(s.mode));
    row(tr!("cli.brightness"), format!("{} / {MAX_BRIGHTNESS}{}", s.brightness, unused(s.mode.uses_brightness())));
    row(tr!("cli.speed"), format!("{} / {MAX_SPEED}{}", s.speed, unused(s.mode.uses_speed())));
    row(tr!("cli.direction"), format!("{}{}", direction_name(s.direction), unused(s.mode.uses_direction())));
    let shown = if s.cycle { tr!("cli.cycle") } else { tr!("cli.slot_shown", slot = s.slot + 1) }
        + &unused(s.mode.uses_colors());
    row(tr!("cli.color"), shown);
    for (i, c) in s.colors.iter().enumerate() {
        let mark = if !s.cycle && usize::from(s.slot) == i { tr!("cli.shown_mark") } else { String::new() };
        row(if i == 0 { tr!("cli.slots") } else { String::new() }, format!("{} {}{mark}", i + 1, format_color(*c)));
    }
}

fn launch_gui(device: Option<&str>, lang: Option<&str>) -> Result<()> {
    let exe = std::env::current_exe()?;
    let gui = exe.with_file_name(format!("kmp6_rgbtool_gui{}", std::env::consts::EXE_SUFFIX));
    if !gui.exists() {
        return Err(Error::Invalid(tr!("cli.gui_not_found", gui = gui.display())));
    }
    let mut cmd = std::process::Command::new(gui);
    if let Some(d) = device {
        cmd.args(["--device", d]);
    }
    if let Some(l) = lang {
        cmd.args(["--lang", l]);
    }
    cmd.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_and_argument_has_help() {
        let missing = help::missing(Cli::command());
        assert!(missing.is_empty(), "without help text in locales/en.yml: {missing:#?}");
    }

    #[test]
    fn german_and_english_arguments_parse() {
        let cli = Cli::try_parse_from([
            "kmp6_rgbtool",
            "set",
            "--mode",
            "atmen",
            "--slot",
            "zyklus",
            "--color",
            "2=blau",
            "--color",
            "7=#102030",
            "--dry-run",
        ])
        .unwrap();
        let Cmd::Set { mode, slot, colors, send, .. } = cli.command else { panic!("set expected") };
        assert_eq!(mode, Some(Mode::Breathe));
        assert_eq!(slot, Some(Shown::Cycle));
        assert_eq!(colors, [(1, [0, 0, 255]), (6, [0x10, 0x20, 0x30])]);
        assert!(send.dry_run && !send.no_save);
        assert!(Cli::try_parse_from(["kmp6_rgbtool", "set", "--brightness", "10"]).is_err());
        assert!(Cli::try_parse_from(["kmp6_rgbtool", "set", "--color", "zyklus=rot"]).is_err());
        assert!(Cli::try_parse_from(["kmp6_rgbtool", "color", "8", "red"]).is_err());
    }
}
