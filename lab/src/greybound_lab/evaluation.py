from __future__ import annotations

from dataclasses import asdict, dataclass
import json
from pathlib import Path
from typing import Iterable

import numpy as np

from greybound_lab.metrics import ComparisonMetrics


@dataclass(frozen=True)
class EvaluationGate:
    name: str
    status: str
    value: float
    warning: float
    severe: float
    unit: str
    note: str


@dataclass(frozen=True)
class EvaluationResult:
    profile: str
    verdict: str
    gates: tuple[EvaluationGate, ...]
    near_clip_count: int
    hard_clip_count: int


def evaluate_metrics(
    metrics: ComparisonMetrics,
    candidate_samples: np.ndarray,
    *,
    profile: str = "amp-tone",
) -> EvaluationResult:
    candidate = np.asarray(candidate_samples, dtype=np.float64)
    near_clip_count = int(np.count_nonzero(np.abs(candidate) >= 0.95))
    hard_clip_count = int(np.count_nonzero(np.abs(candidate) >= 0.999))

    gates = [
        _upper_gate("candidate_peak", metrics.candidate.peak_dbfs, -1.0, -0.1, "dBFS", "raw candidate peak level"),
        _upper_gate("hard_clip_samples", float(hard_clip_count), 0.0, 0.0, "samples", "samples at or above 0.999 FS"),
        _upper_gate("near_clip_samples", float(near_clip_count), 32.0, 512.0, "samples", "samples at or above 0.95 FS"),
        _upper_gate("candidate_dc_mean", metrics.candidate.mean_dbfs, -60.0, -40.0, "dBFS", "raw candidate DC offset"),
        _upper_gate("aligned_dc_delta", metrics.dc_offset_delta_db, -70.0, -50.0, "dBFS", "candidate/reference DC mismatch"),
        _upper_gate("gain_correction_abs", abs(metrics.gain_db), 6.0, 12.0, "dB", "alignment gain correction magnitude"),
    ]
    gates.extend(_noise_floor_gates(metrics))
    gates.extend(_profile_gates(metrics, profile))

    verdict = _worst_status(gate.status for gate in gates)
    return EvaluationResult(
        profile=profile,
        verdict=verdict,
        gates=tuple(gates),
        near_clip_count=near_clip_count,
        hard_clip_count=hard_clip_count,
    )


def write_evaluation_report(
    path: Path,
    *,
    candidate_path: Path,
    reference_path: Path,
    metrics: ComparisonMetrics,
    result: EvaluationResult,
    metadata_path: Path | None = None,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    metadata_line = str(metadata_path) if metadata_path else "not provided"
    path.write_text(
        _render_evaluation_markdown(
            candidate_path=candidate_path,
            reference_path=reference_path,
            metadata_line=metadata_line,
            metrics=metrics,
            result=result,
        ),
        encoding="utf-8",
    )


def write_evaluation_json(path: Path, *, metrics: ComparisonMetrics, result: EvaluationResult) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "profile": result.profile,
                "verdict": result.verdict,
                "metrics": {
                    "latency_samples": metrics.latency_samples,
                    "latency_ms": metrics.latency_ms,
                    "gain_db": metrics.gain_db,
                    "candidate_rms_dbfs": metrics.candidate.rms_dbfs,
                    "candidate_peak_dbfs": metrics.candidate.peak_dbfs,
                    "candidate_crest_db": metrics.candidate.crest_db,
                    "candidate_mean_dbfs": metrics.candidate.mean_dbfs,
                    "null_relative_db": metrics.null_relative_db,
                    "log_spectral_distance_db": metrics.log_spectral_distance_db,
                    "weighted_log_spectral_distance_db": metrics.weighted_log_spectral_distance_db,
                    "spectral_balance_max_abs_delta_db": metrics.spectral_balance.max_abs_delta_db,
                    "spectral_balance_low_delta_db": metrics.spectral_balance.low_delta_db,
                    "spectral_balance_low_mid_delta_db": metrics.spectral_balance.low_mid_delta_db,
                    "spectral_balance_mid_delta_db": metrics.spectral_balance.mid_delta_db,
                    "spectral_balance_presence_delta_db": metrics.spectral_balance.presence_delta_db,
                    "spectral_balance_air_delta_db": metrics.spectral_balance.air_delta_db,
                    "dynamics_candidate_p10_dbfs": metrics.dynamics.candidate_p10_dbfs,
                    "dynamics_candidate_p50_dbfs": metrics.dynamics.candidate_p50_dbfs,
                    "dynamics_candidate_p90_dbfs": metrics.dynamics.candidate_p90_dbfs,
                    "dynamics_reference_p10_dbfs": metrics.dynamics.reference_p10_dbfs,
                    "dynamics_reference_p50_dbfs": metrics.dynamics.reference_p50_dbfs,
                    "dynamics_reference_p90_dbfs": metrics.dynamics.reference_p90_dbfs,
                    "dynamics_p10_delta_db": metrics.dynamics.p10_delta_db,
                    "dynamics_p50_delta_db": metrics.dynamics.p50_delta_db,
                    "dynamics_p90_delta_db": metrics.dynamics.p90_delta_db,
                    "dynamics_range_delta_db": metrics.dynamics.dynamic_range_delta_db,
                    "dynamics_max_abs_percentile_delta_db": metrics.dynamics.max_abs_percentile_delta_db,
                    "level_response_active_windows": metrics.level_response.active_windows,
                    "level_response_quiet_gain_delta_db": metrics.level_response.quiet_gain_delta_db,
                    "level_response_mid_gain_delta_db": metrics.level_response.mid_gain_delta_db,
                    "level_response_loud_gain_delta_db": metrics.level_response.loud_gain_delta_db,
                    "level_response_slope_delta_db": metrics.level_response.slope_delta_db,
                    "level_response_max_abs_delta_db": metrics.level_response.max_abs_delta_db,
                    "phase_mean_abs_group_delay_delta_ms": metrics.phase.mean_abs_group_delay_delta_ms,
                    "phase_max_abs_group_delay_delta_ms": metrics.phase.max_abs_group_delay_delta_ms,
                    "phase_mean_coherence": metrics.phase.mean_coherence,
                    "decay_windows": metrics.decay.decay_windows,
                    "decay_slope_delta_db_per_s": metrics.decay.slope_delta_db_per_s,
                    "decay_late_level_delta_db": metrics.decay.late_level_delta_db,
                    "decay_max_abs_delta": metrics.decay.max_abs_delta,
                    "modulation_depth_delta_db": metrics.modulation.modulation_depth_delta_db,
                    "modulation_envelope_lf_residual_db": metrics.modulation.envelope_lf_residual_db,
                    "global_harmonic_stable_windows": metrics.global_harmonics.stable_windows,
                    "global_harmonic_thd_delta_db": metrics.global_harmonics.thd_delta_db,
                    "global_harmonic_max_abs_delta_db": metrics.global_harmonics.max_abs_delta_db,
                    "global_imd_product_energy_delta_db": metrics.global_imd.product_energy_delta_db,
                    "global_imd_residual_product_dbfs": metrics.global_imd.residual_product_dbfs,
                    "global_imd_chord_smear_delta_db": metrics.global_imd.chord_smear_delta_db,
                    "global_aliasing_near_nyquist_delta_db": metrics.global_aliasing.near_nyquist_delta_db,
                    "global_aliasing_residual_near_nyquist_dbfs": metrics.global_aliasing.residual_near_nyquist_dbfs,
                    "nonlinear_transfer_sample_pairs": metrics.nonlinear_transfer.sample_pairs,
                    "nonlinear_transfer_slope_delta": metrics.nonlinear_transfer.slope_delta,
                    "nonlinear_transfer_curvature_delta": metrics.nonlinear_transfer.curvature_delta,
                    "nonlinear_transfer_asymmetry_delta": metrics.nonlinear_transfer.asymmetry_delta,
                    "nonlinear_transfer_residual_db": metrics.nonlinear_transfer.residual_db,
                    "nonlinear_transfer_max_abs_shape_delta": metrics.nonlinear_transfer.max_abs_shape_delta,
                    "noise_floor_inactive_windows": metrics.noise_floor.inactive_windows,
                    "noise_floor_candidate_p50_dbfs": metrics.noise_floor.candidate_p50_dbfs,
                    "noise_floor_candidate_p90_dbfs": metrics.noise_floor.candidate_p90_dbfs,
                    "noise_floor_reference_p50_dbfs": metrics.noise_floor.reference_p50_dbfs,
                    "noise_floor_reference_p90_dbfs": metrics.noise_floor.reference_p90_dbfs,
                    "noise_floor_p50_delta_db": metrics.noise_floor.p50_delta_db,
                    "noise_floor_p90_delta_db": metrics.noise_floor.p90_delta_db,
                    "transient_count": metrics.transients.transient_count,
                    "transient_peak_delta_db": metrics.transients.peak_delta_db,
                    "transient_crest_delta_db": metrics.transients.crest_delta_db,
                    "transient_high_band_ratio_delta_db": metrics.transients.high_band_ratio_delta_db,
                    "transient_max_abs_delta_db": metrics.transients.max_abs_delta_db,
                    "envelope_error_db": metrics.envelope_error_db,
                    "dc_offset_delta_db": metrics.dc_offset_delta_db,
                    "near_clip_count": result.near_clip_count,
                    "hard_clip_count": result.hard_clip_count,
                },
                "gates": [asdict(gate) for gate in result.gates],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def _profile_gates(metrics: ComparisonMetrics, profile: str) -> list[EvaluationGate]:
    if profile == "regression":
        return [
            _upper_gate("null_relative", metrics.null_relative_db, -60.0, -40.0, "dB", "strict residual regression gate"),
            _upper_gate("weighted_lsd", metrics.weighted_log_spectral_distance_db, 2.0, 6.0, "dB", "weighted spectral regression gate"),
            _upper_gate("spectral_balance_max_abs", metrics.spectral_balance.max_abs_delta_db, 1.0, 3.0, "dB", "max gain-normalized band-balance drift"),
            _upper_gate("dynamics_max_abs_percentile", metrics.dynamics.max_abs_percentile_delta_db, 0.75, 2.0, "dB", "active short-term loudness percentile drift"),
            _absolute_gate("dynamics_range_delta_abs", metrics.dynamics.dynamic_range_delta_db, 1.0, 3.0, "dB", "active short-term dynamic-range drift"),
            *_level_response_gates(metrics, 0.75, 2.0),
            *_global_diagnostic_gates(metrics, phase_ms=(0.20, 0.80), decay=(20.0, 60.0), harmonic=(3.0, 8.0), aliasing=(-90.0, -70.0), transfer=(0.25, 0.75)),
            *_transient_gates(metrics, 1.0, 3.0),
            _upper_gate("envelope_error", metrics.envelope_error_db, -40.0, -24.0, "dB", "strict envelope regression gate"),
        ]
    if profile == "clipper":
        gates = [
            _upper_gate("weighted_lsd", metrics.weighted_log_spectral_distance_db, 12.0, 20.0, "dB", "clipper spectral target gate"),
            _upper_gate("spectral_balance_max_abs", metrics.spectral_balance.max_abs_delta_db, 8.0, 14.0, "dB", "max gain-normalized band-balance drift"),
            _absolute_gate("dynamics_range_delta_abs", metrics.dynamics.dynamic_range_delta_db, 4.0, 8.0, "dB", "active short-term dynamic-range drift"),
            *_level_response_gates(metrics, 4.0, 8.0),
            *_global_diagnostic_gates(metrics, phase_ms=(1.0, 3.0), decay=(80.0, 160.0), harmonic=(8.0, 14.0), aliasing=(-80.0, -60.0), transfer=(1.0, 2.0)),
            *_transient_gates(metrics, 5.0, 9.0),
            _upper_gate("envelope_error", metrics.envelope_error_db, -8.0, -4.0, "dB", "clipper envelope target gate"),
        ]
        for segment in metrics.segments:
            if segment.harmonics is not None:
                gates.append(
                    _upper_gate(
                        f"{segment.name}.thd_delta_abs",
                        abs(segment.harmonics.thd_delta_db),
                        6.0,
                        12.0,
                        "dB",
                        "harmonic distortion shape mismatch",
                    )
                )
            if segment.imd is not None:
                gates.append(
                    _upper_gate(
                        f"{segment.name}.imd_delta_abs",
                        abs(segment.imd.imd_delta_db),
                        6.0,
                        12.0,
                        "dB",
                        "intermodulation shape mismatch",
                    )
                )
            if segment.aliasing is not None:
                gates.append(
                    _upper_gate(
                        f"{segment.name}.residual_high_band",
                        segment.aliasing.residual_high_band_dbfs,
                        -90.0,
                        -70.0,
                        "dBFS",
                        "high-band residual on aliasing stress segment",
                    )
                )
        return gates
    if profile == "amp-tone":
        return [
            _upper_gate("null_relative", metrics.null_relative_db, -6.0, -3.0, "dB", "full-rig residual target gate"),
            _upper_gate("log_spectral_distance", metrics.log_spectral_distance_db, 14.0, 22.0, "dB", "unweighted spectral target gate"),
            _upper_gate("weighted_lsd", metrics.weighted_log_spectral_distance_db, 10.0, 18.0, "dB", "guitar-band spectral target gate"),
            _upper_gate("spectral_balance_max_abs", metrics.spectral_balance.max_abs_delta_db, 6.0, 12.0, "dB", "max gain-normalized band-balance drift"),
            _upper_gate("dynamics_max_abs_percentile", metrics.dynamics.max_abs_percentile_delta_db, 3.0, 6.0, "dB", "active short-term loudness percentile drift"),
            _absolute_gate("dynamics_range_delta_abs", metrics.dynamics.dynamic_range_delta_db, 3.0, 6.0, "dB", "active short-term dynamic-range drift"),
            *_level_response_gates(metrics, 3.0, 6.0),
            *_global_diagnostic_gates(metrics, phase_ms=(0.75, 2.5), decay=(60.0, 120.0), harmonic=(6.0, 12.0), aliasing=(-85.0, -65.0), transfer=(0.75, 1.5)),
            *_transient_gates(metrics, 3.0, 6.0),
            _upper_gate("envelope_error", metrics.envelope_error_db, -8.0, -4.0, "dB", "dynamic-envelope target gate"),
        ]
    raise ValueError(f"unsupported evaluation profile: {profile}")


def _noise_floor_gates(metrics: ComparisonMetrics) -> list[EvaluationGate]:
    if metrics.noise_floor.inactive_windows <= 0:
        return []
    return [
        _upper_gate(
            "noise_floor_candidate_p90",
            metrics.noise_floor.candidate_p90_dbfs,
            -70.0,
            -55.0,
            "dBFS",
            "candidate inactive-window P90 noise floor",
        ),
        _upper_gate(
            "noise_floor_p90_delta",
            metrics.noise_floor.p90_delta_db,
            12.0,
            24.0,
            "dB",
            "candidate/reference inactive-window P90 noise drift",
        ),
    ]


def _level_response_gates(metrics: ComparisonMetrics, warning: float, severe: float) -> list[EvaluationGate]:
    if metrics.level_response.active_windows <= 0:
        return []
    return [
        _upper_gate(
            "level_response_max_abs_delta",
            metrics.level_response.max_abs_delta_db,
            warning,
            severe,
            "dB",
            "active-window gain transfer drift across quiet, mid, and loud windows",
        )
    ]


def _global_diagnostic_gates(
    metrics: ComparisonMetrics,
    *,
    phase_ms: tuple[float, float],
    decay: tuple[float, float],
    harmonic: tuple[float, float],
    aliasing: tuple[float, float],
    transfer: tuple[float, float],
) -> list[EvaluationGate]:
    gates: list[EvaluationGate] = []
    if metrics.phase.mean_coherence > 0.0:
        gates.append(
            _upper_gate(
                "phase_mean_abs_group_delay",
                metrics.phase.mean_abs_group_delay_delta_ms,
                phase_ms[0],
                phase_ms[1],
                "ms",
                "mean absolute group-delay drift in guitar band",
            )
        )
    if metrics.decay.decay_windows > 0:
        gates.append(
            _upper_gate(
                "decay_max_abs_delta",
                metrics.decay.max_abs_delta,
                decay[0],
                decay[1],
                "dB/s or dB",
                "decay slope or late-level drift after detected attacks",
            )
        )
    if metrics.global_harmonics.stable_windows > 0:
        gates.append(
            _upper_gate(
                "global_harmonic_max_abs_delta",
                metrics.global_harmonics.max_abs_delta_db,
                harmonic[0],
                harmonic[1],
                "dB",
                "global stable-window harmonic fingerprint drift",
            )
        )
    gates.append(
        _upper_gate(
            "global_aliasing_residual_near_nyquist",
            metrics.global_aliasing.residual_near_nyquist_dbfs,
            aliasing[0],
            aliasing[1],
            "dBFS",
            "near-Nyquist residual energy triage",
        )
    )
    if metrics.nonlinear_transfer.sample_pairs > 0:
        gates.append(
            _upper_gate(
                "nonlinear_transfer_shape_delta",
                metrics.nonlinear_transfer.max_abs_shape_delta,
                transfer[0],
                transfer[1],
                "linear/dB",
                "paired-sample nonlinear transfer slope, curvature, or asymmetry drift",
            )
        )
    return gates


def _transient_gates(metrics: ComparisonMetrics, warning: float, severe: float) -> list[EvaluationGate]:
    if metrics.transients.transient_count <= 0:
        return []
    return [
        _upper_gate(
            "transient_max_abs_delta",
            metrics.transients.max_abs_delta_db,
            warning,
            severe,
            "dB",
            "median attack peak, crest, or high-band-ratio drift",
        )
    ]


def _upper_gate(name: str, value: float, warning: float, severe: float, unit: str, note: str) -> EvaluationGate:
    if warning == severe:
        status = "severe" if value > severe else "pass"
    elif severe > warning:
        status = "severe" if value >= severe else "warning" if value >= warning else "pass"
    else:
        status = "severe" if value >= severe else "warning" if value >= warning else "pass"
    return EvaluationGate(
        name=name,
        status=status,
        value=float(value),
        warning=float(warning),
        severe=float(severe),
        unit=unit,
        note=note,
    )


def _absolute_gate(name: str, value: float, warning: float, severe: float, unit: str, note: str) -> EvaluationGate:
    return _upper_gate(name, abs(value), warning, severe, unit, note)


def _worst_status(statuses: Iterable[str]) -> str:
    rank = {"pass": 0, "warning": 1, "severe": 2}
    worst = "pass"
    for status in statuses:
        if rank[status] > rank[worst]:
            worst = status
    return worst


def _render_evaluation_markdown(
    *,
    candidate_path: Path,
    reference_path: Path,
    metadata_line: str,
    metrics: ComparisonMetrics,
    result: EvaluationResult,
) -> str:
    gate_lines = [
        "| Gate | Status | Value | Warning | Severe | Note |",
        "| --- | --- | ---: | ---: | ---: | --- |",
    ]
    for gate in result.gates:
        gate_lines.append(
            f"| {gate.name} | {gate.status} | {_fmt(gate.value)} {gate.unit} | "
            f"{_fmt(gate.warning)} | {_fmt(gate.severe)} | {gate.note} |"
        )
    return f"""# WAV Evaluation Report

## Inputs

- Candidate: `{candidate_path}`
- Reference: `{reference_path}`
- Metadata: `{metadata_line}`
- Profile: `{result.profile}`
- Verdict: `{result.verdict}`

## Core Metrics

| Metric | Value |
| --- | ---: |
| Candidate latency | {metrics.latency_samples} samples / {metrics.latency_ms:.3f} ms |
| Gain correction | {metrics.gain_db:.3f} dB |
| Candidate RMS | {metrics.candidate.rms_dbfs:.2f} dBFS |
| Candidate peak | {metrics.candidate.peak_dbfs:.2f} dBFS |
| Candidate DC mean | {metrics.candidate.mean_dbfs:.2f} dBFS |
| Null residual relative | {metrics.null_relative_db:.2f} dB |
| Log-spectral distance | {metrics.log_spectral_distance_db:.2f} dB |
| Weighted guitar-band LSD | {metrics.weighted_log_spectral_distance_db:.2f} dB |
| Spectral balance max abs delta | {metrics.spectral_balance.max_abs_delta_db:.2f} dB |
| Spectral balance low delta | {metrics.spectral_balance.low_delta_db:.2f} dB |
| Spectral balance low-mid delta | {metrics.spectral_balance.low_mid_delta_db:.2f} dB |
| Spectral balance mid delta | {metrics.spectral_balance.mid_delta_db:.2f} dB |
| Spectral balance presence delta | {metrics.spectral_balance.presence_delta_db:.2f} dB |
| Spectral balance air delta | {metrics.spectral_balance.air_delta_db:.2f} dB |
| Dynamics P10 delta | {metrics.dynamics.p10_delta_db:.2f} dB |
| Dynamics P50 delta | {metrics.dynamics.p50_delta_db:.2f} dB |
| Dynamics P90 delta | {metrics.dynamics.p90_delta_db:.2f} dB |
| Dynamics range delta | {metrics.dynamics.dynamic_range_delta_db:.2f} dB |
| Dynamics max abs percentile delta | {metrics.dynamics.max_abs_percentile_delta_db:.2f} dB |
| Level response active windows | {metrics.level_response.active_windows} |
| Level response quiet gain delta | {metrics.level_response.quiet_gain_delta_db:.2f} dB |
| Level response mid gain delta | {metrics.level_response.mid_gain_delta_db:.2f} dB |
| Level response loud gain delta | {metrics.level_response.loud_gain_delta_db:.2f} dB |
| Level response slope delta | {metrics.level_response.slope_delta_db:.2f} dB |
| Level response max abs delta | {metrics.level_response.max_abs_delta_db:.2f} dB |
| Phase mean abs group delay delta | {metrics.phase.mean_abs_group_delay_delta_ms:.3f} ms |
| Phase max abs group delay delta | {metrics.phase.max_abs_group_delay_delta_ms:.3f} ms |
| Decay slope delta | {metrics.decay.slope_delta_db_per_s:.2f} dB/s |
| Decay late-level delta | {metrics.decay.late_level_delta_db:.2f} dB |
| Modulation depth delta | {metrics.modulation.modulation_depth_delta_db:.2f} dB |
| Modulation LF envelope residual | {metrics.modulation.envelope_lf_residual_db:.2f} dB |
| Global harmonic THD delta | {metrics.global_harmonics.thd_delta_db:.2f} dB |
| Global harmonic max abs delta | {metrics.global_harmonics.max_abs_delta_db:.2f} dB |
| Global IMD product delta | {metrics.global_imd.product_energy_delta_db:.2f} dB |
| Global IMD residual product | {metrics.global_imd.residual_product_dbfs:.2f} dBFS |
| Global aliasing residual near Nyquist | {metrics.global_aliasing.residual_near_nyquist_dbfs:.2f} dBFS |
| Nonlinear transfer shape delta | {metrics.nonlinear_transfer.max_abs_shape_delta:.3f} |
| Noise inactive windows | {metrics.noise_floor.inactive_windows} |
| Noise candidate P90 | {metrics.noise_floor.candidate_p90_dbfs:.2f} dBFS |
| Noise reference P90 | {metrics.noise_floor.reference_p90_dbfs:.2f} dBFS |
| Noise P90 delta | {metrics.noise_floor.p90_delta_db:.2f} dB |
| Transient count | {metrics.transients.transient_count} |
| Transient peak delta | {metrics.transients.peak_delta_db:.2f} dB |
| Transient crest delta | {metrics.transients.crest_delta_db:.2f} dB |
| Transient high-band ratio delta | {metrics.transients.high_band_ratio_delta_db:.2f} dB |
| Transient max abs delta | {metrics.transients.max_abs_delta_db:.2f} dB |
| Envelope error | {metrics.envelope_error_db:.2f} dB |
| Aligned DC offset delta | {metrics.dc_offset_delta_db:.2f} dBFS |
| Near-clip samples | {result.near_clip_count} |
| Hard-clip samples | {result.hard_clip_count} |

## Gates

{chr(10).join(gate_lines)}

## Interpretation

`pass` means this profile found no gating issue. `warning` means inspect the
artifact before treating the change as an improvement. `severe` means the render
should not be promoted without explaining why the gate is expected for this
experiment.
"""


def _fmt(value: float) -> str:
    if abs(value) >= 1000.0:
        return f"{value:.0f}"
    return f"{value:.2f}"


__all__ = [
    "EvaluationGate",
    "EvaluationResult",
    "evaluate_metrics",
    "write_evaluation_json",
    "write_evaluation_report",
]
