# Aukey KM-P6 lighting protocol

How the Aukey KM-P6 RGB mouse pad is configured, worked out from the vendor software **G-aim Control Center 1.1.5**
(`G-aim Control Center_v1.1.5.exe`, Inno Setup installer, extracted with `innoextract` without installing it) and
checked on the pad. The vendor files are not part of this repository.

Evidence levels:

- **[code]** read in the G-aim program (address in `G-aim Control Center.exe`, SHA-256 `540d0b5a…2eab`, or the QML
  file of its user interface)
- **[data]** from a data file shipped with G-aim
- **[pad]** confirmed on the pad
- **[guess]** not confirmed

## Device

| | |
|---|---|
| USB | `04D9:A1FA` (Holtek), product string "USB Gaming Keyboard" **[pad]** |
| Light | two zones: the lit edge of the pad and the logo on the connector box (top left) **[pad]** |
| Interface 0 | keyboard (usage page 01, usage 06) - not used **[pad]** |
| Interface 2 | vendor collection, usage page `FF00`, usage `FF00` **[pad]** **[data]** `apConfig.bin` section `[2]` (`type=1`, `pid=a1fa`, `usagePage=ff00`, `isEncode=false`) |
| Report descriptor of interface 2 | `06 00 ff 0a 00 ff a1 01 09 03 15 00 26 ff 00 75 08 95 08 b1 02 c0` → one **feature report without report ID, 8 bytes** **[pad]** |

Every command is a SET_FEATURE with 8 data bytes (hidapi: 9 bytes with report ID 0 in front) **[code]** `0x407230`
(`hid_send_feature_report(dev, buf, 9)`). G-aim never reads from the pad (`hid_get_feature_report` is only used for
the mouse and keyboard, `0x40cc80`), and GET_FEATURE on the pad returns no data **[pad]** → the current setting
cannot be read back; the tool stores it itself.

G-aim calls its pad functions from QML (`ui_View.mousePadSetLedType` / `mousePadSetLedColor`); the HID thread slots
are `hidMsPad_08H_SetLedType` (`0x40ee20`) and `hidMsPad_SetLedColor` (`0x4095c0`), both take 7 bytes and put them
behind the report ID unchanged (the first one prefixes `08`) **[code]**. A third slot, `hidMsPad_LedSync`
(`0x409300`), builds the same `08` report and is used by the "sync" animation below.

## Reports

### `08` - effect

```
08  mode  brightness  speed  colour  R  G  B
```

| Byte | Meaning |
|---|---|
| mode | `00` static, `01` breathe, `02` neon, `03` wave **[code]** (button index of `LightControl_mousePad.qml`, sent unchanged) |
| brightness | device value of the level, see below |
| speed | device value of the level; G-aim sends `00` for static |
| colour | static/breathe: slot in the high nibble (`slot << 4`, slot 0-6) or `70` = cycle through all colours; wave: direction `00` right, `01` left; neon: `00` |
| R G B | colour of slot 1 (static/breathe), otherwise `00 00 00` |

**Off** is not a mode of its own: G-aim sends `08 00 00 <speed> <direction> 00 00 00`, i.e. static with brightness 0
**[code]** (`case 4://關閉`) - both zones dark **[pad]**. Mode `04` (used by the Python prototype) is off as well, even
with brightness `20` and speed `05` **[pad]**.

What the effects look like **[pad]** (2026-10-18):

| Effect | On the pad |
|---|---|
| static | one colour on both zones; with colour byte `70` it jumps through the seven slot colours |
| breathe | pulses in the chosen slot colour (slot nibble works: slot 3 → blue) |
| neon | both zones in the same colour, slowly through the rainbow |
| wave | faster through the rainbow, edge and logo out of step; left and right look the same |
| off | both zones dark |

### `09`, `0A`, `0B` - colours of slots 2-7

```
09  R2 G2 B2  R3 G3 B3  00
0A  R4 G4 B4  R5 G5 B5  00
0B  R6 G6 B6  R7 G7 B7  00
```

Slot 1 travels in the `08` report. **[code]**

### Order

From `saveData` in `LightControl_mousePad.qml` **[code]**:

| Effect | Reports |
|---|---|
| static | `09`, `0A`, `0B`, `08 00 b 00 c R1 G1 B1` - **all twice** |
| breathe | `09`, `0A`, `0B`, `08 01 b s c R1 G1 B1` - **all twice** |
| neon | `08 02 b s 00 00 00 00` |
| wave | `08 03 b s d 00 00 00` |
| off | `08 00 00 s d 00 00 00` |

G-aim sends static and breathe twice with the comment `//再下一次才會改變` ("only changes the next time"). On the pad a
single set was enough (off → static, new colour in slot 2 shown at once) **[pad]**; the tool still sends twice like
G-aim, it costs nothing.

### Refused reports

Now and then the pad refuses a SET_FEATURE: Windows reports `HidD_SetFeature` error `0x1F` ("a device attached to the
system is not functioning"), seen 2 of 14 times when switching effects every 10-12 s **[pad]**. Sending the same
report again shortly after works; the tool retries up to 4 times (40/80/120 ms apart). G-aim does not retry, the
prototype ignored all errors.

## Levels

| | G-aim slider | device values |
|---|---|---|
| brightness | 0-9 | `0, 3, 7, 10, 14, 17, 21, 24, 28, 32` **[code]** (`lightnessSlider.codes`) |
| speed | 0-4 | `0, 1, 3, 4, 5` **[code]** (`rateSlider.codes`) |

G-aim enables brightness for static/breathe/neon/wave, speed for breathe/neon/wave, direction for wave only and the
colour palette for static and breathe only **[code]** (`set_switchBtnEffect`, `colorMask`).

## Factory setting

`setting/pD.bin`, section `[mousePad]` **[data]**:

```
lightValue=0,9,4,1        mode static, brightness level 9, speed level 4, direction 1 (left)
colorValue=255,0,0, 0,255,0, 0,0,255, 255,255,0, 0,255,255, 255,0,255, 255,255,255
isCycle=true              colour byte 70
colorIndex=0
```

## Not device functions

- **Sync** (`lightModeIndex` 1): G-aim animates breathe/neon itself (`syncLed_start`, `syncLed_breatheMode`,
  `syncLed_neonMode`) by sending single static frames to mouse and pad **[code]**. Not implemented.
- **Custom presets** (`mousePadSelfLight*`): named sets of the same values, stored on the PC only **[data]**.

## Button

The connector box has one button that steps through the effects on the pad itself: static seven times (one step per
colour, presumably the seven slots), then wave, breathe, neon, and static again - no "off" **[pad]** (2026-10-18).
The button tells the PC nothing: a USBPcap capture over 35 s with several presses (each one changed the effect)
shows no transfer of the pad at all apart from the descriptor snapshot at the start - no interrupt report on the
keyboard interface or anywhere else **[pad]** (2026-10-18). G-aim only labels the button
in a picture ("Lighting switch", `lightControlLang[42]`) **[code]**. So after a press the stored setting of the tool
no longer matches the pad; sending again restores it.

## Memory

The pad keeps the last setting when it loses power - including "off": switched off and unplugged, it stays dark after
plugging in again **[pad]**. Switching it off at shutdown therefore needs something that sends a setting again after
the start (`kmp6_rgbtool on`, the logon task of `install_shutdown.ps1 -LogonOn`, another program such as OpenRGB) - or
a press on the button.

## Open

- How fast the cycle (`70`) steps, and whether speed affects it **[guess]**
- Which colours the button's seven static steps use (the slot colours?) **[guess]**
- What the wave direction changes (with two zones no difference is visible) **[guess]**
