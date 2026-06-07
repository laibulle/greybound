# 004 NAM Reference Comparison

Status: planned

## Purpose

Use NAM as an integration oracle for the complete Greybound signal path.

The goal is not to copy NAM internally. The goal is to compare Greybound against
a high-realism capture at fixed settings, then use our metrics to decide which
subsystems need work.

## Reference Choice

Preferred reference:

- TONE3000 `VOX AC30` category,
- gear type: `Amp Head`,
- platform: `NAM`,
- clean or edge-of-breakup / Top Boost style capture,
- rendered with the same DI and the same cabinet IR as the Greybound comparison.

Fallback reference:

- TONE3000 `VOX AC30` `Full Rig / Combo`,
- rendered without an extra IR,
- compared against Greybound with IR enabled,
- marked as cab/mic-confounded.

TONE3000 documents the important distinction: `Amp Head` captures need a
separate IR, while `Full Rig / Combo` captures already include the speaker cab.
For Greybound R&D, amp-head NAM is the cleaner reference because it lets us hold
the cab IR constant.

## Initial Candidate Search

Public TONE3000 pages confirm:

- there is a `VOX AC30` category,
- the category exposes `Full Rig`, `Amp Head`, `Pedal`, `Outboard`, and `IR`
  filters,
- TONE3000 has many VOX-family NAM captures,
- some public pages mention VOX AC30 full-rig captures and VOX-style amp-head
  examples.

First manual search:

```text
https://www.tone3000.com/categories/makes/VOX%2BAC30
```

Search/filter criteria:

- gear: `Amp Head`,
- platform: `NAM`,
- tags or title: `AC30`, `Top Boost`, `clean`, `edge`, `breakup`,
- avoid captures that include boost pedals unless explicitly needed.

If the best exact AC30 result is full-rig only, keep it as a fallback but do not
use it as the first diagnostic reference for amp-stage tuning.

## Render Protocol

Use the existing dry guitar sample first:

```text
samples/teenager-electric-guitar-smooth-chords-dry_94bpm_G_major.wav
```

Required render settings:

- sample rate: `44100 Hz`,
- no normalization after rendering,
- no limiter,
- record/render enough duration to cover the complete sample,
- export mono WAV if possible,
- document host/plugin/tool,
- document NAM tone URL and creator,
- document whether the capture includes cab.

For amp-head NAM:

- load NAM amp-head capture,
- load the same Greybound IR after NAM,
- render to `lab/references/nam/<reference-id>.wav`,
- metadata `ir_policy`: `amp-head-plus-greybound-ir`.

For full-rig NAM:

- load NAM full-rig capture,
- do not add Greybound IR,
- render to `lab/references/nam/<reference-id>.wav`,
- metadata `ir_policy`: `full-rig-no-extra-ir`.

## Metadata

Create a metadata file matching:

```text
lab/schemas/nam-reference.schema.json
```

Suggested path:

```text
lab/references/nam/<reference-id>.json
```

Do not commit downloaded model files or rendered WAVs unless redistribution
rights are explicit.

## Comparison Command

Render Greybound:

```sh
uv --project lab run greybound-lab render-rig \
  --rig rigs/nox30-driven.json5 \
  --input-wav samples/teenager-electric-guitar-smooth-chords-dry_94bpm_G_major.wav \
  --output-wav lab/renders/nox30-driven-for-nam.wav \
  --metadata lab/renders/nox30-driven-for-nam.run.json \
  --render-seconds 20 \
  --sample-rate 44100 \
  --period-size 16 \
  --output-db -18 \
  --ir
```

Compare:

```sh
uv --project lab run greybound-lab compare-wav \
  --candidate lab/renders/nox30-driven-for-nam.wav \
  --reference lab/references/nam/<reference-id>.wav \
  --metadata lab/renders/nox30-driven-for-nam.run.json \
  --segments lab/segments/guitar-chords.markers.json \
  --report lab/reports/nox30-driven-vs-nam-<reference-id>.md \
  --max-lag-ms 200
```

## Decision Criteria

The NAM comparison is useful if it identifies one or more dominant gaps:

- gain staging / compression,
- harmonic or IMD mismatch,
- attack overshoot mismatch,
- sag/recovery mismatch,
- stable band residual pointing to tone stack or cab/IR,
- high-band mismatch pointing to anti-aliasing or nonlinear top-end behavior.

The comparison is not useful if:

- the reference is full-rig and cab/mic differences dominate every metric,
- the input gain is unknown or clipped,
- post-processing, limiter, normalization, or room/reverb is present,
- the NAM reference is a stylistic preset rather than a close AC30-like amp
  capture.

## Open Question

If no high-quality AC30 amp-head NAM is available, decide whether to:

- use a close VOX-family amp-head capture such as AC15 Top Boost,
- use an AC30 full-rig only for broad end-to-end sanity,
- or capture/train our own reference later.
