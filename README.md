# Fabric Standard

Fabric Standard is Onoal's opinionated developer layer above Fabric.

Fabric Core provides the universal construction grammar. Fabric Ecosystem provides reusable capabilities, realizations, and compositions. Fabric Standard provides domain developer worlds that make common application shapes pleasant without hiding their Fabric foundation.

The first implemented Standard domain is HTTP.

## Verification

Run the local verification sequence from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```
