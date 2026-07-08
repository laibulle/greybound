from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
from scipy import signal

from greybound_lab.segments import SegmentSpec


EPSILON = 1.0e-12


@dataclass(frozen=True)
class SignalStats:
    rms_dbfs: float
    peak_dbfs: float
    crest_db: float
    mean_dbfs: float = -240.0


@dataclass(frozen=True)
class SpectralBalanceMetrics:
    low_delta_db: float = 0.0
    low_mid_delta_db: float = 0.0
    mid_delta_db: float = 0.0
    presence_delta_db: float = 0.0
    air_delta_db: float = 0.0
    max_abs_delta_db: float = 0.0


@dataclass(frozen=True)
class DynamicsMetrics:
    candidate_p10_dbfs: float = -240.0
    candidate_p50_dbfs: float = -240.0
    candidate_p90_dbfs: float = -240.0
    reference_p10_dbfs: float = -240.0
    reference_p50_dbfs: float = -240.0
    reference_p90_dbfs: float = -240.0
    p10_delta_db: float = 0.0
    p50_delta_db: float = 0.0
    p90_delta_db: float = 0.0
    dynamic_range_delta_db: float = 0.0
    max_abs_percentile_delta_db: float = 0.0


@dataclass(frozen=True)
class LevelResponseMetrics:
    active_windows: int = 0
    quiet_gain_delta_db: float = 0.0
    mid_gain_delta_db: float = 0.0
    loud_gain_delta_db: float = 0.0
    slope_delta_db: float = 0.0
    max_abs_delta_db: float = 0.0


@dataclass(frozen=True)
class PhaseMetrics:
    mean_abs_group_delay_delta_ms: float = 0.0
    max_abs_group_delay_delta_ms: float = 0.0
    mean_coherence: float = 0.0


@dataclass(frozen=True)
class DecayMetrics:
    decay_windows: int = 0
    slope_delta_db_per_s: float = 0.0
    late_level_delta_db: float = 0.0
    max_abs_delta: float = 0.0


@dataclass(frozen=True)
class ModulationMetrics:
    modulation_depth_delta_db: float = 0.0
    envelope_lf_residual_db: float = -240.0
    candidate_depth_db: float = -240.0
    reference_depth_db: float = -240.0


@dataclass(frozen=True)
class GlobalHarmonicMetrics:
    stable_windows: int = 0
    thd_delta_db: float = 0.0
    h2_delta_db: float = 0.0
    h3_delta_db: float = 0.0
    h4_delta_db: float = 0.0
    h5_delta_db: float = 0.0
    max_abs_delta_db: float = 0.0


@dataclass(frozen=True)
class GlobalIntermodulationMetrics:
    product_energy_delta_db: float = 0.0
    residual_product_dbfs: float = -240.0
    chord_smear_delta_db: float = 0.0
    max_abs_delta_db: float = 0.0


@dataclass(frozen=True)
class GlobalAliasingMetrics:
    candidate_near_nyquist_dbfs: float = -240.0
    reference_near_nyquist_dbfs: float = -240.0
    residual_near_nyquist_dbfs: float = -240.0
    near_nyquist_delta_db: float = 0.0


@dataclass(frozen=True)
class NonlinearTransferMetrics:
    sample_pairs: int = 0
    slope_delta: float = 0.0
    curvature_delta: float = 0.0
    asymmetry_delta: float = 0.0
    residual_db: float = -240.0
    max_abs_shape_delta: float = 0.0


@dataclass(frozen=True)
class NoiseFloorMetrics:
    inactive_windows: int = 0
    candidate_p50_dbfs: float = -240.0
    candidate_p90_dbfs: float = -240.0
    reference_p50_dbfs: float = -240.0
    reference_p90_dbfs: float = -240.0
    p50_delta_db: float = 0.0
    p90_delta_db: float = 0.0


@dataclass(frozen=True)
class TransientMetrics:
    transient_count: int = 0
    peak_delta_db: float = 0.0
    crest_delta_db: float = 0.0
    high_band_ratio_delta_db: float = 0.0
    max_abs_delta_db: float = 0.0


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
    weighted_log_spectral_distance_db: float = 0.0
    dc_offset_delta_db: float = 0.0
    spectral_balance: SpectralBalanceMetrics = field(default_factory=SpectralBalanceMetrics)
    dynamics: DynamicsMetrics = field(default_factory=DynamicsMetrics)
    level_response: LevelResponseMetrics = field(default_factory=LevelResponseMetrics)
    phase: PhaseMetrics = field(default_factory=PhaseMetrics)
    decay: DecayMetrics = field(default_factory=DecayMetrics)
    modulation: ModulationMetrics = field(default_factory=ModulationMetrics)
    global_harmonics: GlobalHarmonicMetrics = field(default_factory=GlobalHarmonicMetrics)
    global_imd: GlobalIntermodulationMetrics = field(default_factory=GlobalIntermodulationMetrics)
    global_aliasing: GlobalAliasingMetrics = field(default_factory=GlobalAliasingMetrics)
    nonlinear_transfer: NonlinearTransferMetrics = field(default_factory=NonlinearTransferMetrics)
    noise_floor: NoiseFloorMetrics = field(default_factory=NoiseFloorMetrics)
    transients: TransientMetrics = field(default_factory=TransientMetrics)
    segments: tuple[SegmentComparisonMetrics, ...] = ()


@dataclass(frozen=True)
class SegmentComparisonMetrics:
    name: str
    kind: str
    start_s: float
    end_s: float
    samples: int
    local_gain_db: float
    null_relative_db: float
    log_spectral_distance_db: float
    envelope_error_db: float
    band_residual: BandResidualMetrics
    attack: AttackMetrics | None = None
    harmonics: HarmonicMetrics | None = None
    imd: IntermodulationMetrics | None = None
    aliasing: AliasingMetrics | None = None
    sag: SagMetrics | None = None


@dataclass(frozen=True)
class AttackMetrics:
    candidate_peak_time_ms: float
    reference_peak_time_ms: float
    peak_time_delta_ms: float
    candidate_rise_time_ms: float
    reference_rise_time_ms: float
    rise_time_delta_ms: float
    overshoot_delta_db: float


@dataclass(frozen=True)
class HarmonicMetrics:
    fundamental_hz: float
    candidate_thd_db: float
    reference_thd_db: float
    thd_delta_db: float
    h2_delta_db: float | None
    h3_delta_db: float | None
    h4_delta_db: float | None
    h5_delta_db: float | None


@dataclass(frozen=True)
class IntermodulationMetrics:
    first_hz: float
    second_hz: float
    candidate_imd_db: float
    reference_imd_db: float
    imd_delta_db: float
    lower_sideband_delta_db: float | None
    upper_sideband_delta_db: float | None
    difference_delta_db: float | None
    sum_delta_db: float | None


@dataclass(frozen=True)
class AliasingMetrics:
    candidate_high_band_dbfs: float
    reference_high_band_dbfs: float
    high_band_delta_db: float
    residual_high_band_dbfs: float


@dataclass(frozen=True)
class SagMetrics:
    candidate_drop_db: float
    reference_drop_db: float
    drop_delta_db: float
    candidate_recovery_db: float
    reference_recovery_db: float
    recovery_delta_db: float


@dataclass(frozen=True)
class BandResidualMetrics:
    low_db: float
    low_mid_db: float
    mid_db: float
    presence_db: float
    air_db: float


def compare_signals(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    max_lag_ms: float = 100.0,
    segments: list[SegmentSpec] | None = None,
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
        weighted_log_spectral_distance_db=weighted_log_spectral_distance(
            corrected_candidate,
            aligned_reference,
            sample_rate_hz,
        ),
        dc_offset_delta_db=linear_to_db(
            abs(float(np.mean(corrected_candidate)) - float(np.mean(aligned_reference)))
        ),
        spectral_balance=spectral_balance_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        dynamics=dynamics_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        level_response=level_response_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        phase=phase_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        decay=decay_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        modulation=modulation_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        global_harmonics=global_harmonic_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        global_imd=global_intermodulation_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        global_aliasing=global_aliasing_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        nonlinear_transfer=nonlinear_transfer_metrics(corrected_candidate, aligned_reference),
        noise_floor=noise_floor_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        transients=transient_metrics(corrected_candidate, aligned_reference, sample_rate_hz),
        segments=tuple(
            compare_segment(corrected_candidate, aligned_reference, sample_rate_hz, segment)
            for segment in (segments or [])
        ),
    )


def compare_segment(
    corrected_candidate: np.ndarray,
    aligned_reference: np.ndarray,
    sample_rate_hz: int,
    segment: SegmentSpec,
) -> SegmentComparisonMetrics:
    start = max(0, int(round(segment.start_s * sample_rate_hz)))
    end = min(corrected_candidate.shape[0], aligned_reference.shape[0], int(round(segment.end_s * sample_rate_hz)))
    if end <= start:
        raise ValueError(f"segment {segment.name} is outside the compared audio range")
    candidate = corrected_candidate[start:end]
    reference = aligned_reference[start:end]
    local_gain = optimal_gain(candidate, reference)
    locally_corrected = candidate * local_gain
    residual = locally_corrected - reference
    reference_rms = rms(reference)

    kind = segment.kind.lower()
    return SegmentComparisonMetrics(
        name=segment.name,
        kind=segment.kind,
        start_s=segment.start_s,
        end_s=segment.end_s,
        samples=int(reference.shape[0]),
        local_gain_db=linear_to_db(local_gain),
        null_relative_db=linear_to_db(rms(residual) / max(reference_rms, EPSILON)),
        log_spectral_distance_db=log_spectral_distance(locally_corrected, reference, sample_rate_hz),
        envelope_error_db=envelope_error(locally_corrected, reference),
        band_residual=band_residual_metrics(locally_corrected, reference, sample_rate_hz),
        attack=attack_metrics(locally_corrected, reference, sample_rate_hz) if kind == "attack" else None,
        harmonics=harmonic_metrics(
            locally_corrected,
            reference,
            sample_rate_hz,
            segment.fundamental_hz,
        )
        if kind == "harmonic"
        else None,
        imd=intermodulation_metrics(
            locally_corrected,
            reference,
            sample_rate_hz,
            segment.first_hz,
            segment.second_hz,
        )
        if kind == "imd"
        else None,
        aliasing=aliasing_metrics(locally_corrected, reference, sample_rate_hz) if kind == "aliasing" else None,
        sag=sag_metrics(locally_corrected, reference, sample_rate_hz) if kind == "sag" else None,
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
        mean_dbfs=linear_to_db(float(np.mean(samples))) if samples.size else -240.0,
    )


def log_spectral_distance(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> float:
    if candidate.shape[0] < 32 or reference.shape[0] < 32:
        return 0.0
    nperseg = min(4096, max(256, _largest_power_of_two(candidate.shape[0] // 8)))
    _, _, candidate_stft = signal.stft(candidate, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    _, _, reference_stft = signal.stft(reference, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    candidate_db = 20.0 * np.log10(np.abs(candidate_stft) + EPSILON)
    reference_db = 20.0 * np.log10(np.abs(reference_stft) + EPSILON)
    return float(np.sqrt(np.mean(np.square(candidate_db - reference_db))))


def weighted_log_spectral_distance(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> float:
    if candidate.shape[0] < 32 or reference.shape[0] < 32:
        return 0.0
    nperseg = min(4096, max(256, _largest_power_of_two(candidate.shape[0] // 8)))
    frequencies, _, candidate_stft = signal.stft(candidate, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    _, _, reference_stft = signal.stft(reference, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    candidate_db = 20.0 * np.log10(np.abs(candidate_stft) + EPSILON)
    reference_db = 20.0 * np.log10(np.abs(reference_stft) + EPSILON)
    weights = _guitar_audibility_weights(frequencies)
    squared_error = np.square(candidate_db - reference_db)
    return float(np.sqrt(np.sum(squared_error * weights[:, np.newaxis]) / max(np.sum(weights) * squared_error.shape[1], EPSILON)))


def envelope_error(candidate: np.ndarray, reference: np.ndarray) -> float:
    if candidate.shape[0] < 4 or reference.shape[0] < 4:
        return 0.0
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


def _guitar_audibility_weights(frequencies_hz: np.ndarray) -> np.ndarray:
    frequencies = np.asarray(frequencies_hz, dtype=np.float64)
    weights = np.full(frequencies.shape, 0.35, dtype=np.float64)
    weights[(frequencies >= 80.0) & (frequencies < 250.0)] = 0.65
    weights[(frequencies >= 250.0) & (frequencies < 1_000.0)] = 0.95
    weights[(frequencies >= 1_000.0) & (frequencies < 5_000.0)] = 1.35
    weights[(frequencies >= 5_000.0) & (frequencies < 8_000.0)] = 1.0
    weights[(frequencies >= 8_000.0) & (frequencies < 14_000.0)] = 0.55
    weights[frequencies >= 14_000.0] = 0.25
    return weights


def attack_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> AttackMetrics:
    candidate_env = _smooth_envelope(candidate, sample_rate_hz)
    reference_env = _smooth_envelope(reference, sample_rate_hz)
    candidate_peak_time = _peak_time_ms(candidate_env, sample_rate_hz)
    reference_peak_time = _peak_time_ms(reference_env, sample_rate_hz)
    candidate_rise_time = _rise_time_ms(candidate_env, sample_rate_hz)
    reference_rise_time = _rise_time_ms(reference_env, sample_rate_hz)
    return AttackMetrics(
        candidate_peak_time_ms=candidate_peak_time,
        reference_peak_time_ms=reference_peak_time,
        peak_time_delta_ms=candidate_peak_time - reference_peak_time,
        candidate_rise_time_ms=candidate_rise_time,
        reference_rise_time_ms=reference_rise_time,
        rise_time_delta_ms=candidate_rise_time - reference_rise_time,
        overshoot_delta_db=_overshoot_db(candidate_env) - _overshoot_db(reference_env),
    )


def harmonic_metrics(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    fundamental_hz: float | None,
) -> HarmonicMetrics:
    f0 = fundamental_hz or _estimate_fundamental(reference, sample_rate_hz)
    candidate_harmonics = _harmonic_levels(candidate, sample_rate_hz, f0)
    reference_harmonics = _harmonic_levels(reference, sample_rate_hz, f0)
    candidate_thd = _thd_db(candidate_harmonics)
    reference_thd = _thd_db(reference_harmonics)
    return HarmonicMetrics(
        fundamental_hz=f0,
        candidate_thd_db=candidate_thd,
        reference_thd_db=reference_thd,
        thd_delta_db=candidate_thd - reference_thd,
        h2_delta_db=_harmonic_delta(candidate_harmonics, reference_harmonics, 2),
        h3_delta_db=_harmonic_delta(candidate_harmonics, reference_harmonics, 3),
        h4_delta_db=_harmonic_delta(candidate_harmonics, reference_harmonics, 4),
        h5_delta_db=_harmonic_delta(candidate_harmonics, reference_harmonics, 5),
    )


def aliasing_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> AliasingMetrics:
    residual = candidate - reference
    candidate_high = _band_rms(candidate, sample_rate_hz, 18_000.0, sample_rate_hz / 2.0)
    reference_high = _band_rms(reference, sample_rate_hz, 18_000.0, sample_rate_hz / 2.0)
    residual_high = _band_rms(residual, sample_rate_hz, 18_000.0, sample_rate_hz / 2.0)
    return AliasingMetrics(
        candidate_high_band_dbfs=linear_to_db(candidate_high),
        reference_high_band_dbfs=linear_to_db(reference_high),
        high_band_delta_db=linear_to_db(candidate_high / max(reference_high, EPSILON)),
        residual_high_band_dbfs=linear_to_db(residual_high),
    )


def intermodulation_metrics(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    first_hz: float | None,
    second_hz: float | None,
) -> IntermodulationMetrics:
    f1, f2 = _two_tone_frequencies(reference, sample_rate_hz, first_hz, second_hz)
    candidate_lines = _line_levels(candidate, sample_rate_hz, _imd_frequencies(f1, f2))
    reference_lines = _line_levels(reference, sample_rate_hz, _imd_frequencies(f1, f2))
    candidate_imd = _imd_ratio_db(candidate_lines, f1, f2)
    reference_imd = _imd_ratio_db(reference_lines, f1, f2)
    return IntermodulationMetrics(
        first_hz=f1,
        second_hz=f2,
        candidate_imd_db=candidate_imd,
        reference_imd_db=reference_imd,
        imd_delta_db=candidate_imd - reference_imd,
        lower_sideband_delta_db=_line_ratio_delta(candidate_lines, reference_lines, 2.0 * f1 - f2, f1, f2),
        upper_sideband_delta_db=_line_ratio_delta(candidate_lines, reference_lines, 2.0 * f2 - f1, f1, f2),
        difference_delta_db=_line_ratio_delta(candidate_lines, reference_lines, abs(f2 - f1), f1, f2),
        sum_delta_db=_line_ratio_delta(candidate_lines, reference_lines, f1 + f2, f1, f2),
    )


def band_residual_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> BandResidualMetrics:
    residual = candidate - reference
    return BandResidualMetrics(
        low_db=_band_relative_residual(residual, reference, sample_rate_hz, 40.0, 250.0),
        low_mid_db=_band_relative_residual(residual, reference, sample_rate_hz, 250.0, 1_000.0),
        mid_db=_band_relative_residual(residual, reference, sample_rate_hz, 1_000.0, 4_000.0),
        presence_db=_band_relative_residual(residual, reference, sample_rate_hz, 4_000.0, 8_000.0),
        air_db=_band_relative_residual(residual, reference, sample_rate_hz, 8_000.0, min(18_000.0, sample_rate_hz / 2.0)),
    )


def spectral_balance_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> SpectralBalanceMetrics:
    candidate_levels = _relative_band_levels(candidate, sample_rate_hz)
    reference_levels = _relative_band_levels(reference, sample_rate_hz)
    deltas = {
        band: candidate_levels[band] - reference_levels[band]
        for band in candidate_levels
    }
    return SpectralBalanceMetrics(
        low_delta_db=deltas["low"],
        low_mid_delta_db=deltas["low_mid"],
        mid_delta_db=deltas["mid"],
        presence_delta_db=deltas["presence"],
        air_delta_db=deltas["air"],
        max_abs_delta_db=max(abs(delta) for delta in deltas.values()),
    )


def dynamics_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> DynamicsMetrics:
    candidate_levels, reference_levels = _active_short_term_levels(candidate, reference, sample_rate_hz)
    candidate_p10, candidate_p50, candidate_p90 = _level_percentiles(candidate_levels)
    reference_p10, reference_p50, reference_p90 = _level_percentiles(reference_levels)
    p10_delta = candidate_p10 - reference_p10
    p50_delta = candidate_p50 - reference_p50
    p90_delta = candidate_p90 - reference_p90
    candidate_range = candidate_p90 - candidate_p10
    reference_range = reference_p90 - reference_p10
    return DynamicsMetrics(
        candidate_p10_dbfs=candidate_p10,
        candidate_p50_dbfs=candidate_p50,
        candidate_p90_dbfs=candidate_p90,
        reference_p10_dbfs=reference_p10,
        reference_p50_dbfs=reference_p50,
        reference_p90_dbfs=reference_p90,
        p10_delta_db=p10_delta,
        p50_delta_db=p50_delta,
        p90_delta_db=p90_delta,
        dynamic_range_delta_db=candidate_range - reference_range,
        max_abs_percentile_delta_db=max(abs(p10_delta), abs(p50_delta), abs(p90_delta)),
    )


def level_response_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> LevelResponseMetrics:
    candidate_levels, reference_levels = _active_short_term_levels(candidate, reference, sample_rate_hz)
    length = min(candidate_levels.shape[0], reference_levels.shape[0])
    if length < 3:
        return LevelResponseMetrics()
    candidate_levels = candidate_levels[:length]
    reference_levels = reference_levels[:length]
    gain_delta = candidate_levels - reference_levels
    low_cut, high_cut = np.percentile(reference_levels, [33.333, 66.667])
    quiet = gain_delta[reference_levels <= low_cut]
    mid = gain_delta[(reference_levels > low_cut) & (reference_levels < high_cut)]
    loud = gain_delta[reference_levels >= high_cut]
    if quiet.size == 0 or mid.size == 0 or loud.size == 0:
        return LevelResponseMetrics()
    quiet_delta = float(np.median(quiet))
    mid_delta = float(np.median(mid))
    loud_delta = float(np.median(loud))
    slope_delta = loud_delta - quiet_delta
    return LevelResponseMetrics(
        active_windows=int(length),
        quiet_gain_delta_db=quiet_delta,
        mid_gain_delta_db=mid_delta,
        loud_gain_delta_db=loud_delta,
        slope_delta_db=slope_delta,
        max_abs_delta_db=max(abs(quiet_delta), abs(mid_delta), abs(loud_delta), abs(slope_delta)),
    )


def phase_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> PhaseMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < 1024:
        return PhaseMetrics()
    candidate = candidate[:length]
    reference = reference[:length]
    nperseg = min(8192, max(1024, _largest_power_of_two(length // 8)))
    frequencies, cross = signal.csd(candidate, reference, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    _, candidate_psd = signal.welch(candidate, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    _, reference_psd = signal.welch(reference, fs=sample_rate_hz, nperseg=nperseg, noverlap=nperseg // 2)
    coherence = np.square(np.abs(cross)) / np.maximum(candidate_psd * reference_psd, EPSILON)
    band = (frequencies >= 80.0) & (frequencies <= min(8_000.0, sample_rate_hz / 2.0)) & (coherence >= 0.25)
    if int(np.count_nonzero(band)) < 4:
        return PhaseMetrics()
    phase = np.unwrap(np.angle(cross[band]))
    band_frequencies = frequencies[band]
    group_delay_ms = -np.gradient(phase, band_frequencies) / (2.0 * np.pi) * 1_000.0
    group_delay_ms = group_delay_ms[np.isfinite(group_delay_ms)]
    if group_delay_ms.size == 0:
        return PhaseMetrics(mean_coherence=float(np.mean(coherence[band])))
    return PhaseMetrics(
        mean_abs_group_delay_delta_ms=float(np.mean(np.abs(group_delay_ms))),
        max_abs_group_delay_delta_ms=float(np.max(np.abs(group_delay_ms))),
        mean_coherence=float(np.mean(coherence[band])),
    )


def decay_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> DecayMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < int(round(0.350 * sample_rate_hz)):
        return DecayMetrics()
    candidate = candidate[:length]
    reference = reference[:length]
    onsets = _transient_onsets(reference, sample_rate_hz)
    if onsets.size == 0:
        return DecayMetrics()
    slope_deltas: list[float] = []
    late_deltas: list[float] = []
    for onset in onsets:
        start = int(onset) + int(round(0.040 * sample_rate_hz))
        end = int(onset) + int(round(0.350 * sample_rate_hz))
        if end > length or end - start < int(round(0.120 * sample_rate_hz)):
            continue
        candidate_slope, candidate_late = _decay_shape(candidate[start:end], sample_rate_hz)
        reference_slope, reference_late = _decay_shape(reference[start:end], sample_rate_hz)
        slope_deltas.append(candidate_slope - reference_slope)
        late_deltas.append(candidate_late - reference_late)
    if not slope_deltas:
        return DecayMetrics()
    slope_delta = float(np.median(np.asarray(slope_deltas)))
    late_delta = float(np.median(np.asarray(late_deltas)))
    return DecayMetrics(
        decay_windows=len(slope_deltas),
        slope_delta_db_per_s=slope_delta,
        late_level_delta_db=late_delta,
        max_abs_delta=max(abs(slope_delta), abs(late_delta)),
    )


def modulation_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> ModulationMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < sample_rate_hz:
        return ModulationMetrics()
    candidate_env = _smooth_envelope(candidate[:length], sample_rate_hz)
    reference_env = _smooth_envelope(reference[:length], sample_rate_hz)
    candidate_depth = _low_frequency_envelope_depth(candidate_env, sample_rate_hz)
    reference_depth = _low_frequency_envelope_depth(reference_env, sample_rate_hz)
    residual = candidate_env - reference_env
    residual_depth = _band_limited_rms(residual, sample_rate_hz, 0.5, 12.0)
    reference_level = rms(reference_env)
    return ModulationMetrics(
        modulation_depth_delta_db=candidate_depth - reference_depth,
        envelope_lf_residual_db=linear_to_db(residual_depth / max(reference_level, EPSILON)),
        candidate_depth_db=candidate_depth,
        reference_depth_db=reference_depth,
    )


def global_harmonic_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> GlobalHarmonicMetrics:
    windows = _stable_tone_windows(reference, sample_rate_hz)
    if not windows:
        return GlobalHarmonicMetrics()
    thd: list[float] = []
    harmonic_deltas: dict[int, list[float]] = {2: [], 3: [], 4: [], 5: []}
    for start, end, fundamental_hz in windows:
        harmonic = harmonic_metrics(candidate[start:end], reference[start:end], sample_rate_hz, fundamental_hz)
        thd.append(harmonic.thd_delta_db)
        for index, value in [
            (2, harmonic.h2_delta_db),
            (3, harmonic.h3_delta_db),
            (4, harmonic.h4_delta_db),
            (5, harmonic.h5_delta_db),
        ]:
            if value is not None:
                harmonic_deltas[index].append(value)
    if not thd:
        return GlobalHarmonicMetrics()
    values = [
        float(np.median(np.asarray(thd))),
        _median_or_zero(harmonic_deltas[2]),
        _median_or_zero(harmonic_deltas[3]),
        _median_or_zero(harmonic_deltas[4]),
        _median_or_zero(harmonic_deltas[5]),
    ]
    return GlobalHarmonicMetrics(
        stable_windows=len(thd),
        thd_delta_db=values[0],
        h2_delta_db=values[1],
        h3_delta_db=values[2],
        h4_delta_db=values[3],
        h5_delta_db=values[4],
        max_abs_delta_db=max(abs(value) for value in values),
    )


def global_intermodulation_metrics(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
) -> GlobalIntermodulationMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < 1024:
        return GlobalIntermodulationMetrics()
    candidate = candidate[:length]
    reference = reference[:length]
    residual = candidate - reference
    product_band = (120.0, min(6_000.0, sample_rate_hz / 2.0))
    candidate_product = _spectral_residual_excluding_strong_lines(candidate, sample_rate_hz, *product_band)
    reference_product = _spectral_residual_excluding_strong_lines(reference, sample_rate_hz, *product_band)
    residual_product = _band_rms(residual, sample_rate_hz, *product_band)
    candidate_smear = _spectral_flatness(candidate, sample_rate_hz, *product_band)
    reference_smear = _spectral_flatness(reference, sample_rate_hz, *product_band)
    product_delta = linear_to_db(candidate_product / max(reference_product, EPSILON))
    smear_delta = candidate_smear - reference_smear
    return GlobalIntermodulationMetrics(
        product_energy_delta_db=product_delta,
        residual_product_dbfs=linear_to_db(residual_product),
        chord_smear_delta_db=smear_delta,
        max_abs_delta_db=max(abs(product_delta), abs(smear_delta)),
    )


def global_aliasing_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> GlobalAliasingMetrics:
    nyquist = sample_rate_hz / 2.0
    low = max(16_000.0, nyquist * 0.78)
    high = nyquist * 0.98
    if low >= high:
        return GlobalAliasingMetrics()
    residual = candidate[: min(candidate.shape[0], reference.shape[0])] - reference[: min(candidate.shape[0], reference.shape[0])]
    candidate_high = _band_rms(candidate, sample_rate_hz, low, high)
    reference_high = _band_rms(reference, sample_rate_hz, low, high)
    residual_high = _band_rms(residual, sample_rate_hz, low, high)
    return GlobalAliasingMetrics(
        candidate_near_nyquist_dbfs=linear_to_db(candidate_high),
        reference_near_nyquist_dbfs=linear_to_db(reference_high),
        residual_near_nyquist_dbfs=linear_to_db(residual_high),
        near_nyquist_delta_db=linear_to_db(candidate_high / max(reference_high, EPSILON)),
    )


def nonlinear_transfer_metrics(candidate: np.ndarray, reference: np.ndarray) -> NonlinearTransferMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < 128:
        return NonlinearTransferMetrics()
    x = reference[:length]
    y = candidate[:length]
    active = np.abs(x) >= max(0.005, float(np.percentile(np.abs(x), 55)))
    if int(np.count_nonzero(active)) < 64:
        return NonlinearTransferMetrics()
    x = x[active]
    y = y[active]
    if int(np.count_nonzero(x > 0.0)) < 16 or int(np.count_nonzero(x < 0.0)) < 16:
        return NonlinearTransferMetrics()
    scale = max(float(np.max(np.abs(x))), EPSILON)
    xn = x / scale
    coefficients = np.polyfit(xn, y, 3)
    fitted = np.polyval(coefficients, xn)
    residual = y - fitted
    positive = y[x > 0.0]
    negative = y[x < 0.0]
    positive_level = float(np.percentile(positive, 95)) if positive.size else 0.0
    negative_level = abs(float(np.percentile(negative, 5))) if negative.size else 0.0
    slope = float(coefficients[2] / scale)
    curvature = float(coefficients[0])
    asymmetry = linear_to_db(positive_level / max(negative_level, EPSILON))
    return NonlinearTransferMetrics(
        sample_pairs=int(x.shape[0]),
        slope_delta=slope - 1.0,
        curvature_delta=curvature,
        asymmetry_delta=asymmetry,
        residual_db=linear_to_db(rms(residual) / max(rms(y), EPSILON)),
        max_abs_shape_delta=max(abs(slope - 1.0), abs(curvature), abs(asymmetry)),
    )


def noise_floor_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> NoiseFloorMetrics:
    candidate_levels = _short_term_rms_dbfs(candidate, sample_rate_hz)
    reference_levels = _short_term_rms_dbfs(reference, sample_rate_hz)
    length = min(candidate_levels.shape[0], reference_levels.shape[0])
    if length == 0:
        return NoiseFloorMetrics()
    candidate_levels = candidate_levels[:length]
    reference_levels = reference_levels[:length]
    inactive_threshold = min(-60.0, float(np.percentile(reference_levels, 90)) - 35.0)
    inactive = reference_levels <= inactive_threshold
    if int(np.count_nonzero(inactive)) < 3:
        return NoiseFloorMetrics()
    candidate_inactive = candidate_levels[inactive]
    reference_inactive = reference_levels[inactive]
    candidate_p50, candidate_p90 = np.percentile(candidate_inactive, [50, 90])
    reference_p50, reference_p90 = np.percentile(reference_inactive, [50, 90])
    return NoiseFloorMetrics(
        inactive_windows=int(candidate_inactive.shape[0]),
        candidate_p50_dbfs=float(candidate_p50),
        candidate_p90_dbfs=float(candidate_p90),
        reference_p50_dbfs=float(reference_p50),
        reference_p90_dbfs=float(reference_p90),
        p50_delta_db=float(candidate_p50 - reference_p50),
        p90_delta_db=float(candidate_p90 - reference_p90),
    )


def transient_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> TransientMetrics:
    length = min(candidate.shape[0], reference.shape[0])
    if length < max(64, int(round(0.050 * sample_rate_hz))):
        return TransientMetrics()
    candidate = candidate[:length]
    reference = reference[:length]
    onsets = _transient_onsets(reference, sample_rate_hz)
    if onsets.size == 0:
        return TransientMetrics()

    pre = max(1, int(round(0.005 * sample_rate_hz)))
    post = max(pre + 1, int(round(0.035 * sample_rate_hz)))
    peak_deltas: list[float] = []
    crest_deltas: list[float] = []
    high_ratio_deltas: list[float] = []
    for onset in onsets:
        start = max(0, int(onset) - pre)
        end = min(length, int(onset) + post)
        if end - start < 16:
            continue
        candidate_window = candidate[start:end]
        reference_window = reference[start:end]
        if rms(reference_window) <= EPSILON:
            continue
        candidate_stats = signal_stats(candidate_window)
        reference_stats = signal_stats(reference_window)
        peak_deltas.append(candidate_stats.peak_dbfs - reference_stats.peak_dbfs)
        crest_deltas.append(candidate_stats.crest_db - reference_stats.crest_db)
        high_ratio_deltas.append(
            _high_band_ratio_db(candidate_window, sample_rate_hz)
            - _high_band_ratio_db(reference_window, sample_rate_hz)
        )

    if not peak_deltas:
        return TransientMetrics()
    peak_delta = float(np.median(np.asarray(peak_deltas)))
    crest_delta = float(np.median(np.asarray(crest_deltas)))
    high_ratio_delta = float(np.median(np.asarray(high_ratio_deltas)))
    return TransientMetrics(
        transient_count=len(peak_deltas),
        peak_delta_db=peak_delta,
        crest_delta_db=crest_delta,
        high_band_ratio_delta_db=high_ratio_delta,
        max_abs_delta_db=max(abs(peak_delta), abs(crest_delta), abs(high_ratio_delta)),
    )


def sag_metrics(candidate: np.ndarray, reference: np.ndarray, sample_rate_hz: int) -> SagMetrics:
    candidate_drop, candidate_recovery = _sag_shape(candidate, sample_rate_hz)
    reference_drop, reference_recovery = _sag_shape(reference, sample_rate_hz)
    return SagMetrics(
        candidate_drop_db=candidate_drop,
        reference_drop_db=reference_drop,
        drop_delta_db=candidate_drop - reference_drop,
        candidate_recovery_db=candidate_recovery,
        reference_recovery_db=reference_recovery,
        recovery_delta_db=candidate_recovery - reference_recovery,
    )


def _analysis_window(samples: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    max_samples = min(samples.shape[0], sample_rate_hz * 20)
    return samples[:max_samples]


def _largest_power_of_two(value: int) -> int:
    if value <= 1:
        return 1
    return 1 << (value.bit_length() - 1)


def _smooth_envelope(samples: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    envelope = np.abs(signal.hilbert(samples))
    window = max(1, int(round(sample_rate_hz * 0.001)))
    if window <= 1:
        return envelope
    kernel = np.ones(window) / window
    return np.convolve(envelope, kernel, mode="same")


def _peak_time_ms(envelope: np.ndarray, sample_rate_hz: int) -> float:
    if envelope.size == 0:
        return 0.0
    return 1000.0 * int(np.argmax(envelope)) / sample_rate_hz


def _rise_time_ms(envelope: np.ndarray, sample_rate_hz: int) -> float:
    if envelope.size == 0:
        return 0.0
    peak = float(np.max(envelope))
    if peak <= EPSILON:
        return 0.0
    low = 0.1 * peak
    high = 0.9 * peak
    low_indices = np.flatnonzero(envelope >= low)
    high_indices = np.flatnonzero(envelope >= high)
    if low_indices.size == 0 or high_indices.size == 0:
        return 0.0
    return 1000.0 * max(0, int(high_indices[0]) - int(low_indices[0])) / sample_rate_hz


def _overshoot_db(envelope: np.ndarray) -> float:
    if envelope.size == 0:
        return 0.0
    peak = float(np.max(envelope))
    tail_start = max(0, int(envelope.size * 0.5))
    steady = float(np.median(envelope[tail_start:])) if tail_start < envelope.size else peak
    return linear_to_db(peak / max(steady, EPSILON))


def _estimate_fundamental(samples: np.ndarray, sample_rate_hz: int) -> float:
    spectrum = np.abs(np.fft.rfft(_windowed(samples)))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    mask = (freqs >= 40.0) & (freqs <= min(5_000.0, sample_rate_hz / 2.0))
    if not np.any(mask):
        return 440.0
    return float(freqs[mask][np.argmax(spectrum[mask])])


def _harmonic_levels(samples: np.ndarray, sample_rate_hz: int, fundamental_hz: float) -> dict[int, float]:
    windowed = _windowed(samples)
    spectrum = np.abs(np.fft.rfft(windowed))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    levels: dict[int, float] = {}
    for harmonic in range(1, 6):
        frequency = fundamental_hz * harmonic
        if frequency >= sample_rate_hz / 2.0:
            continue
        index = int(np.argmin(np.abs(freqs - frequency)))
        left = max(0, index - 1)
        right = min(spectrum.shape[0], index + 2)
        levels[harmonic] = float(np.sqrt(np.sum(np.square(spectrum[left:right]))))
    return levels


def _line_levels(samples: np.ndarray, sample_rate_hz: int, frequencies_hz: list[float]) -> dict[float, float]:
    windowed = _windowed(samples)
    spectrum = np.abs(np.fft.rfft(windowed))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    levels: dict[float, float] = {}
    for frequency_hz in frequencies_hz:
        if frequency_hz <= 0.0 or frequency_hz >= sample_rate_hz / 2.0:
            continue
        index = int(np.argmin(np.abs(freqs - frequency_hz)))
        left = max(0, index - 1)
        right = min(spectrum.shape[0], index + 2)
        levels[frequency_hz] = float(np.sqrt(np.sum(np.square(spectrum[left:right]))))
    return levels


def _thd_db(levels: dict[int, float]) -> float:
    fundamental = levels.get(1, 0.0)
    harmonic_power = sum(level * level for harmonic, level in levels.items() if harmonic > 1)
    return linear_to_db(np.sqrt(harmonic_power) / max(fundamental, EPSILON))


def _harmonic_delta(candidate: dict[int, float], reference: dict[int, float], harmonic: int) -> float | None:
    if harmonic not in candidate or harmonic not in reference:
        return None
    candidate_ratio = candidate[harmonic] / max(candidate.get(1, 0.0), EPSILON)
    reference_ratio = reference[harmonic] / max(reference.get(1, 0.0), EPSILON)
    return linear_to_db(candidate_ratio / max(reference_ratio, EPSILON))


def _two_tone_frequencies(
    reference: np.ndarray,
    sample_rate_hz: int,
    first_hz: float | None,
    second_hz: float | None,
) -> tuple[float, float]:
    if first_hz is not None and second_hz is not None:
        return (min(first_hz, second_hz), max(first_hz, second_hz))
    spectrum = np.abs(np.fft.rfft(_windowed(reference)))
    freqs = np.fft.rfftfreq(reference.shape[0], 1.0 / sample_rate_hz)
    mask = (freqs >= 40.0) & (freqs <= min(6_000.0, sample_rate_hz / 2.0))
    candidate_indices = np.flatnonzero(mask)
    if candidate_indices.size < 2:
        return 440.0, 550.0
    strongest = candidate_indices[np.argsort(spectrum[candidate_indices])[-8:]]
    strongest = sorted(strongest, key=lambda index: spectrum[index], reverse=True)
    selected: list[int] = []
    for index in strongest:
        if all(abs(freqs[index] - freqs[other]) > 20.0 for other in selected):
            selected.append(int(index))
        if len(selected) == 2:
            break
    if len(selected) < 2:
        return 440.0, 550.0
    first, second = sorted(float(freqs[index]) for index in selected)
    return first, second


def _imd_frequencies(first_hz: float, second_hz: float) -> list[float]:
    return [
        first_hz,
        second_hz,
        abs(second_hz - first_hz),
        first_hz + second_hz,
        2.0 * first_hz - second_hz,
        2.0 * second_hz - first_hz,
    ]


def _imd_ratio_db(levels: dict[float, float], first_hz: float, second_hz: float) -> float:
    fundamental_power = levels.get(first_hz, 0.0) ** 2 + levels.get(second_hz, 0.0) ** 2
    product_power = 0.0
    for frequency in _imd_frequencies(first_hz, second_hz):
        if frequency in (first_hz, second_hz):
            continue
        product_power += levels.get(frequency, 0.0) ** 2
    return linear_to_db(np.sqrt(product_power) / max(np.sqrt(fundamental_power), EPSILON))


def _line_ratio_delta(
    candidate: dict[float, float],
    reference: dict[float, float],
    frequency_hz: float,
    first_hz: float,
    second_hz: float,
) -> float | None:
    if frequency_hz <= 0.0 or frequency_hz not in candidate or frequency_hz not in reference:
        return None
    candidate_fundamental = np.sqrt(candidate.get(first_hz, 0.0) ** 2 + candidate.get(second_hz, 0.0) ** 2)
    reference_fundamental = np.sqrt(reference.get(first_hz, 0.0) ** 2 + reference.get(second_hz, 0.0) ** 2)
    candidate_ratio = candidate[frequency_hz] / max(candidate_fundamental, EPSILON)
    reference_ratio = reference[frequency_hz] / max(reference_fundamental, EPSILON)
    return linear_to_db(candidate_ratio / max(reference_ratio, EPSILON))


def _band_rms(samples: np.ndarray, sample_rate_hz: int, low_hz: float, high_hz: float) -> float:
    if samples.size < 8:
        return 0.0
    spectrum = np.fft.rfft(_windowed(samples))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    mask = (freqs >= low_hz) & (freqs <= high_hz)
    if not np.any(mask):
        return 0.0
    return float(np.sqrt(np.mean(np.square(np.abs(spectrum[mask])))) / max(samples.shape[0] / 2.0, 1.0))


def _relative_band_levels(samples: np.ndarray, sample_rate_hz: int) -> dict[str, float]:
    high = min(18_000.0, sample_rate_hz / 2.0)
    bands = {
        "low": _band_rms(samples, sample_rate_hz, 40.0, 250.0),
        "low_mid": _band_rms(samples, sample_rate_hz, 250.0, 1_000.0),
        "mid": _band_rms(samples, sample_rate_hz, 1_000.0, 4_000.0),
        "presence": _band_rms(samples, sample_rate_hz, 4_000.0, 8_000.0),
        "air": _band_rms(samples, sample_rate_hz, 8_000.0, high),
    }
    broadband = np.sqrt(sum(level * level for level in bands.values()))
    return {
        band: linear_to_db(level / max(broadband, EPSILON))
        for band, level in bands.items()
    }


def _active_short_term_levels(
    candidate: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
) -> tuple[np.ndarray, np.ndarray]:
    candidate_levels = _short_term_rms_dbfs(candidate, sample_rate_hz)
    reference_levels = _short_term_rms_dbfs(reference, sample_rate_hz)
    length = min(candidate_levels.shape[0], reference_levels.shape[0])
    if length == 0:
        return candidate_levels, reference_levels
    candidate_levels = candidate_levels[:length]
    reference_levels = reference_levels[:length]
    active_threshold = max(-80.0, float(np.percentile(reference_levels, 90)) - 50.0)
    mask = reference_levels >= active_threshold
    if not np.any(mask):
        return candidate_levels, reference_levels
    return candidate_levels[mask], reference_levels[mask]


def _short_term_rms_dbfs(samples: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    if samples.size == 0:
        return np.array([], dtype=np.float64)
    window = max(1, int(round(0.050 * sample_rate_hz)))
    hop = max(1, int(round(0.010 * sample_rate_hz)))
    if samples.shape[0] < window:
        return np.array([linear_to_db(rms(samples))], dtype=np.float64)
    levels = []
    for start in range(0, samples.shape[0] - window + 1, hop):
        levels.append(linear_to_db(rms(samples[start : start + window])))
    return np.asarray(levels, dtype=np.float64)


def _level_percentiles(levels_dbfs: np.ndarray) -> tuple[float, float, float]:
    if levels_dbfs.size == 0:
        return -240.0, -240.0, -240.0
    p10, p50, p90 = np.percentile(levels_dbfs, [10, 50, 90])
    return float(p10), float(p50), float(p90)


def _decay_shape(samples: np.ndarray, sample_rate_hz: int) -> tuple[float, float]:
    envelope = _smooth_envelope(samples, sample_rate_hz)
    if envelope.size < 8:
        return 0.0, -240.0
    levels = 20.0 * np.log10(np.maximum(envelope, EPSILON))
    time = np.arange(levels.shape[0], dtype=np.float64) / sample_rate_hz
    slope = float(np.polyfit(time, levels, 1)[0])
    late_start = int(round(levels.shape[0] * 0.65))
    late_level = float(np.median(levels[late_start:])) if late_start < levels.shape[0] else float(levels[-1])
    return slope, late_level


def _low_frequency_envelope_depth(envelope: np.ndarray, sample_rate_hz: int) -> float:
    if envelope.size < sample_rate_hz // 2:
        return -240.0
    low = _band_limited_rms(envelope - np.mean(envelope), sample_rate_hz, 0.5, 12.0)
    return linear_to_db(low / max(rms(envelope), EPSILON))


def _band_limited_rms(samples: np.ndarray, sample_rate_hz: int, low_hz: float, high_hz: float) -> float:
    if samples.size < 16:
        return 0.0
    sos = signal.butter(4, [low_hz, min(high_hz, sample_rate_hz / 2.0 - 1.0)], btype="bandpass", fs=sample_rate_hz, output="sos")
    return rms(signal.sosfiltfilt(sos, samples))


def _stable_tone_windows(reference: np.ndarray, sample_rate_hz: int) -> list[tuple[int, int, float]]:
    window = max(1024, int(round(0.120 * sample_rate_hz)))
    hop = max(256, window // 2)
    if reference.shape[0] < window:
        return []
    result: list[tuple[int, int, float]] = []
    for start in range(0, reference.shape[0] - window + 1, hop):
        end = start + window
        chunk = reference[start:end]
        if rms(chunk) < 0.005:
            continue
        fundamental = _estimate_fundamental(chunk, sample_rate_hz)
        if fundamental < 70.0 or fundamental > 2_000.0:
            continue
        if _spectral_peak_dominance(chunk, sample_rate_hz, fundamental) < 0.35:
            continue
        result.append((start, end, fundamental))
        if len(result) >= 16:
            break
    return result


def _spectral_peak_dominance(samples: np.ndarray, sample_rate_hz: int, frequency_hz: float) -> float:
    spectrum = np.abs(np.fft.rfft(_windowed(samples)))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    band = (freqs >= 70.0) & (freqs <= min(8_000.0, sample_rate_hz / 2.0))
    if not np.any(band):
        return 0.0
    index = int(np.argmin(np.abs(freqs - frequency_hz)))
    left = max(0, index - 2)
    right = min(spectrum.shape[0], index + 3)
    peak_power = float(np.sum(np.square(spectrum[left:right])))
    total_power = float(np.sum(np.square(spectrum[band])))
    return peak_power / max(total_power, EPSILON)


def _spectral_residual_excluding_strong_lines(
    samples: np.ndarray,
    sample_rate_hz: int,
    low_hz: float,
    high_hz: float,
) -> float:
    if samples.size < 32:
        return 0.0
    spectrum = np.abs(np.fft.rfft(_windowed(samples)))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    mask = (freqs >= low_hz) & (freqs <= high_hz)
    indices = np.flatnonzero(mask)
    if indices.size < 8:
        return 0.0
    power = np.square(spectrum[indices])
    strongest = np.argsort(power)[-min(12, power.shape[0]) :]
    keep = np.ones(power.shape[0], dtype=bool)
    for peak in strongest:
        left = max(0, int(peak) - 2)
        right = min(power.shape[0], int(peak) + 3)
        keep[left:right] = False
    if not np.any(keep):
        return 0.0
    return float(np.sqrt(np.mean(power[keep])) / max(samples.shape[0] / 2.0, 1.0))


def _spectral_flatness(samples: np.ndarray, sample_rate_hz: int, low_hz: float, high_hz: float) -> float:
    if samples.size < 32:
        return -240.0
    spectrum = np.abs(np.fft.rfft(_windowed(samples)))
    freqs = np.fft.rfftfreq(samples.shape[0], 1.0 / sample_rate_hz)
    mask = (freqs >= low_hz) & (freqs <= high_hz)
    if not np.any(mask):
        return -240.0
    power = np.square(spectrum[mask]) + EPSILON
    return linear_to_db(float(np.exp(np.mean(np.log(power))) / max(np.mean(power), EPSILON)))


def _median_or_zero(values: list[float]) -> float:
    if not values:
        return 0.0
    return float(np.median(np.asarray(values)))


def _band_relative_residual(
    residual: np.ndarray,
    reference: np.ndarray,
    sample_rate_hz: int,
    low_hz: float,
    high_hz: float,
) -> float:
    residual_level = _band_rms(residual, sample_rate_hz, low_hz, high_hz)
    reference_level = _band_rms(reference, sample_rate_hz, low_hz, high_hz)
    return linear_to_db(residual_level / max(reference_level, EPSILON))


def _transient_onsets(reference: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    envelope = _smooth_envelope(reference, sample_rate_hz)
    if envelope.size < 3 or float(np.max(envelope)) <= EPSILON:
        return np.array([], dtype=np.int64)
    positive_slope = np.maximum(np.diff(envelope, prepend=envelope[0]), 0.0)
    if float(np.max(positive_slope)) <= EPSILON:
        return np.array([], dtype=np.int64)
    active_floor = float(np.percentile(envelope, 70))
    threshold = max(
        float(np.percentile(positive_slope, 95)),
        float(np.max(positive_slope)) * 0.12,
        EPSILON,
    )
    min_distance = max(1, int(round(0.050 * sample_rate_hz)))
    peaks, properties = signal.find_peaks(positive_slope, height=threshold, distance=min_distance)
    if peaks.size == 0:
        return np.array([], dtype=np.int64)
    heights = properties["peak_heights"]
    active = envelope[peaks] >= active_floor
    peaks = peaks[active]
    heights = heights[active]
    if peaks.size == 0:
        return np.array([], dtype=np.int64)
    strongest = peaks[np.argsort(heights)[-12:]]
    return np.sort(strongest).astype(np.int64)


def _high_band_ratio_db(samples: np.ndarray, sample_rate_hz: int) -> float:
    high = _band_rms(samples, sample_rate_hz, 2_000.0, min(12_000.0, sample_rate_hz / 2.0))
    return linear_to_db(high / max(rms(samples), EPSILON))


def _sag_shape(samples: np.ndarray, sample_rate_hz: int) -> tuple[float, float]:
    early = _window_rms(samples, 0, int(round(0.050 * sample_rate_hz)))
    middle_start = int(round(0.200 * sample_rate_hz))
    middle = _window_rms(samples, middle_start, middle_start + int(round(0.100 * sample_rate_hz)))
    late_start = max(0, samples.shape[0] - int(round(0.150 * sample_rate_hz)))
    late = _window_rms(samples, late_start, samples.shape[0])
    drop = linear_to_db(middle / max(early, EPSILON))
    recovery = linear_to_db(late / max(middle, EPSILON))
    return drop, recovery


def _window_rms(samples: np.ndarray, start: int, end: int) -> float:
    start = max(0, min(start, samples.shape[0]))
    end = max(start, min(end, samples.shape[0]))
    return rms(samples[start:end])


def _windowed(samples: np.ndarray) -> np.ndarray:
    if samples.size == 0:
        return samples
    return samples * signal.windows.hann(samples.shape[0], sym=False)
