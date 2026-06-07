from __future__ import annotations

from pathlib import Path

from greybound_lab.metrics import ComparisonMetrics


def write_markdown_report(
    path: Path,
    candidate_path: Path,
    reference_path: Path,
    metrics: ComparisonMetrics,
    metadata_path: Path | None = None,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    metadata_line = str(metadata_path) if metadata_path else "not provided"
    path.write_text(
        _render_markdown(candidate_path, reference_path, metrics, metadata_line),
        encoding="utf-8",
    )


def _render_markdown(
    candidate_path: Path,
    reference_path: Path,
    metrics: ComparisonMetrics,
    metadata_line: str,
) -> str:
    return f"""# WAV Comparison Report

## Inputs

- Candidate: `{candidate_path}`
- Reference: `{reference_path}`
- Metadata: `{metadata_line}`
- Sample rate: {metrics.sample_rate_hz} Hz
- Candidate samples: {metrics.candidate_samples}
- Reference samples: {metrics.reference_samples}
- Compared samples: {metrics.compared_samples}

## Alignment

- Estimated candidate latency: {metrics.latency_samples} samples ({metrics.latency_ms:.3f} ms)
- Candidate gain correction: {metrics.gain_db:.3f} dB

## Levels

| Signal | RMS dBFS | Peak dBFS | Crest dB |
| --- | ---: | ---: | ---: |
| Candidate input | {metrics.candidate.rms_dbfs:.2f} | {metrics.candidate.peak_dbfs:.2f} | {metrics.candidate.crest_db:.2f} |
| Reference input | {metrics.reference.rms_dbfs:.2f} | {metrics.reference.peak_dbfs:.2f} | {metrics.reference.crest_db:.2f} |
| Candidate aligned | {metrics.aligned_candidate.rms_dbfs:.2f} | {metrics.aligned_candidate.peak_dbfs:.2f} | {metrics.aligned_candidate.crest_db:.2f} |
| Reference aligned | {metrics.aligned_reference.rms_dbfs:.2f} | {metrics.aligned_reference.peak_dbfs:.2f} | {metrics.aligned_reference.crest_db:.2f} |

## Error Metrics

- Null residual RMS: {metrics.null_rms_dbfs:.2f} dBFS
- Null residual relative to reference: {metrics.null_relative_db:.2f} dB
- Log-spectral distance: {metrics.log_spectral_distance_db:.2f} dB
- Envelope error: {metrics.envelope_error_db:.2f} dB

## Engineering Notes

Use this report as a directional diagnostic, not as a single quality score. A
good next analysis pass should inspect whether the residual is dominated by
level, latency, spectral tilt, transient behavior, or nonlinear dynamics.
"""
