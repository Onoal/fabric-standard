# Cloudflare Workers — Fabric Standard HTTP

This example runs a normal `fabric_standard_http::App` on Cloudflare Workers.

## Prerequisites

- Rust
- npm / npx
- the `wasm32-unknown-unknown` Rust target

Install the target if needed:

```bash
rustup target add wasm32-unknown-unknown
```

## Develop Locally

From this directory:

```bash
npx wrangler dev
```

Wrangler runs the configured Rust build command, starts the local Workers
runtime, and rebuilds when source files change.

Wrangler usually serves the Worker at:

```text
http://localhost:8787
```

Use the address printed by Wrangler if it differs.

Try:

```bash
curl http://localhost:8787/
curl http://localhost:8787/users/42
curl http://localhost:8787/health
curl http://localhost:8787/missing
```

## Shape

The Worker entrypoint receives Cloudflare's `worker::Request`, `worker::Env`,
and `worker::Context`, then hands the request to the Fabric Standard Cloudflare
runtime adapter.

The Standard HTTP application remains ordinary route-handler code:

- no Workers Router;
- no Cloudflare types in route handlers;
- no `Env` or `Context` in Standard HTTP application code;
- no Cloudflare KV, R2, D1, Queues, or Durable Objects.

`worker-build` is still used, but Wrangler owns it through `wrangler.toml` as
the custom Rust/Wasm build command. Normal local development starts with
`npx wrangler dev`.
