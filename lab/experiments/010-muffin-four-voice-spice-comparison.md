# Muffin four-voice SPICE comparison

Date: 2026-07-14

## Objective

Replace the former `voicing` control—which only altered a passive tone-stack
pair—with four named component profiles that differ in the gain stages as well
as in the tone network.

The reproducible fixture is:

```bash
ngspice -b tests/fixtures/circuit/muffin_voices.cir
```

It uses the same 40 mV peak / 1 kHz sine through a 10 kOhm source, Sustain at
100%, Tone at noon, Volume open, and writes one transient and one 40 Hz–12 kHz
AC sweep per profile under `/tmp/greybound_muffin_voice_*`.

## Selected schematic families

| Voice | Selected target | Material differences represented in the fixture/runtime |
| --- | --- | --- |
| V3 | 1976/77 red-and-black | 10 k / 150 Ohm clipping stages, 1 uF diode-feedback branches, 470 pF feedback, 39 k / 22 k tone resistors, 15 k / 3.3 k recovery. |
| Ram's Head | 1973 Violet V2 | 15 k / 100 Ohm clipping stages, 8.2 k drive resistors, 100 nF diode-feedback branches, 33 k / 33 k tone resistors, 10 k / 2.2 k recovery. |
| Green Russian | Tall Font V7C | 12 k / 390 Ohm clipping stages, 47 nF diode-feedback branches, 560 pF feedback, 20 k / 22 k tone resistors, 10 k / 2 k recovery. |
| Triangle | early V1 family | 33 k input, 82 k base networks, 390 k feedback, 12 k / 100 Ohm clipping stages, 50 nF diode-feedback branches, 560 pF feedback, 12 k / 2.7 k recovery. |

The component lists are chosen from one documented family per name rather than
averaging production runs. Historical Muffs had substantial unit and revision
variation, so this is a circuit hypothesis—not a claim that every enclosure
with the same artwork contained these values.

Sources used to select the families:

- [ElectroSmash V3 analysis](https://www.electrosmash.com/big-muff-pi-analysis)
- [Aion Violet Ram's Head documentation](https://aionfx.com/app/files/docs/halo_documentation_v1.pdf)
- [Triangle component list](https://www.freestompboxes.org/museum/gaussmarkov.net/layouts/bigmuffpitri/bigmuffpitri-project.pdf)
- [Russian/variant component comparison](https://www.kitrae.net/music/big_muff_guts.html)

## Initial ngspice result

Settled time-domain output at the common 1 kHz, full-Sustain test:

| Voice | RMS | Peak-to-peak |
| --- | ---: | ---: |
| V3 | 0.403 V | 0.911 V |
| Ram's Head | 0.398 V | 0.892 V |
| Green Russian | 0.405 V | 0.915 V |
| Triangle | 0.399 V | 0.895 V |

This intentionally shows why a single 1 kHz, fully clipped test is inadequate:
the output limiter/diode loops converge to similar amplitudes.

The small-signal AC sweep at noon does separate the circuit transfer curves:

| Voice | 100 Hz | 1 kHz | 3 kHz | 6 kHz | 10 kHz |
| --- | ---: | ---: | ---: | ---: | ---: |
| V3 | 60.42 dB | 56.56 dB | 41.48 dB | 26.80 dB | 14.71 dB |
| Ram's Head | 62.37 dB | 59.10 dB | 44.02 dB | 29.67 dB | 17.76 dB |
| Green Russian | 62.23 dB | 58.03 dB | 42.11 dB | 27.15 dB | 14.98 dB |
| Triangle | 63.54 dB | 60.23 dB | 45.13 dB | 30.79 dB | 18.89 dB |

These values are simulation outputs from the fixture, not hardware claims.
They set the required direction for the Rust runtime: the profiles must remain
distinct at low, mid, and high frequencies, rather than only differ by a
small hFE scalar.

## Runtime scope

`Muffin::apply_transistor_voicing` now applies each selected profile to Q1–Q4:
stage input/base/feedback/collector/emitter resistances, diode-feedback and
Miller capacitances, transistor gain/bias, input impedance, and coupling
corners. The regular V3 control value remains `0`; existing Ram's Head and
Green Russian preset values remain `1` and `2`; Triangle is `3`.

The next accuracy gate is stage-node trace comparison of each Rust profile to
its corresponding SPICE fixture at multiple input levels and Tone positions.
