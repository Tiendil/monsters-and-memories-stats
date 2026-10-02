# Collector and dashboard architecture proposal

## Goal of the document

This document describes the proposed technical design for the historical metrics application.

## Scope

The proposal covers component boundaries and the flow from the public metrics source to the GitHub-hosted dashboard.
Detailed UI styling and private implementation structure are outside its scope.

## Approval status

All decisions D1–D7 below are **pending user approval**.
The documentation and Donna/Depmesh foundation implement the requested planning process only.
Application implementation and deployment have not started.
The behavior proposed in [requirements.md](requirements.md) is part of this review, especially metric interpretation and source-change detection.

### D1: Components and layout

The proposal uses one Cargo workspace with `mnm-stats-model`, `mnm-stats-collector`, and `mnm-stats-dashboard` packages.
Project package names MUST use the `mnm-stats-` prefix to distinguish them from external dependencies and reduce naming ambiguity.
Each package MUST reside in `crates/<package-name>/` so directory names match their Cargo package names.
Both applications depend on the model library for the history data contracts and domain validation.
Separate application packages keep collector-specific HTTP and HTML parsing dependencies out of the WASM build.

### D2: Acquisition

The proposal uses a synchronous Rust HTTP request and DOM parser for the public metrics page, conditional on browser/HTTP equivalence verification.
An HTTP collector avoids a browser runtime; its suitability depends on the unresolved [source verification](../docs/source-analysis.md#browser-verification).
Additional usable LiveView data would require revising this decision.

### D3: History

The proposal retains one append-only logical history in `data/history.jsonl` on the default branch, with at most one snapshot per UTC hour.
Each snapshot occupies one JSONL line with its own schema version, so a new observation adds one record to the Git diff.
The frontend build validates and embeds the complete JSONL history, and the dashboard provides a JSON export of that embedded history.
The file and Git history will grow, with partitioning deferred until needed and separately approved.

### D4: Frontend

The proposal uses Leptos client-side rendering, built by Trunk, with Plotters generating SVG charts in Rust.
This supports static GitHub Pages deployment with no backend or handwritten JavaScript chart logic; range controls and exact-value inspection are supplied by the Rust UI.
The UI MUST support the time-frame and entity comparisons defined by [R17](requirements.md#r17-plot-comparisons), including more than two series per comparison.
The complete history MUST be embedded in the compiled frontend and used for both visualization and JSON download, as required by [R18](requirements.md#r18-embedded-history) and [R19](requirements.md#r19-history-download).
This keeps the displayed data and downloaded history tied to the same frontend build.

### D5: Automation and notifications

The proposal uses GitHub Actions for hourly collection and Pages deployment, with native Actions failure notifications configured and verified by the maintainer.
This uses the existing GitHub infrastructure for alerts; delivery depends on the maintainer's notification settings and schedule ownership.

### D6: Source changes and metric semantics

The proposal follows the metric interpretation and collection contracts in [requirements.md](requirements.md).
These detect source-contract changes while accepting harmless styling changes and changes to the dynamically discovered server set.
Changes to required metric labels or the approved zone roster require a reviewed parser/fixture update.

### D7: Development

The proposal uses native Rust tooling locally and on GitHub-hosted Linux runners, with minimal Donna/Depmesh configuration.
This avoids container and external journal infrastructure; Docker remains an alternative the user can choose before scaffolding.

## Components

The proposed data flow is:

```text
Public metrics page
    -> Rust collector in hourly GitHub Actions
    -> data/history.jsonl committed to the default branch
    -> frontend build validates JSONL and embeds the complete history in WASM
    -> GitHub Actions publishes the Pages artifact
    -> Leptos dashboard uses shared Rust snapshot values and exports them as history.json on download
```

The shared `mnm-stats-model` library MUST own the snapshot data types, the JSONL storage and JSON export contracts, and domain validation.
It MUST also own serialization of shared history values into the JSON export so all consumers use the same mapping.
Both applications MUST depend on this workspace-local library and use its contract as follows:

- The collector constructs and validates shared snapshots before serializing versioned JSONL records.
- The dashboard build uses the shared library to read and validate JSONL history before embedding its observations.
- The dashboard uses the embedded history through shared snapshot types and the shared JSON serializer for downloads.

The shared library MUST build for both the native collector and WebAssembly dashboard targets.
Calculations needed by both applications MAY be owned by the shared library.
Calculations used only by the dashboard MUST remain in the dashboard package so presentation-specific analysis stays outside the shared data contract.

The `mnm-stats-collector` package MUST own:

- fetching source data.
- parsing source data.
- validating a source observation.
- safely updating history.

The `mnm-stats-dashboard` package MUST own:

- rendering and user interaction.
- filtering.
- ratio and correlation calculations.
- comparison alignment.
- downloading the complete embedded history as JSON.

To keep the runtime limited to the requested GitHub capabilities, the architecture MUST NOT introduce:

- an application server.
- a database.
- login.
- a separate API.
- a generic ingestion framework.

## Dependencies

These library choices require approval as part of D1, D2, and D4.
Exact compatible versions MUST be selected and locked during the scaffold step; this proposal does not authorize arbitrary dependency additions later.

- `serde`, `serde_json` — shared typed JSONL records and JSON export serialization.
- `chrono` — UTC timestamps and date/range calculations.
- `reqwest` with blocking support and Rustls — collector HTTPS request, explicit timeout, and status handling.
- `scraper` — HTML parsing and scoped DOM selectors; no regex-only HTML extraction.
- `leptos` with CSR — Rust browser UI and reactive controls.
- `plotters` with SVG support — existing Rust chart axes, labels, and series rendering.
- Trunk — Rust/WASM asset builds and local preview.

The application MUST contain no handwritten JavaScript/TypeScript logic; generated WASM glue and third-party build/runtime internals are acceptable supporting artifacts.
The following supporting formats are needed for packaging and orchestration:

- HTML.
- CSS.
- workflow YAML.
- shell commands.

Leptos documents [static CSR deployment, including GitHub Pages](https://book.leptos.dev/deployment/csr.html).
Plotters provides [SVG drawing support](https://docs.rs/plotters/latest/plotters/), and Reqwest provides a [blocking client](https://docs.rs/reqwest/latest/reqwest/blocking/index.html).
These primary references were checked on 2026-10-02; the scaffold MUST prove the selected versions build together for the intended targets.

## Repository layout

This is the proposed ownership layout, not a claim that these implementation files already exist.

```text
AGENTS.md
README.md
Cargo.toml
Cargo.lock
rust-toolchain.toml
crates/
  mnm-stats-model/        # shared data contract and validation
  mnm-stats-collector/    # native CLI and parser fixture tests
    tests/fixtures/      # sanitized public source fixtures
  mnm-stats-dashboard/    # calculations, Leptos source, HTML/CSS, and Trunk configuration
data/
  history.jsonl          # the committed source of collected history
docs/
  source-analysis.md     # dated source investigation evidence
.github/workflows/
  code-checks.yml        # orchestrates shared project checks
  collect.yml            # hourly/manual collection and data commit
  pages.yml              # orchestrates dashboard build and Pages deployment
specs/                   # project specifications, indexed by specs/intro.md
workflows/
  polish.donna.md        # agent check sequence and repair actions
.agents/skills/          # project agent skills
donna.toml
depmesh.toml
bin/                     # shared project commands and agent runner
```

Generated build output and Donna session state MUST be ignored by Git.
Dependencies and build tooling MUST be pinned reproducibly, with `Cargo.lock` committed and CI using locked resolution.
Application files MUST gain appropriate `governed_by` and reverse `governs` rules when introduced.

## Storage contract

The [project dictionary](dictionary.md) defines observations and their stored snapshots.
Repository history uses JSONL: separate versioned snapshot objects, one per line, without an enclosing root object or `snapshots` array.
The dashboard uses history embedded during its build and provides an ordinary JSON document for download.
The internal embedded representation is independent of the JSONL storage and JSON export formats.

### Repository history

`data/history.jsonl` MUST use UTF-8 JSON Lines, with one complete JSON object per line and records ordered by collection timestamp.
Each record MUST contain `schema_version: 1` alongside the snapshot fields defined below.
An empty file MUST represent a valid history with no observations.
The collector MUST write compact records separated and terminated by newlines so each observation occupies one line.

### Snapshot fields

Each snapshot payload has the following fields:

- `observed_at` — actual UTC collection timestamp in RFC 3339 form.
- `active_subscriptions` — published global subscription count.
- `servers` — server records with the fields listed below.
- `servers[].starting_zones` — records with source `id`, source `name`, and `online` count.

Each server record has:

- `id` — source identity.
- `name` — source display name.
- `daily_active` — published daily active count.
- `monthly_active` — published monthly active count.
- `online` — published online population.
- `starting_zones` — the zone records described above.

### Dashboard history

The frontend build MUST embed all validated observations for use through the shared Rust snapshot types.
Embedding MUST preserve every snapshot field and value without filtering, aggregation, or resampling.
The internal embedded representation is an implementation choice; an intermediate JSON document and JSON parsing during dashboard initialization are not required.

**Example:** The build can generate Rust data declarations from validated JSONL records and compile them into WASM.
The dashboard then uses those values directly for charts and serializes the complete history to JSON when the user requests a download.

#### JSON export

The downloadable JSON document MUST have `schema_version: 1` at its root and a `snapshots` array containing all snapshot payloads in collection order.
The per-record `schema_version` field MUST be represented by the document's root version rather than repeated in each array entry.
Export MUST preserve every snapshot field and value without filtering, aggregation, or resampling.
An empty embedded history MUST produce `{"schema_version":1,"snapshots":[]}` when downloaded.
The document MUST be produced from the complete history embedded in the loaded frontend and offered for download as `history.json`.
The JSON export schema defines the download format only.
Generated embedded data and JSON exports MUST NOT be committed as a second source of history.

**Example:** Two JSONL lines each contain an observation and their own `schema_version: 1`.
The build embeds both observations, and a download produces one JSON object with a root `schema_version: 1` and two entries in `snapshots`; those entries contain the original snapshot fields without repeating `schema_version`.
Observation order, timestamps, and metric values remain unchanged.
The download contains that entire document even when only one observation is in the selected chart range.

### Validation

Counts MUST be nonnegative integers and identities MUST be unique within their scope.
Server and zone records MUST have deterministic ordering so unchanged values do not produce reordering-only data changes.
The following values MUST be derived rather than duplicated in storage:

- global online totals.
- starting-zone totals.
- cross-server activity sums.
- ratios.
- correlations.

The parser MUST validate published online and starting-zone totals before discarding that redundancy.
Validation MUST reject the following before a file is changed:

- unknown schema versions.
- malformed or incomplete JSONL records, including blank lines.
- invalid timestamps.
- observations outside chronological order.
- duplicate hours.
- corrupt counts.

New source metrics such as WAU MUST require an explicitly reviewed schema change; they MUST NOT retroactively appear as zeros in old snapshots.

### Collection updates

The current proposal retains hourly observations.
The browser/source verification gate MUST resolve whether genuine historical points are available before this storage choice is implemented.
The project MUST NOT ship a collector that ignores verified, materially richer public data merely to preserve this proposal.

After validation, the collector MUST update history atomically so an interrupted write cannot leave partially written history.
Existing observations MUST remain unchanged.
Adding an observation MUST preserve existing JSONL record lines and add exactly one new record.
A fetch or parse failure MUST leave the original file byte-for-byte intact.
The collector MUST support a fixture input or equivalent offline test entry point so parser failures can be verified without depending on the live page.
Collector HTTP behavior MUST be testable using local responses or simulated failures, with no fallback to the public source during tests.
The source contract MUST be covered by sanitized representative fixtures, including valid changes in the discovered server set.
Server membership MUST NOT be fixed by parser configuration or the storage schema.
Changing required metric labels or the approved zone roster requires review and regression tests.

## Testing

Application tests MUST follow [tests.md](tests.md), including its required coverage and prohibition on internet requests.
Tests MUST be implemented alongside the corresponding features and run through the project's local checks and CI.
Test runs MUST use local data and MUST NOT invoke the production collector against the public statistics page.

## GitHub automation

The following is the proposed D5 design.

GitHub Actions workflows MUST own GitHub event handling and job orchestration, invoking the shared project commands defined in [development.md](development.md#project-commands).
Tool invocations and their options MUST remain in those commands or the underlying tool configuration so local and automated runs share the same implementation.

### Code checks

`code-checks.yml` MUST run the shared verification commands for pull requests and support manual execution.
Its release WASM build MUST verify buildability through the same build command used for Pages.
Artifact publication MUST belong to `pages.yml`.
Pull-request CI MUST have no production write permissions or deployment behavior.

### Collection

`collect.yml` MUST support an hourly UTC `schedule` and `workflow_dispatch` for manual recovery/testing.
The schedule SHOULD avoid the start of the hour to reduce exposure to GitHub's documented load peak; a different collection-time requirement MAY justify another schedule.
GitHub schedules are best-effort and may be delayed or dropped.
The workflow MUST live on the default branch and collect against its latest state.
Overlapping collection runs MUST preserve existing observations and MUST NOT cancel a writer while it is updating history.
It MUST commit only the validated history file using the repository's `GITHUB_TOKEN` and `contents: write` permission.
It MUST skip a commit when the file is unchanged, and MUST NOT force-push or overwrite unrelated concurrent changes.
A push conflict MUST either be resolved without losing samples or fail visibly for a later retry.
Repository rules that disallow the bot's data commit MUST be identified during deployment setup rather than bypassed.
The collection workflow MUST publish any resulting history commit before reporting success.

### Pages delivery

`pages.yml` MUST support approved code pushes to the default branch, completion of the collection workflow, and manual dispatch.
For collection, it MUST use GitHub's [`workflow_run`](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run) event with activity type `completed`, selecting the collection workflow by its declared name and restricting it to the default branch.
The Pages workflow file MUST exist on the default branch for this trigger to operate.
Collection-triggered build and deployment jobs MUST run only when the collection workflow's conclusion is `success`.
They MUST check out the latest default-branch revision after collection completes so the build includes the published history commit.
Pages deployment MUST NOT rely on a bot data push triggering another push workflow: [GitHub documents that `GITHUB_TOKEN` pushes do not do so](https://docs.github.com/en/actions/concepts/security/github_token).

**Example:** Collection starts from commit A and publishes history in commit B.
Its successful completion triggers Pages, which checks out the updated default branch containing B and compiles that history into the frontend.

Concurrent deployments MUST NOT allow an older data build to replace a newer published build.
The frontend build MUST read and validate `data/history.jsonl` using the shared model library and embed all observations in the compiled WASM artifact.
Invalid history MUST fail the build without replacing the last valid published dashboard.
A history change MUST invalidate cached frontend output that would otherwise retain older embedded data.
The dashboard MUST use that embedded history for visualization and JSON export without a separate metrics-data request.
The build MUST support the GitHub Pages repository subpath as well as local preview.
Pages publication MUST use the official artifact/deployment actions and the required `pages: write` and `id-token: write` permissions only for deployment.

### Operational setup

The maintainer MUST configure and verify native Actions failure notifications before unattended collection is considered ready.
GitHub's [workflow notification rules](https://docs.github.com/en/actions/concepts/workflows-and-actions/notifications-for-workflow-runs) tie scheduled notifications to the schedule editor or the person who re-enables the schedule; a failing check alone does not prove delivery to every repository watcher.
The README MUST document who receives these alerts and how to recover a failed or disabled collector.
Repeated source failures MUST keep runs failing until a reviewed fix is in place, and the dashboard MUST expose stale data as specified in [requirements.md](requirements.md).
An issue-based alert can be proposed later if native notifications prove insufficient; it is not silently included in this design.

GitHub documents [schedule delays and automatic disabling after repository inactivity](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule).
The README MUST describe these operational limits and manual recovery.
The Pages setup MUST follow the [custom workflow requirements](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
These references were checked on 2026-10-02.
