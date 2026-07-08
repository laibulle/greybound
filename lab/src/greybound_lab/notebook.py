from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.figure import Figure
from scipy import signal

from greybound_lab.audio import AudioBuffer, read_wav_mono
from greybound_lab.metrics import (
    BandResidualMetrics,
    ComparisonMetrics,
    align_by_latency,
    compare_signals,
    optimal_gain,
)
from greybound_lab.segments import SegmentSpec, load_segments


@dataclass(frozen=True)
class WavComparisonAnalysis:
    candidate: AudioBuffer
    reference: AudioBuffer
    aligned_candidate: np.ndarray
    aligned_reference: np.ndarray
    corrected_candidate: np.ndarray
    residual: np.ndarray
    metrics: ComparisonMetrics
    segments: tuple[SegmentSpec, ...] = ()

    @property
    def sample_rate_hz(self) -> int:
        return self.metrics.sample_rate_hz

    @property
    def duration_seconds(self) -> float:
        return self.corrected_candidate.shape[0] / self.sample_rate_hz

    @property
    def time_seconds(self) -> np.ndarray:
        return np.arange(self.corrected_candidate.shape[0], dtype=np.float64) / self.sample_rate_hz


def load_wav_comparison(
    candidate_path: str | Path,
    reference_path: str | Path,
    segments_path: str | Path | None = None,
    max_lag_ms: float = 100.0,
) -> WavComparisonAnalysis:
    candidate = read_wav_mono(Path(candidate_path))
    reference = read_wav_mono(Path(reference_path))
    if candidate.sample_rate != reference.sample_rate:
        raise ValueError(
            f"sample-rate mismatch: candidate {candidate.sample_rate} Hz, "
            f"reference {reference.sample_rate} Hz"
        )
    segments = tuple(load_segments(Path(segments_path))) if segments_path else ()
    metrics = compare_signals(
        candidate.samples,
        reference.samples,
        candidate.sample_rate,
        max_lag_ms=max_lag_ms,
        segments=list(segments),
    )
    aligned_candidate, aligned_reference = align_by_latency(
        candidate.samples,
        reference.samples,
        metrics.latency_samples,
    )
    gain = optimal_gain(aligned_candidate, aligned_reference)
    corrected_candidate = aligned_candidate * gain
    residual = corrected_candidate - aligned_reference
    return WavComparisonAnalysis(
        candidate=candidate,
        reference=reference,
        aligned_candidate=aligned_candidate,
        aligned_reference=aligned_reference,
        corrected_candidate=corrected_candidate,
        residual=residual,
        metrics=metrics,
        segments=segments,
    )


def print_metric_summary(analysis: WavComparisonAnalysis) -> None:
    metrics = analysis.metrics
    rows = [
        ("Latency", f"{metrics.latency_samples} samples / {metrics.latency_ms:.3f} ms"),
        ("Gain correction", f"{metrics.gain_db:.2f} dB"),
        ("Null residual", f"{metrics.null_relative_db:.2f} dB relative"),
        ("Log-spectral distance", f"{metrics.log_spectral_distance_db:.2f} dB"),
        ("Weighted LSD", f"{metrics.weighted_log_spectral_distance_db:.2f} dB"),
        ("Envelope error", f"{metrics.envelope_error_db:.2f} dB"),
        ("Peak candidate", f"{metrics.candidate.peak_dbfs:.2f} dBFS"),
        ("Peak reference", f"{metrics.reference.peak_dbfs:.2f} dBFS"),
    ]
    width = max(len(label) for label, _ in rows)
    for label, value in rows:
        print(f"{label:<{width}}  {value}")


def plot_overview(
    analysis: WavComparisonAnalysis,
    start_s: float | None = None,
    end_s: float | None = None,
) -> Figure:
    view = _slice_view(analysis, start_s, end_s)
    time = analysis.time_seconds[view]
    candidate = analysis.corrected_candidate[view]
    reference = analysis.aligned_reference[view]
    residual = analysis.residual[view]
    candidate_env = _envelope(candidate, analysis.sample_rate_hz)
    reference_env = _envelope(reference, analysis.sample_rate_hz)

    figure, axes = plt.subplots(3, 1, figsize=(12, 8), sharex=True, constrained_layout=True)
    axes[0].plot(time, reference, label="reference", linewidth=0.9)
    axes[0].plot(time, candidate, label="candidate aligned", linewidth=0.8, alpha=0.78)
    axes[0].set_title("Aligned waveform")
    axes[0].set_ylabel("Amplitude")
    axes[0].legend(loc="upper right")
    axes[0].grid(True, alpha=0.25)

    axes[1].plot(time, reference_env, label="reference envelope", linewidth=1.0)
    axes[1].plot(time, candidate_env, label="candidate envelope", linewidth=1.0)
    axes[1].set_title("Envelope")
    axes[1].set_ylabel("Amplitude")
    axes[1].legend(loc="upper right")
    axes[1].grid(True, alpha=0.25)

    axes[2].plot(time, residual, color="tab:red", linewidth=0.8)
    axes[2].set_title("Residual after latency and gain alignment")
    axes[2].set_xlabel("Time (s)")
    axes[2].set_ylabel("Amplitude")
    axes[2].grid(True, alpha=0.25)
    return figure


def plot_spectrum(analysis: WavComparisonAnalysis) -> Figure:
    frequency, candidate_db = _welch_db(analysis.corrected_candidate, analysis.sample_rate_hz)
    _, reference_db = _welch_db(analysis.aligned_reference, analysis.sample_rate_hz)
    _, residual_db = _welch_db(analysis.residual, analysis.sample_rate_hz)

    figure, axis = plt.subplots(figsize=(12, 5), constrained_layout=True)
    axis.semilogx(frequency, reference_db, label="reference", linewidth=1.0)
    axis.semilogx(frequency, candidate_db, label="candidate aligned", linewidth=1.0)
    axis.semilogx(frequency, residual_db, label="residual", linewidth=0.9, alpha=0.85)
    axis.set_xlim(40.0, min(20_000.0, analysis.sample_rate_hz / 2.0))
    axis.set_ylim(max(-160.0, float(np.nanmax(reference_db)) - 110.0), float(np.nanmax(reference_db)) + 8.0)
    axis.set_title("Welch spectrum")
    axis.set_xlabel("Frequency (Hz)")
    axis.set_ylabel("Level (dBFS/bin)")
    axis.grid(True, which="both", alpha=0.25)
    axis.legend(loc="best")
    return figure


def plot_residual_spectrogram(analysis: WavComparisonAnalysis) -> Figure:
    nperseg = min(2048, max(256, _largest_power_of_two(analysis.residual.shape[0] // 32)))
    frequencies, times, spectrum = signal.spectrogram(
        analysis.residual,
        fs=analysis.sample_rate_hz,
        nperseg=nperseg,
        noverlap=nperseg // 2,
        scaling="spectrum",
    )
    spectrum_db = 10.0 * np.log10(np.maximum(spectrum, 1.0e-24))

    figure, axis = plt.subplots(figsize=(12, 5), constrained_layout=True)
    image = axis.pcolormesh(times, frequencies, spectrum_db, shading="auto", cmap="magma")
    axis.set_ylim(40.0, min(18_000.0, analysis.sample_rate_hz / 2.0))
    axis.set_yscale("log")
    axis.set_title("Residual spectrogram")
    axis.set_xlabel("Time (s)")
    axis.set_ylabel("Frequency (Hz)")
    figure.colorbar(image, ax=axis, label="Residual energy (dB)")
    return figure


def plot_diagnostic_bars(analysis: WavComparisonAnalysis) -> Figure:
    balance = analysis.metrics.spectral_balance
    balance_labels = ["Low", "Low-mid", "Mid", "Presence", "Air"]
    balance_values = [
        balance.low_delta_db,
        balance.low_mid_delta_db,
        balance.mid_delta_db,
        balance.presence_delta_db,
        balance.air_delta_db,
    ]
    dynamics = analysis.metrics.dynamics
    dynamics_labels = ["P10", "P50", "P90", "Range"]
    dynamics_values = [
        dynamics.p10_delta_db,
        dynamics.p50_delta_db,
        dynamics.p90_delta_db,
        dynamics.dynamic_range_delta_db,
    ]

    figure, axes = plt.subplots(1, 2, figsize=(12, 4.5), constrained_layout=True)
    _barh(axes[0], balance_labels, balance_values, "Spectral balance delta (candidate - reference)")
    _barh(axes[1], dynamics_labels, dynamics_values, "Short-term dynamics delta")
    return figure


def plot_segment_band_residuals(analysis: WavComparisonAnalysis) -> Figure | None:
    if not analysis.metrics.segments:
        return None
    labels = [segment.name for segment in analysis.metrics.segments]
    bands = ["Low", "Low-mid", "Mid", "Presence", "Air"]
    values = np.asarray([
        _band_residual_values(segment.band_residual)
        for segment in analysis.metrics.segments
    ])
    x = np.arange(len(labels))
    width = 0.14

    figure, axis = plt.subplots(figsize=(max(10.0, len(labels) * 0.9), 5), constrained_layout=True)
    for index, band in enumerate(bands):
        axis.bar(x + (index - 2) * width, values[:, index], width=width, label=band)
    axis.axhline(0.0, color="black", linewidth=0.8)
    axis.set_title("Segment band residuals")
    axis.set_ylabel("Residual relative to reference band (dB)")
    axis.set_xticks(x)
    axis.set_xticklabels(labels, rotation=30, ha="right")
    axis.grid(True, axis="y", alpha=0.25)
    axis.legend(loc="best")
    return figure


def _slice_view(
    analysis: WavComparisonAnalysis,
    start_s: float | None,
    end_s: float | None,
) -> slice:
    sample_rate = analysis.sample_rate_hz
    start = 0 if start_s is None else max(0, int(round(start_s * sample_rate)))
    end = analysis.corrected_candidate.shape[0] if end_s is None else int(round(end_s * sample_rate))
    end = min(max(start + 1, end), analysis.corrected_candidate.shape[0])
    return slice(start, end)


def _envelope(samples: np.ndarray, sample_rate_hz: int) -> np.ndarray:
    if samples.shape[0] < 4:
        return np.abs(samples)
    envelope = np.abs(signal.hilbert(samples))
    window = max(1, int(round(sample_rate_hz * 0.005)))
    if window <= 1:
        return envelope
    kernel = np.ones(window, dtype=np.float64) / window
    return np.convolve(envelope, kernel, mode="same")


def _welch_db(samples: np.ndarray, sample_rate_hz: int) -> tuple[np.ndarray, np.ndarray]:
    nperseg = min(8192, max(512, _largest_power_of_two(samples.shape[0] // 8)))
    frequency, power = signal.welch(samples, fs=sample_rate_hz, nperseg=nperseg)
    return frequency[1:], 10.0 * np.log10(np.maximum(power[1:], 1.0e-24))


def _barh(axis: plt.Axes, labels: list[str], values: list[float], title: str) -> None:
    colors = ["tab:red" if value > 0.0 else "tab:blue" for value in values]
    axis.barh(labels, values, color=colors, alpha=0.82)
    axis.axvline(0.0, color="black", linewidth=0.8)
    axis.set_title(title)
    axis.set_xlabel("dB")
    axis.grid(True, axis="x", alpha=0.25)


def _band_residual_values(band: BandResidualMetrics) -> list[float]:
    return [band.low_db, band.low_mid_db, band.mid_db, band.presence_db, band.air_db]


def _largest_power_of_two(value: int) -> int:
    return 1 << max(0, int(value).bit_length() - 1)
