from __future__ import annotations

import matplotlib
import numpy as np
from scipy.io import wavfile

matplotlib.use("Agg")

from greybound_lab.notebook import (
    load_wav_comparison,
    plot_diagnostic_bars,
    plot_overview,
    plot_residual_spectrogram,
    plot_spectrum,
)


def test_load_wav_comparison_aligns_and_exposes_residual(tmp_path) -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate, dtype=np.float32) / sample_rate
    reference = 0.25 * np.sin(2.0 * np.pi * 440.0 * time)
    candidate = np.concatenate([np.zeros(120, dtype=np.float32), reference * 0.5])
    reference_path = tmp_path / "reference.wav"
    candidate_path = tmp_path / "candidate.wav"
    wavfile.write(reference_path, sample_rate, reference)
    wavfile.write(candidate_path, sample_rate, candidate)

    analysis = load_wav_comparison(candidate_path, reference_path, max_lag_ms=20.0)

    assert analysis.metrics.latency_samples == 120
    assert abs(analysis.metrics.gain_db - 6.0206) < 0.02
    assert analysis.metrics.null_relative_db < -100.0
    assert analysis.corrected_candidate.shape == analysis.aligned_reference.shape
    assert analysis.residual.shape == analysis.corrected_candidate.shape


def test_notebook_plots_return_figures(tmp_path) -> None:
    sample_rate = 48_000
    time = np.arange(sample_rate // 2, dtype=np.float32) / sample_rate
    reference = 0.25 * np.sin(2.0 * np.pi * 440.0 * time)
    candidate = reference + 0.02 * np.sin(2.0 * np.pi * 2_000.0 * time)
    reference_path = tmp_path / "reference.wav"
    candidate_path = tmp_path / "candidate.wav"
    wavfile.write(reference_path, sample_rate, reference.astype(np.float32))
    wavfile.write(candidate_path, sample_rate, candidate.astype(np.float32))
    analysis = load_wav_comparison(candidate_path, reference_path)

    figures = [
        plot_overview(analysis, start_s=0.0, end_s=0.05),
        plot_spectrum(analysis),
        plot_residual_spectrogram(analysis),
        plot_diagnostic_bars(analysis),
    ]

    assert all(figure.axes for figure in figures)
    for figure in figures:
        figure.clf()
