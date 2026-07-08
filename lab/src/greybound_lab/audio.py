from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.io import wavfile


@dataclass(frozen=True)
class AudioBuffer:
    path: Path
    sample_rate: int
    samples: np.ndarray

    @property
    def duration_seconds(self) -> float:
        return float(self.samples.shape[0] / self.sample_rate)


def read_wav_mono(path: Path) -> AudioBuffer:
    sample_rate, data = wavfile.read(path)
    samples = _to_float32(data)
    if samples.ndim == 2:
        samples = samples.mean(axis=1)
    if samples.ndim != 1:
        raise ValueError(f"{path} has unsupported WAV shape {samples.shape}")
    if not np.all(np.isfinite(samples)):
        raise ValueError(f"{path} contains non-finite samples")
    return AudioBuffer(path=path, sample_rate=int(sample_rate), samples=samples)


def _to_float32(data: np.ndarray) -> np.ndarray:
    if np.issubdtype(data.dtype, np.floating):
        return data.astype(np.float32, copy=False)
    if data.dtype == np.int16:
        return data.astype(np.float32) / float(np.iinfo(np.int16).max)
    if data.dtype == np.int32:
        return data.astype(np.float32) / float(np.iinfo(np.int32).max)
    if data.dtype == np.uint8:
        return (data.astype(np.float32) - 128.0) / 128.0
    raise ValueError(f"unsupported WAV sample dtype {data.dtype}")
