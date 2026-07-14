Circuit fixtures
================

These fixtures are reference targets for the component-level solvers in
`core/src/circuit`. They are intentionally kept outside `core/src/amp` so the
same circuit cells can later be reused by amps, pedals, and utility stages.

`common_cathode_12ax7.cir` is a ngspice starting point for the ECC83/12AX7
common-cathode stage implemented in `circuit::triode`. It writes transient data
to `/tmp/greybound_common_cathode_12ax7.dat`. Use it to compare:

- idle plate voltage, cathode voltage, and B+ sag
- transient gain at 1 kHz with and without cathode bypass
- blocking behavior from the input coupling capacitor and grid leak

The Rust model should eventually load measured or simulated operating points
from these fixtures in regression tests. For now, this file documents the
electrical target while the in-process solver is still evolving.

`muffin_v3.cir` is the stage-by-stage ngspice reference for the 1976/77
red-and-black Big Muff Pi V3 topology.  It writes traces for Sustain at 0,
50%, and 100% to `/tmp/greybound_muffin_v3_sustain_{0,50,100}.dat` and prints
the DC point of Q1–Q4 for each pass.  Its purpose is to validate the original
signal topology before fitting the Rust Muffin model:

- Q1 has collector-to-base shunt feedback and a 470 pF Miller capacitor.
- Sustain is a 100 kOhm divider with the 1 kOhm minimum-stop resistor.
- Q2 and Q3 use antiparallel 1N914s in AC-isolated collector-to-base feedback
  loops; they are not output diodes to ground.
- Q4 is the 15 kOhm / 3.3 kOhm recovery stage after the passive tone blend.

The netlist intentionally treats its BC239 parameters as a high-gain NPN
approximation.  Component values and connectivity are the fixture target;
absolute operating points require a measured transistor or a model fitted to
the particular unit.

`muffin_voices.cir` is the four-profile comparison fixture for V3, Violet
Ram's Head, Tall Font Green Russian, and Triangle. It applies the same input,
Sustain, Tone, and volume conditions to each circuit, writing 40 mV and 200 mV
transient data plus an AC sweep under `/tmp/greybound_muffin_voice_*`; it also
exports 3 kHz and 6 kHz node traces for all voices, plus V3 at Sustain 0/50%
and Tone 0/100%. Its
transient captures
the eight AC signal boundaries (`input_rs`, Q1/Q2/Q3/Q4 collectors, Sustain,
Tone, and output), for direct RMS/peak comparison with the Rust diagnostics.
See
`lab/experiments/010-muffin-four-voice-spice-comparison.md` for the selected
schematic families, component deltas, and measured ngspice table.
Run `lab/tools/muffin_node_compare.py` with the CLI's `--muffin-node-report`
to compare their settled AC RMS/crête per boundary; the procedure and initial
four-voice result are in `lab/experiments/011-muffin-rust-spice-node-comparison.md`.

`muffin_v3_wicker.cir` separately validates the implemented one-control Tone
Wicker macro at 1, 3, and 6 kHz. It opens C2/C6/C9 to the same 5% numerical
residual used by Rust and routes Q3 around the passive Tone stack into Q4.

Current ngspice DC operating point:

- plate: 250.54 V
- cathode: 0.40 V
- B+: 277.32 V
- grid: 0.00 V

Current 1 kHz transient reference with 20 mV sine input:

- input RMS: 14.14 mV
- plate RMS after DC removal: 210.43 mV
- plate gain: 14.88x

`cathode_follower_12ax7.cir` validates the follower cell. It writes transient
data to `/tmp/greybound_cathode_follower_12ax7.dat`.

Current ngspice cathode-follower DC operating point:

- grid: 0.00 V
- cathode: 2.63 V
- B+: 280.00 V

Current 1 kHz transient reference with 20 mV sine input:

- input RMS: 14.14 mV
- grid RMS: 14.14 mV
- cathode RMS after DC removal: 11.79 mV
- cathode gain: 0.834x

`long_tail_pair_12ax7.cir` validates the shared-cathode phase-inverter cell. It
writes transient data to `/tmp/greybound_long_tail_pair_12ax7.dat`.

Current ngspice long-tail-pair DC operating point:

- plate A: 290.37 V
- plate B: 291.94 V
- cathode/tail: 1.95 V
- grid A: 0.00 V
- grid B: 0.00 V
- B+: 300.00 V

Current 1 kHz transient reference with 20 mV sine input on grid A:

- input RMS: 14.14 mV
- grid A RMS: 14.14 mV
- grid B RMS: 0.002 mV
- plate A RMS after DC removal: 97.90 mV
- plate B RMS after DC removal: 41.77 mV
- differential plate RMS after DC removal: 139.67 mV
- differential gain: 9.88x

`none_star_tone_presence.cir` validates the current None Star Clean/Edge
tone-stack and presence hypothesis. It writes AC sweep data to
`/tmp/greybound_none_star_tone_presence.dat`.

This is intentionally not a Mesa/Boogie schematic. It is a project-owned
linearized fixture for the graybox behavior currently supported by the Rust
model: bass/mid/treble branch weighting followed by a presence branch that can
restore high-frequency energy like reduced negative feedback, rather than
acting only as a passive high cut.

Current AC sweep reference at the NAM anchor settings:

- 250 Hz output gain: -7.67 dB
- 1 kHz output gain: -1.12 dB
- 4 kHz output gain: +5.67 dB
- 8 kHz output gain: +7.59 dB
- 16 kHz output gain: +8.64 dB
- presence lift at 8 kHz, output minus tone node: +2.26 dB
- 8 kHz minus 1 kHz output tilt: +8.71 dB
