# Muffin — tone topology repair and spectral validation

## Finding and implementation

The first node comparison found a 3.5–7 dB deficit after Q3. The Q4 recovery
gain itself was already close to SPICE; the defect was in the passive tone
network. The old four-node solve stamped the high-path resistor (`R18`) from
the high-capacitor node to ground. In the circuit it is in series:

```text
Q3 collector -> C10 -> R18 -> top of Tone pot -> wiper
```

`MuffinToneStack` is now a five-node MNA solve: input, low branch,
high-capacitor output, high pot terminal, and wiper. A Rust regression test
holds the V3 noon 1 kHz transfer at 0.28–0.31 V/V; ngspice is 0.2929 V/V.

## SPICE/Rust boundary comparison

Common conditions: 1 kHz, Sustain full, Tone noon, Volume full, Wicker off,
10 kOhm source. The entries below are Rust RMS minus SPICE RMS in dB.

| Profile | Stimulus | Input | Q1 C | Sustain | Q2 C | Q3 C | Tone | Q4 C | Output |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| V3 | 40 mVpk | -0.33 | +1.88 | +1.90 | +0.65 | +0.19 | +0.12 | -0.27 | -0.27 |
| Ram's Head | 40 mVpk | -0.26 | +1.41 | +1.79 | +0.65 | +0.09 | +0.69 | -0.56 | -0.55 |
| Green Russian | 40 mVpk | -0.46 | +1.91 | +2.15 | +1.14 | +0.35 | +0.02 | -1.39 | -1.40 |
| Triangle | 40 mVpk | -0.30 | +1.04 | +1.43 | +0.60 | +0.01 | +0.29 | -0.72 | -0.72 |
| V3 | 200 mVpk | -0.36 | +2.23 | +2.25 | +0.42 | +0.16 | +0.09 | -0.30 | -0.35 |
| Ram's Head | 200 mVpk | -0.28 | +1.55 | +1.98 | +0.57 | -0.17 | +0.46 | -0.82 | -0.80 |
| Green Russian | 200 mVpk | -0.49 | +2.26 | +2.55 | +0.95 | -0.13 | -0.45 | -1.89 | -1.89 |
| Triangle | 200 mVpk | -0.32 | +1.12 | +1.56 | +0.70 | -0.39 | -0.05 | -1.08 | -1.14 |

The first-stage errors remain a bias-model limitation. More importantly, the
nonlinear clipping boundary (Q3) is within 0.4 dB for all eight checks.
Downstream error remains within 1.5 dB except for the Green Russian recovery
at 200 mVpk (-1.89 dB), which is retained as an explicit Q4 approximation
limit rather than corrected with a hidden voice gain.

This table was regenerated after splitting every ngspice `alterparam` into one
assignment per line. ngspice silently ignored the trailing assignments in the
old multi-assignment form, so those traces could not prove the intended voice
component values. The fixture now changes each resistor, capacitor, and drive
level explicitly before `reset`.

### Presence-band sweep

Full Sustain and noon Tone were additionally run at 3 kHz and 6 kHz. Entries
are Rust minus SPICE RMS in dB at Q3 / Tone wiper / output:

| Profile | 3 kHz | 6 kHz |
| --- | --- | --- |
| V3 | +0.23 / -0.42 / -0.86 | +0.36 / -0.34 / -0.84 |
| Ram's Head | +0.29 / +0.34 / -1.04 | +0.30 / +0.32 / -1.03 |
| Green Russian | +0.60 / -1.03 / -2.32 | +1.36 / -0.46 / -1.74 |
| Triangle | +0.15 / -0.27 / -1.24 | +0.28 / -0.22 / -1.28 |

This rules out the former wiring error across the presence band. The residual
high-frequency slope comes from treating Q3 as a finite 10 kOhm Thevenin
source rather than a fully coupled nonlinear collector/load solve. It reaches
-2.32 dB at the Green Russian 3 kHz output; it is kept explicit rather than
hidden by a voice-specific EQ correction.

### V3 harmonic projection, 40 mVpk

Peak component voltages, fundamental / third / fifth:

| Node | SPICE | Rust |
| --- | --- | --- |
| Q3 C | 0.5169 / 0.1611 / 0.0892 | 0.5248 / 0.1658 / 0.0940 |
| Tone wiper | 0.1511 / 0.0421 / 0.0229 | 0.1537 / 0.0402 / 0.0223 |
| Output | 0.5380 / 0.1497 / 0.0819 | 0.5233 / 0.1369 / 0.0758 |

This confirms that the correction fixes both the fundamental and the clipped
odd-harmonic transfer rather than applying an output gain compensation.

### V3 control end points

The same node comparison was run for the quiet/middle Sustain range and the
two Tone ends at 40 mVpk. At Sustain 0, the downstream error is +1.92 dB at
Q3 and +1.53 dB at output; at Sustain 50%, it is +0.29 dB and -0.15 dB. With
Sustain full, Tone 0/100% gives respectively -1.44/-0.58 dB at output. The
controls are continuous and circuit-consistent in the SPICE domain; the
low-Sustain NAM limitation below is an external-reference issue, not a silent
state reset or a numerical glitch.

## Audio spectral validation

The V3 was rendered with the fixed 3 s Mayer DI, V3 T12/full-Sustain NAM
capture, pedal-direct rig, 48 kHz, Tone noon, Sustain full, and no cab. The
before render uses the same fixed input and rig conditions from experiment 008.

| Metric against V3 NAM | Before topology repair | After repair |
| --- | ---: | ---: |
| Log-spectral distance | 22.83 dB | 13.63 dB |
| Weighted guitar-band LSD | 19.37 dB | 11.89 dB |
| Maximum spectral-balance drift | 12.30 dB | 1.16 dB |
| Harmonic fingerprint max error | 2.16 dB | 1.14 dB |
| Global alias residual near Nyquist | n/a | -122.22 dBFS |
| Hard clips | n/a | 0 |

The residual spectrogram shows the remaining error as broad, programme-linked
energy predominantly below about 4 kHz; it does not show a stationary
near-Nyquist stripe or impulsive glitch band. The candidate still has two NAM
warnings: 2.25 ms mean guitar-band group-delay deviation and 1.51 nonlinear
transfer-shape deviation. Those should not be fitted by changing the validated
tone topology.

The render was regenerated from the current source, not inferred from an older
WAV. That pass exposed a polyphase hand-off error: returning the first of the
two decimator clocks yields a severe 28.48 dB weighted LSD despite a deceptively
flat band balance. The implementation now advances both oversampled circuit
steps and returns the second clock, which is the host-rate phase. Under the
same fixed render it restores 11.89 dB weighted LSD, 1.16 dB maximum balance
drift, and -122.22 dBFS near-Nyquist residual energy. The checked-in local
render/report pair was refreshed with that result.

The same source was rendered through the other two public V3 NAM captures:

| NAM control | Rust Sustain | Verdict | Guitar-band LSD | Max balance drift | Aliasing residual |
| --- | ---: | --- | ---: | ---: | ---: |
| T12/full | 1.00 | warning | 11.89 dB | 1.16 dB | -122.22 dBFS |
| T12/12 o'clock | 0.50 | warning | 14.28 dB | 2.05 dB | -133.74 dBFS |
| T12/8 o'clock | 0.10 | severe | 17.07 dB | 19.41 dB | -156.73 dBFS |

The 8-o'clock capture itself is 38.53 dB darker in the 8–18 kHz band than the
full-Sustain capture after gain alignment, while its NAM metadata reports a
different loudness. Neither the shared V3 SPICE topology nor the public capture
metadata identifies a component or volume change that explains this. It is not
valid to add a hidden low-Sustain low-pass just to fit that unpaired capture.
The correct next evidence is a same-unit DI capture or the actual unit
measurements described below.

## Voice separation on the same guitar DI

These are comparisons to the repaired V3, after latency and gain alignment;
they demonstrate separation, not historical accuracy because there is no
matching hardware/NAM capture for the other three units.

| Voice | Guitar-band LSD vs V3 | Maximum balance delta | Dominant result |
| --- | ---: | ---: | --- |
| Ram's Head | 7.74 dB | 1.58 dB | distinct clipping/dynamics with a close overall EQ |
| Green Russian | 14.26 dB | 10.31 dB | substantially darker presence/air response |
| Triangle | 9.19 dB | 4.24 dB | darker mid/presence response and greater transient level |

The Triangle's fixed V3 reference Volume position can exceed 0 dBFS on the
largest DI transients. This is an output-gain-management issue, not a DSP
instability or a SPICE mismatch: its 200 mV node comparison remains within
1.12 dB at output. Use its physical Level control below the V3 reference
setting until a Triangle hardware level reference is available; do not add a
hidden voice-volume compensation to a circuit model.

## Tone Wicker macro — separate limit

The Wicker implementation is deliberately tested in a separate SPICE fixture:
the three Miller capacitors are opened to the same 5% residual conductance
used by Rust and Q3 is coupled directly to Q4. At 1 kHz / 40 mVpk its direct
path agrees at the meaningful nonlinear boundaries: Q3/tone +0.16 dB, Q4
-0.23 dB, output -0.27 dB. This confirms that the bypass itself is not an
inactive control.

At 3 and 6 kHz, Q3 remains close (+0.27/+0.34 dB) but the simplified Rust Q4
cell is respectively -3.07/-2.97 dB below the fully coupled SPICE recovery
collector. Its host-rate output is lower still because the deliberately
anti-aliased decimator removes high-order Wicker harmonics which analogue
SPICE keeps. This is a known recovery-stage modelling limit, **not** a reason
to add a Wicker-only gain or EQ compensation: Q4 is unchanged by the physical
Wicker switches and such a patch would break the normal-mode validation.

## Remaining evidence needed

Per-unit component tolerances cannot be inferred from this comparison. The
measurement template in `lab/fixtures/muffin-unit-measurement.template.json`
is required to capture the actual Q1–Q4 hFE/Vbe/capacitance, diode curves, pot
tapers, and DC bias points of a specific pedal. Those measurements are the
next justified inputs for reducing the remaining Q1 and phase differences.
Until that JSON is populated from one unit, values presented as its “real
tolerances” would be fabricated rather than measured.
