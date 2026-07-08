from __future__ import annotations

import numpy as np

from greybound_lab.evaluation import evaluate_metrics
from greybound_lab.metrics import compare_signals
from greybound_lab.segments import SegmentSpec


def test_regression_profile_passes_matching_signals() -> None:
    sample_rate = 48_000
    reference = _sine(sample_rate, 0.25) * 0.2

    metrics = compare_signals(reference.copy(), reference, sample_rate)
    result = evaluate_metrics(metrics, reference, profile="regression")

    assert result.verdict == "pass"
    assert result.hard_clip_count == 0


def test_evaluation_flags_candidate_clipping_as_severe() -> None:
    sample_rate = 48_000
    reference = _sine(sample_rate, 0.25) * 0.2
    candidate = reference.copy()
    candidate[100:110] = 1.0

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="amp-tone")

    assert result.verdict == "severe"
    assert result.hard_clip_count == 10
    assert any(gate.name == "hard_clip_samples" and gate.status == "severe" for gate in result.gates)


def test_clipper_profile_flags_aliasing_segment_high_band_residual() -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float64) / sample_rate
    reference = 0.1 * np.sin(2.0 * np.pi * 1_000.0 * time)
    candidate = reference + 0.02 * np.sin(2.0 * np.pi * 20_000.0 * time)

    metrics = compare_signals(
        candidate,
        reference,
        sample_rate,
        segments=[SegmentSpec(name="aliasing", start_s=0.0, end_s=1.0, kind="aliasing")],
    )
    result = evaluate_metrics(metrics, candidate, profile="clipper")

    assert result.verdict in {"warning", "severe"}
    assert any(gate.name == "aliasing.residual_high_band" for gate in result.gates)


def test_regression_profile_flags_spectral_balance_drift() -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float64) / sample_rate
    low_mid = np.sin(2.0 * np.pi * 500.0 * time)
    presence = np.sin(2.0 * np.pi * 6_000.0 * time)
    reference = 0.2 * (low_mid + presence)
    candidate = 0.2 * (low_mid + 1.6 * presence)

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="regression")

    assert result.verdict in {"warning", "severe"}
    assert any(gate.name == "spectral_balance_max_abs" for gate in result.gates)


def test_regression_profile_flags_dynamic_range_drift() -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float64) / sample_rate
    envelope = np.linspace(0.05, 0.8, sample_rate)
    reference = envelope * np.sin(2.0 * np.pi * 997.0 * time)
    candidate = np.tanh(reference * 3.0) / 3.0

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="regression")

    assert result.verdict in {"warning", "severe"}
    assert any(gate.name == "dynamics_range_delta_abs" for gate in result.gates)


def test_regression_profile_flags_level_response_drift() -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float64) / sample_rate
    envelope = np.linspace(0.05, 0.8, sample_rate)
    reference = envelope * np.sin(2.0 * np.pi * 997.0 * time)
    candidate = np.tanh(reference * 3.0) / 3.0

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="regression")

    assert result.verdict in {"warning", "severe"}
    assert any(gate.name == "level_response_max_abs_delta" for gate in result.gates)


def test_evaluation_flags_inactive_noise_floor() -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float64) / sample_rate
    active = 0.2 * np.sin(2.0 * np.pi * 997.0 * time[: sample_rate // 2])
    silence = np.zeros(sample_rate // 2, dtype=np.float64)
    reference = np.concatenate([active, silence])
    candidate = reference.copy()
    candidate[sample_rate // 2 :] = 0.01

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="regression")

    assert result.verdict == "severe"
    assert any(gate.name == "noise_floor_candidate_p90" for gate in result.gates)


def test_regression_profile_flags_transient_drift() -> None:
    sample_rate = 48_000
    reference = _plucked_transients(sample_rate)
    smoothing = np.ones(64, dtype=np.float64) / 64.0
    candidate = np.convolve(reference, smoothing, mode="same")

    metrics = compare_signals(candidate, reference, sample_rate)
    result = evaluate_metrics(metrics, candidate, profile="regression")

    assert result.verdict in {"warning", "severe"}
    assert any(gate.name == "transient_max_abs_delta" for gate in result.gates)


def _sine(sample_rate: int, seconds: float) -> np.ndarray:
    time = np.arange(int(sample_rate * seconds), dtype=np.float64) / sample_rate
    return np.sin(2.0 * np.pi * 997.0 * time)


def _plucked_transients(sample_rate: int) -> np.ndarray:
    samples = np.zeros(sample_rate, dtype=np.float64)
    burst_samples = int(round(0.060 * sample_rate))
    burst_time = np.arange(burst_samples, dtype=np.float64) / sample_rate
    burst = np.exp(-burst_time * 45.0) * np.sin(2.0 * np.pi * 1_500.0 * burst_time)
    click = np.zeros_like(burst)
    click[:12] = np.hanning(24)[:12]
    burst += 0.5 * click
    for start in (4_800, 14_400, 24_000, 33_600):
        samples[start : start + burst_samples] += burst
    return 0.3 * samples
