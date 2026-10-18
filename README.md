# KM-P6 RGB Tool

*[Deutsche Fassung](README.de.md)*

A lightweight lighting tool for the **Aukey KM-P6** RGB mouse pad (Holtek, USB `04D9:A1FA`), replacing the vendor
software G-aim Control Center. Written in Rust, for Windows and Linux. Two programs that need no installation and
no runtime:

* `kmp6_rgbtool`: command line
* `kmp6_rgbtool_gui`: graphical user interface (egui)

Unofficial, not affiliated with Aukey or G-aim.

**Status 0.1:** All lighting functions of G-aim: the effects static, wave, breathe, neon and off, brightness (10
levels), speed (5 levels), wave direction, seven colour slots and "cycle through all colours". The reports follow
G-aim Control Center 1.1.5 exactly (worked out from the program, see [`docs/PROTOCOL.md`](docs/PROTOCOL.md)) and were
checked on the pad under Windows. Linux builds and passes all tests but has not been tried with the pad yet.

Good to know:

* The pad **cannot be read back**. The tool stores the last setting itself (`%APPDATA%\kmp6_rgbtool\settings.json`,
  on Linux `~/.config/kmp6_rgbtool/settings.json`) and always starts from it.
* The pad **remembers** its last setting without power - **including "off"**. If you switch it off at shutdown,
  something has to switch it on again after the start (`kmp6_rgbtool on`, the logon task below, or OpenRGB etc.).
* It has two light zones, the edge and the logo on the connector box; neither can be set separately.
* The **button** on the connector box steps through the effects on the pad itself (seven static colours, wave,
  breathe, neon). The PC does not learn about it, so the tool keeps showing its own last setting. After a press, click
  "Pressed the button on the pad" in the GUI (or run `kmp6_rgbtool next`): the tool takes the same step and the two
  agree again. A press also switches the pad on again after "off".

**Languages:** English and German (`--lang en|de`, otherwise `KMP6_LANG` or the system language; the GUI has a
switch). Names are understood in both languages, e.g. `--mode breathe` or `--mode atmen`, `2=blue` or `2=blau`. A new
language is just a new file `locales/<code>.yml`.

The original Python prototype (Tkinter) and the PowerShell shutdown script it started with are kept in
[`prototype/`](prototype/). Project status and open points: [`docs/HANDOVER.md`](docs/HANDOVER.md).

## Download

Ready-made programs for Windows and Linux are on the [releases page](../../releases).

## Building

Windows: [rustup](https://rustup.rs) and the Visual Studio Build Tools (workload "Desktop development with C++").
```
cargo build --release
```
The programs are then in `target\release\`.

Linux (Debian/Ubuntu): Rust via rustup, plus for the GUI
`sudo apt install libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`.
Access to the pad (`/dev/hidraw*`, no hidapi; the kernel driver stays attached):
```
sudo cp linux/70-kmp6-rgbtool.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

## Graphical user interface

```
kmp6_rgbtool_gui            # or: kmp6_rgbtool gui
kmp6_rgbtool_gui --fake     # dry run against a simulated pad, nothing is stored
kmp6_rgbtool_gui --lang de  # start in German
```

Pick an effect, move the sliders, click a colour slot - every change goes to the pad right away and is stored. The
drawing of the pad runs at the speeds measured on the real one. The wave direction is not offered in the GUI (left and right look the same on the pad).

## Command line

```
kmp6_rgbtool info                               # is the pad there, what is stored (sends nothing)
kmp6_rgbtool show                               # the stored setting
kmp6_rgbtool set --mode breathe --slot 3 --speed 2
kmp6_rgbtool set --mode static --slot 1 --color 1=#FF8000 --brightness 7
kmp6_rgbtool set --mode wave --direction right
kmp6_rgbtool set --slot cycle                   # all seven colours in turn
kmp6_rgbtool color 2 blue                       # name, #RRGGBB or R,G,B
kmp6_rgbtool off                                # LEDs off, the stored setting stays
kmp6_rgbtool on                                 # send the stored setting again
kmp6_rgbtool reset                              # factory setting of G-aim
kmp6_rgbtool next                               # one step like the button on the pad
kmp6_rgbtool set --mode neon --dry-run          # only print the reports
```

Levels are those of G-aim: brightness 0-9, speed 0-4. `--no-save` sends without storing. `--json` for `info`/`show`.

## LEDs off at shutdown

**Windows:** in the release folder (or `scripts\windows\` after `cargo build --release`), right-click
`install_shutdown.ps1` → "Run with PowerShell" and confirm the administrator prompt. It copies `kmp6_rgbtool.exe` to
`C:\Program Files\kmp6_rgbtool` and registers `kmp6_rgbtool.exe off` as a shutdown script of the local group policy.
Other shutdown scripts are kept; the entry of the old PowerShell version (`mauspad_off.ps1`) is replaced.

```
powershell -ExecutionPolicy Bypass -File install_shutdown.ps1 -LogonOn
```
additionally sends the stored setting at every logon (scheduled task), because the pad would otherwise stay dark.
`uninstall_shutdown.ps1` removes everything again.

**Linux** (systemd):
```
sudo cp kmp6_rgbtool /usr/local/bin/
sudo cp linux/kmp6-off.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now kmp6-off.service
```
To switch the LEDs on again after login, add `kmp6_rgbtool on` to the autostart of your desktop.

## Together with OpenRGB

OpenRGB knows the pad as "Holtek Mousemat" and sends the same effect report, with fixed full brightness and only
slot 1 (details in [`docs/PROTOCOL.md`](docs/PROTOCOL.md#openrgb)). Both can be used side by side; OpenRGB only
writes when it changes something or loads a profile - for example at every start, when its service loads the active
profile. Like the button, the tool does not see such a change: "Send again" or `kmp6_rgbtool on` brings the tool's
setting back. If the tool's setting should win after every logon, install the shutdown script with `-LogonOn`; if
only the tool should drive the pad, switch off the "Holtek Mousemat" detector in OpenRGB.

## License

GPL-2.0-or-later, see [LICENSE](LICENSE).
