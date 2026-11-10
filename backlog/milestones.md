# Milestone ladder ([ADR 0005](../docs/adr/0005-release-scope-1-0.md))
<a id="M-contract-proof"></a> **M-contract-proof** (0.2.0) needs: (none) — contract proof.
- exit: `cargo nextest run --locked --all-features` passes (ERRPATHS-003/TRANSFER-004 oracles); `cargo test --locked --doc --all-features` passes; no open correctness/verification items on M-contract-proof.
<a id="M-consumer-validation"></a> **M-consumer-validation** (0.3.0) needs: M-contract-proof — real consumer.
- exit: ATLAS-HARMONIA-CONSUME-005 acceptance satisfied with named consumer evidence; `cargo clippy --locked --all-targets --all-features -- -D warnings` passes.
<a id="M-1.0"></a> **M-1.0** (1.0.0) needs: M-consumer-validation — release-ready (authority-gated).
- exit: `cargo semver-checks` (release gate) passes; publish dry-run readiness satisfied; `cargo doc --locked --no-deps --all-features` and `cargo run --locked --example coupled_decay` pass; no open correctness items.
