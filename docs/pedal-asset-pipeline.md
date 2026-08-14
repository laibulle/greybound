# Industrial Pedal Asset Pipeline

This is the only supported way to create or regenerate a Greybound pedal.
It is designed so that an image model is never responsible for exact geometry,
typography, interactive hardware, or integration.

## The boundary

An image model generates **one thing only**: an empty enclosure faceplate,
with optional model wordmark. It does not generate knobs, holes, tick marks,
labels, footswitches, LEDs, jack sockets, screws, shadows, or a background.

Greybound owns every functional element. The renderer applies the canonical
hardware assets and UI typography at normalized anchors from the model render
spec. This makes their placement, size, ink, on/off state, and click target
deterministic. It also means labels can never be misspelled, hidden behind a
knob, or detached from their control.

The model-owned recipe is `PedalAssetGenerationSpec`, colocated with its
`ModelRenderSpec` in `ui/src/lib.rs`. It contains the revision, wordmark,
destination, and art direction; `RenderControlSpec` is the single source of
truth for all hardware anchors, assets, rotation pivots, labels, and hit areas.

## Regenerate from scratch

From `greybound/`:

```bash
# Export the exact machine-readable contract and a construction guide for every pedal.
cargo run -p xtask -- pedal-assets export docs/generated/pedal-asset-contracts

# Print the self-contained image-model prompt for one model.
cargo run -p xtask -- pedal-assets prompt auralith
```

Give the generated `<model>.construction.svg` to the image model only as a
construction reference and use the printed prompt verbatim. Generate an empty
faceplate. Do not ask the model to reproduce the pink/yellow guide geometry.

The guide is deliberately generated **before** any AI call from the exact
model control anchors. It is the deterministic mechanical layout. After a
candidate passes validation, create a build artifact that superimposes its
reserved knob, label, jack, LED, and footswitch zones:

```bash
cargo run -p xtask -- pedal-assets preview <model-id> \
  ui/assets/pedals/<model>@4x.png \
  target/<model>-layout-proof.png
```

This is not an image-model suggestion: it is the same source contract the UI
uses at runtime. The final app independently composites the real control
assets at these locations, so AI can neither move nor erase a functional part.

Use ImageGen for the faceplate. Generate it against a flat chroma-key field,
remove that field with the standard helper, then validate the candidate before
it can replace an application asset:

```bash
python "${CODEX_HOME:-$HOME/.codex}/skills/.system/imagegen/scripts/remove_chroma_key.py" \
  --input <candidate-with-flat-key.png> \
  --out ui/assets/pedals/<model>@4x.png \
  --auto-key border --soft-matte --transparent-threshold 12 --opaque-threshold 220 --despill

cargo run -p xtask -- pedal-assets validate <model-id> ui/assets/pedals/<model>@4x.png
cargo test -p greybound-ui
```

`validate` rejects wrong dimensions, non-transparent canvas corners, visible
content outside the enclosure contract, missing enclosure pixels, and invalid
control/label geometry. It does not accept resizing or cropping as a repair.
Regenerate on failure.

## Acceptance gate

A candidate is eligible only when all of the following are true:

- It is a `1200 × 2172` RGBA PNG with transparent corners.
- Its visible enclosure stays inside `x=72..1128`, `y=72..2100`.
- It is an orthographic empty faceplate: no baked functional hardware or
  control typography.
- The model wordmark is the only optional raster text. Editable control labels
  are rendered by Greybound.
- `pedal-assets validate`, UI tests, and an in-app visual pass at minimum,
  default, and maximum zoom pass.

## Why this is stricter than a better prompt

Prompting alone cannot guarantee physical alignment or text. The pipeline
therefore makes the untrusted part deliberately small: AI supplies material,
finish, and non-functional visual identity; code supplies the product’s
mechanical and semantic layer. No manual retouching or one-off coordinate
patches are permitted.
