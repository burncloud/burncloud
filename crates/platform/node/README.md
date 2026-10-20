# BurnCloud Node Runtime Skeleton

This crate owns only local machine/runtime orchestration primitives.

## Reconcile rail

```text
Absent
  -> Resolving
  -> PreparingArtifact
  -> ArtifactReady
  -> PreparingRuntime
  -> Starting
  -> WaitingReady
  -> Ready
  -> Routable
```

`Starting` means a process is being spawned. `WaitingReady` means a process exists but readiness has not been proven. `Ready` requires explicit readiness evidence. A fixed sleep is not readiness evidence.

## Boundary

The reconciler may request actions, but it does not own the business implementation of model resolution, artifact selection, Provider selection, or routing policy. Those contracts and decisions must remain with their owning BurnCloud domains.

The Node runtime must not create a second HTTP server, listener, or ModelRouter. `Routable` means attachment to the existing BurnCloud routing path.

## Artifact preparation

`ArtifactPreparer::prepare` accepts a supported HTTP(S) `Url` and a required
normalized `Digest`. It downloads into the storage module's `models` directory,
verifies the file asynchronously, retries one time after a mismatch, and returns
an absolute verified path. A second mismatch returns a `Failed` receipt with an
empty path; download, read, cleanup, and lifecycle failures return
`ArtifactPrepareError`.
