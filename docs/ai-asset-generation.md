# Greybound AI Asset Generation

Use this guide when generating or regenerating photorealistic Greybound hardware assets.

## When To Regenerate

Do not regenerate a working asset only to satisfy the new ratio. Keep Minotaur and Springfield as visual references unless the composition itself must change. Regenerate when:

- the pedal body is visibly distorted in the app
- typography or baked hardware must move
- a future asset needs to be created from scratch
- the source bitmap cannot be cropped without damaging the artwork

## Pedal Faceplate Prompt

Attach `docs/templates/pedal-standard-1200x2172.svg` as the layout reference and the closest existing Greybound render as the style reference. The SVG is only a construction guide; its colored lines and placeholder text must not appear in the final image.

```text
Create a single photorealistic boutique guitar pedal faceplate as a transparent-background PNG.

Canvas and layout:
- exact canvas: 1200 x 2172 pixels
- straight-on orthographic product render, no perspective tilt
- centered vertical pedal enclosure, rounded corners, realistic bevels
- no external background, no floor, no cast shadow outside the transparent canvas
- body must fill the safe body area without touching the canvas edges
- preserve circular hardware proportions; do not stretch the footswitch, jacks, LED bezel, or knob holes

Visual quality:
- same high-end photorealistic quality as the Greybound Minotaur and Springfield renders
- physically plausible metal, enamel, glass, pearloid, engraved or screen-printed typography
- crisp details at 4x UI resolution
- premium product photography look, clean front view, no cartoon or illustration style

Static faceplate contents:
- include the pedal enclosure artwork
- for `typography = BakedIntoAsset`, include all typography baked into the image
- for `typography = DrawnByUi`, omit editable text and leave clean visual space
  for labels drawn by the UI
- include model name: <MODEL_NAME> only when the target render spec bakes it
- include control labels: <CONTROL_LABELS> only when the target render spec bakes them
- include static input/output/DC labels if the design needs them
- include static footswitch hardware if it does not need a pressed/unpressed animation
- include static LED bezel or socket only if the jewel/glow is supplied as a separate dynamic PNG

Do not include:
- rotating knob caps
- knob pointers or indicators
- dynamic LED jewel color/glow
- UI shadows or contact shadows that would conflict with the app
- guide lines, placeholder circles, placeholder text, crop marks, watermark, logo unless explicitly requested
- screws on the front face unless the real design intentionally has visible front screws

Output:
- one transparent PNG faceplate
- exact dimensions 1200 x 2172
- no padding beyond the transparent canvas
```

## Control Asset Prompt

Use this for knobs and LEDs that the UI rotates or swaps at runtime.

```text
Create one isolated photorealistic hardware control as a transparent-background square PNG.

Canvas:
- exact canvas: 512 x 512 for knobs, 256 x 256 for LEDs
- centered object
- no background
- no external cast shadow or contact shadow

Knobs:
- pointer must indicate the zero/minimum position in the source PNG
- for Greybound pedal knobs, zero/minimum means the pointer is already at the
  lower-left stop, around 225 degrees on the knob face
- do not generate the pointer at noon, at the right side, or in a neutral/default
  position; the runtime uses the source image itself as value 0.0
- highlights must belong to the material itself, not to the environment
- circular knob body must remain perfectly round

LEDs:
- make separate off and on PNGs
- on-state glow must remain inside the transparent canvas
```

## Integration Checklist

- Faceplate PNG is exactly `1200 x 2172` for new assets.
- If using an older `1200 x 2260` or cropped legacy asset, verify it still looks correct in-app before replacing it.
- Typography matches the model's `RenderTypographyPolicy`.
- Knobs and LED jewels are separate PNG controls when they rotate or change state.
- Static footswitch hardware may stay baked into the faceplate; keep only the hit area in the UI.
- Run `cargo check -p greybound-ui`, `cargo check -p greybound-free`, and `cargo check -p greybound-plugin` after wiring assets.
