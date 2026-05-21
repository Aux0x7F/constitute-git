# constitute-git

Rust CLI and library for Constitution source/version graph records.

`constitute-git` is the corporeal interface for source refs, branch/tag updates,
writer grants, and import proof. It does not build releases, host modules, or
own storage bytes. Source objects and pack material are represented by storage
refs and evidence refs.

## Commands

```powershell
cargo run -- fixture graph
cargo run -- ref update --state applied --branch main --from source:snapshot:old --to source:snapshot:new
cargo run -- ref reduce --branch main --from source:snapshot:parent --to source:snapshot:head
cargo run -- status
```

## Boundary

- Source graph owns source refs, snapshots, branch/tag refs, writer grants, and
  source import proof.
- Ref movement reduces against graph policy, writer grants, known snapshots,
  signature evidence, and witness evidence before it can become applied
  posture.
- Storage owns encrypted object and chunk fulfillment.
- Build contracts and runners consume source refs; they do not become source
  truth.
- Future Git compatibility should adapt Git pack/ref expectations to these
  source records instead of making Git the semantic owner.
