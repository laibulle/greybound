# Greybound AI Asset Generation

Use this guide when generating or regenerating photorealistic Greybound hardware assets.

> Pedal faceplates now use the mandatory [industrial pedal asset pipeline](./pedal-asset-pipeline.md).
> Do not use the legacy complete-faceplate prompt below for a new or replacement
> pedal: it delegates mechanical placement and labels to the image model. Use
> `cargo xtask pedal-assets prompt <model-id>` and its exported construction
> guide instead. The remaining sections are retained for legacy assets and
> isolated runtime-control generation.

## When To Regenerate

Do not regenerate a working asset only to satisfy the new ratio. Keep Minotaur and Springfield as visual references unless the composition itself must change. Regenerate when:

- the pedal body is visibly distorted in the app
- typography or baked hardware must move
- a future asset needs to be created from scratch
- the source bitmap cannot be cropped without damaging the artwork

## Pedal Faceplate Prompt

Attach `docs/templates/pedal-standard-1200x2172.svg` as the layout reference and the closest existing Greybound render as the style reference. The SVG is only a construction guide; its colored lines and placeholder text must not appear in the final image.

### Shared pedal geometry

The template standardizes the mechanical silhouette only; it deliberately does
not prescribe a knob grid. New pedal designs use:

- canvas: `1200 x 2172 px`, transparent;
- file boundary: the solid grey outline in the template is the exact PNG edge
  at `x=0..1200`, `y=0..2172`; use it to judge the final filled area, but do
  not render it into the asset;
- enclosure body: `x=72`, `y=72`, `1056 x 2028 px`, outer corner radius
  `72 px`, with a uniform `24 px` visible bevel. Its inside edge is
  `x=96`, `y=96`, `1008 x 1980 px`, corner radius `48 px`;
- side I/O jack placeholders: identical `72 x 136 px` rectangles, fully
  outside the front-face boundary. Input occupies `x=0..72`, output
  `x=1128..1200`, both centred at `y=1086`; no part may extend onto the front
  face;
- I/O labels: `IN` has a start-aligned baseline at `(120, 1096)` and `OUT` an
  end-aligned baseline at `(1080, 1096)`; both sit inside the enclosure and
  align vertically with their jack; use a `44 px` source font size;
- footswitch placement: the small unlabelled cross at `(600, 1780)` marks its
  centre only; it is a guide and must not be rendered into the final asset;
- control-label baseline: centred horizontally on its control anchor and placed
  `177 px` below it.

Controls themselves remain model-specific: their number, anchor positions,
sizes, and arrangement are free. The label rule is an offset convention, not a
requirement to draw labels into a faceplate that uses `DrawnByUi` typography.

```text
Create a single photorealistic boutique guitar pedal faceplate as a transparent-background PNG.

Canvas and layout:
- exact canvas: 1200 x 2172 pixels
- straight-on orthographic product render, no perspective tilt
- centered vertical pedal enclosure, rounded corners, realistic bevels
- use a uniform 24 px enclosure bevel; the outer corner radius is 72 px and
  the bevel's inside edge has a 48 px radius
- no external background, no floor, no cast shadow outside the transparent canvas
- body must fill the safe body area without touching the canvas edges
- preserve circular hardware proportions; do not stretch the footswitch, jacks, LED bezel, or knob holes
- use the shared pedal geometry from the attached template: side I/O jacks use
  the fixed rectangles and the small footswitch cross marks its centre, while
  controls may be arranged freely

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
