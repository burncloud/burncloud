Workflow documentation moved to ../README.md.

This file previously contained the single line "@ readme". The overview, trigger coverage,
reproducibility limits, known-failing checks, conventions and the list of planned renames now live
in .github/README.md; a second, stale copy here would be a second source of truth.

The file itself stays until the workflows are renamed, because deleting it now while the directory
layout is still mid-migration would leave the workflows/ directory without a pointer.

The matrix of which crate is verified by which job is in ../test-plan/coverage-matrix.md.
