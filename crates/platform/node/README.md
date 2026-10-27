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

## Deployment monitoring

`NodeComposition::start` accepts the monitor error receiver returned by `DeploymentService::new`. Keep one deployment service alive with the Node composition, and pass the same `Arc` process manager and `HttpProbe` to both. Read background monitor failures with `next_deployment_monitor_error`; it returns `None` after the receiver closes. A production wiring example is in the API documentation for `NodeComposition::start`.
