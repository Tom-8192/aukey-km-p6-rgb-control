# KM-P6 RGB Tool

*[English version](README.md)*

Ein schlankes Beleuchtungs-Tool für das **Aukey KM-P6** RGB-Mauspad (Holtek, USB `04D9:A1FA`) als Ersatz für die
Herstellersoftware G-aim Control Center. Geschrieben in Rust, für Windows und Linux. Zwei Programme ohne Installation
und ohne Laufzeitumgebung:

* `kmp6_rgbtool`: Kommandozeile
* `kmp6_rgbtool_gui`: grafische Oberfläche (egui)

Inoffiziell, nicht mit Aukey oder G-aim verbunden.

**Stand 0.1:** Alle Beleuchtungsfunktionen von G-aim: die Effekte Statisch, Welle, Atmen, Neon und Aus, Helligkeit
(10 Stufen), Tempo (5 Stufen), Richtung der Welle, sieben Farbplätze und „alle Farben im Wechsel“. Die Pakete
entsprechen genau G-aim Control Center 1.1.5 (aus dem Programm ermittelt, siehe [`docs/PROTOCOL.md`](docs/PROTOCOL.md))
und wurden unter Windows am Pad geprüft. Unter Linux baut das Tool und besteht alle Tests, am Pad ausprobiert ist es
dort noch nicht.

Gut zu wissen:

* Das Pad lässt sich **nicht auslesen**. Das Tool speichert die letzte Einstellung selbst
  (`%APPDATA%\kmp6_rgbtool\settings.json`, unter Linux `~/.config/kmp6_rgbtool/settings.json`) und geht immer davon aus.
* Das Pad **merkt sich** die letzte Einstellung auch ohne Strom – **auch „Aus“**. Wer es beim Herunterfahren
  ausschaltet, braucht nach dem Start etwas, das es wieder einschaltet (`kmp6_rgbtool on`, die Anmelde-Aufgabe unten
  oder OpenRGB o. Ä.).
* Es hat zwei Leuchtzonen, den Rand und das Logo an der Steckerbox; einzeln einstellen lassen sie sich nicht.
* Der **Knopf** an der Steckerbox schaltet direkt am Pad durch die Effekte (sieben statische Farben, Welle, Atmen,
  Neon). Der PC erfährt davon nichts, das Tool zeigt also weiter seine letzte Einstellung. Nach einem Druck in der
  Oberfläche „Knopf am Pad gedrückt“ klicken (oder `kmp6_rgbtool next`): Das Tool macht denselben Schritt, dann
  stimmen beide wieder überein. Nach „Aus“ schaltet ein Druck das Pad auch wieder ein.

**Sprachen:** Englisch und Deutsch (`--lang en|de`, sonst `KMP6_LANG` oder die Systemsprache; die Oberfläche hat einen
Umschalter). Namen werden in beiden Sprachen verstanden, z. B. `--mode atmen` oder `--mode breathe`, `2=blau` oder
`2=blue`. Eine neue Sprache ist nur eine neue Datei `locales/<code>.yml`.

Der ursprüngliche Python-Prototyp (Tkinter) und das PowerShell-Skript fürs Herunterfahren liegen weiter in
[`prototype/`](prototype/). Projektstand und offene Punkte: [`docs/HANDOVER.md`](docs/HANDOVER.md).

## Download

Fertige Programme für Windows und Linux gibt es auf der [Release-Seite](../../releases).

## Bauen

Windows: [rustup](https://rustup.rs) und die Visual Studio Build Tools (Workload „Desktopentwicklung mit C++“).
```
cargo build --release
```
Die Programme liegen dann in `target\release\`.

Linux (Debian/Ubuntu): Rust per rustup, für die Oberfläche zusätzlich
`sudo apt install libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`.
Zugriff auf das Pad (`/dev/hidraw*`, ohne hidapi; der Kerneltreiber bleibt aktiv):
```
sudo cp linux/70-kmp6-rgbtool.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

## Grafische Oberfläche

```
kmp6_rgbtool_gui            # oder: kmp6_rgbtool gui
kmp6_rgbtool_gui --fake     # Probelauf mit simuliertem Pad, nichts wird gespeichert
kmp6_rgbtool_gui --lang de  # auf Deutsch starten
```

Effekt wählen, Regler schieben, Farbplatz anklicken – jede Änderung geht sofort ans Pad und wird gespeichert. Die
Zeichnung des Pads läuft mit den am echten Pad gemessenen Geschwindigkeiten. Die Richtung der Welle bietet die Oberfläche nicht an (links und rechts sehen am Pad gleich
aus).

## Kommandozeile

```
kmp6_rgbtool info                               # ist das Pad da, was ist gespeichert (sendet nichts)
kmp6_rgbtool show                               # die gespeicherte Einstellung
kmp6_rgbtool set --mode atmen --slot 3 --speed 2
kmp6_rgbtool set --mode statisch --slot 1 --color 1=#FF8000 --brightness 7
kmp6_rgbtool set --mode welle --direction rechts
kmp6_rgbtool set --slot zyklus                  # alle sieben Farben im Wechsel
kmp6_rgbtool color 2 blau                       # Name, #RRGGBB oder R,G,B
kmp6_rgbtool off                                # LEDs aus, gespeicherte Einstellung bleibt
kmp6_rgbtool on                                 # gespeicherte Einstellung erneut senden
kmp6_rgbtool reset                              # Werkseinstellung von G-aim
kmp6_rgbtool next                               # einen Schritt weiter wie der Knopf am Pad
kmp6_rgbtool set --mode neon --dry-run          # Pakete nur anzeigen
```

Die Stufen sind die von G-aim: Helligkeit 0–9, Tempo 0–4. `--no-save` sendet, ohne zu speichern. `--json` für
`info`/`show`.

## LEDs aus beim Herunterfahren

**Windows:** Im Release-Ordner (oder nach `cargo build --release` in `scripts\windows\`) auf `install_shutdown.ps1`
rechtsklicken → „Mit PowerShell ausführen“ und die Administrator-Abfrage bestätigen. Das Skript kopiert
`kmp6_rgbtool.exe` nach `C:\Program Files\kmp6_rgbtool` und trägt `kmp6_rgbtool.exe off` als Skript zum
Herunterfahren in die lokale Gruppenrichtlinie ein. Andere Skripte bleiben erhalten; der Eintrag der alten
PowerShell-Version (`mauspad_off.ps1`) wird ersetzt.

```
powershell -ExecutionPolicy Bypass -File install_shutdown.ps1 -LogonOn
```
sendet zusätzlich bei jeder Anmeldung die gespeicherte Einstellung (geplante Aufgabe), weil das Pad sonst dunkel
bliebe. `uninstall_shutdown.ps1` entfernt alles wieder.

**Linux** (systemd):
```
sudo cp kmp6_rgbtool /usr/local/bin/
sudo cp linux/kmp6-off.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now kmp6-off.service
```
Damit die LEDs nach der Anmeldung wieder leuchten, `kmp6_rgbtool on` in den Autostart der Desktopumgebung eintragen.

## Lizenz

GPL-2.0-or-later, siehe [LICENSE](LICENSE).
