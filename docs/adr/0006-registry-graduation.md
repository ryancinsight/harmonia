# ADR 0006: Graduate the coupling crate when two downstream packages can consume it

- Status: Proposed
- Date: 2026-11-22
- Class: [arch]

## Context

Harmonia 0.1.4 is source-distributed under [ADR 0005](0005-release-scope-1-0.md). Its manifest selects Athena Core and Horae Time through Git. Atlas's crate-order audit reports a required dependency from both ares-coupling and cfd-2d to this unpublished crate. The existing Git contract provides a compatibility baseline.

## Proposed decision

Keep Git distribution in force during development. Seek approval for a first registry release of the existing coupling crate after M-contract-proof and a real consumer demonstrate M-consumer-validation. Use the reviewed version appropriate to the public API diff. Normalize each dependency to the exact compatible registry cohort, including the public Athena types. The publication owner decides whether and when to enable publishing.

The alternatives are to retain a Git-only stack for these consumers or to move the coupling implementation into them. Git remains viable for source integration and leaves registry packaging unresolved. Moving the loop would duplicate the transaction and replay contract that Harmonia owns.

## Acceptance and handoff

Run a real partition with non-identity transfer, compare exchanged fields and residuals, and verify that a rejected window leaves all four caller-owned slices bit-identical to their inputs. Partition checkpoints govern replay between fixed-point evaluations. A partition can retain its last attempted internal state on return. Verify a clean staged package and compare against the Git API baseline. After authorized publication, bind the archive checksum to the candidate revision. From outside Atlas, resolve ares-coupling and cfd-2d separately against that archive and check their public coupling types agree. A missing registry edge, divergent type identity or failed consumer keeps the affected downstream release blocked.
