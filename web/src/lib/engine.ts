/// <reference types="vite/client" />

// The playscript engine, compiled to wasm. `init` must run once (it fetches
// and instantiates the .wasm) before `run` can be called.
import init, { run } from 'playscript-wasm';
import wasmUrl from 'playscript-wasm/playscript_wasm_bg.wasm?url';

export type JsonValue = number | boolean | string | JsonValue[];

// Mirrors the JSON the bridge emits (see wasm/src/lib.rs).
export type TraceEvent = { op: number; depth: number } & (
	| { kind: 'assign'; target: string; value: JsonValue }
	| { kind: 'call'; name: string; args: JsonValue[] }
	| { kind: 'return'; name: string; value: JsonValue }
	| { kind: 'branch'; construct: string; value: boolean }
);

export type RunResult =
	| { ok: true; value: JsonValue | null; ops: number; trace: TraceEvent[] }
	| { ok: false; error: string; ops: number; trace: TraceEvent[] };

let ready: Promise<unknown> | null = null;

/** Load the wasm engine once; safe to await repeatedly. */
export function load(): Promise<unknown> {
	if (!ready) {
		ready = init({ module_or_path: wasmUrl });
	}

	return ready;
}

/** Run a program and parse the bridge's JSON outcome. Requires `load()` first. */
export function runProgram(source: string): RunResult {
	return JSON.parse(run(source)) as RunResult;
}
