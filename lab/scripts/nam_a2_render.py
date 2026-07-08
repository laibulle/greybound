from __future__ import annotations

import argparse
import json
from math import gcd
from pathlib import Path

import numpy as np
import torch
from nam.models import init_from_nam
from scipy.io import wavfile
from scipy.signal import resample_poly


def main() -> None:
    parser = argparse.ArgumentParser(prog="nam_a2_render.py")
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--sample-rate", type=int, default=48_000)
    parser.add_argument("--seconds", type=float)
    parser.add_argument("--input-db", type=float, default=0.0)
    parser.add_argument("--output-db", type=float, default=0.0)
    args = parser.parse_args()

    render_a2(
        model_path=args.model,
        input_wav=args.input,
        output_wav=args.output,
        output_sample_rate_hz=args.sample_rate,
        seconds=args.seconds,
        input_gain_db=args.input_db,
        output_gain_db=args.output_db,
    )


def render_a2(
    *,
    model_path: Path,
    input_wav: Path,
    output_wav: Path,
    output_sample_rate_hz: int,
    seconds: float | None,
    input_gain_db: float,
    output_gain_db: float,
) -> None:
    model_config = _load_a2_model_config(model_path)
    model_sample_rate_hz = int(float(model_config.get("sample_rate") or 48_000))
    model = init_from_nam(model_config)
    model.eval()

    input_sample_rate_hz, samples = _read_wav_mono(input_wav)
    if seconds is not None:
        samples = samples[: int(round(seconds * input_sample_rate_hz))]
    if input_sample_rate_hz != model_sample_rate_hz:
        samples = _resample(samples, input_sample_rate_hz, model_sample_rate_hz)
    samples = samples * _db_to_gain(input_gain_db)

    with torch.inference_mode():
        tensor = torch.from_numpy(samples.astype(np.float32))
        rendered = model(tensor, pad_start=True).detach().cpu().numpy().astype(np.float32)
    rendered = rendered * _db_to_gain(output_gain_db)

    if output_sample_rate_hz != model_sample_rate_hz:
        rendered = _resample(rendered, model_sample_rate_hz, output_sample_rate_hz)

    output_wav.parent.mkdir(parents=True, exist_ok=True)
    wavfile.write(output_wav, output_sample_rate_hz, rendered.astype(np.float32))


def _load_a2_model_config(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("architecture") != "SlimmableContainer":
        return data
    submodels = data.get("config", {}).get("submodels", [])
    if not submodels:
        raise ValueError(f"SlimmableContainer has no submodels: {path}")
    selected = max(submodels, key=lambda item: float(item["max_value"]))
    model = selected["model"]
    if "sample_rate" not in model and "sample_rate" in data:
        model["sample_rate"] = data["sample_rate"]
    return model


def _read_wav_mono(path: Path) -> tuple[int, np.ndarray]:
    sample_rate_hz, samples = wavfile.read(path)
    samples = np.asarray(samples)
    if samples.ndim == 2:
        samples = samples.mean(axis=1)
    if np.issubdtype(samples.dtype, np.integer):
        max_value = float(np.iinfo(samples.dtype).max)
        samples = samples.astype(np.float32) / max_value
    else:
        samples = samples.astype(np.float32)
    return int(sample_rate_hz), samples


def _resample(samples: np.ndarray, source_hz: int, target_hz: int) -> np.ndarray:
    divisor = gcd(source_hz, target_hz)
    return resample_poly(samples, target_hz // divisor, source_hz // divisor).astype(np.float32)


def _db_to_gain(db: float) -> float:
    return float(10.0 ** (db / 20.0))


if __name__ == "__main__":
    main()
