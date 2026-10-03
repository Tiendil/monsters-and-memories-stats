# Application testing

## Goal of the document

This document defines required application test coverage, test data conventions, and offline execution rules.

## Scope

This specification covers project test code, test data, and test execution during development and CI.
Production collection and operational verification are outside its scope except for their separation from test execution.

## Offline execution

Tests MUST NOT make internet requests.
This includes optional or ignored tests when explicitly invoked.
Tests MUST NOT request the public statistics page or any other external service.
Test setup and teardown MUST follow the same restriction and MUST NOT download or refresh fixtures.

Local loopback and project-local Compose network connections MAY be used for fixture servers and dashboard previews.

Installing toolchains, dependencies, and browser binaries MAY use the internet; this is separate from test execution and MUST NOT fetch source metrics or refresh fixtures.
Missing local test inputs MUST produce an explicit failure instead of triggering a download.

**Example:** A collector acquisition test receives a saved response from a loopback server even when the public statistics page is unavailable.
Deleting that saved response causes a local missing-fixture failure, without a request to the public page.

## Test data

Parser tests MUST use committed source fixtures or locally constructed variants of those fixtures.
Captured fixtures MUST record their source and inspection date and exclude credentials or session-specific data.
Fixture updates MUST be reviewed separately from test execution.
Expected results MUST describe the fixture's contents without making its server identities a hardcoded production roster.

Historical test data MUST be synthetic or a committed fixture, with explicit timestamps and expected values.
Time-sensitive tests MUST control the current time so their results do not depend on the date they run.
Generated test inputs MUST be reproducible.
Test scratch files MUST use isolated locations under ignored `.session/` and MUST NOT modify committed history or fixtures.

## Test coverage

Tests MUST verify the behavioral and data contracts defined in [requirements.md](requirements.md) and [architecture.md](architecture.md).
Expected results MUST be derived independently of the function under test.
Test coverage MUST include successful behavior and relevant failure or boundary cases.

### Shared data model

Tests for `mnm-stats-model` MUST cover:

- Serialization and deserialization of versioned JSONL records and serialization of shared history values into JSON exports.
- Valid empty and populated histories loaded from JSONL and their corresponding JSON exports.
- Rejection of unknown schema versions and malformed records, including blank lines and a truncated final JSONL record.
- Count and identity validation.
- Timestamp validity, chronological ordering, and duplicate-hour rejection.
- Preservation of all observations, their order, source values, and identities when reading JSONL into shared history values and exporting those values as JSON.
- Mapping of per-record schema versions to the JSON export's root version.

### Collector

Tests for `mnm-stats-collector` MUST cover:

- Extraction of expected values from a representative source fixture, including published zeros.
- Rejection of initial placeholder values before every discovered server's asynchronous metrics load completes.
- Temporary zone-loading elements in initial and intermediate responses, followed by successful extraction from a valid completed response.
- Rejection of invalid completed zone elements with useful element diagnostics and preservation of existing history.
- Readiness with legitimately zero activity counts and delayed updates, without requiring a fixed number of chart points.
- Extraction of current-state values without storing the source's historical series or backfilling earlier observations.
- Selection of a final historical point only when the source contract establishes last-known-state semantics, and rejection of an unverified historical bucket as a current value.
- Dynamic discovery of added, removed, and reordered server cards.
- Preservation of source display names.
- Rejection of missing or duplicate identities and incompatible required fields.
- Rejection of malformed counts and inconsistent published totals.
- Acceptance of cosmetic HTML changes that preserve the source contract.
- HTTP initialization and WebSocket session handling using local responses or simulated failures, including status errors, missing session information, malformed messages, disconnection, and timeouts.
- Matching identifying User-Agent headers on HTTP and WebSocket requests, including revision/branch formatting, the local development fallback, and changed build metadata with reused caches.
- Adding exactly one JSONL record for a new UTC-hour observation, preserving earlier lines and unchanged values in a new hour.
- Same-hour no-op behavior and missing intervals without fabricated samples.
- Preservation of existing history after failure at any collection stage, including writing.
- Safe handling of interrupted writes and overlapping collection runs.
- CLI success and failure results using local inputs and isolated output files.

**Example:** A fixture adds a server with a previously unseen ID while preserving valid metrics and totals.
Collection includes that server without a parser change; removing a required count from its card makes collection fail while leaving earlier history unchanged.

### Dashboard calculations

Tests for calculations owned by `mnm-stats-dashboard` MUST cover:

- Server and starting-zone aggregation without claiming deduplicated global activity.
- Ratio values, including zero denominators and values above 100 percent.
- Correlation with known expected results and selection of the last jointly available sample per UTC day.
- Unavailable correlation for insufficient paired days or zero variance.
- All required time ranges and their UTC boundaries.
- Stale-data detection and gaps between observations.
- Equal-duration and calendar-period comparison alignment, including unequal month lengths and leap days.
- Comparisons with more than two periods or entities and with missing observations.

### Dashboard integration

Automated browser tests MUST run against a locally built dashboard with embedded test history and locally available assets.
They MUST cover:

- Rendering and exact-value inspection for the supported metric families.
- Mouse hover details with exact counts, ratio numerators and denominators, series identity, and original UTC timestamps, including comparison plots, overlapping points, and scaled or horizontally scrolled charts.
- Clearing hover details when leaving a plot or changing selections, without showing values inside gaps or for unavailable observations.
- Range selection and server selection, including historical servers.
- Unavailable metrics and empty or invalid data states.
- Month-to-month and year-to-year comparisons with at least three periods.
- Server-to-server and all-servers-to-server comparisons with at least three series.
- Adding and removing comparison selections.
- Complete-history JSON downloads independent of filters, including valid empty history.
- Operation under a repository subpath.
- Absence of separate runtime requests for metrics data.
- Charts and downloads containing only the collected current-state snapshots, with gaps for missed collections.

Build integration tests MUST verify that a history-only change updates the embedded dataset even when caches are reused.
They MUST verify that empty and populated JSONL fixtures become equivalent embedded history available through the dashboard's shared Rust snapshot types.
These tests MUST verify preserved observations without depending on a particular internal embedded representation.
Invalid JSONL history MUST fail the build without silently omitting records.
The downloaded `history.json` MUST be a valid JSON document matching the complete embedded history and preserving every observation from the build's JSONL input.

### Local preview

Browser integration tests MUST exercise the supported preview launcher as well as release assets.
They MUST verify explicit history selection, populated charts, and complete JSON downloads.
A history-only change outside the dashboard crate MUST trigger a rebuild and update the running page without restarting the preview.
Coverage MUST include valid empty history, invalid input diagnostics, and changing between demo and ordinary history without retaining the previous dataset.
Demo generation MUST be checked using a fixed timestamp; interactive demos MAY use the current time.
Browser inspection through MCP MUST use local synthetic data or existing local history and MUST NOT refresh source fixtures.

### Automation

Notification-probe coverage MUST verify an intentional failure from local input without changing repository history.
Automated checks MUST validate GitHub workflow syntax and expressions without dispatching production workflows or deploying the dashboard.

## Delivery and reporting

Each implementation step MUST include passing tests for the behavior it introduces.
Regression fixes to covered behavior MUST include a test that demonstrates the corrected result.
Tests run locally and in CI MUST follow the same rules for local data and internet requests.
Required suites MUST run through Donna polish once their implementation exists.
Missing or skipped required tests MUST NOT be reported as passing coverage.
Test reports MUST identify failed cases and the relevant local inputs without relying on live service responses.

Live source investigation and GitHub deployment or notification verification MUST remain separate operational activities outside test commands and recurring test jobs.
Those activities MAY contact live services when needed for an approved delivery step; they MUST NOT become test-suite prerequisites or automatic fixture-refresh hooks.
Controlled failure-notification verification MUST use a local invalid fixture or simulated collector failure rather than repeated requests to the statistics page.
