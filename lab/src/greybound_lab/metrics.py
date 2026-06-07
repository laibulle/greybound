from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from scipy import signal


EPSILON = 1.0e-12


@dataclass(frozen=True)
class SignalStats:
    rms_dbfs: float
    peak_dbfs: float
    crest_db: float


@dataclass(frozen=True)
class ComparisonMetrics:
    sample_rate_hz: int
    candidate_samples: int
    reference_samples: int
    compared_samples: int
    latency_samples: int
    latency_ms: float
    gain_db: float
    candidate: SignalStats
    reference: SignalStats
    aligned_candidate: SignalStats
    aligned_reference: SignalStats
    null_rms_dbfs: float
    null_relative_db: float
    log_spectral_distance_db: float
    envelope_error_db: float


def compare_signals(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    max_lag_ms: float = 100.0,
) -> ComparisonMetrics:
    candidate = np.asarray(candidate, dtype=np.float64)
    reference = np.asarray(reference, dtype=np.float64)
    latency_samples = estimate_latency(candidate, reference, sample_rate_hz, max_lag_ms)
    aligned_candidate, aligned_reference = align_by_latency(candidate, reference, latency_samples)
    gain = optimal_gain(aligned_candidate, aligned_reference)
    corrected_candidate = aligned_candidate * gain
    residual = corrected_candidate - aligned_reference

    candidate_stats = signal_stats(candidate)
    reference_stats = signal_stats(reference)
    corrected_stats = signal_stats(corrected_candidate)
    aligned_reference_stats = signal_stats(aligned_reference)
    residual_rms = rms(residual)
    reference_rms = rms(aligned_reference)

    return ComparisonMetrics(
        sample_rate_hz=sample_rate_hz,
        candidate_samples=int(candidate.shape[0]),
        reference_samples=int(reference.shape[0]),
        compared_samples=int(aligned_reference.shape[0]),
        latency_samples=int(latency_samples),
        latency_ms=1000.0 * latency_samples / sample_rate_hz,
        gain_db=linear_to_db(gain),
        candidate=candidate_stats,
        reference=reference_stats,
        aligned_candidate=corrected_stats,
        aligned_reference=aligned_reference_stats,
        null_rms_dbfs=linear_to_db(residual_rms),
        null_relative_db=linear_to_db(residual_rms / max(reference_rms, EPSILON)),
        log_spectral_distance_db=log_spectral_distance(corrected_candidate, aligned_reference, sample_rate_hz),
        envelope_error_db=envelope_error(corrected_candidate, aligned_reference),
    )


def estimate_latency(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    max_lag_ms: float,
) -> int:
    max_lag = int(round(sample_rate_hz * max_lag_ms / 1000.0))
    candidate_window = _analysis_window(candidate, sample_rate_hz)
    reference_window = _analysis_window(reference, sample_rate_hz)
    length = min(candidate_window.shape[0], reference_window.shape[0])
    if length < 8:
        raise ValueError("signals are too short to estimate latency")
    candidate_window = candidate_window[:length] - np.mean(candidate_window[:length])
    reference_window = reference_window[:length] - np.mean(reference_window[:length])
    correlation = signal.correlate(candidate_window, reference_window, mode="full", method="fft")
    lags = signal.correlation_lags(candidate_window.shape[0], reference_window.shape[0], mode="full")
    mask = np.abs(lags) <= max_lag
    if not np.any(mask):
        raise ValueError("no correlation lags available")
    return int(lags[mask][np.argmax(np.abs(correlation[mask]))])


def align_by_latency(
    candidate: np.ndarray,
    reference: np.ndarray,
    latency_samples: int,
) -> tuple[np.ndarray, np.ndarray]:
    if latency_samples >= 0:
        candidate_start = latency_samples
        reference_start = 0
    else:
        candidate_start = 0
        reference_start = -latency_samples
    length = min(candidate.shape[0] - candidate_start, reference.shape[0] - reference_start)
    if length <= 0:
        raise ValueError("latency alignment produced no overlapping samples")
    return (
        candidate[candidate_start : candidate_start + length],
        reference[reference_start : reference_start + length],
    )


def optimal_gain(candidate: np.ndarray, reference: np.ndarray) -> float:
    denominator = float(np.dot(candidate, candidate))
    if denominator <= EPSILON:
        return 1.0
    return float(np.dot(reference, candidate) / denominator)


def signal_stats(samples: np.ndarray) -> SignalStats:
    sample_rms = rms(samples)
    sample_peak = float(np.max(np.abs(samples))) if samples.size else 0.0
    return SignalStats(
        rms_dbfs=linear_to_db(sample_rms),
        peak_dbfs=linear_to_db(sample_peak),
        crest_db=linear_to_db(sample_peak / max(sample_rms, EPSILON)),
    )


def log_spectral_distance(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> float:
    nperseg = min(4096, max(256, _largest_power_of_two(candidate.shape[0] // 8)))
    _, _, candidate_stft = signal.stft(candidate, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    _, _, reference_stft = signal.stft(reference, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    candidate_db = 20.0 * np.log10(np.abs(candidate_stft) + EPSILON)
    reference_db = 20.0 * np.log10(np.abs(reference_stft) + EPSILON)
    return float(np.sqrt(np.mean(np.square(candidate_db - reference_db))))


def envelope_error(candidate: np.ndarray, reference: np.ndarray) -> float:
    candidate_env = np.abs(signal.hilbert(candidate))
    reference_env = np.abs(signal.hilbert(reference))
    error = rms(candidate_env - reference_env)
    return linear_to_db(error / max(rms(reference_env), EPSILON))


def rms(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(np.square(samples))))


def linear_to_db(value: float) -> float:
    return 20.0 * float(np.log10(max(abs(value), EPSILON)))


def _analysis_window(samples: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    max_samples = min(samples.shape[0], sample_rate_hz * 20)
    return samples[:max_samples]


def _largest_power_of_two(value: int) -> int:
    if value <= 1:
        return 1
    return 1 << (value.bit_length() - 1)
