# Monsters & Memories historical statistics

A Rust collector and static dashboard for the public [Monsters & Memories metrics](https://account.monstersandmemories.com/metrics), using repository JSONL storage and, when deployment is implemented, GitHub Actions and GitHub Pages.

The workspace provides the Rust HTTP/WebSocket collector, shared history model, local history-validation CLI, and a Leptos dashboard scaffold. The dashboard embeds validated history in WASM, shows the collection interval and stale-data status, and downloads the complete history as JSON. Plots, comparisons, and GitHub workflows are not implemented yet. The committed history starts empty.

- [Specification index](specs/intro.md)
- [Requirements](specs/requirements.md)
- [Architecture](specs/architecture.md)
- [Development workflow](specs/development.md)
- [Tests and offline execution](specs/tests.md)
- [Source analysis](docs/source-analysis.md)
- [Sanitized source fixtures](mnm-stats/mnm-stats-collector/tests/fixtures/README.md)
- [Agent instructions](AGENTS.md)

## Setup

Install Rust through rustup. `rust-toolchain.toml` pins Rust 1.98.0, rustfmt, Clippy, and the `wasm32-unknown-unknown` target. `Cargo.lock` pins application dependencies. From the repository root, install dependencies and build tools before running checks:

```bash
rustup show
cargo fetch --locked
cargo install trunk --version 0.21.14 --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

Prebuilt binaries from the [Trunk releases](https://github.com/trunk-rs/trunk/releases/tag/v0.21.14) and [wasm-bindgen releases](https://github.com/wasm-bindgen/wasm-bindgen/releases/tag/0.2.129) also work. Put the executables on `PATH`. The Trunk and wasm-bindgen versions are recorded in `mnm-stats/mnm-stats-dashboard/Trunk.toml`; update the latter alongside the matching Cargo dependency when upgrading.

Browser tests additionally need Python 3, Google Chrome, and a compatible ChromeDriver on `PATH`. Install ChromeDriver separately using the official [Chrome for Testing downloads](https://googlechromelabs.github.io/chrome-for-testing/). Set `CHROME` and `CHROMEDRIVER` to absolute executable paths if necessary. These tools serve local assets and drive browser tests; collection does not need a browser.

Dependency and browser installation may use the internet. Tests use local inputs and never download fixtures or contact the public statistics page. Missing browser tools cause an explicit failure.

## Local use

Run commands from the repository root:

```bash
./bin/validate-history.sh
./bin/validate-history.sh path/to/history.jsonl
./bin/serve-dashboard.sh
./bin/build-dashboard.sh
```

The validation command reads the given file without changing it. Local preview defaults to `http://127.0.0.1:8080/`. The release build writes static assets to ignored `dist/`.

To build for a repository subpath:

```bash
./bin/build-dashboard.sh --public-url /monsters-and-memories-stats/
```

The default build input is `data/history.jsonl`. For a local experiment, set `MNM_STATS_HISTORY` to an absolute path to another JSONL file. Build output and the `history.json` download preserve every observation. Changing the input file reruns build-time validation and embedding. Invalid history fails the build. The frontend makes no separate metrics-data request.

History contains hourly current-state snapshots only. Pre-collection history and missed intervals are unavailable. The source does not provide WAU, deduplicated global activity, or per-server subscription counts. Daily/monthly activity fields have unverified counting semantics; subscriptions are not assumed to represent unique people.

## Collection

Collect the current public state into the repository history:

```bash
./bin/collect.sh --history data/history.jsonl
```

Each run initializes an anonymous HTTP session and receives the page's LiveView WebSocket updates. The collector discovers server identities from the rendered page and validates required metrics, names, counts, the approved zone roster, and published totals. HTTP, WebSocket, readiness, and lock waits are bounded. A failed collection exits with a diagnostic and leaves history unchanged.

Readiness requires a complete server batch and replacement of every server's initial chart payload. This is evidence from the observed source protocol, not an explicit upstream completion flag. Zero activity and empty chart payloads are accepted when that replacement is observable. If a server's completed output is indistinguishable from its initial placeholder, collection fails rather than storing an uncertain zero. An upstream semantic change with unchanged observable structure cannot be detected reliably. See the [source analysis](docs/source-analysis.md#collector-verification) for the evidence and limitation.

Only current-state fields enter the snapshot; chart history is not imported. One new UTC hour adds one JSONL record. A previously recorded hour is a successful no-op, and missed hours remain gaps. A stable `.jsonl.lock` sidecar serializes local writers while `.jsonl.tmp` stages an atomic replacement. These files are ignored by Git; leave the lock file in place while collectors may be running. An interrupted staging file is overwritten by the next successful write.

For a local source fixture server, `--source URL` replaces the public URL. To exercise the full rendering, readiness, parsing, and storage path without network access, replay the committed protocol fixture into an isolated file:

```bash
mkdir -p .session/manual-replay
./bin/collect.sh --history .session/manual-replay/history.jsonl \
  --replay mnm-stats/mnm-stats-collector/tests/fixtures/liveview.json \
  --observed-at 2026-10-02T15:20:00Z
```

`--observed-at` is available only with fixture replay. Live collection always uses its actual collection time. A replay never falls back to the public source, and its synthetic timestamp must not be used for repository history.

## Checks

```bash
./bin/check-format.sh
./bin/check-lints.sh
./bin/test.sh
./bin/test-browser.sh
./bin/build-dashboard.sh
```

Native tests cover JSONL validation, source-value preservation, JSON export, source parsing, LiveView readiness, loopback HTTP/WebSocket acquisition, atomic history updates, overlapping writers, CLI success/failure, and the stale-data boundary. Browser tests build empty and synthetic histories, verify rendering and downloads at the site root and a repository subpath, inspect runtime requests, reuse build caches after history-only changes, and verify that an invalid build leaves the previous site usable. They run against loopback servers and keep scratch inputs, browser profiles, and logs under ignored `.session/tests/`. They never modify `data/history.jsonl` or captured source fixtures.

Donna runs these checks with focused repair actions:

```bash
donna -p llm status
donna -p llm list
depmesh -p llm relations
donna -p llm run @/workflows/polish.donna.md
```

Depmesh exposes only `governs` and `governed_by`. Agents use those relationships to review changes against the specifications. Each implementation step ends with user review and a commit before the next begins; task plans and approval records stay under `.session/`.

Scheduled GitHub collection, Pages deployment, and failure notifications are not configured yet; their setup belongs to the automation step.
