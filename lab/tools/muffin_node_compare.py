#!/usr/bin/env python3
"""Compare matching AC-domain Muffin nodes from ngspice and Greybound.

The SPICE traces contain absolute, biased transistor voltages; the Rust model
contains only the AC audio signal at the corresponding component boundaries.
For each SPICE node this tool removes its mean over the settled window, then
compares RMS and peak with the Rust report. It intentionally does not claim a
comparison of base/emitter DC operating points.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


NODES = (
    "input_rs",
    "q1_c",
    "sustain_wiper",
    "q2_c",
    "q3_c",
    "tone_wiper",
    "q4_c",
    "output",
)


def parse_spice_trace(
    path: Path, settle_s: float, fundamental_hz: float
) -> dict[str, tuple[float, float, list[float]]]:
    times = []
    samples = [[] for _ in NODES]
    for raw_line in path.read_text().splitlines():
        values = raw_line.split()
        if len(values) != len(NODES) * 2:
            continue
        parsed = [float(value) for value in values]
        time_s = parsed[0]
        if time_s < settle_s:
            continue
        times.append(time_s)
        for index in range(len(NODES)):
            samples[index].append(parsed[index * 2 + 1])

    if not samples[0]:
        raise ValueError(f"no settled samples in {path}")

    result = {}
    for node, values in zip(NODES, samples):
        mean = sum(values) / len(values)
        ac_values = [value - mean for value in values]
        rms = math.sqrt(sum(value * value for value in ac_values) / len(ac_values))
        peak = max(abs(value) for value in ac_values)
        sine = [0.0] * 8
        cosine = [0.0] * 8
        # Each wrdata pair carries the same transient time. No phase
        # alignment is assumed between the two implementations.
        for time_s, value in zip(times, values):
            phase = math.tau * fundamental_hz * time_s
            for index in range(8):
                sine[index] += value * math.sin(phase * (index + 1))
                cosine[index] += value * math.cos(phase * (index + 1))
        harmonic_peak = [
            2.0 * math.hypot(sine[index], cosine[index]) / len(values)
            for index in range(8)
        ]
        result[node] = (rms, peak, harmonic_peak)
    return result


def ratio_db(actual: float, reference: float) -> float | None:
    if actual <= 1.0e-12 or reference <= 1.0e-12:
        return None
    return 20.0 * math.log10(actual / reference)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--spice", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--settle-s", type=float, default=0.050)
    parser.add_argument("--fundamental-hz", type=float, default=1000.0)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    spice = parse_spice_trace(args.spice, args.settle_s, args.fundamental_hz)
    rust_report = json.loads(args.rust.read_text())
    rust_nodes = rust_report["nodes"]
    rows = []
    for node in NODES:
        spice_rms, spice_peak, spice_harmonics = spice[node]
        rust = rust_nodes[node]
        rust_rms = float(rust["rms_v"])
        rust_peak = float(rust["peak_v"])
        rows.append(
            {
                "node": node,
                "spice_rms_v_ac": spice_rms,
                "rust_rms_v_ac": rust_rms,
                "rms_delta_db": ratio_db(rust_rms, spice_rms),
                "spice_peak_v_ac": spice_peak,
                "rust_peak_v_ac": rust_peak,
                "peak_delta_db": ratio_db(rust_peak, spice_peak),
                "spice_harmonics_peak_v": spice_harmonics,
                "rust_harmonics_peak_v": rust["harmonics_peak_v"],
                "harmonics_delta_db": [
                    ratio_db(float(actual), float(reference))
                    for actual, reference in zip(
                        rust["harmonics_peak_v"], spice_harmonics
                    )
                ],
            }
        )

    report = {
        "schema_version": 1,
        "comparison": "ac_boundary_rms_and_peak",
        "spice_trace": str(args.spice),
        "rust_report": str(args.rust),
        "settle_s": args.settle_s,
        "fundamental_hz": args.fundamental_hz,
        "notes": [
            "SPICE DC offsets are removed before calculating its metrics.",
            "This compares AC-coupled audio boundaries only; it does not validate BJT DC bias points.",
            "Raw samples are intentionally not phase-aligned because the Rust model is oversampled.",
        ],
        "nodes": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
