/// <reference path="./rscpp-wasm.d.ts" />
import type { RunResult } from "./types.js";

type WasmApi = {
  default: (module_or_path?: unknown) => Promise<unknown>;
  run: (source: string) => unknown;
  run_method: (source: string, method: string, args: unknown) => unknown;
};

let api: WasmApi | null = null;
let initError: string | null = null;

function failed(message: string): RunResult {
  return { ok: false, events: [], error: { message } };
}

function normalize(raw: unknown): RunResult {
  if (!raw || typeof raw !== "object") {
    return failed("invalid run result from wasm");
  }
  const r = raw as Record<string, unknown>;
  const errorRaw = r.error as
    | { message?: string; span?: { start: number; end: number } }
    | string
    | undefined;
  let error: RunResult["error"];
  if (typeof errorRaw === "string") {
    error = { message: errorRaw };
  } else if (errorRaw && typeof errorRaw === "object" && errorRaw.message) {
    error = {
      message: errorRaw.message,
      span: errorRaw.span,
    };
  }
  return {
    ok: Boolean(r.ok),
    value: r.value,
    events: Array.isArray(r.events) ? (r.events as RunResult["events"]) : [],
    error,
  };
}

/**
 * Load wasm-bindgen glue. `wasmUrl` should point at `rscpp_wasm_bg.wasm`
 * (or let the glue default relative to the JS module).
 */
export async function initRunner(wasmUrl?: string): Promise<void> {
  try {
    const mod = (await import("rscpp-wasm")) as unknown as WasmApi;
    if (wasmUrl) {
      await mod.default({ module_or_path: wasmUrl });
    } else {
      await mod.default();
    }
    api = mod;
    initError = null;
  } catch (e) {
    api = null;
    initError = e instanceof Error ? e.message : String(e);
  }
}

export async function run(source: string): Promise<RunResult> {
  if (!api) {
    return failed(initError ? `wasm not initialized: ${initError}` : "wasm not initialized");
  }
  try {
    return normalize(api.run(source));
  } catch (e) {
    return failed(e instanceof Error ? e.message : String(e));
  }
}

export async function runMethod(
  source: string,
  method: string,
  args: unknown[],
): Promise<RunResult> {
  if (!api) {
    return failed(initError ? `wasm not initialized: ${initError}` : "wasm not initialized");
  }
  try {
    return normalize(api.run_method(source, method, args));
  } catch (e) {
    return failed(e instanceof Error ? e.message : String(e));
  }
}

export type { RunResult, RunError } from "./types.js";
