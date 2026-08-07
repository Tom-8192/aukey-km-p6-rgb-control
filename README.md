# Holtek / Aukey KM-P6 RGB Mousepad Controller

A lightweight Python Tkinter application to control RGB lighting effects, dynamic colors, brightness, and effect speeds on Holtek-based gaming mousepads (VID: `0x04D9`, PID: `0xA1FA`) without needing proprietary vendor software like G-aim.

## Features

* **Interactive GUI:** Built with Tkinter featuring native color picker support and dynamic button contrast.
* **7 Color Slots:** Assign distinct RGB values to 7 color slots.
* **Lighting Modes:** Switch between Static, Breathing (Atmen), Neon, Rainbow (Regenbogen), and Off (Aus).
* **Speed & Brightness Control:** Fine-tune animation speed (0–5) and brightness levels (0–32).
* **Direct USB HID Communication:** Sends raw HID feature reports using `hidapi`.

## Prerequisites

* Python 3.8+
* `hid` (`pip install hidapi`)
* `tkinter` (included standard with most Python installations)
