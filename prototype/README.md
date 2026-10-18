# Prototypes

Kept for reference; the tool itself is the Rust workspace in `crates/`.

* `python/Mauspad_GUI.py` - the first Tkinter controller (needs `pip install hidapi`). It writes to every interface of
  the pad and ignores errors; its "off" is mode `04` and its "Regenbogen" is the wave of G-aim.
* `powershell/` - the first "LEDs off at shutdown" setup (C# P/Invoke in PowerShell, `psscripts.ini`). Replaced by
  `scripts/windows/install_shutdown.ps1`, which registers `kmp6_rgbtool.exe off` and removes the old entry.
