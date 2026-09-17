# pewterdesk

Non-custodial, multi-venue crypto derivatives trading terminal (desktop-first,
web later). No backend — talks directly to exchange APIs from the user's own
machine. Hyperliquid at launch. TypeScript throughout except the Tauri shell.

## Architecture rule (read this before touching packages/)

`packages/core` defines the only cross-cutting contract: `ExchangeAdapter` plus
the domain types (`Order`, `Position`, `Market`, `OrderBook`, ...). Every
exchange package (`packages/exchange-hyperliquid`, and any future venue)
implements that interface and depends on `core` — never the other way around.
`packages/ui` and the two `apps/*` depend on `core` for types but should never
import a specific exchange package directly; that's what makes adding a second
venue a new adapter, not a rewrite of the UI.

## Layout

- `packages/core` — exchange-agnostic types + the `ExchangeAdapter` interface
- `packages/exchange-hyperliquid` — Hyperliquid adapter (REST/WS client, EIP-712 signing via viem)
- `packages/ui` — shared React components (order ticket, position table, chart wrapper, hotkeys)
- `apps/desktop` — the shipped app: Tauri (Rust shell) + this workspace's React frontend
- `apps/web` — v2, deferred. Same frontend stack, but browser CORS means most
  exchanges need a thin proxy in front of this build; the desktop app doesn't,
  since Tauri's Rust side makes requests natively.

## Commands

- `pnpm install` — from repo root, resolves the whole workspace
- `pnpm -r run typecheck` — typecheck every package
- `pnpm --filter @pewterdesk/web dev` — run the web app locally
- `pnpm --filter @pewterdesk/web run build` — production build of the web app
- `apps/desktop` has no Rust toolchain wired up yet — that's the next real piece of work, not yet started

## Security-sensitive code — extra care here

`packages/exchange-hyperliquid/src/signing.ts` (not yet written) is the
highest-stakes file in the repo: it's what turns a private key into a signed
exchange action. Keys are expected to come from the OS keychain via the Tauri
app layer — never read from disk, env vars, or logged. Treat any change here
(or to key storage generally) as needing extra scrutiny, and call it out
explicitly in PRs (see the org's `.github` PR template).

## Status

Early scaffold. Most `ExchangeAdapter` methods are unimplemented stubs.
`apps/desktop`'s Tauri shell doesn't exist yet — `apps/web` is the only app
that currently builds end-to-end (`vite build`).

## Related repos (same org)

- `.github` — org profile, issue/PR templates, shared CI workflow
- `pewterdesk-docs` — install guide, security policy (not yet created)
