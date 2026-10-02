# Monsters & Memories historical statistics

A planned Rust collector and static dashboard for the public [Monsters & Memories metrics](https://account.monstersandmemories.com/metrics), using GitHub Actions, repository JSONL storage, and GitHub Pages.

The repository contains specifications, development tooling, source investigation notes, and sanitized fixtures. The collector and dashboard are not implemented yet.

- [Specification index](specs/intro.md)
- [Requirements](specs/requirements.md)
- [Dated source analysis](docs/source-analysis.md)
- [Architecture](specs/architecture.md)
- [Development workflow](specs/development.md)
- [Test coverage and offline execution](specs/tests.md)
- [Shared terminology](specs/dictionary.md)
- [Specification conventions](specs/meta/general.md)
- [Agent instructions](AGENTS.md)

The architecture defines hourly current-state observations, a shared Rust data model, and a Leptos frontend. Collection starts by obtaining the page's anonymous session information over HTTP, then receiving completed metrics through LiveView's WebSocket interface. Snapshots contain the current values from each collection, without importing the source's rolling chart history. A final chart point can supply a value only if its meaning as the last known state is established. The source does not provide WAU, deduplicated global activity, or per-server subscription counts.

Collected history will be committed as `data/history.jsonl`, with one versioned snapshot per line. The frontend build will validate and embed the complete history in WASM for use through shared Rust snapshot types. Its internal representation is an implementation choice. The dashboard will export the complete embedded history as `history.json` for download without a separate data request.

Each implementation step ends with user review and a commit before the next step begins. Task-specific plans and approval records stay under ignored `.session/`.

The documentation foundation uses the installed Donna and Depmesh CLIs:

```bash
donna -p llm status
donna -p llm list
depmesh -p llm relations
donna -p llm run @/workflows/polish.donna.md
```

Depmesh exposes only `governs` and `governed_by`. Polish runs the current deterministic checks, with focused repair actions on failure. Agents review specification consistency directly using Depmesh. Application checks will be added with the Rust implementation.

Application tests will use local fixtures and synthetic history without making internet requests. They will never request the public statistics page or download fixtures during a test run. Live source investigation and deployment verification are separate operational activities.

[Sanitized source fixtures](crates/mnm-stats-collector/tests/fixtures/README.md) preserve both stages of one public-page load for future parser tests. No application test suite exists yet.
