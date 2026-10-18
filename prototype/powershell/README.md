# Automatic Mousepad LED Shutdown Setup

This directory contains PowerShell scripts designed to automatically turn off the RGB LEDs of the **Holtek / G-aim Mousepad** (USB Vendor ID `0x04D9`, Product ID `0xA1FA`) when Windows shuts down.

---

## 📁 Directory Files

* **`install_shutdown.ps1`**: The automated installer script. Copies the shutdown script to a safe system location (`C:\Skripte\mauspad_off.ps1`) and registers it in Windows Group Policy (GPO) as a machine shutdown script.
* **`mousepad_off.ps1`**: The core script that sends a raw USB HID Feature Report directly to the mousepad microcontroller to disable all LEDs.
* **`mauspad_off.ps1`**: Compatibility wrapper for German script naming.
* **`README.md`**: This documentation file.

---

## 🚀 How to Install

1. Right-click on **`install_shutdown.ps1`**.
2. Select **"Run with PowerShell"**.
3. If prompted by User Account Control (UAC), confirm Administrator permissions.
4. The script will automatically:
   - Create `C:\Skripte` (if it doesn't exist yet).
   - Copy `mousepad_off.ps1` to `C:\Skripte\mauspad_off.ps1`.
   - Update `%windir%\System32\GroupPolicy\Machine\Scripts\psscripts.ini`.
   - Execute `gpupdate /force` to apply the policy immediately.

---

## 🛠️ How It Works (Technical Breakdown)

### 1. USB HID Protocol Control
The mousepad uses standard USB HID Feature Reports to configure modes and colors.
To switch off the LEDs, an 8-byte packet (prefixed with `0x00` Report ID for Windows `hidapi` compatibility) is sent to the device:

| Byte Index | Value | Meaning |
| :--- | :--- | :--- |
| `0` | `0x00` | USB HID Report ID |
| `1` | `0x08` | Packet Type 08 (Controls static slots & active LED mode) |
| `2` | `0x04` | **Mode: Off** (`0x00` = Static, `0x01` = Breathe, `0x02` = Neon, `0x03` = Rainbow, `0x04` = **OFF**) |
| `3` | `0x00` | Brightness (0 = Minimum / Off) |
| `4` | `0x00` | Speed (0 = Default) |
| `5 - 8` | `0x00` | Reserved / Null padding |

### 2. Native Win32 API Interop in PowerShell
Since standard PowerShell lacks native USB HID access methods, `mousepad_off.ps1` uses P/Invoke to compile dynamic C# calls to Windows Native APIs:
* **`SetupDiGetClassDevs`** & **`SetupDiEnumDeviceInterfaces`**: Scans attached HID devices for hardware matches (`vid_04d9&pid_a1fa`).
* **`CreateFile`**: Opens a Win32 file handle to the raw USB HID device path.
* **`HidD_SetFeature`**: Transmits the 9-byte array directly to the mousepad microcontroller.

### 3. Windows Group Policy Shutdown Trigger
Windows natively executes machine shutdown scripts registered in `%windir%\System32\GroupPolicy\Machine\Scripts\psscripts.ini`.
Upon system shutdown, Windows launches `C:\Skripte\mauspad_off.ps1` under the `SYSTEM` account before USB power is terminated.

---

## 🔍 How to Test Manually

You can test turning off the LEDs at any time without shutting down your PC:
1. Open PowerShell.
2. Run:
   ```powershell
   powershell -ExecutionPolicy Bypass -File "C:\Users\Tom\Desktop\Mousepad Shutdown Script\mousepad_off.ps1"
   ```
3. The mousepad LEDs will immediately turn off.

---

## ❌ How to Uninstall / Remove

To disable the automatic shutdown behavior:
1. Open Windows Group Policy Editor (`gpedit.msc`).
2. Navigate to: `Computer Configuration` -> `Windows Settings` -> `Scripts (Startup/Shutdown)` -> `Shutdown`.
3. Select `mauspad_off.ps1` and click **Remove**.
4. Run `gpupdate /force` in Command Prompt or PowerShell.
