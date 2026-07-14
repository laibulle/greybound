# Muffin — comparison of Rust and SPICE AC boundaries

## Scope

This comparison covers V3, Violet Ram's Head, Tall Font Green Russian, and
Triangle with the same 40 mV peak / 1 kHz source, 10 kOhm source resistance,
Sustain full, Tone noon, Volume full, and Wicker off.

`muffin_voices.cir` models absolute transistor voltages, including their DC
bias. The Rust cell model holds only audio-domain signals. The comparable
nodes are therefore the AC boundaries below, after removing the settled SPICE
mean: `input_rs`, Q1 collector, Sustain wiper, Q2 collector, Q3 collector,
Tone wiper, Q4 collector, and output. This is explicitly **not** a claim that
base, emitter, collector DC operating points match.

## Reproduction

```sh
ngspice -b tests/fixtures/circuit/muffin_voices.cir
cargo build -p greybound-cli
target/debug/greybound-cli --rig rigs/muffin-nam-reference.json5 \
  --muffin-node-report /tmp/muffin-rust-v3.json \
  --diagnostic-frequency 1000 --diagnostic-input-v 0.04 \
  --diagnostic-seconds 0.12 --diagnostic-sustain 1 \
  --diagnostic-tone 0.5 --diagnostic-level 1 --diagnostic-voice 0
python3 lab/tools/muffin_node_compare.py \
  --spice /tmp/greybound_muffin_voice_v3_tran.dat \
  --rust /tmp/muffin-rust-v3.json --output /tmp/muffin-v3-comparison.json
```

Repeat the final two arguments with voice `1`, `2`, `3` and the corresponding
`rams_head`, `green_russian`, `triangle` trace names.

## Initial result

The following is `Rust RMS / SPICE AC RMS`, in dB. Values close to zero are
matched boundary levels; a negative value means Rust is quieter.

| Profile | Input | Q1 C | Sustain | Q2 C | Q3 C | Tone | Q4 C | Output |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| V3 | -0.33 | +1.88 | +1.90 | +0.65 | +0.19 | -6.64 | -7.03 | -7.03 |
| Ram's Head | -0.26 | +1.71 | +1.73 | +0.86 | +0.49 | -3.84 | -4.47 | -4.47 |
| Green Russian | -0.24 | -0.69 | -0.68 | -0.10 | +0.06 | -4.25 | -4.27 | -4.27 |
| Triangle | -0.35 | -0.26 | -0.24 | +0.54 | +0.69 | -3.54 | -4.42 | -4.42 |

## Historical conclusion

The input and three fuzz-stage boundaries were reasonably close for this one
stimulus (within about 2 dB), but the passive Tone and recovery/output
boundaries were not: Rust was 3.5–7 dB too quiet after Q3. This isolated a
topology error: the high-branch resistor had been stamped to ground rather
than in series between the high capacitor and the potentiometer. The fixed
five-node MNA result, hot-drive recheck, and audio spectrum are recorded in
`012-muffin-tone-topology-and-spectral-validation.md`.

Further sweep work remains: low/mid/full Sustain, Tone at both extremes, and
a per-unit hardware calibration. Those are distinct from the resolved
high-branch topology fault.
