from __future__ import annotations

from pathlib import Path

import numpy as np

from greybound_lab.spice import FIXTURES, common_cathode_dataset_cases, common_cathode_dataset_manifest
from greybound_lab.spice import common_cathode_generated_netlist, common_cathode_metrics, klon_centaur_dataset_cases
from greybound_lab.spice import klon_centaur_generated_netlist, klon_centaur_metrics
from greybound_lab.spice import daybreaker_classic_tmb_metrics, daybreaker_presence_filter_metrics
from greybound_lab.spice import daybreaker_sss002_high_low_metrics
from greybound_lab.spice import daybreaker_sss002_high_low_chain_metrics
from greybound_lab.spice import daybreaker_sss002_tone_deep_metrics
from greybound_lab.spice import daybreaker_sss002_u37_recovery_metrics
from greybound_lab.spice import daybreaker_sss002_u4_plate_metrics
from greybound_lab.spice import daybreaker_sss002_u5_volume_u4_metrics
from greybound_lab.spice import daybreaker_tmb_recovery_metrics
from greybound_lab.spice import none_star_tone_presence_metrics, parse_wrdata
from greybound_lab.spice import sha256_file


def test_parse_wrdata_time_value_pairs(tmp_path: Path) -> None:
    path = tmp_path / "trace.dat"
    path.write_text(
        "\n".join(
            [
                "0.0 0.0 0.0 250.0",
                "0.1 1.0 0.1 249.0",
                "0.2 0.0 0.2 250.0",
            ]
        ),
        encoding="utf-8",
    )

    trace = parse_wrdata(path, ("input", "plate"))

    assert trace.time_s.tolist() == [0.0, 0.1, 0.2]
    assert trace.signals["input"].tolist() == [0.0, 1.0, 0.0]
    assert trace.signals["plate"].tolist() == [250.0, 249.0, 250.0]


def test_common_cathode_metrics(tmp_path: Path) -> None:
    path = tmp_path / "common.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        input_v = 0.02 if index % 2 == 0 else -0.02
        grid_v = input_v * 0.98
        plate_v = 250.0 - input_v * 15.0
        cathode_v = 0.4 + input_v * 0.1
        bplus_v = 277.0
        values = [input_v, grid_v, plate_v, cathode_v, bplus_v]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, ("input", "grid", "plate", "cathode", "bplus"))
    metrics = common_cathode_metrics(trace, settle_time_s=0.01)

    assert 249.0 < metrics.plate_dc_v < 251.0
    assert 14.0 < metrics.plate_gain < 16.0
    assert metrics.grid_coupling_loss_db < 0.0


def test_klon_centaur_metrics_parse_expected_columns(tmp_path: Path) -> None:
    path = tmp_path / "klon.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        sign = 1.0 if index % 2 == 0 else -1.0
        values = [
            0.08 * sign,
            4.5 + 0.08 * sign,
            4.5 + 0.01 * sign,
            4.5 + 0.35 * sign,
            4.5 + 0.25 * sign,
            4.5 + 0.90 * sign,
            4.5 + 0.30 * sign,
            4.5 + 0.70 * sign,
        ]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["klon-centaur"].signals)
    metrics = klon_centaur_metrics(trace, settle_time_s=0.01)

    assert 0.06 < metrics.input_rms_v < 0.10
    assert 0.30 < metrics.drive_rms_v < 0.40
    assert 0.20 < metrics.clip_peak_v < 0.30
    assert metrics.output_gain > 7.0


def test_none_star_tone_presence_fixture_is_registered() -> None:
    fixture = FIXTURES["none-star-tone-presence"]

    assert fixture.netlist_path.name == "none_star_tone_presence.cir"
    assert fixture.signals == ("input", "tone", "output")


def test_none_star_tone_presence_metrics_interpolate_ac_sweep(tmp_path: Path) -> None:
    path = tmp_path / "none-star-ac.dat"
    rows = []
    for frequency, tone_gain, output_gain in [
        (250.0, 0.50, 0.55),
        (1000.0, 0.80, 0.90),
        (4000.0, 1.10, 1.50),
        (8000.0, 1.20, 1.80),
        (16000.0, 1.25, 2.00),
    ]:
        values = [1.0, tone_gain, output_gain]
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["none-star-tone-presence"].signals)
    metrics = none_star_tone_presence_metrics(trace)

    assert metrics.presence_lift_8khz_db > 3.0
    assert metrics.output_minus_1khz_8khz_db > 5.0
    assert metrics.air_16khz_db > metrics.mid_1khz_db


def test_daybreaker_presence_filter_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-presence-filter"]

    assert fixture.netlist_path.name == "daybreaker_presence_filter.cir"
    assert fixture.signals == ("input", "transformer", "presence_band", "output")


def test_daybreaker_presence_filter_metrics_interpolate_ac_sweep(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-presence-ac.dat"
    rows = []
    for frequency, transformer_gain, presence_gain, output_gain in [
        (250.0, 0.99, 0.16, 1.08),
        (1000.0, 0.98, 0.52, 1.21),
        (4000.0, 0.77, 0.72, 1.24),
        (8000.0, 0.52, 0.46, 0.82),
        (16000.0, 0.29, 0.23, 0.45),
    ]:
        values = [1.0, transformer_gain, presence_gain, output_gain]
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-presence-filter"].signals)
    metrics = daybreaker_presence_filter_metrics(trace)

    assert metrics.output_minus_1khz_4khz_db > 0.0
    assert metrics.output_minus_1khz_16khz_db < -5.0


def test_daybreaker_classic_tmb_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-classic-tmb"]

    assert fixture.netlist_path.name == "daybreaker_classic_tmb.cir"
    assert fixture.signals == ("source", "input", "tone", "output")


def test_daybreaker_sss002_classic_tmb_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-classic-tmb"]

    assert fixture.netlist_path.name == "daybreaker_sss002_classic_tmb.cir"
    assert fixture.signals == ("source", "input", "tone", "output")


def test_daybreaker_sss002_high_low_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-high-low-filters"]

    assert fixture.netlist_path.name == "daybreaker_sss002_high_low_filters.cir"
    assert fixture.signals == (
        "source",
        "high_1", "high_2", "high_3", "high_4", "high_5", "high_6", "high_7",
        "low_1", "low_2", "low_3", "low_4", "low_5", "low_6", "low_7",
    )


def test_daybreaker_sss002_high_low_chain_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-high-low-chain"]

    assert fixture.netlist_path.name == "daybreaker_sss002_high_low_chain.cir"
    assert fixture.signals == ("source", "high_input", "output", "low_common")


def test_daybreaker_sss002_tone_deep_fixtures_are_registered() -> None:
    expected_signals = ("source", "plate", "tone_source", "treble_wiper", "bass_wiper", "u5_input", "volume_wiper", "grid")

    asc = FIXTURES["daybreaker-sss002-tone-deep-asc"]
    layout = FIXTURES["daybreaker-sss002-tone-deep-layout"]

    assert asc.netlist_path.name == "daybreaker_sss002_tone_deep_asc.cir"
    assert layout.netlist_path.name == "daybreaker_sss002_tone_deep_layout.cir"
    assert asc.signals == expected_signals
    assert layout.signals == expected_signals


def test_daybreaker_sss002_u37_recovery_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-high-low-u37-recovery"]

    assert fixture.netlist_path.name == "daybreaker_sss002_high_low_u37_recovery.cir"
    assert fixture.signals == ("source", "high_input", "filter_output", "plate", "cath", "recovery_output", "bplus")


def test_daybreaker_sss002_u4_plate_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-u4-plate-stage"]

    assert fixture.netlist_path.name == "daybreaker_sss002_u4_plate_stage.cir"
    assert fixture.signals == ("source", "grid", "plate", "cath", "output", "hta")


def test_daybreaker_sss002_u5_volume_u4_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-sss002-u5-volume-u4"]

    assert fixture.netlist_path.name == "daybreaker_sss002_u5_volume_u4.cir"
    assert fixture.signals == ("source", "wiper", "grid", "plate", "cath", "output", "hta")


def test_daybreaker_sss002_high_low_metrics_resolve_switch_curves(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-high-low-ac.dat"
    rows = []
    for frequency in (100.0, 1_000.0, 8_000.0):
        values = [1.0]
        values.extend([0.15, 0.12, 0.10, 0.09, 0.08, 0.07, 0.06])
        values.extend([0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70])
        if frequency == 8_000.0:
            values[2] = 0.60
            values[8] = 0.05
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-high-low-filters"].signals)
    metrics = daybreaker_sss002_high_low_metrics(trace)

    assert len(metrics.high_1khz_db) == 7
    assert metrics.high_8khz_minus_1khz_db[1] > 10.0
    assert metrics.low_8khz_minus_1khz_db[0] < 0.0


def test_daybreaker_sss002_high_low_chain_metrics_interpolate_ac_sweep(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-high-low-chain-ac.dat"
    rows = []
    for frequency, output_gain in [(100.0, 0.01), (1_000.0, 0.10), (8_000.0, 0.25), (16_000.0, 0.20)]:
        values = [1.0, 0.5, output_gain, 0.05]
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-high-low-chain"].signals)
    metrics = daybreaker_sss002_high_low_chain_metrics(trace)

    assert metrics.output_1khz_db < -10.0
    assert metrics.output_minus_1khz_8khz_db > 7.0


def test_daybreaker_sss002_tone_deep_metrics_interpolate_ac_sweep(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-tone-deep-ac.dat"
    rows = []
    for frequency, grid_gain in [(100.0, 0.01), (1_000.0, 0.02), (8_000.0, 0.04), (16_000.0, 0.02)]:
        values = [1.0, 0.7, 0.4, 0.3, 0.2, 0.25, 0.04, grid_gain]
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-tone-deep-asc"].signals)
    metrics = daybreaker_sss002_tone_deep_metrics(trace)

    assert metrics.grid_1khz_db < -30.0
    assert metrics.grid_minus_1khz_8khz_db > 5.0


def test_daybreaker_sss002_u37_recovery_metrics_use_settled_transient(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-u37-recovery.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        sign = 1.0 if index % 2 == 0 else -1.0
        values = [
            0.02 * sign,
            0.004 * sign,
            0.0023 * sign,
            177.01 - 0.132 * sign,
            1.230,
            0.132 * sign,
            300.0,
        ]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-high-low-u37-recovery"].signals)
    metrics = daybreaker_sss002_u37_recovery_metrics(trace, settle_time_s=0.010)

    assert 176.0 < metrics.plate_dc_v < 178.0
    assert 55.0 < metrics.plate_gain < 60.0
    assert metrics.recovery_output_rms_v > 0.12


def test_daybreaker_sss002_u4_plate_metrics_use_settled_transient(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-u4-plate.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        sign = 1.0 if index % 2 == 0 else -1.0
        values = [
            0.02 * sign,
            0.0132 * sign,
            263.66 - 0.681 * sign,
            2.645,
            0.681 * sign,
            440.0,
        ]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-u4-plate-stage"].signals)
    metrics = daybreaker_sss002_u4_plate_metrics(trace, settle_time_s=0.010)

    assert 263.0 < metrics.plate_dc_v < 264.0
    assert 50.0 < metrics.plate_gain < 52.0
    assert metrics.output_rms_v > 0.60


def test_daybreaker_sss002_u5_volume_u4_metrics_use_settled_transient(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-sss002-u5-volume-u4.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        sign = 1.0 if index % 2 == 0 else -1.0
        values = [
            0.02 * sign,
            0.002 * sign,
            0.001405 * sign,
            263.66 - 0.0723 * sign,
            2.645,
            0.0723 * sign,
            440.0,
        ]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-sss002-u5-volume-u4"].signals)
    metrics = daybreaker_sss002_u5_volume_u4_metrics(trace, settle_time_s=0.010)

    assert 0.09 < metrics.wiper_gain < 0.11
    assert 50.0 < metrics.plate_gain < 52.0
    assert metrics.output_rms_v > 0.07


def test_daybreaker_classic_tmb_metrics_interpolate_ac_sweep(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-classic-tmb-ac.dat"
    rows = []
    for frequency, input_gain, tone_gain, output_gain in [
        (100.0, 0.80, 0.10, 0.025),
        (250.0, 0.76, 0.18, 0.055),
        (1000.0, 0.72, 0.34, 0.130),
        (4000.0, 0.70, 0.38, 0.155),
        (8000.0, 0.70, 0.39, 0.160),
        (16000.0, 0.70, 0.39, 0.160),
    ]:
        values = [1.0, input_gain, tone_gain, output_gain]
        rows.append(" ".join(f"{item:.9g}" for pair in [(frequency, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-classic-tmb"].signals)
    metrics = daybreaker_classic_tmb_metrics(trace)

    assert metrics.input_1khz_db < 0.0
    assert metrics.output_1khz_db < -10.0
    assert metrics.output_minus_1khz_4khz_db > 0.0


def test_daybreaker_tmb_recovery_fixture_is_registered() -> None:
    fixture = FIXTURES["daybreaker-tmb-recovery-12ax7"]

    assert fixture.netlist_path.name == "daybreaker_tmb_recovery_12ax7.cir"
    assert fixture.signals == ("source", "input", "tone", "stack_output", "grid", "plate", "recovery_output", "cath", "bplus")


def test_daybreaker_tmb_recovery_metrics_use_settled_transient(tmp_path: Path) -> None:
    path = tmp_path / "daybreaker-tmb-recovery.dat"
    rows = []
    for index in range(100):
        time = index * 0.001
        sign = 1.0 if index % 2 == 0 else -1.0
        values = [
            0.02 * sign,
            0.014 * sign,
            0.002 * sign,
            0.002 * sign,
            0.002 * sign,
            252.4 - 0.029 * sign,
            0.026 * sign,
            0.542,
            277.0,
        ]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    path.write_text("\n".join(rows), encoding="utf-8")

    trace = parse_wrdata(path, FIXTURES["daybreaker-tmb-recovery-12ax7"].signals)
    metrics = daybreaker_tmb_recovery_metrics(trace, settle_time_s=0.010)

    assert 252.0 < metrics.plate_dc_v < 253.0
    assert 14.0 < metrics.plate_gain < 15.0
    assert metrics.grid_rms_v > 0.001


def test_common_cathode_dataset_manifest(tmp_path: Path) -> None:
    repo_root = Path.cwd()
    fixture = FIXTURES["common-cathode-12ax7"]
    data_path = tmp_path / "common.dat"
    dataset_path = tmp_path / "common.dataset.npz"
    report_path = tmp_path / "common.md"
    rows = []
    for index in range(100):
        time = index * 0.001
        input_v = 0.02 if index % 2 == 0 else -0.02
        grid_v = input_v * 0.98
        plate_v = 250.0 - input_v * 15.0
        cathode_v = 0.4 + input_v * 0.1
        bplus_v = 277.0
        values = [input_v, grid_v, plate_v, cathode_v, bplus_v]
        rows.append(" ".join(f"{item:.9g}" for pair in [(time, value) for value in values] for item in pair))
    data_path.write_text("\n".join(rows), encoding="utf-8")
    np.savez(dataset_path, input_v=np.array([0.0, 1.0]), plate_v=np.array([250.0, 249.0]))
    report_path.write_text("# report\n", encoding="utf-8")

    trace = parse_wrdata(data_path, fixture.signals)
    metrics = common_cathode_metrics(trace, settle_time_s=0.01)
    manifest = common_cathode_dataset_manifest(
        fixture=fixture,
        repo_root=repo_root,
        data_path=data_path,
        dataset_path=dataset_path,
        report_path=report_path,
        metrics=metrics,
    )

    assert manifest["schema_version"] == 1
    assert manifest["fixture_id"] == "common-cathode-12ax7"
    assert manifest["cell_kind"] == "triode_gain_stage"
    assert manifest["sample_rate_hz"] == 1000
    assert manifest["stimuli"][0]["kind"] == "settled_sine"
    assert manifest["artifacts"][0]["sha256"] == sha256_file(dataset_path)


def test_common_cathode_dataset_cases_cover_splits(tmp_path: Path) -> None:
    cases = common_cathode_dataset_cases()
    splits = {case.split for case in cases}

    assert splits == {"train", "validation", "test"}
    assert any(case.kind == "two_tone_imd" for case in cases)
    assert any(case.kind == "dynamic_burst" and case.split == "train" for case in cases)
    assert any(case.kind == "dynamic_decay" and case.split == "test" for case in cases)
    assert any(case.kind == "dynamic_bias_recovery" and case.split == "test" for case in cases)
    assert any(case.stimulus_id == "sine_1khz_400mv" and case.split == "train" for case in cases)
    assert any(case.stimulus_id == "sine_1khz_300mv" and case.split == "validation" for case in cases)
    assert any(case.stimulus_id == "sine_1khz_120mv" and case.split == "test" for case in cases)

    dynamic_case = next(case for case in cases if case.kind == "dynamic_burst")
    netlist = common_cathode_generated_netlist(dynamic_case, tmp_path / "case.dat")
    assert "BVIN in 0" in netlist
    assert "tanh((time-0.032)" in netlist
    assert "12AX7_KOREN" in netlist


def test_klon_centaur_dataset_cases_generate_parametric_netlists(tmp_path: Path) -> None:
    cases = klon_centaur_dataset_cases()
    splits = {case.split for case in cases}

    assert splits == {"train", "validation", "test"}
    assert any(case.kind == "gain_control_sweep" and case.gain > 0.75 for case in cases)
    assert any(case.kind == "treble_control_sweep" and case.treble > 0.80 for case in cases)
    assert any(case.kind == "two_tone_imd" for case in cases)
    assert any(case.kind == "dynamic_decay" and case.split == "test" for case in cases)

    source = Path("tests/fixtures/circuit/klon_centaur.cir").read_text(encoding="utf-8")
    dynamic_case = next(case for case in cases if case.kind == "dynamic_burst")
    netlist = klon_centaur_generated_netlist(source, dynamic_case, tmp_path / "case.dat")

    assert ".param GAIN=0.55" in netlist
    assert ".param TREBLE=0.6" in netlist
    assert "BVIN guitar 0 V={" in netlist
    assert "tanh((time-0.032)" in netlist
    assert f"wrdata {(tmp_path / 'case.dat').resolve()}" in netlist
