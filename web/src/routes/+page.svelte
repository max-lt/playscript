<script lang="ts">
	import { onMount } from 'svelte';
	import { load, runProgram, type JsonValue, type RunResult, type TraceEvent } from '$lib/engine';

	const EXAMPLES: Record<string, string> = {
		'Fibonacci récursif': `function fib(n) {\n  if (n < 2) { return n }\n  return fib(n - 1) + fib(n - 2)\n}\n\nfib(6)`,
		'Somme (boucle)': `var sum = 0;\nvar i = 1;\nwhile (i <= 5) {\n  sum = sum + i;\n  i = i + 1\n}\nsum`,
		'Tri à bulles': `var a = [5, 3, 8, 1, 9];\nvar n = len(a);\nvar i = 0;\nwhile (i < n) {\n  var j = 0;\n  while (j < n - 1) {\n    if (a[j] > a[j + 1]) {\n      var t = a[j];\n      a[j] = a[j + 1];\n      a[j + 1] = t\n    }\n    j = j + 1\n  }\n  i = i + 1\n}\na`,
		'Monade Maybe': `function unit(x) { return [x] }\nfunction bind(m, f) {\n  if (len(m) == 0) { return [] }\n  return f(m[0])\n}\nfunction safediv(a, b) {\n  if (b == 0) { return [] }\n  return [a / b]\n}\n\nbind(bind(unit(20), x => safediv(x, 2)), y => safediv(y, 5))`
	};

	let source = $state(EXAMPLES['Fibonacci récursif']);
	let ready = $state(false);
	let result = $state<RunResult | null>(null);
	let step = $state(0);
	let traceEl = $state<HTMLElement>();

	const events = $derived<TraceEvent[]>(result?.trace ?? []);

	onMount(async () => {
		await load();
		ready = true;
		run();
	});

	function run() {
		if (!ready) return;
		result = runProgram(source);
		step = result.trace.length;
	}

	function useExample(name: string) {
		source = EXAMPLES[name];
		run();
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			run();
		}
	}

	// Keep the current event in view as the scrubber moves.
	$effect(() => {
		step;
		traceEl?.querySelector('[data-current]')?.scrollIntoView({ block: 'nearest' });
	});

	function fmtValue(v: JsonValue | null): string {
		if (v === null) return '—';

		if (typeof v === 'string') return JSON.stringify(v);

		if (Array.isArray(v)) return '[' + v.map(fmtValue).join(', ') + ']';

		return String(v);
	}

	function detail(e: TraceEvent): string {
		switch (e.kind) {
			case 'assign':
				return `${e.target} = ${fmtValue(e.value)}`;
			case 'call':
				return `${e.name}(${e.args.map(fmtValue).join(', ')})`;
			case 'return':
				return `${e.name} → ${fmtValue(e.value)}`;
			case 'branch':
				return `→ ${e.value}`;
		}
	}

	function badge(e: TraceEvent): string {
		return e.kind === 'branch'
			? e.construct
			: { call: 'call', return: 'ret', assign: 'set' }[e.kind];
	}

	const kindColor: Record<TraceEvent['kind'], string> = {
		call: 'text-sky-400',
		return: 'text-emerald-400',
		branch: 'text-amber-400',
		assign: 'text-slate-400'
	};
</script>

<div class="flex h-screen flex-col bg-slate-950 text-slate-200">
	<header class="flex items-baseline gap-3 border-b border-slate-800 px-5 py-3">
		<h1 class="text-lg font-semibold tracking-tight">playscript</h1>
		<p class="hidden text-sm text-slate-500 sm:block">
			un langage déterministe et métré — le programme, et son déroulé
		</p>
	</header>

	<main class="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-2">
		<!-- Left: the program -->
		<section class="flex min-h-0 flex-col border-b border-slate-800 md:border-r md:border-b-0">
			<div class="flex flex-wrap gap-2 border-b border-slate-800 px-4 py-2">
				{#each Object.keys(EXAMPLES) as name (name)}
					<button
						class="rounded bg-slate-800 px-2.5 py-1 text-xs text-slate-300 hover:bg-slate-700"
						onclick={() => useExample(name)}
					>
						{name}
					</button>
				{/each}
			</div>
			<textarea
				class="min-h-40 flex-1 resize-none bg-slate-950 p-4 font-mono text-sm leading-relaxed text-slate-100 outline-none"
				bind:value={source}
				onkeydown={onKey}
				spellcheck="false"></textarea>
			<div class="flex items-center gap-3 border-t border-slate-800 px-4 py-2.5">
				<button
					class="rounded bg-indigo-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50"
					onclick={run}
					disabled={!ready}
				>
					{ready ? 'Exécuter' : 'Chargement…'}
				</button>
				<span class="text-xs text-slate-500">⌘/Ctrl + Entrée</span>
			</div>
		</section>

		<!-- Right: the execution -->
		<section class="flex min-h-0 flex-col">
			{#if result}
				<div
					class="flex items-center justify-between gap-3 border-b border-slate-800 px-4 py-2.5 text-sm"
				>
					{#if result.ok}
						<span class="truncate">
							<span class="text-slate-500">résultat</span>
							<span class="ml-2 font-mono text-emerald-400"> {fmtValue(result.value)} </span>
						</span>
					{:else}
						<span class="truncate font-mono text-rose-400"> {result.error} </span>
					{/if}
					<span class="shrink-0 font-mono text-xs text-slate-400">
						{result.ops.toLocaleString('fr-FR')} ops
					</span>
				</div>

				{#if events.length}
					<div class="flex items-center gap-3 border-b border-slate-800 px-4 py-2">
						<button
							class="rounded px-2 py-0.5 text-slate-400 hover:bg-slate-800"
							onclick={() => (step = Math.max(0, step - 1))}
						>
							◀
						</button>
						<input
							type="range"
							min="0"
							max={events.length}
							bind:value={step}
							class="flex-1 accent-indigo-500"
						/>
						<button
							class="rounded px-2 py-0.5 text-slate-400 hover:bg-slate-800"
							onclick={() => (step = Math.min(events.length, step + 1))}
						>
							▶
						</button>
						<span class="w-14 shrink-0 text-right font-mono text-xs text-slate-400">
							{step}/{events.length}
						</span>
					</div>

					<div bind:this={traceEl} class="min-h-0 flex-1 overflow-auto p-2 font-mono text-sm">
						{#each events as event, i (i)}
							{@const isCurrent = i === step - 1}
							<div
								data-current={isCurrent ? '' : undefined}
								class="flex items-center gap-3 rounded px-2 py-0.5 {i >= step
									? 'opacity-30'
									: ''} {isCurrent ? 'bg-slate-800' : ''}"
							>
								<span class="w-12 shrink-0 text-right text-xs text-slate-600"> {event.op} </span>
								<span class="w-9 shrink-0 text-xs {kindColor[event.kind]}"> {badge(event)} </span>
								<span class="text-slate-200" style="padding-left: {event.depth * 1.1}rem">
									{detail(event)}
								</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="p-4 text-sm text-slate-500">Aucun événement observable.</div>
				{/if}
			{:else}
				<div class="p-4 text-sm text-slate-500">Chargement du moteur…</div>
			{/if}
		</section>
	</main>
</div>
