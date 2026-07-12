from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
from pathlib import Path

import numpy as np

from greybound_lab.metrics import linear_to_db, rms
from greybound_lab.render import git_revision, relative_or_absolute


@dataclass(frozen=True)
class SpiceFixture:
    name: str
    netlist_path: Path
    tmp_data_path: Path
    signals: tuple[str, ...]


@dataclass(frozen=True)
class SpiceTrace:
    time_s: np.ndarray
    signals: dict[str, np.ndarray]


@dataclass(frozen=True)
class CommonCathodeSpiceMetrics:
    plate_dc_v: float
    cathode_dc_v: float
    bplus_dc_v: float
    input_rms_v: float
    grid_rms_v: float
    plate_rms_v: float
    cathode_rms_v: float
    plate_gain: float
    plate_gain_db: float
    grid_coupling_loss_db: float


@dataclass(frozen=True)
class KlonCentaurSpiceMetrics:
    input_rms_v: float
    buffer_rms_v: float
    clean_rms_v: float
    drive_rms_v: float
    clip_rms_v: float
    mix_rms_v: float
    tone_rms_v: float
    output_rms_v: float
    output_peak_v: float
    output_gain: float
    output_gain_db: float
    clip_peak_v: float
    clip_asymmetry_v: float


@dataclass(frozen=True)
class NoneStarTonePresenceSpiceMetrics:
    low_250hz_db: float
    mid_1khz_db: float
    presence_4khz_db: float
    presence_8khz_db: float
    air_16khz_db: float
    tone_8khz_db: float
    output_8khz_db: float
    presence_lift_8khz_db: float
    output_minus_1khz_8khz_db: float


@dataclass(frozen=True)
class DaybreakerPresenceFilterSpiceMetrics:
    transformer_1khz_db: float
    presence_band_1khz_db: float
    output_1khz_db: float
    output_4khz_db: float
    output_8khz_db: float
    output_16khz_db: float
    output_minus_1khz_4khz_db: float
    output_minus_1khz_16khz_db: float


@dataclass(frozen=True)
class DaybreakerClassicTmbSpiceMetrics:
    input_1khz_db: float
    output_100hz_db: float
    output_250hz_db: float
    output_1khz_db: float
    output_4khz_db: float
    output_8khz_db: float
    output_16khz_db: float
    output_minus_1khz_4khz_db: float
    output_minus_1khz_16khz_db: float


@dataclass(frozen=True)
class DaybreakerSss002HighLowSpiceMetrics:
    high_100hz_db: tuple[float, ...]
    high_1khz_db: tuple[float, ...]
    high_8khz_db: tuple[float, ...]
    high_8khz_minus_1khz_db: tuple[float, ...]
    low_100hz_db: tuple[float, ...]
    low_1khz_db: tuple[float, ...]
    low_8khz_db: tuple[float, ...]
    low_8khz_minus_1khz_db: tuple[float, ...]


@dataclass(frozen=True)
class DaybreakerSss002HighLowChainSpiceMetrics:
    output_100hz_db: float
    output_1khz_db: float
    output_8khz_db: float
    output_16khz_db: float
    output_minus_1khz_8khz_db: float


@dataclass(frozen=True)
class DaybreakerSss002ToneDeepSpiceMetrics:
    grid_100hz_db: float
    grid_1khz_db: float
    grid_8khz_db: float
    grid_16khz_db: float
    grid_minus_1khz_8khz_db: float


@dataclass(frozen=True)
class DaybreakerSss002U37RecoverySpiceMetrics:
    plate_dc_v: float
    cathode_dc_v: float
    bplus_dc_v: float
    filter_output_rms_v: float
    plate_rms_v: float
    recovery_output_rms_v: float
    plate_gain: float
    plate_gain_db: float


@dataclass(frozen=True)
class DaybreakerSss002U4PlateSpiceMetrics:
    plate_dc_v: float
    cathode_dc_v: float
    hta_dc_v: float
    grid_rms_v: float
    plate_rms_v: float
    output_rms_v: float
    plate_gain: float
    plate_gain_db: float


@dataclass(frozen=True)
class DaybreakerSss002U5VolumeU4SpiceMetrics:
    plate_dc_v: float
    cathode_dc_v: float
    hta_dc_v: float
    source_rms_v: float
    wiper_rms_v: float
    grid_rms_v: float
    output_rms_v: float
    wiper_gain: float
    wiper_gain_db: float
    plate_gain: float
    plate_gain_db: float


@dataclass(frozen=True)
class DaybreakerTmbRecoverySpiceMetrics:
    plate_dc_v: float
    cathode_dc_v: float
    bplus_dc_v: float
    stack_output_rms_v: float
    grid_rms_v: float
    plate_rms_v: float
    recovery_output_rms_v: float
    plate_gain: float
    plate_gain_db: float


@dataclass(frozen=True)
class CommonCathodeDatasetCase:
    stimulus_id: str
    kind: str
    expression: str
    parameters: dict[str, float | str]
    split: str
    settle_time_s: float = 0.030
    transient_stop_s: float = 0.060
    transient_step_s: float = 1.0e-6


@dataclass(frozen=True)
class KlonCentaurDatasetCase:
    stimulus_id: str
    kind: str
    expression: str
    parameters: dict[str, float | str]
    split: str
    gain: float
    treble: float
    level: float = 0.70
    transient_stop_s: float = 0.120
    transient_step_s: float = 2.0e-6


FIXTURES = {
    "common-cathode-12ax7": SpiceFixture(
        name="common-cathode-12ax7",
        netlist_path=Path("tests/fixtures/circuit/common_cathode_12ax7.cir"),
        tmp_data_path=Path("/tmp/greybound_common_cathode_12ax7.dat"),
        signals=("input", "grid", "plate", "cathode", "bplus"),
    ),
    "klon-centaur": SpiceFixture(
        name="klon-centaur",
        netlist_path=Path("tests/fixtures/circuit/klon_centaur.cir"),
        tmp_data_path=Path("/tmp/greybound_klon_centaur.dat"),
        signals=("input", "buffer", "clean", "drive", "clip", "mix", "tone", "output"),
    ),
    "none-star-tone-presence": SpiceFixture(
        name="none-star-tone-presence",
        netlist_path=Path("tests/fixtures/circuit/none_star_tone_presence.cir"),
        tmp_data_path=Path("/tmp/greybound_none_star_tone_presence.dat"),
        signals=("input", "tone", "output"),
    ),
    "daybreaker-presence-filter": SpiceFixture(
        name="daybreaker-presence-filter",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_presence_filter.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_presence_filter.dat"),
        signals=("input", "transformer", "presence_band", "output"),
    ),
    "daybreaker-classic-tmb": SpiceFixture(
        name="daybreaker-classic-tmb",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_classic_tmb.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_classic_tmb.dat"),
        signals=("source", "input", "tone", "output"),
    ),
    "daybreaker-sss002-classic-tmb": SpiceFixture(
        name="daybreaker-sss002-classic-tmb",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_classic_tmb.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_classic_tmb.dat"),
        signals=("source", "input", "tone", "output"),
    ),
    "daybreaker-sss002-high-low-filters": SpiceFixture(
        name="daybreaker-sss002-high-low-filters",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_high_low_filters.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_high_low_filters.dat"),
        signals=(
            "source",
            "high_1", "high_2", "high_3", "high_4", "high_5", "high_6", "high_7",
            "low_1", "low_2", "low_3", "low_4", "low_5", "low_6", "low_7",
        ),
    ),
    "daybreaker-sss002-high-low-chain": SpiceFixture(
        name="daybreaker-sss002-high-low-chain",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_high_low_chain.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_high_low_chain.dat"),
        signals=("source", "high_input", "output", "low_common"),
    ),
    "daybreaker-sss002-tone-deep-asc": SpiceFixture(
        name="daybreaker-sss002-tone-deep-asc",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_tone_deep_asc.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_tone_deep_asc.dat"),
        signals=("source", "plate", "tone_source", "treble_wiper", "bass_wiper", "u5_input", "volume_wiper", "grid"),
    ),
    "daybreaker-sss002-tone-deep-layout": SpiceFixture(
        name="daybreaker-sss002-tone-deep-layout",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_tone_deep_layout.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_tone_deep_layout.dat"),
        signals=("source", "plate", "tone_source", "treble_wiper", "bass_wiper", "u5_input", "volume_wiper", "grid"),
    ),
    "daybreaker-sss002-high-low-u37-recovery": SpiceFixture(
        name="daybreaker-sss002-high-low-u37-recovery",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_high_low_u37_recovery.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_high_low_u37_recovery.dat"),
        signals=("source", "high_input", "filter_output", "plate", "cath", "recovery_output", "bplus"),
    ),
    "daybreaker-sss002-u4-plate-stage": SpiceFixture(
        name="daybreaker-sss002-u4-plate-stage",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_u4_plate_stage.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_u4_plate_stage.dat"),
        signals=("source", "grid", "plate", "cath", "output", "hta"),
    ),
    "daybreaker-sss002-u5-volume-u4": SpiceFixture(
        name="daybreaker-sss002-u5-volume-u4",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_sss002_u5_volume_u4.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_sss002_u5_volume_u4.dat"),
        signals=("source", "wiper", "grid", "plate", "cath", "output", "hta"),
    ),
    "daybreaker-tmb-recovery-12ax7": SpiceFixture(
        name="daybreaker-tmb-recovery-12ax7",
        netlist_path=Path("tests/fixtures/circuit/daybreaker_tmb_recovery_12ax7.cir"),
        tmp_data_path=Path("/tmp/greybound_daybreaker_tmb_recovery_12ax7.dat"),
        signals=("source", "input", "tone", "stack_output", "grid", "plate", "recovery_output", "cath", "bplus"),
    ),
}


def run_spice_fixture(name: str, output_dir: Path, repo_root: Path) -> tuple[Path, Path]:
    fixture = FIXTURES.get(name)
    if fixture is None:
        supported = ", ".join(sorted(FIXTURES))
        raise ValueError(f"unknown SPICE fixture {name!r}; supported fixtures: {supported}")

    output_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(["ngspice", "-b", str(fixture.netlist_path)], cwd=repo_root, check=True)
    if not fixture.tmp_data_path.exists():
        raise FileNotFoundError(f"SPICE did not produce {fixture.tmp_data_path}")

    data_path = output_dir / f"{fixture.name}.dat"
    report_path = output_dir / f"{fixture.name}.md"
    shutil.copyfile(fixture.tmp_data_path, data_path)
    trace = parse_wrdata(data_path, fixture.signals)
    if fixture.name == "common-cathode-12ax7":
        metrics = common_cathode_metrics(trace)
        write_common_cathode_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "klon-centaur":
        metrics = klon_centaur_metrics(trace)
        write_klon_centaur_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "none-star-tone-presence":
        metrics = none_star_tone_presence_metrics(trace)
        write_none_star_tone_presence_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-presence-filter":
        metrics = daybreaker_presence_filter_metrics(trace)
        write_daybreaker_presence_filter_report(report_path, fixture, data_path, metrics)
    elif fixture.name in {"daybreaker-classic-tmb", "daybreaker-sss002-classic-tmb"}:
        metrics = daybreaker_classic_tmb_metrics(trace)
        write_daybreaker_classic_tmb_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-sss002-high-low-filters":
        metrics = daybreaker_sss002_high_low_metrics(trace)
        write_daybreaker_sss002_high_low_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-sss002-high-low-chain":
        metrics = daybreaker_sss002_high_low_chain_metrics(trace)
        write_daybreaker_sss002_high_low_chain_report(report_path, fixture, data_path, metrics)
    elif fixture.name in {"daybreaker-sss002-tone-deep-asc", "daybreaker-sss002-tone-deep-layout"}:
        metrics = daybreaker_sss002_tone_deep_metrics(trace)
        write_daybreaker_sss002_tone_deep_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-sss002-high-low-u37-recovery":
        metrics = daybreaker_sss002_u37_recovery_metrics(trace)
        write_daybreaker_sss002_u37_recovery_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-sss002-u4-plate-stage":
        metrics = daybreaker_sss002_u4_plate_metrics(trace)
        write_daybreaker_sss002_u4_plate_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-sss002-u5-volume-u4":
        metrics = daybreaker_sss002_u5_volume_u4_metrics(trace)
        write_daybreaker_sss002_u5_volume_u4_report(report_path, fixture, data_path, metrics)
    elif fixture.name == "daybreaker-tmb-recovery-12ax7":
        metrics = daybreaker_tmb_recovery_metrics(trace)
        write_daybreaker_tmb_recovery_report(report_path, fixture, data_path, metrics)
    else:
        raise ValueError(f"no report writer for {fixture.name}")
    return data_path, report_path


def write_spice_dataset(
    name: str,
    output_dir: Path,
    repo_root: Path,
) -> tuple[Path, Path]:
    fixture = FIXTURES.get(name)
    if fixture is None:
        supported = ", ".join(sorted(FIXTURES))
        raise ValueError(f"unknown SPICE fixture {name!r}; supported fixtures: {supported}")
    if fixture.name == "klon-centaur":
        return write_klon_centaur_dataset(fixture, output_dir, repo_root)
    if fixture.name != "common-cathode-12ax7":
        raise ValueError(f"no dataset writer for {fixture.name}")

    output_dir.mkdir(parents=True, exist_ok=True)
    netlist_dir = output_dir / "netlists"
    trace_dir = output_dir / "traces"
    netlist_dir.mkdir(parents=True, exist_ok=True)
    trace_dir.mkdir(parents=True, exist_ok=True)

    cases = common_cathode_dataset_cases()
    traces: dict[str, SpiceTrace] = {}
    raw_paths: dict[str, Path] = {}
    netlist_paths: dict[str, Path] = {}
    for case in cases:
        netlist_path = netlist_dir / f"{case.stimulus_id}.cir"
        raw_path = trace_dir / f"{case.stimulus_id}.dat"
        netlist_path.write_text(
            common_cathode_generated_netlist(case, raw_path),
            encoding="utf-8",
        )
        subprocess.run(["ngspice", "-b", str(netlist_path)], cwd=repo_root, check=True)
        if not raw_path.exists():
            raise FileNotFoundError(f"SPICE did not produce {raw_path}")
        raw_paths[case.stimulus_id] = raw_path
        netlist_paths[case.stimulus_id] = netlist_path
        traces[case.stimulus_id] = parse_wrdata(raw_path, fixture.signals)

    reference_case = "sine_1khz_20mv"
    trace = traces[reference_case]
    metrics = common_cathode_metrics(trace, settle_time_s=0.030)
    dataset_path = output_dir / f"{fixture.name}.dataset.npz"
    manifest_path = output_dir / f"{fixture.name}.dataset.json"
    report_path = output_dir / f"{fixture.name}.dataset.md"

    arrays = {}
    for stimulus_id, case_trace in traces.items():
        prefix = stimulus_id + "__"
        arrays[prefix + "time_s"] = case_trace.time_s.astype(np.float64)
        arrays[prefix + "input_v"] = case_trace.signals["input"].astype(np.float64)
        arrays[prefix + "grid_v"] = case_trace.signals["grid"].astype(np.float64)
        arrays[prefix + "plate_v"] = case_trace.signals["plate"].astype(np.float64)
        arrays[prefix + "cathode_v"] = case_trace.signals["cathode"].astype(np.float64)
        arrays[prefix + "bplus_v"] = case_trace.signals["bplus"].astype(np.float64)
        arrays[prefix + "plate_ac_v"] = _remove_dc(case_trace.signals["plate"]).astype(np.float64)
    np.savez(dataset_path, **arrays)

    write_common_cathode_dataset_report(report_path, fixture, cases, metrics)
    manifest = common_cathode_sweep_dataset_manifest(
        fixture=fixture,
        repo_root=repo_root,
        cases=cases,
        raw_paths=raw_paths,
        netlist_paths=netlist_paths,
        dataset_path=dataset_path,
        report_path=report_path,
        metrics=metrics,
    )
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return dataset_path, manifest_path


def write_klon_centaur_dataset(
    fixture: SpiceFixture,
    output_dir: Path,
    repo_root: Path,
) -> tuple[Path, Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    netlist_dir = output_dir / "netlists"
    trace_dir = output_dir / "traces"
    netlist_dir.mkdir(parents=True, exist_ok=True)
    trace_dir.mkdir(parents=True, exist_ok=True)

    cases = klon_centaur_dataset_cases()
    traces: dict[str, SpiceTrace] = {}
    raw_paths: dict[str, Path] = {}
    netlist_paths: dict[str, Path] = {}
    source_netlist = (repo_root / fixture.netlist_path).read_text(encoding="utf-8")
    for case in cases:
        netlist_path = netlist_dir / f"{case.stimulus_id}.cir"
        raw_path = trace_dir / f"{case.stimulus_id}.dat"
        netlist_path.write_text(
            klon_centaur_generated_netlist(source_netlist, case, raw_path),
            encoding="utf-8",
        )
        subprocess.run(["ngspice", "-b", str(netlist_path)], cwd=repo_root, check=True)
        if not raw_path.exists():
            raise FileNotFoundError(f"SPICE did not produce {raw_path}")
        raw_paths[case.stimulus_id] = raw_path
        netlist_paths[case.stimulus_id] = netlist_path
        traces[case.stimulus_id] = parse_wrdata(raw_path, fixture.signals)

    reference_case = "sine_1khz_120mv_gain55_treble60"
    metrics = klon_centaur_metrics(traces[reference_case])
    dataset_path = output_dir / f"{fixture.name}.dataset.npz"
    manifest_path = output_dir / f"{fixture.name}.dataset.json"
    report_path = output_dir / f"{fixture.name}.dataset.md"

    arrays = {}
    for stimulus_id, case_trace in traces.items():
        prefix = stimulus_id + "__"
        arrays[prefix + "time_s"] = case_trace.time_s.astype(np.float64)
        for signal_name in fixture.signals:
            samples = case_trace.signals[signal_name]
            arrays[prefix + signal_name + "_v"] = samples.astype(np.float64)
            arrays[prefix + signal_name + "_ac_v"] = _remove_dc(samples).astype(np.float64)
    np.savez(dataset_path, **arrays)

    write_klon_centaur_dataset_report(report_path, fixture, cases, metrics)
    manifest = klon_centaur_dataset_manifest(
        fixture=fixture,
        repo_root=repo_root,
        cases=cases,
        raw_paths=raw_paths,
        netlist_paths=netlist_paths,
        dataset_path=dataset_path,
        report_path=report_path,
        metrics=metrics,
    )
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return dataset_path, manifest_path


def klon_centaur_dataset_cases() -> list[KlonCentaurDatasetCase]:
    return [
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_40mv_gain55_treble60",
            kind="sine_level_sweep",
            expression="0.040*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.040},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_120mv_gain55_treble60",
            kind="sine_level_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_240mv_gain55_treble60",
            kind="sine_level_sweep",
            expression="0.240*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.240},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_120mv_gain25_treble60",
            kind="gain_control_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="train",
            gain=0.25,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_120mv_gain80_treble60",
            kind="gain_control_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="validation",
            gain=0.80,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_120mv_gain55_treble30",
            kind="treble_control_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.30,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_1khz_120mv_gain55_treble85",
            kind="treble_control_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="validation",
            gain=0.55,
            treble=0.85,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_250hz_120mv_gain55_treble60",
            kind="frequency_sweep",
            expression="0.120*sin(2*pi*250*time)",
            parameters={"frequency_hz": 250.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_500hz_120mv_gain55_treble60",
            kind="frequency_sweep",
            expression="0.120*sin(2*pi*500*time)",
            parameters={"frequency_hz": 500.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_2khz_120mv_gain55_treble60",
            kind="frequency_sweep",
            expression="0.120*sin(2*pi*2000*time)",
            parameters={"frequency_hz": 2000.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_4khz_120mv_gain55_treble60",
            kind="frequency_sweep",
            expression="0.120*sin(2*pi*4000*time)",
            parameters={"frequency_hz": 4000.0, "amplitude_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="sine_3khz_120mv_gain55_treble60",
            kind="frequency_sweep",
            expression="0.120*sin(2*pi*3000*time)",
            parameters={"frequency_hz": 3000.0, "amplitude_v": 0.120},
            split="test",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="two_tone_997_1499_120mv_gain55_treble60",
            kind="two_tone_imd",
            expression="0.060*sin(2*pi*997*time)+0.060*sin(2*pi*1499*time)",
            parameters={"first_hz": 997.0, "second_hz": 1499.0, "combined_peak_v": 0.120},
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="two_tone_701_1301_120mv_gain55_treble60",
            kind="two_tone_imd",
            expression="0.060*sin(2*pi*701*time)+0.060*sin(2*pi*1301*time)",
            parameters={"first_hz": 701.0, "second_hz": 1301.0, "combined_peak_v": 0.120},
            split="test",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="burst_1khz_180mv_gain55_treble60",
            kind="dynamic_burst",
            expression=(
                "0.180*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
                "*(0.5-0.5*tanh((time-0.072)/0.0003))"
            ),
            parameters={
                "frequency_hz": 1000.0,
                "amplitude_v": 0.180,
                "event_start_s": 0.032,
                "event_stop_s": 0.072,
                "edge_time_s": 0.0003,
            },
            split="validation",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="pluck_750hz_160mv_gain55_treble60",
            kind="dynamic_decay",
            expression=(
                "0.160*sin(2*pi*750*time)"
                "*exp(-(time-0.032)/0.028)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
            ),
            parameters={
                "frequency_hz": 750.0,
                "amplitude_v": 0.160,
                "event_start_s": 0.032,
                "decay_time_s": 0.028,
                "edge_time_s": 0.0003,
            },
            split="train",
            gain=0.55,
            treble=0.60,
        ),
        KlonCentaurDatasetCase(
            stimulus_id="pluck_1100hz_140mv_gain55_treble60",
            kind="dynamic_decay",
            expression=(
                "0.140*sin(2*pi*1100*time)"
                "*exp(-(time-0.032)/0.022)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
            ),
            parameters={
                "frequency_hz": 1100.0,
                "amplitude_v": 0.140,
                "event_start_s": 0.032,
                "decay_time_s": 0.022,
                "edge_time_s": 0.0003,
            },
            split="test",
            gain=0.55,
            treble=0.60,
        ),
    ]


def klon_centaur_generated_netlist(source_netlist: str, case: KlonCentaurDatasetCase, raw_path: Path) -> str:
    replacements = {
        ".param GAIN=0.55": f".param GAIN={case.gain:g}",
        ".param TREBLE=0.60": f".param TREBLE={case.treble:g}",
        ".param LEVEL=0.70": f".param LEVEL={case.level:g}",
        "VIN guitar 0 SIN(0 120m 1k)": f"BVIN guitar 0 V={{ {case.expression} }}",
        "tran 1u 120m 0 1u": f"tran {case.transient_step_s:g} {case.transient_stop_s:g} 0 {case.transient_step_s:g}",
        "wrdata /tmp/greybound_klon_centaur.dat v(j2_tip) v(u2a_out) v(clean_feed) v(u2b_out) v(clip) v(mix_out) v(treble_wiper) v(vout)": (
            f"wrdata {raw_path.resolve()} v(j2_tip) v(u2a_out) v(clean_feed) "
            "v(u2b_out) v(clip) v(mix_out) v(treble_wiper) v(vout)"
        ),
    }
    generated = source_netlist
    for old, new in replacements.items():
        if old not in generated:
            raise ValueError(f"cannot generate Klon dataset netlist; missing line: {old}")
        generated = generated.replace(old, new, 1)
    generated = generated.replace(".param V9=9", ".param pi=3.141592653589793\n.param V9=9", 1)
    return f"* Generated Greybound Klon dataset case: {case.stimulus_id}\n" + generated


def common_cathode_dataset_cases() -> list[CommonCathodeDatasetCase]:
    return [
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_5mv",
            kind="sine_level_sweep",
            expression="0.005*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.005},
            split="train",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_20mv",
            kind="sine_level_sweep",
            expression="0.020*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.020},
            split="train",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_80mv",
            kind="sine_level_sweep",
            expression="0.080*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.080},
            split="train",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_400mv",
            kind="sine_level_sweep",
            expression="0.400*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.400},
            split="train",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_40mv",
            kind="sine_level_sweep",
            expression="0.040*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.040},
            split="validation",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_300mv",
            kind="sine_level_sweep",
            expression="0.300*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.300},
            split="validation",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_1khz_120mv",
            kind="sine_level_sweep",
            expression="0.120*sin(2*pi*1000*time)",
            parameters={"frequency_hz": 1000.0, "amplitude_v": 0.120},
            split="test",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="two_tone_997_1499_20mv",
            kind="two_tone_imd",
            expression="0.010*sin(2*pi*997*time)+0.010*sin(2*pi*1499*time)",
            parameters={"first_hz": 997.0, "second_hz": 1499.0, "combined_peak_v": 0.020},
            split="train",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="two_tone_997_1499_80mv",
            kind="two_tone_imd",
            expression="0.040*sin(2*pi*997*time)+0.040*sin(2*pi*1499*time)",
            parameters={"first_hz": 997.0, "second_hz": 1499.0, "combined_peak_v": 0.080},
            split="test",
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_burst_1khz_80mv",
            kind="dynamic_burst",
            expression=(
                "0.080*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
                "*(0.5-0.5*tanh((time-0.052)/0.0003))"
            ),
            parameters={
                "frequency_hz": 1000.0,
                "amplitude_v": 0.080,
                "event_start_s": 0.032,
                "event_stop_s": 0.052,
                "edge_time_s": 0.0003,
            },
            split="train",
            transient_stop_s=0.080,
        ),
        CommonCathodeDatasetCase(
            stimulus_id="sine_burst_1khz_40mv",
            kind="dynamic_burst",
            expression=(
                "0.040*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
                "*(0.5-0.5*tanh((time-0.052)/0.0003))"
            ),
            parameters={
                "frequency_hz": 1000.0,
                "amplitude_v": 0.040,
                "event_start_s": 0.032,
                "event_stop_s": 0.052,
                "edge_time_s": 0.0003,
            },
            split="validation",
            transient_stop_s=0.080,
        ),
        CommonCathodeDatasetCase(
            stimulus_id="pluck_decay_750hz_90mv",
            kind="dynamic_decay",
            expression=(
                "0.090*sin(2*pi*750*time)"
                "*exp(-(time-0.032)/0.018)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
            ),
            parameters={
                "frequency_hz": 750.0,
                "amplitude_v": 0.090,
                "event_start_s": 0.032,
                "decay_time_s": 0.018,
                "edge_time_s": 0.0003,
            },
            split="test",
            transient_stop_s=0.100,
        ),
        CommonCathodeDatasetCase(
            stimulus_id="bias_recovery_probe_20mv_after_400mv",
            kind="dynamic_bias_recovery",
            expression=(
                "0.020*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.032)/0.0003))"
                "*(0.5-0.5*tanh((time-0.052)/0.0003))"
                "+0.400*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.060)/0.0003))"
                "*(0.5-0.5*tanh((time-0.130)/0.0003))"
                "+0.020*sin(2*pi*1000*time)"
                "*(0.5+0.5*tanh((time-0.150)/0.0003))"
                "*(0.5-0.5*tanh((time-0.190)/0.0003))"
            ),
            parameters={
                "frequency_hz": 1000.0,
                "probe_amplitude_v": 0.020,
                "stress_amplitude_v": 0.400,
                "pre_probe_start_s": 0.032,
                "pre_probe_stop_s": 0.052,
                "stress_start_s": 0.060,
                "stress_stop_s": 0.130,
                "post_probe_start_s": 0.150,
                "post_probe_stop_s": 0.190,
                "edge_time_s": 0.0003,
            },
            split="test",
            transient_stop_s=0.210,
        ),
    ]


def common_cathode_generated_netlist(case: CommonCathodeDatasetCase, raw_path: Path) -> str:
    return f"""* Generated Greybound common-cathode dataset case: {case.stimulus_id}
.param BRAW=280
.param pi=3.141592653589793

VRAW raw 0 DC {{BRAW}}
RSUP raw bplus 10k
CSUP bplus 0 22u IC={{BRAW}}

BVIN in 0 V={{ {case.expression} }}
CIN in grid 22n
RGRID grid 0 1Meg

RPLATE bplus plate 100k
RK cath 0 1.5k
CK cath 0 25u
XTRIODE plate grid cath 12AX7_KOREN

.save v(in) v(grid) v(plate) v(cath) v(bplus)

.control
set filetype=ascii
tran {case.transient_step_s:g} {case.transient_stop_s:g} 0 {case.transient_step_s:g}
wrdata {raw_path.resolve()} v(in) v(grid) v(plate) v(cath) v(bplus)
quit
.endc

.subckt 12AX7_KOREN P G K
.param MU=100 EX=1.4 KG1=1060 KP=600 KVB=300
E1 n1 0 VALUE={{ln(1 + exp(KP * (1 / MU + V(G,K) / max(V(P,K), 1)))) / KP}}
G1 P K VALUE={{(V(P,K) / KG1) * pow(max(V(n1), 0), EX) * sqrt(max(V(P,K), 0) / KVB)}}
Cpk P K 1.7p
Cgp G P 1.6p
Cgk G K 1.6p
.ends 12AX7_KOREN

.end
"""


def parse_wrdata(path: Path, signals: tuple[str, ...]) -> SpiceTrace:
    data = np.loadtxt(path, dtype=np.float64)
    if data.ndim != 2:
        raise ValueError(f"{path} does not contain tabular data")
    expected_columns = len(signals) * 2
    if data.shape[1] != expected_columns:
        raise ValueError(f"{path} has {data.shape[1]} columns, expected {expected_columns}")
    time_s = data[:, 0]
    parsed = {}
    for index, signal_name in enumerate(signals):
        time_column = data[:, index * 2]
        if not np.allclose(time_column, time_s, rtol=1e-7, atol=1e-12):
            raise ValueError(f"{path} has mismatched time column for {signal_name}")
        parsed[signal_name] = data[:, index * 2 + 1]
    return SpiceTrace(time_s=time_s, signals=parsed)


def common_cathode_dataset_manifest(
    *,
    fixture: SpiceFixture,
    repo_root: Path,
    data_path: Path,
    dataset_path: Path,
    report_path: Path,
    metrics: CommonCathodeSpiceMetrics,
) -> dict:
    return {
        "schema_version": 1,
        "dataset_id": fixture.name + "-settled-sine-v1",
        "fixture_id": fixture.name,
        "cell_kind": "triode_gain_stage",
        "created_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "generator": {
            "name": "greybound-lab spice-dataset",
            "version": "0.1.0",
            "git_revision": git_revision(repo_root),
        },
        "spice": {
            "engine": "ngspice",
            "version": _ngspice_version(repo_root),
            "options": {
                "filetype": "ascii",
                "transient_step_s": 1.0e-6,
                "transient_stop_s": 0.100,
            },
        },
        "circuit": {
            "netlist_sha256": sha256_file(repo_root / fixture.netlist_path),
            "source_impedance_ohm": 0.0,
            "load_impedance_ohm": 1_000_000.0,
            "operating_point": {
                "plate_dc_v": metrics.plate_dc_v,
                "cathode_dc_v": metrics.cathode_dc_v,
                "bplus_dc_v": metrics.bplus_dc_v,
            },
            "components": {
                "tube_model": "12AX7_KOREN",
                "vin": "SIN(0 20m 1k)",
                "input_coupling_cap_f": 22.0e-9,
                "grid_leak_ohm": 1_000_000.0,
                "plate_resistor_ohm": 100_000.0,
                "cathode_resistor_ohm": 1_500.0,
                "cathode_bypass_cap_f": 25.0e-6,
                "raw_supply_v": 280.0,
                "supply_resistor_ohm": 10_000.0,
                "supply_cap_f": 22.0e-6,
            },
        },
        "sample_rate_hz": _sample_rate_from_trace(data_path, fixture.signals),
        "oversampling": {
            "factor": 1,
            "filter": "none",
        },
        "stimuli": [
            {
                "id": "settled_1khz_20mv_sine",
                "kind": "settled_sine",
                "path": relative_or_absolute(data_path, repo_root),
                "sha256": sha256_file(data_path),
                "parameters": {
                    "frequency_hz": 1000.0,
                    "amplitude_v": 0.020,
                    "settle_time_s": 0.050,
                },
            }
        ],
        "targets": [
            {"node": "in", "unit": "V", "role": "input"},
            {"node": "grid", "unit": "V", "role": "state"},
            {"node": "plate", "unit": "V", "role": "output"},
            {"node": "cathode", "unit": "V", "role": "state"},
            {"node": "bplus", "unit": "V", "role": "reference"},
        ],
        "splits": {
            "train": ["settled_1khz_20mv_sine"],
            "validation": [],
            "test": [],
            "policy": "Bootstrap dataset only. Future datasets must hold out stimulus families and level ranges.",
        },
        "artifacts": [
            {
                "path": relative_or_absolute(dataset_path, repo_root),
                "kind": "output",
                "sha256": sha256_file(dataset_path),
            },
            {
                "path": relative_or_absolute(report_path, repo_root),
                "kind": "report",
                "sha256": sha256_file(report_path),
            },
        ],
        "notes": (
            "Bootstrap dataset from the first common-cathode fixture. It is useful "
            "for testing the data/export loop, but it is not sufficient for training "
            "a robust neural cell."
        ),
    }


def common_cathode_sweep_dataset_manifest(
    *,
    fixture: SpiceFixture,
    repo_root: Path,
    cases: list[CommonCathodeDatasetCase],
    raw_paths: dict[str, Path],
    netlist_paths: dict[str, Path],
    dataset_path: Path,
    report_path: Path,
    metrics: CommonCathodeSpiceMetrics,
) -> dict:
    train = [case.stimulus_id for case in cases if case.split == "train"]
    validation = [case.stimulus_id for case in cases if case.split == "validation"]
    test = [case.stimulus_id for case in cases if case.split == "test"]
    artifacts = [
        {
            "path": relative_or_absolute(dataset_path, repo_root),
            "kind": "output",
            "sha256": sha256_file(dataset_path),
        },
        {
            "path": relative_or_absolute(report_path, repo_root),
            "kind": "report",
            "sha256": sha256_file(report_path),
        },
    ]
    for case in cases:
        artifacts.append(
            {
                "path": relative_or_absolute(netlist_paths[case.stimulus_id], repo_root),
                "kind": "netlist",
                "sha256": sha256_file(netlist_paths[case.stimulus_id]),
            }
        )

    return {
        "schema_version": 1,
        "dataset_id": fixture.name + "-sweep-current",
        "fixture_id": fixture.name,
        "cell_kind": "triode_gain_stage",
        "created_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "generator": {
            "name": "greybound-lab spice-dataset",
            "version": "0.1.0",
            "git_revision": git_revision(repo_root),
        },
        "spice": {
            "engine": "ngspice",
            "version": _ngspice_version(repo_root),
            "options": {
                "filetype": "ascii",
                "transient_step_s": 1.0e-6,
                "transient_stop_s": 0.060,
            },
        },
        "circuit": {
            "netlist_sha256": sha256_file(netlist_paths[cases[0].stimulus_id]),
            "source_impedance_ohm": 0.0,
            "load_impedance_ohm": 1_000_000.0,
            "operating_point": {
                "plate_dc_v": metrics.plate_dc_v,
                "cathode_dc_v": metrics.cathode_dc_v,
                "bplus_dc_v": metrics.bplus_dc_v,
            },
            "components": {
                "tube_model": "12AX7_KOREN",
                "input_coupling_cap_f": 22.0e-9,
                "grid_leak_ohm": 1_000_000.0,
                "plate_resistor_ohm": 100_000.0,
                "cathode_resistor_ohm": 1_500.0,
                "cathode_bypass_cap_f": 25.0e-6,
                "raw_supply_v": 280.0,
                "supply_resistor_ohm": 10_000.0,
                "supply_cap_f": 22.0e-6,
            },
        },
        "sample_rate_hz": _sample_rate_from_trace(raw_paths[cases[0].stimulus_id], fixture.signals),
        "oversampling": {
            "factor": 1,
            "filter": "none",
        },
        "stimuli": [
            {
                "id": case.stimulus_id,
                "kind": case.kind,
                "path": relative_or_absolute(raw_paths[case.stimulus_id], repo_root),
                "sha256": sha256_file(raw_paths[case.stimulus_id]),
                "parameters": {
                    **case.parameters,
                    "transient_stop_s": case.transient_stop_s,
                    "settle_time_s": case.settle_time_s,
                },
            }
            for case in cases
        ],
        "targets": [
            {"node": "in", "unit": "V", "role": "input"},
            {"node": "grid", "unit": "V", "role": "state"},
            {"node": "plate", "unit": "V", "role": "output"},
            {"node": "cathode", "unit": "V", "role": "state"},
            {"node": "bplus", "unit": "V", "role": "reference"},
        ],
        "splits": {
            "train": train,
            "validation": validation,
            "test": test,
            "policy": (
                "Train covers low/nominal/hot sine plus a nominal two-tone case. "
                "Training also includes a deliberately high 400 mV sine so the "
                "bias-recovery stress test is not merely an amplitude extrapolation "
                "case. Validation holds out intermediate 40 mV and 300 mV sine "
                "levels. Test holds out an extra-hot sine and a hotter two-tone IMD "
                "case. Dynamic burst and decay cases probe whether static curve "
                "fits survive onset and release behavior. The bias recovery probe "
                "repeats the same small signal before and after a hot stress window "
                "to expose state memory."
            ),
        },
        "artifacts": artifacts,
        "notes": (
            "First multi-stimulus common-cathode dataset. It is suitable for a "
            "baseline MLP/TCN training smoke test and now includes first dynamic "
            "burst/decay and bias-recovery probes. It still lacks source/load "
            "impedance sweeps, B+ perturbation, component tolerances, and real DI."
        ),
    }


def klon_centaur_dataset_manifest(
    *,
    fixture: SpiceFixture,
    repo_root: Path,
    cases: list[KlonCentaurDatasetCase],
    raw_paths: dict[str, Path],
    netlist_paths: dict[str, Path],
    dataset_path: Path,
    report_path: Path,
    metrics: KlonCentaurSpiceMetrics,
) -> dict:
    train = [case.stimulus_id for case in cases if case.split == "train"]
    validation = [case.stimulus_id for case in cases if case.split == "validation"]
    test = [case.stimulus_id for case in cases if case.split == "test"]
    artifacts = [
        {"path": relative_or_absolute(dataset_path, repo_root), "kind": "output", "sha256": sha256_file(dataset_path)},
        {"path": relative_or_absolute(report_path, repo_root), "kind": "report", "sha256": sha256_file(report_path)},
    ]
    for case in cases:
        artifacts.append(
            {
                "path": relative_or_absolute(netlist_paths[case.stimulus_id], repo_root),
                "kind": "netlist",
                "sha256": sha256_file(netlist_paths[case.stimulus_id]),
            }
        )

    return {
        "schema_version": 1,
        "dataset_id": fixture.name + "-drive-clip-tone-v1",
        "fixture_id": fixture.name,
        "cell_kind": "klon_drive_clip_tone",
        "created_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "generator": {
            "name": "greybound-lab spice-dataset",
            "version": "0.1.0",
            "git_revision": git_revision(repo_root),
        },
        "spice": {
            "engine": "ngspice",
            "version": _ngspice_version(repo_root),
            "options": {
                "filetype": "ascii",
                "transient_step_s": 2.0e-6,
                "transient_stop_s": 0.120,
            },
        },
        "sample_rate_hz": _sample_rate_from_trace(raw_paths[cases[0].stimulus_id], fixture.signals),
        "signals": {
            "inputs": ["input_v", "buffer_v"],
            "controls": ["gain", "treble", "level"],
            "targets": ["drive_ac_v", "clip_ac_v", "mix_ac_v", "tone_ac_v"],
            "context": ["clean_ac_v", "output_ac_v"],
        },
        "reference_metrics": asdict(metrics),
        "stimuli": [
            {
                "id": case.stimulus_id,
                "kind": case.kind,
                "path": relative_or_absolute(raw_paths[case.stimulus_id], repo_root),
                "sha256": sha256_file(raw_paths[case.stimulus_id]),
                "parameters": {
                    **case.parameters,
                    "gain": case.gain,
                    "treble": case.treble,
                    "level": case.level,
                    "transient_stop_s": case.transient_stop_s,
                },
                "split": case.split,
            }
            for case in cases
        ],
        "targets": [
            {"node": "j2_tip", "unit": "V", "role": "input"},
            {"node": "u2a_out", "unit": "V", "role": "buffer"},
            {"node": "clean_feed", "unit": "V", "role": "analytic_context"},
            {"node": "u2b_out", "unit": "V", "role": "drive"},
            {"node": "clip", "unit": "V", "role": "clip_target"},
            {"node": "mix_out", "unit": "V", "role": "mix_target"},
            {"node": "treble_wiper", "unit": "V", "role": "tone_target"},
            {"node": "vout", "unit": "V", "role": "output_guardrail"},
        ],
        "splits": {
            "train": train,
            "validation": validation,
            "test": test,
            "policy": (
                "Train covers nominal level, low/hot amplitude, gain, treble, and low-frequency cases. "
                "Validation holds out high gain, bright treble, and burst dynamics. "
                "Test holds out high-frequency, IMD, and decay probes."
            ),
        },
        "artifacts": artifacts,
        "notes": (
            "Synthetic SPICE corpus for a targeted Klon drive/clip/tone neural cell. "
            "It is intentionally not a full-pedal black-box dataset."
        ),
    }


def common_cathode_metrics(trace: SpiceTrace, settle_time_s: float = 0.050) -> CommonCathodeSpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("SPICE trace is too short for settled metrics")
    input_v = trace.signals["input"][mask]
    grid_v = trace.signals["grid"][mask]
    plate_v = trace.signals["plate"][mask]
    cathode_v = trace.signals["cathode"][mask]
    bplus_v = trace.signals["bplus"][mask]

    input_ac = _remove_dc(input_v)
    grid_ac = _remove_dc(grid_v)
    plate_ac = _remove_dc(plate_v)
    cathode_ac = _remove_dc(cathode_v)
    input_rms = rms(input_ac)
    grid_rms = rms(grid_ac)
    plate_rms = rms(plate_ac)

    return CommonCathodeSpiceMetrics(
        plate_dc_v=float(np.mean(plate_v)),
        cathode_dc_v=float(np.mean(cathode_v)),
        bplus_dc_v=float(np.mean(bplus_v)),
        input_rms_v=input_rms,
        grid_rms_v=rms(grid_ac),
        plate_rms_v=plate_rms,
        cathode_rms_v=rms(cathode_ac),
        plate_gain=plate_rms / max(input_rms, 1.0e-12),
        plate_gain_db=linear_to_db(plate_rms / max(input_rms, 1.0e-12)),
        grid_coupling_loss_db=linear_to_db(rms(grid_ac) / max(input_rms, 1.0e-12)),
    )


def klon_centaur_metrics(trace: SpiceTrace, settle_time_s: float = 0.050) -> KlonCentaurSpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("SPICE trace is too short for settled metrics")

    input_ac = _remove_dc(trace.signals["input"][mask])
    buffer_ac = _remove_dc(trace.signals["buffer"][mask])
    clean_ac = _remove_dc(trace.signals["clean"][mask])
    drive_ac = _remove_dc(trace.signals["drive"][mask])
    clip_ac = _remove_dc(trace.signals["clip"][mask])
    mix_ac = _remove_dc(trace.signals["mix"][mask])
    tone_ac = _remove_dc(trace.signals["tone"][mask])
    output_ac = _remove_dc(trace.signals["output"][mask])

    input_rms = rms(input_ac)
    output_rms = rms(output_ac)
    clip_positive = float(np.max(clip_ac))
    clip_negative = float(np.min(clip_ac))

    return KlonCentaurSpiceMetrics(
        input_rms_v=input_rms,
        buffer_rms_v=rms(buffer_ac),
        clean_rms_v=rms(clean_ac),
        drive_rms_v=rms(drive_ac),
        clip_rms_v=rms(clip_ac),
        mix_rms_v=rms(mix_ac),
        tone_rms_v=rms(tone_ac),
        output_rms_v=output_rms,
        output_peak_v=float(np.max(np.abs(output_ac))),
        output_gain=output_rms / max(input_rms, 1.0e-12),
        output_gain_db=linear_to_db(output_rms / max(input_rms, 1.0e-12)),
        clip_peak_v=max(abs(clip_positive), abs(clip_negative)),
        clip_asymmetry_v=clip_positive + clip_negative,
    )


def none_star_tone_presence_metrics(trace: SpiceTrace) -> NoneStarTonePresenceSpiceMetrics:
    frequencies = trace.time_s
    if np.any(frequencies <= 0.0):
        raise ValueError("None Star tone/presence AC trace requires positive frequency values")

    input_mag = np.maximum(trace.signals["input"], 1.0e-12)
    tone_mag = np.maximum(trace.signals["tone"], 1.0e-12)
    output_mag = np.maximum(trace.signals["output"], 1.0e-12)
    output_gain_db = 20.0 * np.log10(output_mag / input_mag)
    tone_gain_db = 20.0 * np.log10(tone_mag / input_mag)

    def at_hz(values: np.ndarray, frequency_hz: float) -> float:
        return float(np.interp(np.log10(frequency_hz), np.log10(frequencies), values))

    mid_1khz = at_hz(output_gain_db, 1_000.0)
    output_8khz = at_hz(output_gain_db, 8_000.0)
    tone_8khz = at_hz(tone_gain_db, 8_000.0)
    return NoneStarTonePresenceSpiceMetrics(
        low_250hz_db=at_hz(output_gain_db, 250.0),
        mid_1khz_db=mid_1khz,
        presence_4khz_db=at_hz(output_gain_db, 4_000.0),
        presence_8khz_db=output_8khz,
        air_16khz_db=at_hz(output_gain_db, 16_000.0),
        tone_8khz_db=tone_8khz,
        output_8khz_db=output_8khz,
        presence_lift_8khz_db=output_8khz - tone_8khz,
        output_minus_1khz_8khz_db=output_8khz - mid_1khz,
    )


def daybreaker_presence_filter_metrics(trace: SpiceTrace) -> DaybreakerPresenceFilterSpiceMetrics:
    frequencies = trace.time_s
    if np.any(frequencies <= 0.0):
        raise ValueError("Daybreaker presence-filter AC trace requires positive frequency values")

    input_mag = np.maximum(trace.signals["input"], 1.0e-12)
    transformer_mag = np.maximum(trace.signals["transformer"], 1.0e-12)
    presence_mag = np.maximum(trace.signals["presence_band"], 1.0e-12)
    output_mag = np.maximum(trace.signals["output"], 1.0e-12)
    transformer_gain_db = 20.0 * np.log10(transformer_mag / input_mag)
    presence_gain_db = 20.0 * np.log10(presence_mag / input_mag)
    output_gain_db = 20.0 * np.log10(output_mag / input_mag)

    def at_hz(values: np.ndarray, frequency_hz: float) -> float:
        return float(np.interp(np.log10(frequency_hz), np.log10(frequencies), values))

    output_1khz = at_hz(output_gain_db, 1_000.0)
    output_4khz = at_hz(output_gain_db, 4_000.0)
    output_16khz = at_hz(output_gain_db, 16_000.0)
    return DaybreakerPresenceFilterSpiceMetrics(
        transformer_1khz_db=at_hz(transformer_gain_db, 1_000.0),
        presence_band_1khz_db=at_hz(presence_gain_db, 1_000.0),
        output_1khz_db=output_1khz,
        output_4khz_db=output_4khz,
        output_8khz_db=at_hz(output_gain_db, 8_000.0),
        output_16khz_db=output_16khz,
        output_minus_1khz_4khz_db=output_4khz - output_1khz,
        output_minus_1khz_16khz_db=output_16khz - output_1khz,
    )


def daybreaker_classic_tmb_metrics(trace: SpiceTrace) -> DaybreakerClassicTmbSpiceMetrics:
    frequencies = trace.time_s
    if np.any(frequencies <= 0.0):
        raise ValueError("Daybreaker classic-TMB AC trace requires positive frequency values")

    source_mag = np.maximum(trace.signals["source"], 1.0e-12)
    input_mag = np.maximum(trace.signals["input"], 1.0e-12)
    output_mag = np.maximum(trace.signals["output"], 1.0e-12)
    input_gain_db = 20.0 * np.log10(input_mag / source_mag)
    output_gain_db = 20.0 * np.log10(output_mag / source_mag)

    def at_hz(values: np.ndarray, frequency_hz: float) -> float:
        return float(np.interp(np.log10(frequency_hz), np.log10(frequencies), values))

    output_1khz = at_hz(output_gain_db, 1_000.0)
    output_4khz = at_hz(output_gain_db, 4_000.0)
    output_16khz = at_hz(output_gain_db, 16_000.0)
    return DaybreakerClassicTmbSpiceMetrics(
        input_1khz_db=at_hz(input_gain_db, 1_000.0),
        output_100hz_db=at_hz(output_gain_db, 100.0),
        output_250hz_db=at_hz(output_gain_db, 250.0),
        output_1khz_db=output_1khz,
        output_4khz_db=output_4khz,
        output_8khz_db=at_hz(output_gain_db, 8_000.0),
        output_16khz_db=output_16khz,
        output_minus_1khz_4khz_db=output_4khz - output_1khz,
        output_minus_1khz_16khz_db=output_16khz - output_1khz,
    )


def daybreaker_sss002_high_low_metrics(trace: SpiceTrace) -> DaybreakerSss002HighLowSpiceMetrics:
    frequencies = trace.time_s
    if np.any(frequencies <= 0.0):
        raise ValueError("Daybreaker SSS #002 High/Low AC trace requires positive frequency values")

    source_mag = np.maximum(trace.signals["source"], 1.0e-12)

    def gains(prefix: str) -> tuple[tuple[float, ...], tuple[float, ...], tuple[float, ...], tuple[float, ...]]:
        values = []
        for index in range(1, 8):
            magnitude = np.maximum(trace.signals[f"{prefix}_{index}"], 1.0e-12)
            response_db = 20.0 * np.log10(magnitude / source_mag)
            at_hz = lambda frequency_hz: float(
                np.interp(np.log10(frequency_hz), np.log10(frequencies), response_db)
            )
            at_100hz = at_hz(100.0)
            at_1khz = at_hz(1_000.0)
            at_8khz = at_hz(8_000.0)
            values.append((at_100hz, at_1khz, at_8khz, at_8khz - at_1khz))
        return tuple(tuple(row[column] for row in values) for column in range(4))

    high_100hz, high_1khz, high_8khz, high_tilt = gains("high")
    low_100hz, low_1khz, low_8khz, low_tilt = gains("low")
    return DaybreakerSss002HighLowSpiceMetrics(
        high_100hz_db=high_100hz,
        high_1khz_db=high_1khz,
        high_8khz_db=high_8khz,
        high_8khz_minus_1khz_db=high_tilt,
        low_100hz_db=low_100hz,
        low_1khz_db=low_1khz,
        low_8khz_db=low_8khz,
        low_8khz_minus_1khz_db=low_tilt,
    )


def daybreaker_sss002_high_low_chain_metrics(
    trace: SpiceTrace,
) -> DaybreakerSss002HighLowChainSpiceMetrics:
    frequencies = trace.time_s
    if np.any(frequencies <= 0.0):
        raise ValueError("Daybreaker SSS #002 High/Low chain AC trace requires positive frequency values")

    source_mag = np.maximum(trace.signals["source"], 1.0e-12)
    output_mag = np.maximum(trace.signals["output"], 1.0e-12)
    output_gain_db = 20.0 * np.log10(output_mag / source_mag)

    def at_hz(frequency_hz: float) -> float:
        return float(np.interp(np.log10(frequency_hz), np.log10(frequencies), output_gain_db))

    output_1khz = at_hz(1_000.0)
    output_8khz = at_hz(8_000.0)
    return DaybreakerSss002HighLowChainSpiceMetrics(
        output_100hz_db=at_hz(100.0),
        output_1khz_db=output_1khz,
        output_8khz_db=output_8khz,
        output_16khz_db=at_hz(16_000.0),
        output_minus_1khz_8khz_db=output_8khz - output_1khz,
    )


def daybreaker_sss002_tone_deep_metrics(trace: SpiceTrace) -> DaybreakerSss002ToneDeepSpiceMetrics:
    frequencies = trace.time_s
    source_magnitude = np.maximum(trace.signals["source"], 1.0e-12)
    grid_magnitude = np.maximum(trace.signals["grid"], 1.0e-12)
    grid_gain_db = 20.0 * np.log10(grid_magnitude / source_magnitude)

    def at_hz(frequency_hz: float) -> float:
        return float(np.interp(np.log10(frequency_hz), np.log10(frequencies), grid_gain_db))

    grid_1khz_db = at_hz(1_000.0)
    grid_8khz_db = at_hz(8_000.0)
    return DaybreakerSss002ToneDeepSpiceMetrics(
        grid_100hz_db=at_hz(100.0),
        grid_1khz_db=grid_1khz_db,
        grid_8khz_db=grid_8khz_db,
        grid_16khz_db=at_hz(16_000.0),
        grid_minus_1khz_8khz_db=grid_8khz_db - grid_1khz_db,
    )


def daybreaker_sss002_u37_recovery_metrics(
    trace: SpiceTrace,
    settle_time_s: float = 0.080,
) -> DaybreakerSss002U37RecoverySpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("Daybreaker SSS #002 U37 trace is too short for settled metrics")

    filter_output = trace.signals["filter_output"][mask]
    plate = trace.signals["plate"][mask]
    recovery_output = trace.signals["recovery_output"][mask]
    cathode = trace.signals["cath"][mask]
    bplus = trace.signals["bplus"][mask]
    filter_output_rms = rms(_remove_dc(filter_output))
    plate_rms = rms(_remove_dc(plate))
    return DaybreakerSss002U37RecoverySpiceMetrics(
        plate_dc_v=float(np.mean(plate)),
        cathode_dc_v=float(np.mean(cathode)),
        bplus_dc_v=float(np.mean(bplus)),
        filter_output_rms_v=filter_output_rms,
        plate_rms_v=plate_rms,
        recovery_output_rms_v=rms(_remove_dc(recovery_output)),
        plate_gain=plate_rms / max(filter_output_rms, 1.0e-12),
        plate_gain_db=linear_to_db(plate_rms / max(filter_output_rms, 1.0e-12)),
    )


def daybreaker_sss002_u4_plate_metrics(
    trace: SpiceTrace,
    settle_time_s: float = 0.080,
) -> DaybreakerSss002U4PlateSpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("Daybreaker SSS #002 U4 trace is too short for settled metrics")

    grid = trace.signals["grid"][mask]
    plate = trace.signals["plate"][mask]
    output = trace.signals["output"][mask]
    cathode = trace.signals["cath"][mask]
    hta = trace.signals["hta"][mask]
    grid_rms = rms(_remove_dc(grid))
    plate_rms = rms(_remove_dc(plate))
    return DaybreakerSss002U4PlateSpiceMetrics(
        plate_dc_v=float(np.mean(plate)),
        cathode_dc_v=float(np.mean(cathode)),
        hta_dc_v=float(np.mean(hta)),
        grid_rms_v=grid_rms,
        plate_rms_v=plate_rms,
        output_rms_v=rms(_remove_dc(output)),
        plate_gain=plate_rms / max(grid_rms, 1.0e-12),
        plate_gain_db=linear_to_db(plate_rms / max(grid_rms, 1.0e-12)),
    )


def daybreaker_sss002_u5_volume_u4_metrics(
    trace: SpiceTrace,
    settle_time_s: float = 0.080,
) -> DaybreakerSss002U5VolumeU4SpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("Daybreaker SSS #002 U5/U4 trace is too short for settled metrics")

    source = trace.signals["source"][mask]
    wiper = trace.signals["wiper"][mask]
    grid = trace.signals["grid"][mask]
    plate = trace.signals["plate"][mask]
    output = trace.signals["output"][mask]
    cathode = trace.signals["cath"][mask]
    hta = trace.signals["hta"][mask]
    source_rms = rms(_remove_dc(source))
    wiper_rms = rms(_remove_dc(wiper))
    grid_rms = rms(_remove_dc(grid))
    plate_rms = rms(_remove_dc(plate))
    return DaybreakerSss002U5VolumeU4SpiceMetrics(
        plate_dc_v=float(np.mean(plate)),
        cathode_dc_v=float(np.mean(cathode)),
        hta_dc_v=float(np.mean(hta)),
        source_rms_v=source_rms,
        wiper_rms_v=wiper_rms,
        grid_rms_v=grid_rms,
        output_rms_v=rms(_remove_dc(output)),
        wiper_gain=wiper_rms / max(source_rms, 1.0e-12),
        wiper_gain_db=linear_to_db(wiper_rms / max(source_rms, 1.0e-12)),
        plate_gain=plate_rms / max(grid_rms, 1.0e-12),
        plate_gain_db=linear_to_db(plate_rms / max(grid_rms, 1.0e-12)),
    )


def daybreaker_tmb_recovery_metrics(
    trace: SpiceTrace,
    settle_time_s: float = 0.060,
) -> DaybreakerTmbRecoverySpiceMetrics:
    mask = trace.time_s >= settle_time_s
    if not np.any(mask):
        raise ValueError("Daybreaker TMB-recovery trace is too short for settled metrics")

    stack_output = trace.signals["stack_output"][mask]
    grid = trace.signals["grid"][mask]
    plate = trace.signals["plate"][mask]
    recovery_output = trace.signals["recovery_output"][mask]
    cathode = trace.signals["cath"][mask]
    bplus = trace.signals["bplus"][mask]
    stack_output_rms = rms(_remove_dc(stack_output))
    plate_rms = rms(_remove_dc(plate))
    return DaybreakerTmbRecoverySpiceMetrics(
        plate_dc_v=float(np.mean(plate)),
        cathode_dc_v=float(np.mean(cathode)),
        bplus_dc_v=float(np.mean(bplus)),
        stack_output_rms_v=stack_output_rms,
        grid_rms_v=rms(_remove_dc(grid)),
        plate_rms_v=plate_rms,
        recovery_output_rms_v=rms(_remove_dc(recovery_output)),
        plate_gain=plate_rms / max(stack_output_rms, 1.0e-12),
        plate_gain_db=linear_to_db(plate_rms / max(stack_output_rms, 1.0e-12)),
    )


def write_common_cathode_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: CommonCathodeSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch run

## DC Operating Point

| Node | Voltage |
| --- | ---: |
| Plate | {metrics.plate_dc_v:.3f} V |
| Cathode | {metrics.cathode_dc_v:.3f} V |
| B+ | {metrics.bplus_dc_v:.3f} V |

## Settled 1 kHz Transient

Metrics are computed after the first 50 ms to avoid startup bias.

| Metric | Value |
| --- | ---: |
| Input RMS | {metrics.input_rms_v * 1000.0:.3f} mV |
| Grid RMS | {metrics.grid_rms_v * 1000.0:.3f} mV |
| Plate RMS after DC removal | {metrics.plate_rms_v * 1000.0:.3f} mV |
| Cathode RMS after DC removal | {metrics.cathode_rms_v * 1000.0:.3f} mV |
| Plate gain | {metrics.plate_gain:.2f}x |
| Plate gain | {metrics.plate_gain_db:.2f} dB |
| Grid coupling loss | {metrics.grid_coupling_loss_db:.2f} dB |

## Engineering Notes

This is a cell-level electrical reference, not a full Greybound rig reference.
Use it to validate the common-cathode stage before fitting or tuning higher-level
amp behavior.
""",
        encoding="utf-8",
    )


def write_none_star_tone_presence_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: NoneStarTonePresenceSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch AC sweep

## Circuit Scope

This fixture is a project-owned linearized graybox reference for the None Star
Clean/Edge tone stack and presence hypothesis. It is not a Mesa schematic and
does not include nonlinear tube, power, transformer, speaker, or cab behavior.

The fixture captures the intended behavior validated against the local full-rig
NAM reference: the tone stack should not erase the mid/presence band, and the
presence branch may restore high-frequency energy like reduced negative
feedback rather than behave as a passive high cut.

## AC Sweep

Gains are relative to the 1 V AC input source.

| Frequency / Metric | Output gain |
| --- | ---: |
| 250 Hz | {metrics.low_250hz_db:.2f} dB |
| 1 kHz | {metrics.mid_1khz_db:.2f} dB |
| 4 kHz | {metrics.presence_4khz_db:.2f} dB |
| 8 kHz | {metrics.presence_8khz_db:.2f} dB |
| 16 kHz | {metrics.air_16khz_db:.2f} dB |
| Presence lift at 8 kHz, output minus tone node | {metrics.presence_lift_8khz_db:.2f} dB |
| 8 kHz minus 1 kHz output tilt | {metrics.output_minus_1khz_8khz_db:.2f} dB |

## Engineering Notes

Use this fixture as a regression guard for the current component-level
hypothesis. It is useful if a future change accidentally turns the None Star
presence control back into a passive high-frequency attenuation stage.
""",
        encoding="utf-8",
    )


def write_daybreaker_presence_filter_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerPresenceFilterSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch AC sweep

## Circuit Scope

This project-owned fixture isolates the Daybreaker post-tone high-filter and
transformer-rolloff hypothesis. It is not a Dumble schematic. The filter uses a
`22 kOhm / 4.7 nF` high-pass followed by a `22 kOhm / 1.5 nF` low-pass presence
band, summed through an active recovery path with the transformer low-pass.

The hypothesis is useful only if it moves the NAM comparison toward more
1-8 kHz energy while reducing excess 8-18 kHz air without a global EQ change.

## AC Sweep

Gains are relative to the 1 V AC input source.

| Frequency / Metric | Gain |
| --- | ---: |
| Transformer path at 1 kHz | {metrics.transformer_1khz_db:.2f} dB |
| Presence band at 1 kHz | {metrics.presence_band_1khz_db:.2f} dB |
| Output at 1 kHz | {metrics.output_1khz_db:.2f} dB |
| Output at 4 kHz | {metrics.output_4khz_db:.2f} dB |
| Output at 8 kHz | {metrics.output_8khz_db:.2f} dB |
| Output at 16 kHz | {metrics.output_16khz_db:.2f} dB |
| 4 kHz minus 1 kHz output tilt | {metrics.output_minus_1khz_4khz_db:.2f} dB |
| 16 kHz minus 1 kHz output tilt | {metrics.output_minus_1khz_16khz_db:.2f} dB |

## Engineering Notes

Promote this cell only when the matching Rust implementation improves the fixed
Daybreaker-vs-NAM report and preserves clean monitor health. Otherwise retain
the fixture as a rejected hypothesis and move to the next component.
""",
        encoding="utf-8",
    )


def write_daybreaker_classic_tmb_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerClassicTmbSpiceMetrics,
) -> None:
    source_note = (
        "This project-owned fixture uses a `38 kOhm` plate-source boundary, `470 kOhm` "
        "recovery-grid load, `100 kOhm` slope resistor, `250 pF` treble capacitor, "
        "and `22 nF` bass/mid capacitors."
        if fixture.name == "daybreaker-classic-tmb"
        else "This source-informed SSS #002 hypothesis uses a `68 kOhm` plate-source "
        "boundary, `470 kOhm` recovery-grid load, `100 kOhm` slope resistor, `250 pF` "
        "treble capacitor, `100 nF` bass capacitor, `47 nF` mid capacitor, `250 kOhm` "
        "bass/treble pots, and `100 kOhm` mid pot."
    )
    source_resistance = "38 kOhm" if fixture.name == "daybreaker-classic-tmb" else "68 kOhm"
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch AC sweep

## Circuit Scope

This is Greybound's source/load-aware low-plate classic TMB hypothesis for the
Daybreaker. {source_note} These values establish a reproducible implementation
boundary; they are not proof of the NAM source revision.

## AC Sweep

Gains are relative to the 1 V source before the `{source_resistance}` source resistance.

| Frequency / Metric | Gain |
| --- | ---: |
| Tone-stack input at 1 kHz | {metrics.input_1khz_db:.2f} dB |
| Output at 100 Hz | {metrics.output_100hz_db:.2f} dB |
| Output at 250 Hz | {metrics.output_250hz_db:.2f} dB |
| Output at 1 kHz | {metrics.output_1khz_db:.2f} dB |
| Output at 4 kHz | {metrics.output_4khz_db:.2f} dB |
| Output at 8 kHz | {metrics.output_8khz_db:.2f} dB |
| Output at 16 kHz | {metrics.output_16khz_db:.2f} dB |
| 4 kHz minus 1 kHz output tilt | {metrics.output_minus_1khz_4khz_db:.2f} dB |
| 16 kHz minus 1 kHz output tilt | {metrics.output_minus_1khz_16khz_db:.2f} dB |

## Engineering Notes

The deliberate insertion loss must be recovered by a downstream gain stage;
do not normalize it inside this cell. Promote the matching runtime component
only if fixed-control, fixed-stimulus Rust and NAM checks agree with this AC
reference and preserve monitor health.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_high_low_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002HighLowSpiceMetrics,
) -> None:
    def rows(
        at_100hz: tuple[float, ...],
        at_1khz: tuple[float, ...],
        at_8khz: tuple[float, ...],
        tilt: tuple[float, ...],
    ) -> str:
        return "\n".join(
            f"| {index} | {at_100hz[index - 1]:.2f} dB | {at_1khz[index - 1]:.2f} dB | "
            f"{at_8khz[index - 1]:.2f} dB | {tilt[index - 1]:+.2f} dB |"
            for index in range(1, 8)
        )

    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch AC sweep

## Circuit Scope

This is a component-routing reference transcribed from the public SSS #002
drawing. It is not a Dumble production schematic and does not identify the NAM
capture's switch positions, tube operating points, or source/load impedances.

Each High and Low position is measured as an isolated copy so that the other
six measurement loads cannot alter its transfer. The explicit 1 kOhm source
and 1 MOhm load are fixture boundaries, not inferred amplifier values.

High position 1 directly bypasses `R70`; positions 2 through 7 select `C39`,
`C40`, `C41`, `C42`, `C43`, and `C38`, respectively. `C44 = 3 nF` remains a
fixed shunt. Low position 1 through 7 select successive taps on `R72..R78`;
`C45 = 10 nF` spans the top and bottom of that ladder.

## High Switch Sweep

Gains are relative to the 1 V AC fixture source.

| Position | 100 Hz | 1 kHz | 8 kHz | 8 kHz minus 1 kHz |
| ---: | ---: | ---: | ---: | ---: |
{rows(metrics.high_100hz_db, metrics.high_1khz_db, metrics.high_8khz_db, metrics.high_8khz_minus_1khz_db)}

## Low Switch Sweep

| Position | 100 Hz | 1 kHz | 8 kHz | 8 kHz minus 1 kHz |
| ---: | ---: | ---: | ---: | ---: |
{rows(metrics.low_100hz_db, metrics.low_1khz_db, metrics.low_8khz_db, metrics.low_8khz_minus_1khz_db)}

## Engineering Notes

This fixture is evidence for the individual switch-network topology and its
relative response only. Do not promote a position into the runtime model until
the position is justified by reference capture settings or a whole-rig NAM
comparison with the neighboring tube/load boundary held fixed.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_high_low_chain_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002HighLowChainSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch AC sweep

## Circuit Scope

This project-owned fixture traces the source drawing's relevant connection:
the High output remains the audio output and `R79` feeds the selected Low
ladder as a shunt load. The source drawing does not override the rotary
subcircuit's `POS=1` default, so this fixture uses High position 1 (the direct
`R70` bypass) and Low position 1 (the ladder top). Those are drawing defaults,
not claimed NAM capture settings.

The source uses U4's `R5 = 100 kOhm` plate-load boundary, followed by the
drawn C6/R53/L2/R34/C37 network; the output uses an explicit 1 MOhm load. This
still excludes the triode's finite plate resistance, bias point, and all other
tube stages.

## AC Sweep

Gains are relative to the 1 V AC fixture source.

| Frequency / Metric | Output gain |
| --- | ---: |
| 100 Hz | {metrics.output_100hz_db:.2f} dB |
| 1 kHz | {metrics.output_1khz_db:.2f} dB |
| 8 kHz | {metrics.output_8khz_db:.2f} dB |
| 16 kHz | {metrics.output_16khz_db:.2f} dB |
| 8 kHz minus 1 kHz tilt | {metrics.output_minus_1khz_8khz_db:+.2f} dB |

## Engineering Notes

Use this chain as the integration reference, not the isolated High or Low
position sweeps. A runtime default remains unjustified until the capture's
switch settings or the adjoining physical stage boundary are evidenced.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_tone_deep_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002ToneDeepSpiceMetrics,
) -> None:
    revision = "ASC values" if fixture.name.endswith("-asc") else "public-layout values"
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice AC sweep

## Circuit Scope

This is the SSS #002 U2/U3/U5 and U34/U35 tone/Deep branch using the ASC wire
topology and its executable defaults (`GUITARMIC = 1`, `DEEP = 0`). This report
uses the distinct **{revision}** candidate. It does not select a NAM setting,
claim a hardware revision, or authorize a runtime port.

## AC Response at U4 Grid Boundary

Gains are relative to the fixture's 1 V AC source with its explicit 100 kOhm
plate-source boundary and 1 GOhm U4-grid measurement load.

| Frequency / Metric | Grid gain |
| --- | ---: |
| 100 Hz | {metrics.grid_100hz_db:.2f} dB |
| 1 kHz | {metrics.grid_1khz_db:.2f} dB |
| 8 kHz | {metrics.grid_8khz_db:.2f} dB |
| 16 kHz | {metrics.grid_16khz_db:.2f} dB |
| 8 kHz minus 1 kHz tilt | {metrics.grid_minus_1khz_8khz_db:+.2f} dB |

## Engineering Notes

Compare this report only with the other discrete revision candidate. Do not mix
values across the ASC and public-layout sources; their disagreement is large
enough to change both the midband level and the spectrum materially.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_u4_plate_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002U4PlateSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice operating point and settled 1 kHz transient

## Circuit Scope

This fixture isolates U4 before the C6/High-Low network: `R5 = 100 kOhm`,
`R4/C5 = 1.5 kOhm / 5 uF`, `R69 = 68 kOhm`, and `C6 = 10 nF`. `HT-A = 440 V`
follows the source drawing's V1. The 1 Mohm grid return and following load are
explicit measurement boundaries for the unresolved U5 volume-pot interaction.

## Operating Point and Settled Response

The source is a 20 mV-peak 1 kHz sine. AC metrics exclude the first 80 ms.

| Metric | Value |
| --- | ---: |
| Plate DC | {metrics.plate_dc_v:.3f} V |
| Cathode DC | {metrics.cathode_dc_v:.3f} V |
| HT-A DC | {metrics.hta_dc_v:.3f} V |
| Grid RMS | {metrics.grid_rms_v * 1000.0:.3f} mV |
| Plate RMS after DC removal | {metrics.plate_rms_v * 1000.0:.3f} mV |
| C6 output RMS | {metrics.output_rms_v * 1000.0:.3f} mV |
| Plate gain relative to grid | {metrics.plate_gain:.2f}x |
| Plate gain | {metrics.plate_gain_db:.2f} dB |

## Engineering Notes

Use this cell to establish the plate-side source seen by the following passive
network. Do not infer the U5 pot setting or a normalized audio gain from it.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_u5_volume_u4_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002U5VolumeU4SpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice operating point and settled 1 kHz transient

## Circuit Scope

This fixture replaces U4's provisional 1 Mohm grid-return measurement boundary
with the source drawing's U5 volume control. U5 is `pot_pow`, `Rtot = 1 Mohm`,
`Rtap = 100 kOhm`, `tap = 0.5`, and the drawing's explicit `VOL = 0.5`; that
is a 900 kOhm source-to-wiper leg and a 100 kOhm wiper-to-ground leg. The wiper
feeds the drawn `R69 = 68 kOhm` grid series resistor.

The ideal source at U5's top is still a declared pre-volume measurement
boundary. This establishes the U5/U4 component behavior, not the complete
loaded U5 boundary: R65 returns from U5's wiper into the switchable Deep
network, while U2/U3 form the actual source network. It does not claim the
upstream tone-stack source impedance or a NAM capture volume setting.

## Operating Point and Settled Response

The source is a 20 mV-peak 1 kHz sine. AC metrics exclude the first 80 ms.

| Metric | Value |
| --- | ---: |
| Plate DC | {metrics.plate_dc_v:.3f} V |
| Cathode DC | {metrics.cathode_dc_v:.3f} V |
| HT-A DC | {metrics.hta_dc_v:.3f} V |
| Source RMS | {metrics.source_rms_v * 1000.0:.3f} mV |
| U5 wiper RMS | {metrics.wiper_rms_v * 1000.0:.3f} mV |
| U4 grid RMS | {metrics.grid_rms_v * 1000.0:.3f} mV |
| C6 output RMS | {metrics.output_rms_v * 1000.0:.3f} mV |
| U5 wiper gain | {metrics.wiper_gain:.4f}x |
| U5 wiper gain | {metrics.wiper_gain_db:.2f} dB |
| U4 plate gain relative to grid | {metrics.plate_gain:.2f}x |
| U4 plate gain | {metrics.plate_gain_db:.2f} dB |

## Engineering Notes

This supersedes the standalone U4 fixture's arbitrary grid-return boundary for
the drawing-default `VOL = 0.5` condition. It remains a component-level port
until the upstream tone-stack output impedance is traced and represented.
""",
        encoding="utf-8",
    )


def write_daybreaker_sss002_u37_recovery_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerSss002U37RecoverySpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice operating point and settled 1 kHz transient

## Circuit Scope

This fixture joins the High-1/Low-1 drawing-default filter chain to U37: the
drawing's Sylvania 7025 common-cathode stage with `R80 = 100 kOhm` and
`R81/C46 = 1 kOhm / 1 uF`. It uses the `R5 = 100 kOhm` plate-source boundary plus the
drawn C6/R53/L2/R34/C37 network. `HT4 =
300 V` and the following `470 kOhm` load are explicit hypotheses, not NAM
capture facts.

## Operating Point and Settled Response

The source is a 20 mV-peak 1 kHz sine. AC metrics exclude the first 80 ms.

| Metric | Value |
| --- | ---: |
| Plate DC | {metrics.plate_dc_v:.3f} V |
| Cathode DC | {metrics.cathode_dc_v:.3f} V |
| B+ DC | {metrics.bplus_dc_v:.3f} V |
| Filter output RMS | {metrics.filter_output_rms_v * 1000.0:.3f} mV |
| Plate RMS after DC removal | {metrics.plate_rms_v * 1000.0:.3f} mV |
| Coupled recovery output RMS | {metrics.recovery_output_rms_v * 1000.0:.3f} mV |
| Plate gain relative to filter output | {metrics.plate_gain:.2f}x |
| Plate gain | {metrics.plate_gain_db:.2f} dB |

## Engineering Notes

This verifies that the passive network's insertion loss is followed by a real
gain stage. Establish the HT4 rail and following-stage load more strongly
before using the fixture to set normalized runtime gain or nonlinear drive.
""",
        encoding="utf-8",
    )


def write_daybreaker_tmb_recovery_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: DaybreakerTmbRecoverySpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice operating point and settled 1 kHz transient

## Circuit Scope

This project-owned fixture connects the Daybreaker passive classic-TMB
hypothesis to a `470 kOhm` grid return and ECC83 common-cathode recovery stage.
It establishes the recovery gain and loading boundary that a runtime integration
must preserve. It is not a Dumble schematic and does not identify the NAM
capture's revision or component values.

## Operating Point and Settled Response

The source is a 20 mV-peak 1 kHz sine. AC metrics exclude the first 60 ms.

| Metric | Value |
| --- | ---: |
| Plate DC | {metrics.plate_dc_v:.3f} V |
| Cathode DC | {metrics.cathode_dc_v:.3f} V |
| B+ DC | {metrics.bplus_dc_v:.3f} V |
| TMB output RMS | {metrics.stack_output_rms_v * 1000.0:.3f} mV |
| Grid RMS | {metrics.grid_rms_v * 1000.0:.3f} mV |
| Plate RMS after DC removal | {metrics.plate_rms_v * 1000.0:.3f} mV |
| Coupled recovery output RMS | {metrics.recovery_output_rms_v * 1000.0:.3f} mV |
| Plate gain relative to TMB output | {metrics.plate_gain:.2f}x |
| Plate gain | {metrics.plate_gain_db:.2f} dB |

## Engineering Notes

The recovery stage provides electrical makeup for passive insertion loss. A
runtime port must make its normalized-voltage conversion explicit and match this
cell before the whole-amp NAM comparison is used to judge the change.
""",
        encoding="utf-8",
    )


def write_klon_centaur_report(
    path: Path,
    fixture: SpiceFixture,
    data_path: Path,
    metrics: KlonCentaurSpiceMetrics,
) -> None:
    path.write_text(
        f"""# SPICE Fixture Report: {fixture.name}

## Inputs

- Netlist: `{fixture.netlist_path}`
- Data: `{data_path}`
- Source: ngspice batch run

## Circuit Scope

This fixture models the full Klon-style pedal path as a practical ngspice macro:
input buffer, clean/drive split, non-inverting gain stage, antiparallel germanium
clipping around Vref, passive tone/level shaping, output buffer, and high-Z load.
The passive component values are sourced from the public Klon Centaur BOM. The
TL072 stages use the Texas Instruments SLOJ067 PSpice macromodel, copied locally
for ngspice with the documented `RP` supply-current correction. The charge-pump
switching network is not simulated in this audio-path fixture; the op-amp rails
are idealized nominal Klon rails.

Primary references:

- Zpag Klon Centaur schematic/BOM: `https://www.zpag.net/Electroniques/Guitar/klon_centaur_schematic.html`
- Tiburonboy MNA/LTspice Klon analysis: `https://tiburonboy.github.io/Symbolic-Modified-Nodal-Analysis-using-Python/Klon%20Centaur%20part%202v0.html`
- TI TL072 PSpice model SLOJ067: `https://www.ti.com/lit/zip/sloj067`
- TI E2E TL072 model RP correction: `https://e2e.ti.com/support/tools/simulation-hardware-system-design-tools-group/sim-hw-system-design/f/simulation-hardware-system-design-tools-forum/622836/tina-spice-tl072-supply-current-result-of-tl072-spice-model`

## Settled 1 kHz Transient

Metrics are computed after the first 50 ms to avoid startup bias. All values are
AC after DC removal around the 4.5 V bias point.

| Metric | Value |
| --- | ---: |
| Input RMS | {metrics.input_rms_v * 1000.0:.3f} mV |
| Buffer RMS | {metrics.buffer_rms_v * 1000.0:.3f} mV |
| Clean path RMS | {metrics.clean_rms_v * 1000.0:.3f} mV |
| Drive stage RMS | {metrics.drive_rms_v * 1000.0:.3f} mV |
| Clip node RMS | {metrics.clip_rms_v * 1000.0:.3f} mV |
| Mix node RMS | {metrics.mix_rms_v * 1000.0:.3f} mV |
| Tone node RMS | {metrics.tone_rms_v * 1000.0:.3f} mV |
| Output RMS | {metrics.output_rms_v * 1000.0:.3f} mV |
| Output peak | {metrics.output_peak_v * 1000.0:.3f} mV |
| Output gain | {metrics.output_gain:.2f}x |
| Output gain | {metrics.output_gain_db:.2f} dB |
| Clip peak | {metrics.clip_peak_v * 1000.0:.3f} mV |
| Clip asymmetry | {metrics.clip_asymmetry_v * 1000.0:.3f} mV |

## Engineering Notes

Use this as a component-level reference before tuning the Minotaur Rust model.
The next useful step is generating the same fixture across `GAIN`, `TREBLE`, and
`LEVEL` parameter sweeps, then comparing those traces to Greybound pedal-only
renders and the local NAM Klon references.
""",
        encoding="utf-8",
    )


def write_common_cathode_dataset_report(
    path: Path,
    fixture: SpiceFixture,
    cases: list[CommonCathodeDatasetCase],
    metrics: CommonCathodeSpiceMetrics,
) -> None:
    rows = "\n".join(
        f"| `{case.stimulus_id}` | `{case.kind}` | `{case.split}` | `{case.expression}` |"
        for case in cases
    )
    path.write_text(
        f"""# SPICE Dataset Report: {fixture.name}

## Purpose

This dataset is the first multi-stimulus common-cathode corpus for Greybound's
SPICE-to-neural-cell workflow. It is intended for baseline training and export
smoke tests, not for final tube-stage model acceptance.

## Fixture

- Cell: 12AX7/ECC83 common-cathode gain stage
- Plate resistor: `100k`
- Cathode resistor: `1.5k`
- Cathode bypass capacitor: `25u`
- Input coupling capacitor: `22n`
- Grid leak: `1Meg`
- Raw supply: `280 V`
- Supply resistor: `10k`
- SPICE model: Koren-style `12AX7_KOREN`

## Reference Operating Point

Computed from the held nominal `sine_1khz_20mv` case after settling.

| Node | Voltage |
| --- | ---: |
| Plate | {metrics.plate_dc_v:.3f} V |
| Cathode | {metrics.cathode_dc_v:.3f} V |
| B+ | {metrics.bplus_dc_v:.3f} V |

| Metric | Value |
| --- | ---: |
| Input RMS | {metrics.input_rms_v * 1000.0:.3f} mV |
| Plate RMS after DC removal | {metrics.plate_rms_v * 1000.0:.3f} mV |
| Plate gain | {metrics.plate_gain:.2f}x |
| Plate gain | {metrics.plate_gain_db:.2f} dB |

## Stimuli

| Stimulus | Kind | Split | Expression |
| --- | --- | --- | --- |
{rows}

## Limitations

- Source impedance is still idealized at `0 ohm`.
- Load is still the grid leak / fixture context, not a downstream tone stack.
- B+ is fixed; there is no supply perturbation or sag corpus yet.
- Component tolerances are not swept.
- The corpus does not include real DI phrases yet.

Use this dataset to prove the training/export/runtime loop before drawing
conclusions about final model quality.
""",
        encoding="utf-8",
    )


def write_klon_centaur_dataset_report(
    path: Path,
    fixture: SpiceFixture,
    cases: list[KlonCentaurDatasetCase],
    metrics: KlonCentaurSpiceMetrics,
) -> None:
    rows = "\n".join(
        f"| `{case.stimulus_id}` | `{case.kind}` | `{case.split}` | {case.gain:.2f} | {case.treble:.2f} | `{case.expression}` |"
        for case in cases
    )
    path.write_text(
        f"""# SPICE Dataset Report: {fixture.name}

## Purpose

This dataset is the first Klon/Minotaur corpus for Greybound's targeted neural
work. It is designed for a small causal drive/clip/tone cell, not for a
full-pedal black-box replacement.

## Fixture

- Cell target: Klon drive stage, germanium clip node, summing/mix, and treble output
- Preserved analytic context: input buffer, clean feed-forward path, level/output recovery
- Op-amp model: local TI TL072 ngspice copy with the documented supply-current correction
- Rails: idealized nominal Klon rails

## Reference Case

Computed from `sine_1khz_120mv_gain55_treble60` after the first 50 ms.

| Metric | Value |
| --- | ---: |
| Input RMS | {metrics.input_rms_v * 1000.0:.3f} mV |
| Drive stage RMS | {metrics.drive_rms_v * 1000.0:.3f} mV |
| Clip node RMS | {metrics.clip_rms_v * 1000.0:.3f} mV |
| Mix node RMS | {metrics.mix_rms_v * 1000.0:.3f} mV |
| Tone node RMS | {metrics.tone_rms_v * 1000.0:.3f} mV |
| Output RMS | {metrics.output_rms_v * 1000.0:.3f} mV |
| Output gain | {metrics.output_gain_db:.2f} dB |
| Clip peak | {metrics.clip_peak_v * 1000.0:.3f} mV |
| Clip asymmetry | {metrics.clip_asymmetry_v * 1000.0:.3f} mV |

## Stimuli

| Stimulus | Kind | Split | Gain | Treble | Expression |
| --- | --- | --- | ---: | ---: | --- |
{rows}

## Training Scope

Use `buffer_v` plus normalized `gain`, `treble`, and short causal history as the
primary input. The initial targets should be `clip_ac_v`, `mix_ac_v`, and
`tone_ac_v`; keep the clean path and output level as analytic guardrails.

## Limitations

- Component tolerances and diode part variation are not swept yet.
- The charge-pump switching network remains out of scope.
- The dataset is synthetic SPICE only; NAM comparison remains the audio-level acceptance target.
- There is no real DI phrase in this SPICE corpus yet.
""",
        encoding="utf-8",
    )


def _remove_dc(samples: np.ndarray) -> np.ndarray:
    return samples - np.mean(samples)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _sample_rate_from_trace(path: Path, signals: tuple[str, ...]) -> int:
    trace = parse_wrdata(path, signals)
    if trace.time_s.shape[0] < 2:
        raise ValueError("SPICE trace is too short to infer sample rate")
    step_s = float(np.median(np.diff(trace.time_s)))
    if step_s <= 0.0:
        raise ValueError("SPICE trace has non-positive time step")
    return int(round(1.0 / step_s))


def _ngspice_version(repo_root: Path) -> str:
    try:
        result = subprocess.run(
            ["ngspice", "--version"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"
    return (result.stdout or result.stderr).strip().splitlines()[0]
