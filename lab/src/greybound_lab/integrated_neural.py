from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from greybound_lab.audio import read_wav_mono
from greybound_lab.metrics import ComparisonMetrics, compare_signals
from greybound_lab.render import render_rig
from greybound_lab.segments import load_segments


SHADOW_RE = re.compile(
    r"shadow first abs err avg/max (?P<avg>[0-9.]+)/(?P<max>[0-9.]+) V n (?P<count>[0-9]+)"
)


@dataclass(frozen=True)
class IntegratedNeuralReport:
    analytic_wav: Path
    shadow_wav: Path
    replace_wav: Path
    shadow_monitor_log: Path
    replace_vs_analytic: ComparisonMetrics
    shadow_error_avg_v: float | None
    shadow_error_max_v: float | None
    shadow_error_count: int


def evaluate_integrated_neural_cell(
    *,
    repo_root: Path,
    binary: Path,
    rig: Path,
    input_wav: Path,
    descriptor: Path,
    output_dir: Path,
    report: Path,
    component: str = "nox30.first_stage",
    render_seconds: float = 20.0,
    sample_rate_hz: int = 48_000,
    period_size: int = 16,
    input_gain_db: float = 0.0,
    output_gain_db: float = -12.0,
    ir_enabled: bool = True,
    ir_wav: Path | None = None,
    segments: Path | None = None,
) -> IntegratedNeuralReport:
    output_dir.mkdir(parents=True, exist_ok=True)
    analytic_wav = output_dir / "analytic.wav"
    shadow_wav = output_dir / "shadow.wav"
    replace_wav = output_dir / "replace.wav"
    analytic_metadata = output_dir / "analytic.run.json"
    shadow_metadata = output_dir / "shadow.run.json"
    replace_metadata = output_dir / "replace.run.json"
    shadow_log = output_dir / "shadow.monitor.log"

    render_rig(
        repo_root=repo_root,
        binary=binary,
        rig=rig,
        input_wav=input_wav,
        output_wav=analytic_wav,
        metadata=analytic_metadata,
        render_seconds=render_seconds,
        sample_rate_hz=sample_rate_hz,
        period_size=period_size,
        input_gain_db=input_gain_db,
        output_gain_db=output_gain_db,
        ir_enabled=ir_enabled,
        ir_wav=ir_wav,
    )
    render_rig(
        repo_root=repo_root,
        binary=binary,
        rig=rig,
        input_wav=input_wav,
        output_wav=shadow_wav,
        metadata=shadow_metadata,
        render_seconds=render_seconds,
        sample_rate_hz=sample_rate_hz,
        period_size=period_size,
        input_gain_db=input_gain_db,
        output_gain_db=output_gain_db,
        ir_enabled=ir_enabled,
        ir_wav=ir_wav,
        monitor_enabled=True,
        monitor_log=shadow_log,
        neural_cell=(component, descriptor),
        neural_cell_mode="shadow",
    )
    render_rig(
        repo_root=repo_root,
        binary=binary,
        rig=rig,
        input_wav=input_wav,
        output_wav=replace_wav,
        metadata=replace_metadata,
        render_seconds=render_seconds,
        sample_rate_hz=sample_rate_hz,
        period_size=period_size,
        input_gain_db=input_gain_db,
        output_gain_db=output_gain_db,
        ir_enabled=ir_enabled,
        ir_wav=ir_wav,
        neural_cell=(component, descriptor),
        neural_cell_mode="replace",
    )

    analytic = read_wav_mono(analytic_wav)
    replace = read_wav_mono(replace_wav)
    if analytic.sample_rate != replace.sample_rate:
        raise ValueError("integrated neural render sample-rate mismatch")
    metrics = compare_signals(
        replace.samples,
        analytic.samples,
        analytic.sample_rate,
        max_lag_ms=100.0,
        segments=load_segments(segments) if segments else None,
    )
    shadow_avg, shadow_max, shadow_count = parse_shadow_error(shadow_log)
    result = IntegratedNeuralReport(
        analytic_wav=analytic_wav,
        shadow_wav=shadow_wav,
        replace_wav=replace_wav,
        shadow_monitor_log=shadow_log,
        replace_vs_analytic=metrics,
        shadow_error_avg_v=shadow_avg,
        shadow_error_max_v=shadow_max,
        shadow_error_count=shadow_count,
    )
    write_integrated_neural_report(report, result, component, descriptor, rig, input_wav)
    return result


def parse_shadow_error(path: Path) -> tuple[float | None, float | None, int]:
    if not path.exists():
        return None, None, 0
    latest: tuple[float | None, float | None, int] = (None, None, 0)
    for line in path.read_text(encoding="utf-8").splitlines():
        match = SHADOW_RE.search(line)
        if match:
            latest = (
                float(match.group("avg")),
                float(match.group("max")),
                int(match.group("count")),
            )
    return latest


def write_integrated_neural_report(
    path: Path,
    result: IntegratedNeuralReport,
    component: str,
    descriptor: Path,
    rig: Path,
    input_wav: Path,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    metrics = result.replace_vs_analytic
    path.write_text(
        f"""# Integrated Neural Cell Report

## Inputs

- Component: `{component}`
- Descriptor: `{descriptor}`
- Rig: `{rig}`
- Input WAV: `{input_wav}`
- Analytic render: `{result.analytic_wav}`
- Shadow render: `{result.shadow_wav}`
- Replace render: `{result.replace_wav}`
- Shadow monitor log: `{result.shadow_monitor_log}`

## Shadow Telemetry

- First-stage absolute error average: {_format_optional_v(result.shadow_error_avg_v)}
- First-stage absolute error max: {_format_optional_v(result.shadow_error_max_v)}
- Shadow telemetry samples: {result.shadow_error_count}

## Replace vs Analytic Audio

| Metric | Value |
| --- | ---: |
| Compared samples | {metrics.compared_samples} |
| Estimated latency | {metrics.latency_samples} samples / {metrics.latency_ms:.3f} ms |
| Gain correction | {metrics.gain_db:.3f} dB |
| Null residual RMS | {metrics.null_rms_dbfs:.2f} dBFS |
| Null residual relative | {metrics.null_relative_db:.2f} dB |
| Log-spectral distance | {metrics.log_spectral_distance_db:.2f} dB |
| Envelope error | {metrics.envelope_error_db:.2f} dB |

## Decision

This report is a first integration diagnostic. `shadow` measures component error
without changing audio. `replace` shows how much the complete rendered chain
changes when the neural counterpart feeds the rest of Nox30.
""",
        encoding="utf-8",
    )


def _format_optional_v(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.6f} V"
