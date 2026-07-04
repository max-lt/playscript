## General

- Use `bunx` instead of `npx`
- Use `bun check` to run svelte-check, `bun format` to run prettier
- Leave an empty line before and after code blocks (if, for, try, etc.)

## SvelteKit version

Current SvelteKit (`@sveltejs/kit` 2.6x+, `vite-plugin-svelte` 7, Vite 8) configures the adapter **inside `vite.config.ts`**. There is **no `svelte.config.js`** — do not create one.

## Cloudflare adapter

- Deployed to Cloudflare Workers with static assets via `@sveltejs/adapter-cloudflare`. The adapter emits the Worker + assets to `.svelte-kit/cloudflare/`.
- Runtime config (worker entry, assets binding, compatibility flags, bindings) lives in `wrangler.jsonc`.
- `bun run deploy` = `vite build && wrangler deploy`.
- After adding or changing a binding in `wrangler.jsonc`, run `bun run cf-typegen` to regenerate the `Env` type in `worker-configuration.d.ts`.
- Server-side bindings are reached via `platform.env`. Only `env` is typed in `src/app.d.ts` (via a triple-slash reference to `worker-configuration.d.ts`); `cf`/`ctx`/`caches` come from the adapter's ambient types.

## Cloudflare D1 (database)

D1 uses standard prepared statements with `?` placeholders and `.bind()`. **No explicit type casts are needed** (unlike the OpenWorkers Postgres driver).

```ts
const { results } = await platform.env.DB.prepare('SELECT id, name FROM users WHERE id = ?')
	.bind(id)
	.all();
```

## @types/node

SvelteKit hardcodes `types: ['node']` in its generated tsconfig, so `@types/node` is a required dev dependency for type-checking config files and `node:*` imports. It is dev-only and never bundled into the Worker.
