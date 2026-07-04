# playscript

A tiny, **deterministic**, fully **metered** scripting language with a tree-walking
interpreter written in Rust — plus a wasm bridge and a browser visualizer that
lets you _watch a program run_.

## Why

Most languages are shaped by the pursuit of speed, and pay for it with undefined
behavior, silent coercions, and nondeterminism. playscript makes the opposite
trade: it is a tree-walker (slow, and that is accepted forever), and in exchange
it buys properties fast languages cannot afford:

- **Deterministic** — a program is a pure function of its inputs. Same result,
  same operation count, on any machine, forever.
- **Metered** — every step costs _fuel_. `while (true) {}` does not hang; it hits
  the budget and stops. The operation counter doubles as a logical clock.
- **Legible** — strict typing, no `null`, no coercion, value semantics (no
  aliasing). You can be the interpreter in your own head, and be right.

It grew out of a conversation about [Leek Wars](https://leekwars.com)' LeekScript
and its per-turn operations budget.

## A taste

```
function fib(n) {
  if (n < 2) { return n }
  return fib(n - 1) + fib(n - 2)
}

fib(20)
```

Because everything is metered, the op counter is an honest cost meter: recursive
`fib(20)` costs **470,656 ops**, while the iterative version costs **332** — a
~1400× gap that reflects the real algorithmic difference, not micro-syntax. That
is the whole idea: a language you can _measure_ and _replay_.

## Layout

A cargo workspace; the engine stays at the root and is dependency-free.

```
.
├── src/       the engine: lexer, parser, AST, tree-walking interpreter (crate "playscript", lib + CLI)
├── tests/     integration tests
├── examples/  a stack-depth probe
├── wasm/      playscript-wasm — the wasm bridge: run(src) -> trace as JSON
└── web/       SvelteKit visualizer (deployed to Cloudflare Workers)
```

## The interpreter (CLI)

```sh
cargo run                       # REPL
cargo run -- "1 + 2 * 3"        # evaluate one program and print the result
cargo run -- --trace "..."      # print an execution trace
cargo test --workspace          # run all tests (engine + bridge)
```

The REPL reports the result and the operations spent — your golf score:

```
> var s = 0; var i = 1; while (i <= 100) { s = s + i; i = i + 1 }; s
5050  [1210 ops]
```

## The visualizer (web)

The engine compiles to wasm; the SvelteKit app puts the program on one side and
its execution trace on the other, with a scrubber to step through the run.

```sh
wasm-pack build wasm --target web   # build the wasm package (web/ links to it; pkg/ is gitignored)
cd web && bun install               # link playscript-wasm
bun run dev                         # http://localhost:5173
```

Deploy to Cloudflare with `bun run deploy:cf`.

## Language reference

- **Values** — `number` (f64), `bool`, `string`, `array`, `function`.
- **Variables** — `var x = 5` declares in the current scope; `x = 6` reassigns an
  existing binding (assigning an undeclared name is an error).
- **Operators** — `+ - * / %`, comparisons `< <= > >= == !=`, logical `&& ||`
  (short-circuiting), unary `- !`. Strict throughout: no coercion (`1 == true` is
  `false`), no truthiness (`if (1)` is a type error).
- **Control flow** — `if` / `else if` / `else` and `while`; braces are mandatory.
- **Functions** — `function f(a, b) { ... }`, lambdas `x => x + 1` and
  `(a, b) => { ... }`; first-class and recursive. Closures capture surrounding
  locals **by value** (globals stay live).
- **Strings** — immutable; `+` concatenates two strings.
- **Arrays** — value semantics via copy-on-write: `var b = a; b[0] = 9` leaves `a`
  untouched, and function arguments never alias.
- **No null** — `var` requires an initial value, and every function must `return`
  one.
- **Builtins** — `print`, `getOperations`, `getOperationsLimit`, `str`, `len`,
  `array(n, fill)`, `push(arr, v)`.

## Design principles

- **Tree-walker, forever** — no bytecode / JIT / compiled backend. The slowness is
  the price of total observability, and it is paid once.
- **Fuel is per-node metering** — a default budget of 1,000,000 operations per run,
  charged inside `eval`. Tracing is a pure observer: it never consumes fuel, so a
  traced run and a plain run agree on both result and op count.
- **Closed and deterministic** — no wall clock, no unseeded randomness; effects only
  through explicit host builtins. A recording is therefore just its inputs, and any
  run is replayable to the operation, forever.

---

A weekend project, built one small step at a time.
