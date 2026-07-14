# Muffin V3 — SPICE topology validation

Date: 2026-07-14

## Purpose

Validate the original red-and-black Big Muff Pi V3 signal path before changing
the Rust `Muffin` model.  This experiment deliberately does **not** fit the
TONE3000 NAM captures and does not use a Sustain taper as a substitute for a
circuit topology.

The project-owned fixture is
`tests/fixtures/circuit/muffin_v3.cir`.  It records the topology and values,
not a third-party schematic image.

## Sources and scope

- [ElectroSmash — Big Muff Pi Analysis](https://www.electrosmash.com/big-muff-pi-analysis)
  identifies the target as the 1976/77 American V3 and documents the four
  common-emitter stages, 1N914 feedback clipping and the V3 component list.
- [alberand's V3-based redraw](https://cdn.hackaday.io/files/1787287646698752/schematics.pdf)
  was used only to resolve node connectivity.  It is not stored in this
  repository.

This is a V3 reference circuit, matching the local Big Muff V3 NAM family.  It
does not prove that a Ram's Head/Violet unit has the same values, and it does
not yet model the hFE/capacitances of a particular vintage BC239.

## Reproduce

```bash
ngspice -b tests/fixtures/circuit/muffin_v3.cir
```

The command prints the Q1–Q4 DC points and writes 1 kHz transient traces for
Sustain at 0, 50 and 100 percent to:

```text
/tmp/greybound_muffin_v3_sustain_0.dat
/tmp/greybound_muffin_v3_sustain_50.dat
/tmp/greybound_muffin_v3_sustain_100.dat
```

Test stimulus: 40 mV peak, 1 kHz sine at the input of a 10 kOhm guitar-source
resistance; Tone is at noon and Volume is fully open.

## Topology gate

The verified V3 path is:

```text
Q1 shunt-feedback booster
  -> 100 k Sustain divider + 1 k minimum-stop resistor
  -> Q2 collector-to-base feedback (470 k || 470 p || 1 u + antiparallel diodes)
  -> Q3 collector-to-base feedback (same topology)
  -> passive 3.9 n/22 k and 39 k/10 n tone blend
  -> Q4 15 k / 3.3 k recovery stage
```

The 1 µF capacitors in the Q2/Q3 diode branches are essential: they isolate
the diode feedback branches at DC.  The diodes therefore shape the *AC
feedback error*; they are not diode pairs from the stage output to ground.

## SPICE result

With the documented high-gain BC239 approximation (`BF=300`), the DC points
are stable and do not move with Sustain because the controls are AC-coupled:

| Stage | Base (V) | Collector (V) | Emitter (V) |
| --- | ---: | ---: | ---: |
| Q1 input booster | 0.625 | 7.134 | 0.017 |
| Q2 clip stage | 0.697 | 4.623 | 0.065 |
| Q3 clip stage | 0.697 | 4.623 | 0.065 |
| Q4 recovery | 1.109 | 6.545 | 0.503 |

The values are a simulation baseline, not acceptance voltages for any physical
unit: Q1 especially is sensitive to the selected transistor's gain.  The
important acceptance condition here is a non-saturated DC point and the
correct feedback topology.

After 80 ms of settling, the resulting output is:

| Sustain | Output RMS | Output peak-to-peak | THD (harmonics 2–8) |
| --- | ---: | ---: | ---: |
| 0 % | 179 mV | 484 mV | 4.6 % |
| 50 % | 376 mV | 863 mV | 28.6 % |
| 100 % | 403 mV | 911 mV | 33.2 % |

At Sustain 0, the Q2 collector is only 14 mV RMS while Q3 and Q4 provide the
remaining modest, rounded gain.  That is the expected direction for a creamy
low-gain response: some colour remains because the minimum-stop resistor does
not mute the path, but the double clipping chain is not driven into its plateau.

## Runtime comparison — do not tune yet

`core/src/pedal/muffin.rs` is not presently a V3 topology match:

- Q1/Q2/Q3/Q4 collector and emitter values differ materially from the V3
  fixture (for example, 39 k/390 Ohm and 100 k/390 Ohm in the runtime versus
  V3's 10 k/100 Ohm, 10 k/150 Ohm and 15 k/3.3 k recovery values).
- `SiliconDiodePair` is applied after Q2/Q3 as a ground-referenced source
  clipper.  V3 places each pair in an AC-isolated collector-to-base feedback
  branch.
- `sustain` is currently an empirical multiplier on both clipping-stage inputs.
  V3 uses one passive 100 k divider before Q2, with R6 retaining a finite
  minimum drive.

Therefore adjusting the current Sustain curve, collector capacitors, or NAM
gain to make the low setting sound better would hide a topology defect.  The
next implementation step is a bounded Q2/Q3 feedback-loop solve, followed by
the physical Sustain divider; only then should the Rust stage traces be
compared against this fixture.

## 2026-07-14 implementation follow-up

The runtime now implements that next step in
`core/src/circuit/muffin.rs::MuffinFeedbackClippingStage` and
`core/src/pedal/muffin.rs`:

- Q1–Q4 solve their base, collector and emitter nodes together. Q2/Q3 also
  retain the separate node between C5/C8 and their antiparallel 1N914 pair.
  The 470 kOhm branch, 470 pF Miller capacitor, 1 uF diode-branch capacitor,
  finite transistor beta, and rail-limited collector current are therefore in
  the same KCL system; capacitor state is committed only after convergence.
- Sustain is now the loaded 100 kOhm V3 divider with R6 = 1 kOhm at the low
  stop; it appears only before Q2.
- Q1, Q2/Q3 and Q4 use the V3 10 k/100 Ohm, 10 k/150 Ohm, and 15 k/3.3 kOhm
  stage families respectively; Q4 now includes its documented 470 kOhm
  collector-to-base feedback resistor. The 100 k/390 Ohm and post-stage
  ground-diode hypothesis has been removed.

Focused Rust tests verify that the feedback stage has a finite,
level-dependent transfer, that the pedal remains bounded under hot drive, and
that Sustain 0 has a non-hard-clipped crest factor while 50 and 100 percent
remain distinct.

An amp/cab-bypassed render using the TONE3000 Mayer input at the documented
`-18 dB` calibration and the same V3 rig produced zero xruns and zero hard
clips.  During the first comparable monitor segment, Sustain 0 measured
`-29.6 dBFS RMS` versus `-6.1 dBFS RMS` at Sustain 100.  This verifies that
the physical low stop now substantially reduces drive; it is not yet a claim
of spectral equivalence to the NAM capture.
