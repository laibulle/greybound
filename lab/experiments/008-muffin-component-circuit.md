# Muffin Component-Circuit Candidate

## Objective

Replace the scalar Muffin fuzz approximation with a bounded component-circuit
candidate, then establish a repeatable lab baseline before fitting it to a
matching reference.

## Candidate

- Four incremental BJT common-emitter stages with explicit emitter-bypass and
  collector-capacitor state.
- Two finite-source antiparallel silicon diode Shockley solves.
- Four-node trapezoidal MNA Muffin tone stack with Q3 source and Q4 base load.
- 117 kOhm input and 25 kOhm output electrical boundaries.

The source of truth is `core/src/pedal/muffin.rs`; the semantic topology is in
`core/src/circuit_descriptor.rs::MUFFIN_CIRCUIT`, with the renderer graph in
`knowledge/models/pedals/fuzz/diagrams/muffin.diagram.json5`.

## NAM Reference

The first direct NAM anchor is a local-only, public TONE3000 A2 pedal capture:

- reference manifest: `lab/references/nam/manifests/big-muff-v3-58283.json`
- source page: `https://www.tone3000.com/tones/big-muff-made-in-usa-58283`
- selected model: `BIG MUFF V3 T12 Sfull -18db`
- topology: Big Muff Pi V3 pedal only; no amp, cab, or IR
- fixed reference DI: `lab/references/tone3000-inputs/Mayer - Guitar.wav`

This reference is close enough to validate the BJT Muff family, but it is not
evidence that the Ram's Head/Violet component hypothesis has the same values as
the 1990s V3 capture.

## Fixed Render Conditions

```sh
cargo build -p greybound-cli --release

target/release/greybound-cli \
  --rig rigs/muffin-nam-reference.json5 \
  --input-wav "lab/references/tone3000-inputs/Mayer - Guitar.wav" \
  --output-wav lab/renders/muffin-component-v3-t12-sfull-3s.wav \
  --render-seconds 3 --sample-rate 48000 --period-size 16 --output-db 0 \
  --input-db -18
```

`muffin-nam-reference` bypasses amp and cab because the NAM is pedal-only. The
pedal level is `0.12`, the sustain is full, and the tone position is noon.

## Scalar Regression Diagnostic

The previous HEAD implementation was rendered from a detached worktree with
the same rig, DI, IR, rate, duration, and output gain. The lab comparison was:

```sh
uv --project lab run greybound-lab compare-wav \
  --candidate lab/renders/muffin-nox-component-circuit-3s.wav \
  --reference lab/renders/muffin-nox-scalar-baseline-3s.wav \
  --report lab/reports/muffin-nox-component-circuit-vs-scalar-baseline-3s.md \
  --metadata lab/renders/muffin-nox-component-circuit-vs-scalar-baseline-3s.run.json
```

| Metric | Result |
| --- | ---: |
| Alignment | -3 samples / -0.062 ms |
| Gain correction | -2.71 dB |
| Weighted guitar-band LSD | 9.33 dB |
| Null residual relative to scalar baseline | -3.14 dB |
| Envelope error | -6.68 dB |

This is a regression/difference measurement, not a quality score. The new
candidate is materially brighter than the scalar baseline above 4 kHz and
needs a matching reference before component values or anti-alias filtering are
tuned.

## NAM Iteration

The A2 render is local-only and rendered through the optional PyTorch/NAM tool
environment, leaving the lightweight lab dependencies unchanged:

```sh
uv --project lab run greybound-lab render-nam \
  --model "lab/references/nam/BigMuffV3/BIG MUFF V3 T12 Sfull -18db.nam" \
  --input-wav "lab/references/tone3000-inputs/Mayer - Guitar.wav" \
  --output-wav lab/references/nam/renders/big-muff-v3-t12-sfull-3s.wav \
  --metadata lab/references/nam/renders/big-muff-v3-t12-sfull-3s.run.json \
  --renderer-command 'uv --project lab run --with torch --with neural-amp-modeler python lab/scripts/nam_a2_render.py --model {model} --input {input_wav} --output {output_wav} --sample-rate {sample_rate} --seconds {render_seconds} --input-db {input_db} --output-db {output_db}' \
  --render-seconds 3 --sample-rate 48000 --input-db -18 --output-db 0

uv --project lab run greybound-lab evaluate-wav \
  --candidate lab/renders/muffin-component-v3-t12-sfull-3s.wav \
  --reference lab/references/nam/renders/big-muff-v3-t12-sfull-3s.wav \
  --report lab/reports/muffin-component-v3-t12-sfull-vs-nam-evaluation-3s.md \
  --json lab/reports/muffin-component-v3-t12-sfull-vs-nam-evaluation-3s.json \
  --profile clipper
```

The initial rail-unbounded candidate was `severe`: it was numerically clipping
and could not be judged against NAM. Replacing infeasible Q2/Q3 operating
currents with 45 uA, adding 9 V collector headroom, and rendering the nonlinear
circuit at 2x removed the numerical failure. A fixed tone sweep then showed
that noon retained too much high-frequency energy. Increasing the working
collector smoothing capacitors from 220 pF to 2.2 nF reduced the noon-setting
weighted guitar-band LSD from `27.63 dB` to `19.37 dB` and maximum
gain-normalized spectral drift from `25.05 dB` to `12.30 dB`.

The final run is `warning`, not promotion-quality proof: it has zero near/hard
clips, `-0.83 dB` gain correction, `-8.09 dB` null residual relative to NAM,
and remaining presence/air excess. A matching Ram's Head/Violet SPICE fixture
or hardware capture with Q1-Q4 probe points is the next component-exact gate.
