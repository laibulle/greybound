from __future__ import annotations

import numpy as np

from greybound_lab.metrics import align_by_latency, compare_signals, estimate_latency


def test_estimates_positive_candidate_latency() -> None:
    sample_rate = 48_000
    reference = _sine(sample_rate, 0.5)
    candidate = np.concatenate([np.zeros(240), reference])

    latency = estimate_latency(candidate, reference, sample_rate, max_lag_ms=20)

    assert latency == 240


def test_alignment_and_gain_reduce_residual() -> None:
    sample_rate = 48_000
    reference = _sine(sample_rate, 0.5)
    candidate = np.concatenate([np.zeros(120), reference * 0.5])

    metrics = compare_signals(candidate, reference, sample_rate, max_lag_ms=20)

    assert metrics.latency_samples == 120
    assert abs(metrics.gain_db - 6.0206) < 0.02
    assert metrics.null_relative_db < -100.0


def test_aligns_negative_candidate_latency() -> None:
    reference = np.concatenate([np.zeros(10), np.arange(5, dtype=np.float64)])
    candidate = np.arange(5, dtype=np.float64)

    aligned_candidate, aligned_reference = align_by_latency(candidate, reference, -10)

    np.testing.assert_array_equal(aligned_candidate, aligned_reference)


def _sine(sample_rate: int, seconds: float) -> np.ndarray:
    time = np.arange(int(sample_rate * seconds), dtype=np.float64) / sample_rate
    return np.sin(2.0 * np.pi * 997.0 * time)
