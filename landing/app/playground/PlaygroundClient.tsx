"use client";

import { useEffect, useState } from "react";

type GreyboundWasmModule = {
  default: (input?: string | URL | Request) => Promise<unknown>;
  run: () => void;
};

const WASM_VERSION = "web-audio-next-route-20260709";

function isControlFlowException(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("Using exceptions for control flow");
}

export default function PlaygroundClient() {
  const [status, setStatus] = useState("Loading Greybound...");

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        const nativeImport = new Function("specifier", "return import(specifier)") as (
          specifier: string,
        ) => Promise<GreyboundWasmModule>;
        const module = await nativeImport(
          `/greybound-web/pkg/greybound_wasm.js?v=${WASM_VERSION}`,
        );

        try {
          await module.default(
            `/greybound-web/pkg/greybound_wasm_bg.wasm?v=${WASM_VERSION}`,
          );
          module.run();
        } catch (error) {
          if (!isControlFlowException(error)) {
            throw error;
          }
        }

        if (!cancelled) {
          setStatus("");
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : String(error);
          setStatus(`Greybound could not start: ${message}`);
        }
      }
    }

    boot();

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="playgroundPage" aria-label="Greybound playground">
      <a className="playgroundBack" href="/" aria-label="Back to Greybound landing">
        Greybound
      </a>
      <div id="greybound-web-root" className="playgroundRoot">
        {status ? <div className="playgroundBoot">{status}</div> : null}
      </div>
    </main>
  );
}
