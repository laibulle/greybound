import type { GreyboundNox30 } from "./greybound-wasm/greybound_wasm";
import type { AmpControlId } from "./rigs";

type GreyboundWasmModule = typeof import("./greybound-wasm/greybound_wasm");
type AmpValues = Record<AmpControlId, number>;

let wasmModulePromise: Promise<GreyboundWasmModule> | null = null;

export async function loadGreyboundWasm(): Promise<GreyboundWasmModule> {
  if (!wasmModulePromise) {
    wasmModulePromise = import("./greybound-wasm/greybound_wasm").then(async (module) => {
      await module.default();
      return module;
    });
  }
  return wasmModulePromise;
}

export async function createNox30WasmEngine(sampleRate: number): Promise<GreyboundNox30> {
  const module = await loadGreyboundWasm();
  return new module.GreyboundNox30(sampleRate);
}

export function applyNox30AmpControls(engine: GreyboundNox30, values: AmpValues, output = 1.0) {
  engine.set_amp_controls(
    values.volume,
    values.bass,
    values.treble,
    values.cut,
    values.drive,
    values.presence,
    values.sag,
    output,
  );
}
