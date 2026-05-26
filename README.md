# constitute-git

Rust CLI and library for Constitution source/version graph records.

`constitute-git` is the corporeal interface for source refs, branch/tag updates,
writer grants, import proof, and project/work-item link posture. It does not
build releases, host modules, own storage bytes, or make GitHub Project the
source primitive. Source objects and pack material are represented by storage
refs and evidence refs.

## Commands

```powershell
cargo run -- init --state target/source-graph-state.json
cargo run -- fixture graph
cargo run -- import snapshot --state target/source-graph-state.json --storage-object storage:object:pack-next
cargo run -- project link --state target/source-graph-state.json --project project:constituency --work-item work-item:git-project-hardening
cargo run -- ref update --state applied --branch main --from source:snapshot:old --to source:snapshot:new
cargo run -- ref reduce --branch main --from source:snapshot:parent --to source:snapshot:head
cargo run -- ref apply --state target/source-graph-state.json --from source:snapshot:head --to source:snapshot:new
cargo run -- store journal --input target/applied-source-ref.json --storage-object storage:object:store
cargo run -- store replay --input target/source-ref-store.json --expected-target source:ref:native-dev:repo:main
cargo run -- status --state target/source-graph-state.json
```

## Boundary

- Source graph owns source refs, snapshots, branch/tag refs, writer grants, and
  source import proof.
- Ref movement reduces against graph policy, writer grants, known snapshots,
  signature evidence, and witness evidence before it can become applied
  posture.
- Storage owns encrypted object and chunk fulfillment.
- Source snapshot imports create storage graph-edge evidence from source
  snapshots to storage object refs; they do not move a branch until a ref update
  is reduced and applied.
- Project/work-item links are source project operation posture. GitHub Project
  can be adapter evidence, but source graph truth only stores virtual project
  and work-item refs.
- Source graph fixtures and imports emit a host-fabric source-content-index
  member contribution so fabric can reduce host composition posture without
  owning source truth, branch movement, or storage bytes.
- Source-ref store journal/replay reduction lives at the source/version
  boundary. It consumes protocol applied-ref projection records, emits protocol
  source-ref-store journal/replay posture, and treats Storage refs as byte
  availability rather than source-state ownership.
- Build contracts and runners consume source refs; they do not become source
  truth.
- Future Git compatibility should adapt Git pack/ref expectations to these
  source records instead of making Git the semantic owner.
