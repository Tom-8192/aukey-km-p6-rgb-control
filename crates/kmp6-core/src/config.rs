//! The last setting, stored by the tool itself because the pad cannot be read back.
//!
//! File: `%APPDATA%\kmp6_rgbtool\settings.json` (Windows), `$XDG_CONFIG_HOME/kmp6_rgbtool/settings.json` or
//! `~/.config/kmp6_rgbtool/settings.json` (Linux); the environment variable `KMP6_CONFIG` names another file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::protocol::Settings;
use crate::{Error, Result, tr};

pub const FILE_VERSION: u32 = 1;
const DIR: &str = "kmp6_rgbtool";
const FILE: &str = "settings.json";

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    settings: Settings,
}

/// Where the settings live (None if the system tells no config directory, e.g. no HOME).
pub fn path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("KMP6_CONFIG").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    config_dir().map(|d| d.join(DIR).join(FILE))
}

#[cfg(windows)]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").filter(|p| !p.is_empty()).map(PathBuf::from)
}

#[cfg(not(windows))]
fn config_dir() -> Option<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()).map(PathBuf::from);
    xdg.or_else(|| std::env::var_os("HOME").filter(|p| !p.is_empty()).map(|h| PathBuf::from(h).join(".config")))
}

/// Stored setting; `None` if nothing was stored yet.
pub fn load_from(file: &Path) -> Result<Option<Settings>> {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let damaged = |e: &dyn std::fmt::Display| Error::Invalid(tr!("config.damaged", file = file.display(), e = e));
    let parsed: File = serde_json::from_str(&text).map_err(|e| damaged(&e))?;
    if parsed.version != FILE_VERSION {
        return Err(damaged(&tr!("config.version", version = parsed.version)));
    }
    parsed.settings.validate().map_err(|e| damaged(&e))?;
    Ok(Some(parsed.settings))
}

pub fn save_to(file: &Path, settings: &Settings) -> Result<()> {
    settings.validate()?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(&File { version: FILE_VERSION, settings: settings.clone() })
        .map_err(|e| Error::Invalid(e.to_string()))?;
    // write next to it and rename, so a crash never leaves half a file
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, text + "\n")?;
    std::fs::rename(&tmp, file)?;
    Ok(())
}

/// Stored setting, otherwise the factory one.
pub fn load() -> Result<Settings> {
    match path() {
        Some(p) => Ok(load_from(&p)?.unwrap_or_default()),
        None => Ok(Settings::default()),
    }
}

pub fn save(settings: &Settings) -> Result<()> {
    let p = path().ok_or_else(|| Error::Invalid(tr!("config.no_dir")))?;
    save_to(&p, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Mode;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kmp6_test_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("sub").join(FILE)
    }

    #[test]
    fn missing_file_is_none_and_round_trip_works() {
        let file = temp("roundtrip");
        assert!(load_from(&file).unwrap().is_none());
        let s = Settings { mode: Mode::Neon, speed: 1, ..Settings::default() };
        save_to(&file, &s).unwrap();
        assert_eq!(load_from(&file).unwrap(), Some(s));
        let _ = std::fs::remove_dir_all(file.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn damaged_file_is_an_error() {
        let file = temp("damaged");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "{\"version\": 1, \"settings\": {\"brightness\": 12}}").unwrap();
        assert!(matches!(load_from(&file), Err(Error::Invalid(_))));
        std::fs::write(&file, "{\"version\": 9, \"settings\": {}}").unwrap();
        assert!(matches!(load_from(&file), Err(Error::Invalid(_))));
        let _ = std::fs::remove_dir_all(file.parent().unwrap().parent().unwrap());
    }
}
