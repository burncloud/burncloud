# Crate layout

BurnCloud production crates are organized by long-term ownership domain. Package names and public APIs remain unchanged by this directory migration.

| Domain | Packages |
| --- | --- |
| `kernel` | Reserved for a future minimal implementation; no empty directory is created |
| `identity` | User persistence, user service, token service |
| `supply` | Model and channel persistence and services |
| `traffic` | Router, router adapters, router persistence and logs |
| `commerce` | Billing persistence and service |
| `trust` | Inference trust and registration capability |
| `platform` | Configuration, lifecycle, observability, storage and templates |
| `interfaces` | CLI, HTTP server, web client, legacy common/service facades and end-to-end tests |

The migration changes physical paths only. It does not rename Cargo packages, alter public contracts, change database schemas or repair existing implementation issues. New work should use the domain paths in this tree rather than recreating the former technical-layer directories.
