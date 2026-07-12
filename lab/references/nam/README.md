# NAM References

This directory is for local Neural Amp Modeler reference renders and metadata.

Do not commit downloaded NAM models, downloaded tone packs, or rendered WAVs
unless their redistribution license is explicit and compatible with the project.
Commit only source-safe metadata and experiment notes.

Preferred reference policy:

1. Use **NAM A2** only.
2. Use an **Amp Head** NAM capture when possible.
3. Render it with the same dry DI used for Greybound.
4. Do not add an IR to the NAM render.
5. Compare it against a Greybound render with cab/IR disabled.

This repository treats the current AC30HWH NAM pack as an amp-head reference
without IR. Speaker and cabinet IR tests are useful, but they are a separate
comparison axis from the NAM amp-core match.

Fallback policy:

1. Use a **Full Rig / Combo** NAM capture only when no suitable amp-head capture
   is available.
2. Do not add an extra Greybound IR to the NAM side.
3. Treat all cab/mic differences as part of the reference mismatch.

## None Star full-rig reference

The local None Star calibration reference is a TONE3000 NAM A2 full-rig capture:

- Tone URL: `https://www.tone3000.com/tones/mesa-boogie-lone-star-mateus-asato-69546`
- Local model path: `NoneStar/Mesa Boogie Lone Star - Mateus Asato.nam`
- Source-safe manifest: `manifests/none-star-mateus-asato-69546.json`
- Policy: `full-rig-embedded-cab`

The `.nam` file is local-only and ignored by git. The manifest records that the
metadata classifies the model as `gear_type: amp_cab` and `tone_type: clean`.
Render it without adding an IR:

```sh
make lab-inspect-none-star-nam
make lab-render-none-star-nam NAM_INPUT_WAV="lab/references/tone3000-inputs/Mayer - Guitar.wav"
```

The NAM side is therefore an end-to-end amp+cab/mic reference. Compare it
against a Greybound None Star full chain with an explicit cab/IR candidate, not
against the raw amp core alone.

## Daybreaker 50 amp-head reference

The Daybreaker 50 clean/edge calibration anchor is the local TONE3000 Dumble
Steel String Singer amp-head pack:

- Tone URL: `https://www.tone3000.com/tones/dumble-steel-string-singer-29285`
- Local model directory: `DumbleSteelStringSinger/`
- Source-safe manifest: `manifests/dumble-steel-string-singer-29285.json`
- Policy: `amp-head-no-ir`
- Priority capture: `Dumble Steel SS Clean`

The pack also contains `Drive 1` and `Drive 2` captures. Use the clean capture
as the initial Daybreaker clean/edge target. Render the NAM without an IR and
compare it to a Daybreaker Greybound rig with the cab disabled; cabinet
selection is a separate validation axis.

```sh
export TONE3000_ACCESS_TOKEN='…'
make lab-download-daybreaker-nam
make lab-inspect-daybreaker-nam
make lab-render-daybreaker-nam NAM_INPUT_WAV="lab/references/tone3000-inputs/Mayer - Guitar.wav"
```

The `.nam` files are local-only and ignored by git. The manifest is the
versioned record of their provenance, metadata, policy, and priority capture.
The download command uses TONE3000's authenticated API; never commit an OAuth
token or pass it through a rig file.

Suggested first search target:

- Provider: TONE3000
- Candidate: https://www.tone3000.com/tones/ac30hwh-6580
- Category: VOX AC30
- Gear filter: Amp Head
- Platform: NAM
- Architecture: A2
- Tone family: clean or edge-of-breakup AC30/Top Boost

The `AC30HWH-6580` page exposes a useful capture grid in model names: Normal
Bright, Top Boost, and Hot Mode variants at gain positions 3, 5, 7, or Full,
with optional Top Cut. Treat that as semi-structured capture semantics, not as a
complete knob schema.

After manually downloading the pack, inspect it with:

```sh
make lab-inspect-nam-pack
```

This writes `manifests/ac30hwh-6580.json`, which is source-safe to commit. The
manifest records the 22 model files, local paths, NAM architecture, sample rate,
training metadata, parsed capture semantics, and the four priority models for
the first comparison pass. The `.nam` files themselves remain ignored by git.
