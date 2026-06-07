from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from greybound_lab.neural_cell import infer_artifact_numpy, infer_mlp_numpy, read_mlp_weights, write_mlp_weights


def test_mlp_weight_roundtrip_and_numpy_inference(tmp_path: Path) -> None:
    weights_path = tmp_path / "weights.greybound.bin"
    layers = [
        {
            "weight": np.array([[2.0], [-1.0]], dtype=np.float32),
            "bias": np.array([0.5, -0.25], dtype=np.float32),
        },
        {
            "weight": np.array([[1.0, -0.5]], dtype=np.float32),
            "bias": np.array([0.1], dtype=np.float32),
        },
    ]

    write_mlp_weights(weights_path, layers)
    descriptor = {
        "weights": {
            "layout": [
                {"in_features": 1, "out_features": 2},
                {"in_features": 2, "out_features": 1},
            ]
        }
    }
    loaded = read_mlp_weights(weights_path, descriptor)
    x = np.array([[0.0], [0.5]], dtype=np.float32)

    np.testing.assert_allclose(loaded[0]["weight"], layers[0]["weight"])
    np.testing.assert_allclose(infer_mlp_numpy(x, loaded), infer_mlp_numpy(x, layers))


def test_infer_artifact_numpy_applies_normalization(tmp_path: Path) -> None:
    weights_path = tmp_path / "weights.greybound.bin"
    descriptor_path = tmp_path / "model.greybound.json"
    layers = [
        {
            "weight": np.array([[1.0]], dtype=np.float32),
            "bias": np.array([0.0], dtype=np.float32),
        }
    ]
    write_mlp_weights(weights_path, layers)
    descriptor = {
        "io": {
            "normalization": {
                "input_mean": 1.0,
                "input_std": 2.0,
                "output_mean": 10.0,
                "output_std": 4.0,
            }
        },
        "weights": {
            "path": "weights.greybound.bin",
            "layout": [{"in_features": 1, "out_features": 1}],
        },
    }
    descriptor_path.write_text(json.dumps(descriptor), encoding="utf-8")

    output = infer_artifact_numpy(descriptor_path, np.array([1.0, 3.0], dtype=np.float32))

    np.testing.assert_allclose(output, np.array([10.0, 14.0], dtype=np.float32), rtol=1e-6)
