# pewterdesk

Non-custodial, multi-venue crypto derivatives trading terminal for desktop and
web. No backend, no custody — talks directly to exchange APIs from your own
machine. Hyperliquid first. TypeScript + Tauri.

> **Status: early scaffold.** The workspace, build tooling, and package
> boundaries are in place; the trading functionality is not. `packages/core`
> does not yet define `ExchangeAdapter`, the Hyperliquid adapter is an empty
> module, and `apps/desktop` doesn't exist. `apps/web` is currently the only
> app that builds, and it renders a placeholder. Don't point this at a funded
> account — there's nothing there to point yet.

## Why it's built this way

Two decisions drive the rest of the design:

- **No backend.** Requests go from your machine straight to the exchange. There
  is no pewterdesk server to trust, to breach, or to go down. The desktop app
  can do this because Tauri's Rust side makes requests natively; the browser
  can't, which is why the web build is v2 (see below).
- **No custody.** Keys are expected to live in the OS keychain and be read
  through the Tauri app layer — never from disk, env vars, or logs. Nothing in
  the TypeScript packages should ever hold a key longer than a signing call
  needs it.

## Layout

```
packages/core                   exchange-agnostic types + the ExchangeAdapter interface
packages/exchange-hyperliquid   Hyperliquid adapter (REST/WS client, EIP-712 signing via viem)
packages/ui                     shared React components (order ticket, position table, chart, hotkeys)
apps/desktop                    the shipped app: Tauri shell + this frontend  (not created yet)
apps/web                        v2, deferred — same frontend stack, needs a proxy for CORS
```

### The one architectural rule

`packages/core` owns the only cross-cutting contract: the `ExchangeAdapter`
interface plus the domain types (`Order`, `Position`, `Market`, `OrderBook`,
…). Dependencies point one way:

```
apps/*  ─┬─→  packages/ui  ──→  packages/core  ←──  packages/exchange-hyperliquid
         └───────────────────────────────────┘
```

Every exchange package implements `ExchangeAdapter` and depends on `core` —
never the reverse. `packages/ui` and the apps depend on `core` for types and
must **not** import a specific exchange package. That constraint is the whole
point: adding a second venue should be a new adapter, not a rewrite of the UI.

To start a new venue, use the `add-exchange-adapter` scaffold in
[.claude/commands/](.claude/commands/add-exchange-adapter.md).

## Getting started

Requires Node ≥ 20 and pnpm 9 (the repo pins `pnpm@9.12.0` via `packageManager`).

```sh
pnpm install                              # from the repo root — resolves the whole workspace
pnpm --filter @pewterdesk/web dev         # run the web app locally
```

Copy [apps/web/.env.example](apps/web/.env.example) to `apps/web/.env` if you
need to override the Hyperliquid endpoints (e.g. to point at testnet).
**Non-secret config only** — Vite inlines these into client-side JS, so
anything you put there is effectively public. Keys never go in `.env`.

### Workspace scripts

Run from the repo root; each fans out across every package with `pnpm -r`.

| Command | What it does |
| --- | --- |
| `pnpm typecheck` | typecheck every package |
| `pnpm build` | build every package (`tsc`, plus `vite build` for the web app) |
| `pnpm test` | run each package's `vitest run` |
| `pnpm lint` | run each package's `eslint src` |
| `pnpm desktop:dev` | run the desktop app — **fails today**, `apps/desktop` doesn't exist |

## Security-sensitive code

`packages/exchange-hyperliquid/src/signing.ts` (not yet written) will be the
highest-stakes file in the repo: it turns a private key into a signed exchange
action. Changes there, or to key storage generally, need extra scrutiny and
should be called out explicitly in the PR description — see the org's `.github`
PR template.

There's a `security-review` checklist in
[.claude/commands/](.claude/commands/security-review.md) covering signing, key
storage, and exchange auth. Run through it before opening any PR that touches
those paths.

## Roadmap

1. Define `ExchangeAdapter` and the domain types in `packages/core`.
2. Implement the Hyperliquid adapter against it — REST/WS client, then signing.
3. Stand up the `apps/desktop` Tauri shell (Rust toolchain isn't wired up yet;
   this is the next real piece of work).
4. Build out `packages/ui` and wire it to the adapter.
5. Revisit `apps/web` once the desktop app ships — it needs a thin proxy in
   front of it for most venues, because browsers enforce CORS.

## Related repos

- `.github` — org profile, issue/PR templates, shared CI workflow
- `pewterdesk-docs` — install guide, security policy (not yet created)

## License

MIT — see [LICENSE](LICENSE).
