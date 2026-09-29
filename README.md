# Fabric Standard

Fabric Standard is Onoal's opinionated developer layer above Fabric.

Fabric Core provides the universal construction grammar. Fabric Ecosystem provides reusable capabilities, realizations, and compositions. Fabric Standard provides domain developer worlds that make common application shapes pleasant without hiding their Fabric foundation.

The first implemented Standard domain is HTTP. It currently has a portable HTTP
application crate plus Native Fabric and Cloudflare Workers runtime crates:

- `onoal-fabric-standard-http` owns application semantics.
- `onoal-fabric-standard-http-runtime-native` runs those applications through
  the Native Fabric HTTP/TCP runtime.
- `onoal-fabric-standard-http-runtime-cloudflare` adapts Cloudflare Workers
  fetch invocations to the same HTTP application semantics.

## Verification

Run the local verification sequence from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Dependency Sources

Normal clone/build uses committed registry and Git dependency sources. Local
cross-repository development overrides are developer-local and must not be
committed. If a developer needs to test unpublished lower-layer changes, use a
local Cargo override outside the repository with placeholder paths such as:

```toml
[patch.crates-io]
onoal-fabric = { path = "<local-fabric-checkout>/sdk" }
```
