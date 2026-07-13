# 007 Amp Sag Control Calibration

## Purpose

Give the Nox30 and Daybreaker 50 a repeatable sag contract without pretending
that their power supplies are the same circuit.

The Nox30 is a cathode-biased EL84 design with a shared B+ network. The
Daybreaker 50 is a fixed-bias, high-headroom 6L6 design calibrated against the
local Dumble Steel String Singer Clean NAM. Their UI values must therefore not
be compared as a direct percentage of output compression.

The runtime maps the raw UI control through a model-specific calibration curve,
while keeping the two physical endpoints intact:

| Model | Runtime mapping |
| --- | --- |
| Nox30 | `raw^1.25` — reduces mid-range B+ sensitivity |
| Daybreaker 50 | `raw^0.70` — makes clean 6L6 sag usable earlier |

At their UI defaults this yields approximately `0.37` for Nox30 and `0.30` for
Daybreaker before their distinct circuit solvers consume the value. This aligns
the useful control region without forcing the two amplifier topologies to have
the same compression signature.

## Common programme

At 48 kHz, feed both amps a 110 Hz sine programme:

1. 300 ms at 0.020 amplitude to establish the operating point;
2. 400 ms at the burst amplitude;
3. 400 ms back at 0.020 amplitude.

The output measurements are:

- **compression**: late loud-burst RMS relative to early loud-burst RMS;
- **post-burst recovery**: late low-level RMS relative to early low-level RMS.

The latter deliberately measures the complete audible return, including supply,
bias and coupling-capacitor state. It is not labelled pure sag.

## Control contract

`core/src/amp.rs::nox30_and_daybreaker_sag_ranges_stay_calibrated` protects the
following bands:

| Model / setting | Measurement | Contract |
| --- | --- | --- |
| Nox30 default (`Sag 0.45`), normal burst | compression | within ±0.20 dB |
| Nox30 default (`Sag 0.45`), normal burst | post-burst recovery | 0.20–0.75 dB |
| Daybreaker default (`Sag 0.18`), normal burst | compression | -0.15 to -0.02 dB |
| Daybreaker default (`Sag 0.18`), normal burst | post-burst recovery | 0.00–0.12 dB |
| Daybreaker loud burst, default to maximum Sag | compression / recovery | at least 0.07 / 0.02 dB additional movement |
| Nox30 loud burst, `Sag 0` to `1` | shared B+ rail | 12–20 V drop |

The Nox rail is measured directly rather than inferred from its output. Loud
Nox notes also engage PI grid-charge recovery, which is audible but independent
of the Sag control; treating it as B+ sag would make the control falsely appear
too strong.

## Decision

The Daybreaker stays intentionally stiff at its NAM-calibrated clean default.
Its Sag control has a verified audible range under loud playing, rather than
adding compression at idle or normal clean levels. The Nox retains a somewhat
longer return as part of its cathode-biased voice, while its actual B+ sag is
kept separately measurable and bounded.

Any later attempt to make the knobs numerically identical must use a second,
source-backed target for the Nox30. It must not compensate by breaking the
Daybreaker's NAM clean calibration.
