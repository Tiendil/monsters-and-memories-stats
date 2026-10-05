# Collector and dashboard architecture

## Goal of the document

This document describes the technical design for the historical metrics application.

## Scope

The architecture covers component boundaries and the flow from the public metrics source to the GitHub-hosted dashboard.
Detailed UI styling and private implementation structure are outside its scope.

## Design

### Components and layout

The project uses one Cargo workspace with `mnm-stats-model`, `mnm-stats-collector`, and `mnm-stats-dashboard` packages.
Project package names MUST use the `mnm-stats-` prefix to distinguish them from external dependencies and reduce naming ambiguity.
Each package MUST reside in `mnm-stats/<package-name>/` so directory names match their Cargo package names.
Both applications depend on the model library for the history data contracts and domain validation.
Separate application packages keep collector-specific HTTP, WebSocket, and HTML parsing dependencies out of the WASM build.

### Acquisition

The collector uses a Rust HTTP client to initialize the public page session, then a Rust WebSocket client to receive its LiveView metrics updates.
The WebSocket connection provides the completed metric values required by the collection contract.
The [source analysis](../docs/source-analysis.md#direct-access-verification) describes the upstream protocol.
The initialization response supplies the anonymous session cookie, CSRF token, and LiveView session information needed to join the page's WebSocket channel.
These session values MUST be obtained from the source rather than hardcoded or committed.
The collector MUST parse the completed metric values from the LiveView rendering updates using the reviewed source contract.
It MUST NOT require a browser runtime for collection.

The collector MUST send the same identifying `User-Agent` on HTTP initialization requests and WebSocket upgrade requests.
Its product identifier MUST be `mnm-stats-collector/<revision>`, where `<revision>` is the first 12 characters of the build's commit hash, or `dev` when revision metadata is unavailable.
The header comment MUST include `+https://github.com/Tiendil/monsters-and-memories-stats` and MUST include `branch=<branch>` when branch metadata is available, with valid HTTP comment escaping.
Revision and branch metadata MUST be supplied at build time; changing those inputs MUST update the header even when build caches are reused.
CI MUST supply the revision of the checked-out source and its branch, rather than assume the workflow trigger's revision matches the checkout.
Local builds without metadata MUST use `dev` and omit the branch.

Collection MUST wait for completion of the asynchronous metrics load for every discovered server before accepting its values.
A successful WebSocket join, a fixed delay, or nonzero activity counts alone MUST NOT establish readiness.
HTTP initialization, WebSocket connection, and readiness waits MUST have finite timeouts and fail collection without changing history when readiness cannot be established.
The collector MUST close the source connection after success or failure.
Session initialization, protocol handling, and parsing MUST remain Rust application logic.

### History

The collector retains one append-only logical history in `data/history.jsonl` on the default branch, with at most one snapshot per UTC hour.
Each snapshot occupies one JSONL line with its own schema version, so a new observation adds one record to the Git diff.
Each snapshot MUST contain only the current reported state selected under the [collection contract](requirements.md#history-and-collection).
Source-provided rolling history MUST NOT be stored in snapshots or imported as additional observations.
The frontend build validates and embeds the complete JSONL history, and the dashboard provides a JSON export of that embedded history.

### Frontend

The dashboard uses Leptos client-side rendering, built by Trunk, with Plotly.rs constructing figures in Rust and Plotly.js rendering interactive SVG charts in the browser.
Rust owns metric calculations, time alignment, gap detection, series selection, and exact hover text; Plotly owns chart layout and native hover labels.
The frontend MUST load a pinned Plotly.js basic bundle compatible with the Rust wrapper from Plotly's official CDN.
The versioned CDN URL MUST be owned by the frontend HTML and work at both root and Pages subpath URLs.
Browser tests MUST load the same CDN script as ordinary builds and previews.
The browser bundle MUST NOT be committed to the repository or packaged with the site.
Rust browser bindings MUST initialize charts after their DOM nodes mount, report initialization errors, and purge chart resources on removal.
Charts MUST resize with the viewport, with horizontal scrolling confined to chart regions where needed under [dashboard-design.md](dashboard-design.md#responsive-and-accessible-interaction).
Chart height MUST accommodate simultaneous hover labels as comparison series are added; the renderer MUST NOT silently drop series details to fit a fixed chart height.
The shared range controls govern the visible interval; chart-local zoom and the Plotly toolbar are disabled.
This supports static GitHub Pages deployment with no backend or handwritten JavaScript application logic; the Rust UI provides complete-history JSON downloads alongside native hover labels.
The UI MUST support the time-frame and entity comparisons defined by [R17](requirements.md#r17-plot-comparisons), including more than two series per comparison.
The complete history MUST be embedded in the compiled frontend and used for both visualization and JSON download, as required by [R18](requirements.md#r18-embedded-history) and [R19](requirements.md#r19-history-download).
This keeps the displayed data and downloaded history tied to the same frontend build.

### Design tokens

The dashboard owns the presentation contract defined in [design-tokens.md](design-tokens.md), including the machine-readable token artifact in the dashboard crate.
The token artifact MUST select palette and shared style values from Tailwind's pinned default theme through the import convention in the token specification.
Rust build tooling MUST resolve the theme references, validate the artifact, and generate CSS custom properties, typed values for Plotly figure configuration, and a self-contained DTCG document from one resolved token set.
This keeps browser styles and chart presentation consistent without introducing a separate runtime styling service.
Generation MUST use the project's supported DTCG profile and existing Cargo/Trunk build flow.
Handwritten CSS MUST retain selectors and layout rules, with token references supplying reusable presentation values.
Generated CSS MUST be embedded in the compiled frontend and applied when the WASM application mounts, without a separate runtime stylesheet request.
Build-time resolution MUST supply token values in CSS contexts that cannot use custom properties.
Generated output MUST remain in ignored build locations.
The preview MUST watch the selected token artifact and rebuild both representations when it changes, including when an override selects a file outside the application crate.

### Automation and notifications

The project uses GitHub Actions for hourly collection and Pages deployment, with native Actions failure notifications configured and verified by the maintainer.
This uses the existing GitHub infrastructure for alerts; delivery depends on the maintainer's notification settings and schedule ownership.

### Source changes and metric semantics

The collector and dashboard follow the metric interpretation and collection contracts in [requirements.md](requirements.md).
These detect source-contract changes while accepting harmless styling changes and changes to the dynamically discovered server set.
Starting-zone membership is discovered separately for each server and may change between observations.
Changes to required metric labels or row structure require a reviewed parser/fixture update.

### Development

Development builds, checks, and local preview MUST use the shared Docker Compose development image through project commands.
The image MUST provide the pinned Rust and WebAssembly toolchain and the tools needed by existing browser tests so host installations do not change their behavior.
Development tool installation MUST be a separate setup operation.
Donna and Depmesh run on the host and orchestrate those commands.

The dashboard preview MUST run in Compose and expose the same built application to the user's browser and the agent's Playwright MCP browser.
The MCP server MUST run in a temporary stdio container on the project's Compose network, with browser artifacts retained under ignored `.session/playwright/`.
Its image MUST be pinned, and project-scoped Codex configuration MUST declare its launcher.
The preview MUST publish to host loopback by default; the browser container MUST reach it through Compose service discovery.

The preview launcher MUST support an explicit history path and a synthetic demo mode.
It MUST identify its selected history and observation count, rebuild when that history changes, and keep preview output separate from release and test output.
Demo generation MUST remain Rust tooling, support reproducible input timestamps, and include recent observations visible in the default range.
The UI MUST identify demo data as synthetic.
Preview and test operations MUST NOT collect source metrics or modify committed history.
These development services do not form part of the deployed static dashboard.

## Components

The data flow is:

```text
Public metrics page and its connected LiveView updates
    -> Rust HTTP and WebSocket collector in hourly GitHub Actions
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

The application uses the following libraries and build tools:

- `serde`, `serde_json` — shared typed JSONL records and JSON export serialization.
- `chrono` — UTC timestamps and date/range calculations.
- `reqwest` with blocking support and Rustls — HTTP initialization, session cookies, explicit timeout, and status handling.
- `tungstenite` with Rustls — synchronous WebSocket connection and message transport for the LiveView session.
- `scraper` — HTML parsing and scoped DOM selectors; no regex-only HTML extraction.
- `leptos` with CSR — Rust browser UI and reactive controls.
- `plotly` (Plotly.rs) — typed Rust figure configuration and serialization.
- Plotly.js basic bundle — browser rendering and hover interaction, loaded from the CDN at a version supported by the Rust wrapper.
- Trunk — Rust/WASM asset builds and local preview.
- Tailwind CSS default theme — pinned upstream palette and shared style values, installed in the development image for build-time token resolution.
- `cssparser` and `csscolorparser` — build-time CSS theme parsing and color conversion for the token adapter.

The application MUST contain no handwritten JavaScript/TypeScript logic; generated WASM glue and third-party build/runtime internals are acceptable supporting artifacts.
The following supporting formats are needed for packaging and orchestration:

- HTML.
- CSS.
- workflow YAML.
- shell commands.

Leptos documents [static CSR deployment, including GitHub Pages](https://book.leptos.dev/deployment/csr.html).
Plotly.rs provides [typed chart configuration](https://docs.rs/plotly/0.14.1/plotly/).
Plotly documents [loading its browser engine from the CDN](https://plotly.com/javascript/getting-started/).
Reqwest provides a [blocking HTTP client](https://docs.rs/reqwest/latest/reqwest/blocking/index.html), and Tungstenite provides [WebSocket transport with TLS support](https://docs.rs/tungstenite/latest/tungstenite/).
Selected dependency versions MUST build together for the native collector and WebAssembly dashboard targets.

## Repository layout

Repository paths and component ownership are:

```text
AGENTS.md
README.md
Cargo.toml
Cargo.lock
rust-toolchain.toml
docker-compose.yml       # shared development, preview, and browser services
docker/dev/              # development image
.codex/                  # project browser MCP configuration
mnm-stats/
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

Each metric field MUST contain one current-state value, not a historical series.
When the collection contract permits selecting the final source-history point as the last known state, only its metric value MUST enter the snapshot.
`observed_at` MUST remain the actual collection timestamp, not that point's source timestamp.

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

New source metrics MUST require an explicitly reviewed schema change; they MUST NOT retroactively appear as zeros in old snapshots.

### Collection updates

The design retains hourly current-state observations without source-history backfill, overlap resolution, or source-revision records.

After validation, the collector MUST update history atomically so an interrupted write cannot leave partially written history.
Existing observations MUST remain unchanged.
Adding an observation MUST preserve existing JSONL record lines and add exactly one new record.
A fetch or parse failure MUST leave the original file byte-for-byte intact.
The collector MUST support a fixture input or equivalent offline test entry point so parser failures can be verified without depending on the live page.
Collector HTTP, WebSocket, and readiness behavior MUST be testable using local responses or simulated failures, with no fallback to the public source during tests.
The source contract MUST be covered by sanitized representative fixtures, including valid changes in the discovered server set.
Server membership and per-server starting-zone membership MUST NOT be fixed by parser configuration or the storage schema.
Changing required metric labels or row structure requires review and regression tests.

## Testing

Application tests MUST follow [tests.md](tests.md), including its required coverage and prohibition on contacting the original statistics service.
Tests MUST be implemented alongside the corresponding features and run through the project's local checks and CI.
Test runs MUST use local data and MUST NOT invoke the production collector against the public statistics page.

## GitHub automation

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
Collector commits MUST use the subject `collector: record snapshot at <observed_at>`, where `<observed_at>` is the latest snapshot's stored collection timestamp, formatted as RFC 3339 UTC with a `Z` suffix and any nonzero fractional seconds preserved.
The message MUST use the observation time rather than the workflow or commit execution time.
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
Repeated source failures MUST keep runs failing until a reviewed fix is in place.
The dashboard MUST continue to display the latest collection time from its embedded history as specified in [requirements.md](requirements.md).

GitHub documents [schedule delays and automatic disabling after repository inactivity](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule).
The README MUST describe these operational limits and manual recovery.
The Pages setup MUST follow the [custom workflow requirements](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
