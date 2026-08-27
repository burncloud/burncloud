# BurnCloud UI Checkpoints

This directory contains source archives for restoring the `burncloud-ui` workspace.

## Checkpoint

- Created: 2026-08-26 16:03:37 +08:00
- Project: `E:\newProject\burncloud-ui`
- Branch: `codex/buyer-overview-rust`
- Git baseline: `e0ad2cc`
- Scope: current source and configuration, including the Rust `buyer/playground` migration

## Restore

1. Stop any running `burncloud-ui` development server.
2. Extract the checkpoint ZIP into a temporary directory.
3. Copy the extracted files over the project directory, preserving the project root.
4. Reinstall generated dependencies/build outputs if needed with `npm install` and the normal project commands.

The archive intentionally excludes `.git`, `node_modules`, `dist`, `logs`, `rust-ui/target`, and the archive directory itself because these are generated or environment-specific.
