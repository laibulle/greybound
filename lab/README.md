# Greybound Lab

`lab/` is the offline R&D workspace for Greybound. It is separate from the
real-time engine on purpose: experiments may use slower tools, generated WAV
files, SPICE renders, NAM references, plots, and training artifacts. The runtime
crates should only consume artifacts after they have been reviewed and frozen.

## Setup

The lab is a Python scientific workspace managed with `uv`. From the repository
root:

```sh
uv --project lab sync --dev
uv --project lab run pytest
```

Run the first comparison tool with:

```sh
uv --project lab run greybound-lab compare-wav \
  --candidate lab/renders/nox30-driven.wav \
  --reference lab/references/nox30-reference.wav \
  --segments lab/segments/guitar-chords.markers.json \
  --report lab/reports/nox30-driven-vs-reference.md
```

From inside `lab/`, use `uv run ...` and drop the leading `lab/` path
components.

Render a Greybound rig into the lab with reproducible metadata:

```sh
uv --project lab run greybound-lab render-rig \
  --rig rigs/nox30-driven.json5 \
  --input-wav samples/teenager-electric-guitar-smooth-chords-dry_94bpm_G_major.wav \
  --output-wav lab/renders/nox30-driven.wav \
  --metadata lab/renders/nox30-driven.run.json \
  --render-seconds 10 \
  --sample-rate 44100 \
  --period-size 16 \
  --output-db -18 \
  --ir
```

Generate standard lab stimuli for focused metrics:

```sh
uv --project lab run greybound-lab generate-stimuli \
  --output-dir lab/stimuli \
  --sample-rate 44100
```

Run the first SPICE cell reference:

```sh
uv --project lab run greybound-lab spice-run \
  --fixture common-cathode-12ax7 \
  --output-dir lab/references/spice
```

Write the first local SPICE dataset artifact and manifest:

```sh
uv --project lab run greybound-lab spice-dataset \
  --fixture common-cathode-12ax7 \
  --output-dir lab/datasets/spice
```

This first dataset is a small multi-stimulus corpus. It runs generated SPICE
netlists for several 1 kHz sine levels plus two-tone IMD cases, writes raw
traces, packs a `.npz`, and records hashes, node roles, train/validation/test
splits, component values, and generated netlists. It is useful for the first
trainer/export smoke test, but it is not yet broad enough for final tube-stage
acceptance.

Download public TONE3000 DI input WAV files for local NAM and Greybound
integration tests:

```sh
uv --project lab run greybound-lab download-tone3000-inputs \
  --output-dir lab/references/tone3000-inputs
```

Download public TONE3000 IR WAV files for local cab/reference tests:

```sh
uv --project lab run greybound-lab download-tone3000-irs \
  --output-dir lab/references/tone3000-irs
```

NAM references are imported manually for now. See:

- [004 NAM Reference Comparison](experiments/004-nam-reference-comparison.md)
- [005 AC30HWH NAM A2 First Render](experiments/005-ac30hwh-nam-a2-first-render.md)
- [references/nam/README.md](references/nam/README.md)

Inspect a manually downloaded NAM pack and write a source-safe manifest:

```sh
uv --project lab run greybound-lab inspect-nam-pack \
  --pack-dir lab/references/nam/AC30HWH \
  --manifest lab/references/nam/manifests/ac30hwh-6580.json \
  --tone-url https://www.tone3000.com/tones/ac30hwh-6580
```

Render a NAM model once an external NAM A2 renderer is installed:

```sh
uv --project lab run greybound-lab render-nam \
  --model lab/references/nam/AC30HWH/TopBoost-Gain5.nam \
  --input-wav samples/teenager-electric-guitar-smooth-chords-dry_94bpm_G_major.wav \
  --output-wav lab/references/nam/renders/ac30hwh-6580-topboost-gain5.wav \
  --metadata lab/references/nam/renders/ac30hwh-6580-topboost-gain5.run.json \
  --renderer-command "uv run --python 3.11 --with neural-amp-modeler==0.13.0 --with scipy python lab/scripts/nam_a2_render.py --model {model} --input {input_wav} --output {output_wav} --sample-rate {sample_rate} --seconds {render_seconds} --input-db {input_db} --output-db {output_db}" \
  --sample-rate 48000 \
  --render-seconds 20 \
  --input-db -40 \
  --output-db -24
```

The default Makefile renderer uses the official Python `neural-amp-modeler`
package in a temporary `uv run` environment and the local
`lab/scripts/nam_a2_render.py` adapter. The lab still keeps NAM inference out of
the runtime engine.

## Start Here

The first R&D target is not training. It is measurement.

Before replacing circuit cells with fitted micro-models, we need a repeatable
way to compare:

- Greybound rig renders,
- reference WAV files from NAM or real captures,
- SPICE-generated cell outputs,
- previous Greybound model versions.

The first experiment is:

- [001 Chain Reference Analysis](experiments/001-chain-reference-analysis.md)
- [002 Nox30 Stimulus Batch](experiments/002-nox30-stimulus-batch.md)
- [003 Common-Cathode SPICE Reference](experiments/003-common-cathode-spice-reference.md)
- [004 NAM Reference Comparison](experiments/004-nam-reference-comparison.md)
- [005 AC30HWH NAM A2 First Render](experiments/005-ac30hwh-nam-a2-first-render.md)
- [006 SPICE To Neural Cell Plan](experiments/006-spice-to-neural-cell-plan.md)

These define the minimum useful analysis loop and the first controlled-stimulus
comparison between Greybound rigs, then bridge into the first cell-level SPICE
reference, first NAM A2 integration comparison, and the planned neural-cell
artifact boundary.

## Neural Cell Strategy

The current R&D decision is:

```text
PyTorch trains.
Greybound exports.
Rust runs.
ONNX verifies.
```

Python and PyTorch are the research stack for fitting micro-models from SPICE
datasets. Accepted cells should be exported as versioned Greybound artifacts,
then consumed by a small specialized Rust runtime rather than by a generic
Python or ONNX runtime in the live audio path.

This is still an experimental decision. The first complete benchmark may change
the details if SPICE data quality, export stability, CPU cost, or validation
metrics point in another direction. See
[006 SPICE To Neural Cell Plan](experiments/006-spice-to-neural-cell-plan.md)
for the current disclaimer, architecture contract, and milestone sequence.

## Directory Layout

`experiments/`

: Human-readable experiment plans. These are committed and should explain the
  purpose, protocol, inputs, expected outputs, and decision criteria.

`schemas/`

: JSON schemas for lab metadata. These are committed so generated datasets and
  reports have stable structure. `spice-dataset-manifest.schema.json` describes
  generated SPICE datasets, and `neural-cell-artifact.schema.json` describes the
  planned Greybound neural-cell export descriptor.

`segments/`

: Committed marker files that define named regions for local diagnostics:
  attacks, sustains, sag windows, high-band checks, and future harmonic tests.

`stimuli/`

: Generated synthetic WAV stimuli and marker files for harmonic, intermodulation,
  aliasing, sag, and attack analysis. Ignored by git by default.

`datasets/`

: Generated or imported training datasets. Keep large data out of git unless it
  is tiny, source-safe, and necessary for tests.

`models/`

: Local experimental neural-cell artifacts. Keep descriptors, weights,
  checkpoints, ONNX exports, and generated plots out of git by default unless an
  artifact has been explicitly reviewed and promoted.

`references/`

: External references such as NAM renders, measured pedal captures, or SPICE
  exports. Treat this as local working data unless redistribution rights are
  explicit. Public TONE3000 input WAV files can be downloaded into
  `references/tone3000-inputs/`; the WAVs, generated manifest, and generated
  README remain ignored by git. Public TONE3000 IR WAV files can be downloaded
  into `references/tone3000-irs/` with the same local-only rule.

`renders/`

: Greybound offline WAV renders.

`reports/`

: Generated metric reports, plots, and comparison summaries.

## Lab Rules

- Keep raw third-party captures and generated audio out of git by default.
- Every report should point to a metadata file that describes its inputs.
- Every accepted result should be reproducible from committed code and declared
  local assets.
- Do not require Python, SPICE, or neural tooling in the live Rust runtime.
- Promote only reviewed artifacts into the runtime crates.

## First Implementation Boundary

The first lab tool consumes WAV pairs and produces a Markdown report with:

- sample rate and channel validation,
- gain and latency alignment,
- RMS, peak, and crest factor,
- STFT or log-spectrum distance,
- transient envelope error,
- null residual after alignment,
- optional segment-level diagnostics with `--segments`,
- attack, harmonic, high-band/aliasing, and sag metrics for typed segments,
- band residual metrics for each segment,
- intermodulation metrics for generated two-tone segments,
- short engineering notes for the next model decision.

This gives us a useful baseline before NAM, SPICE, or training choices become
expensive.
