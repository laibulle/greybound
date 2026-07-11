# Greybound Render Specs

This document defines the visual asset contract consumed by `greybound-ui`.
For AI generation prompts and a reusable layout guide, see `ai-asset-generation.md`.

## Pedal Assets

- Logical display size: `300 x 543`
- Recommended PNG size for new assets: `1200 x 2172` (`4x`)
- Existing generated assets may still use legacy sizes. Known accepted legacy
  sizes are `1200 x 2260` for early full-height renders and `914 x 1721` for
  the cropped Minotaur render. When touching them, prefer regenerating or
  cropping to the canonical ratio instead of stretching in the UI.
- Format: PNG RGBA, transparent background
- Orientation: portrait
- Safe body area: fill the full logical frame; keep shadows inside the PNG bounds
- Preferred asset path convention: `assets/pedals/<model-id>@4x.png`
- Versioned legacy paths such as `assets/pedals/<model-id>-v2@4x.png` are allowed
  when replacing the path would create unnecessary churn.
- Every `RenderAssetSpec` must declare the actual embedded PNG dimensions. The
  static decoder also asserts exact dimensions for known embedded assets.

The enclosure PNG is the complete static faceplate. For
`typography = BakedIntoAsset`, it must include:

- enclosure/body artwork
- all typography
- model name
- control labels
- min/max marks
- decorative LED bezels and static footswitch hardware if they do not change state
- screw heads, wear, shadows inside the pedal bounds, and non-interactive hardware

For `typography = DrawnByUi`, the static faceplate may omit model name, labels,
and marks that should remain editable from Rust. In that mode, the UI draws
control labels above the faceplate while dynamic controls still render on top.

It must not include dynamic state:

- rotating knob caps
- moving slider handles
- on/off LED jewels
- pressed/unpressed footswitch caps, unless the model intentionally uses a static baked footswitch with only the hit area handled by the UI

Controls are placed in normalized coordinates relative to the logical frame:

- `anchor_x`: `0.0` left, `1.0` right
- `anchor_y`: `0.0` top, `1.0` bottom
- `radius`: visual control radius in logical pixels, used for vector fallback and default image bounds
- `hit_radius`: pointer hit radius in logical pixels

The app renders dynamic controls above the static faceplate PNG. If a model has `typography = BakedIntoAsset`, the UI must not draw any text for that model.

## Amp Assets

- Logical size: `1240 x 500`
- Recommended PNG size: `2480 x 1000` (`2x`)
- Cropped amp-head surfaces can declare their own logical and recommended
  dimensions. The current Nox30 head uses a cropped `1620 x 856` PNG on a
  dedicated cropped amp surface.
- Format: PNG RGBA, transparent background
- Orientation: landscape
- Asset path convention: `assets/amps/<model-id>@2x.png`

Amp controls use the same normalized control contract as pedals. Current amp heads still use the legacy vector renderer, but `AppAmpModelDescriptor.render` is already the handoff point for PNG-backed skins.
An amp-head asset may include a partial cabinet underneath when that is part of the intended product render; keep the whole composition inside the declared amp surface and keep dynamic knobs/switches as separate control assets.

## Control Widgets

Supported widget kinds:

- `Pot`: rotary potentiometer
- `Slider`: linear slider
- `Toggle`: binary switch
- `Footswitch`: bypass/action switch
- `Led`: status indicator

Supported control roles:

- `Parameter(ControlKind)`: maps to a normalized audio/control value
- `Bypass`: maps to the device bypass state

Each control can define its own PNG assets:

- `image`: default PNG for a control
- `active_image`: optional state PNG for LED/bypass-on states
- `pressed_image`: optional state PNG for footswitch press animation
- `rotation`: optional normalized-value rotation spec

### Pot PNGs

- Recommended size: square PNG RGBA, `256 x 256` or `512 x 512`
- Transparent background
- Knob pointer must point to the zero/minimum angle in the source PNG. For a standard pedal pot this is usually the lower-left stop, so half rotation lands at the top and max lands at the lower-right stop.
- The PNG may include highlights, bevels, and reflections that belong to the knob material itself, but it must not include external cast shadows, contact shadows, drop shadows, or background reflections. Those would rotate with the control and look wrong.
- Rotation is performed around `pivot_x` / `pivot_y`, normalized inside the PNG
- Default rotation range: `-135deg` to `135deg`
- Asset path convention: `assets/controls/knobs/<knob-id>@2x.png`
- Do not store one PNG per knob position. The UI loads one source PNG and rotates it at runtime, with an internal memory cache when quantization is useful.

### LED PNGs

- Recommended size: square PNG RGBA, `128 x 128` or `256 x 256`
- Use two assets when the visual state changes strongly:
  - off/default in `image`
  - on/active in `active_image`
- For `Bypass`-driven LEDs, the UI treats the device as active/on when it is not bypassed and selects `active_image`.
- Asset path convention: `assets/controls/leds/<led-id>-off@2x.png` and `assets/controls/leds/<led-id>-on@2x.png`

### Footswitch PNGs

- Recommended size: square PNG RGBA, `256 x 256` or `512 x 512`
- Use `pressed_image` for mouse-down/pressed state when available
- Current asset path convention: `assets/controls/buttons/<switch-id>@2x.png`

## Application Profiles

Each app profile provides the visible catalogue:

- `AppAmpModelDescriptor`: id, label, visual template, render spec, circuit provider
- `AppDeviceModelDescriptor`: id, label, kind, visual template, runtime config, render spec, circuit provider

The open-source UI owns the generic rendering contract. Private apps can provide private model ids, labels, runtime bindings, circuit providers, and PNG assets without adding private names to `greybound-ui`.
