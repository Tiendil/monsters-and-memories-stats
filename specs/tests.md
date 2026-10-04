# Application testing

## Goal of the document

This document defines required application test coverage, test data conventions, and network access rules.

## Scope

This specification covers project test code, test data, and test execution during development and CI.
Production collection and operational verification are outside its scope except for their separation from test execution.

## Test network access

Tests MUST NOT contact the original statistics service, including its public page and HTTP or WebSocket endpoints.
This includes optional or ignored tests when explicitly invoked.
Test setup and teardown MUST follow the same restriction and MUST NOT download or refresh fixtures.
Metric inputs MUST come from local fixtures or synthetic data.

Local loopback and project-local Compose network connections MAY be used for fixture servers and dashboard previews.
Tests MAY load third-party runtime assets from the internet, including the dashboard's pinned Plotly script from its CDN.

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
- Dynamic discovery of added, omitted, and reordered starting zones independently for each server, including fewer zones on one server than on others.
- Preservation of explicit zone zeros without inventing rows for omitted zones, and acceptance of an empty list only with a published zero total.
- Rejection of missing zone lists, duplicate or empty zone identities, malformed rows, and inconsistent totals with useful diagnostics and unchanged history.
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
- Unavailable individual-zone values and chart gaps when a server omits a zone, including all-server views that would otherwise show a partial sum.
- Zero starting-zone totals for validated empty lists, historical zone discovery, and complete JSON exports preserving differing per-server zone membership.
- Ratio values, including zero denominators and values above 100 percent.
- Correlation with known expected results and selection of the last jointly available sample per UTC day.
- Unavailable correlation for insufficient paired days or zero variance.
- All required time ranges and their UTC boundaries.
- Stale-data detection and gaps between observations.
- Equal-duration and calendar-period comparison alignment, including unequal month lengths and leap days.
- Comparisons with more than two periods or entities and with missing observations.

### Dashboard integration

Automated browser tests MUST run against a locally built dashboard with embedded test history.
They MUST use the same chart-engine URL and asset-loading behavior as ordinary builds and previews.

Browser coverage MUST include:

- Rendering and exact-value inspection for the supported metric families.
- Mouse hover details with exact counts, ratio numerators and denominators, series identity, and original UTC timestamps, including comparison plots, overlapping points, and resized or horizontally scrolled charts.
- Native chart hover on dense series without visible point markers.
- Chart-engine loading from the pinned CDN URL at both root and subpath URLs, and successful chart initialization after selection changes.
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

### Dashboard presentation

Presentation coverage MUST verify [dashboard-design.md](dashboard-design.md) alongside the metric and comparison contracts.
Automated tests MUST cover:

- Navigation to every content section while retaining the selected scope, range, and comparisons.
- Headline counts from the same last snapshot inside the selected interval, including an absent historical server, a published zero, and an empty interval.
- Explicit global subscription scope and all-server activity aggregation labels in the headline summary.
- Replacement of the ordinary summary when a comparison is active.
- Dynamic individual-zone selection, including historical and unavailable zones, without losing other selections.
- Comparison controls that expose only relevant inputs and identify the separate scope of correlations.
- Stable series encodings when adding or removing other selections, with at least seven series and overlapping points.
- Keyboard operation of navigation, disclosures, comparison editing, exact-value inspection, and downloads.
- Accessible names, selected and expanded states, useful focus retention, and local validation messages.
- Empty-range recovery through all time, preserving server scope.
- Chart-engine failure leaving summary values, exact-value tables, and history download usable.

Rendered review MUST cover:

- The initial-view hierarchy at 1440 by 900 and 375 by 812 CSS pixels, including the synthetic-data notice and long source names.
- Page reflow at 320 CSS pixels, intermediate widths, and 200-percent text enlargement.
- Readable axis labels and legends, including dense comparisons and any confined horizontal scrolling.
- Text and essential-graphic contrast from the actual token colors and rendered backgrounds.
- Distinctions between series without relying solely on color, and matching line, legend, and hover encodings.
- Visible focus, touch target sizes, control states, and unobscured content when controls are sticky.
- Stale, empty, unavailable, initialization, and chart-failure states.
- Reduced-motion behavior and preservation of chart information without decorative animation.

Synthetic fixture values MUST establish expected summary observations and missing-entity cases independently of the presentation implementation.
Visual inspection MUST assess hierarchy and readability, not merely the presence of expected DOM elements.
Passing preexisting browser checks MUST NOT be reported as coverage of presentation behavior that has not been implemented.

### Design tokens

Token tests MUST use local token fixtures and the project's authored artifact, following [design-tokens.md](design-tokens.md).
They MUST cover:

- Supported structured values and inherited token types.
- Semantic aliases and chained resolution.
- Missing references, cycles, type mismatches, unsupported format features, and invalid values.
- CSS and Rust name collisions and diagnostics identifying the offending token path.
- Deterministic generation and equivalent CSS and Rust values, including color alpha and dimension units.
- Build-time breakpoint resolution into usable CSS media queries.
- A token-only change updating CSS and chart values when build caches are reused.
- Shared series colors across SVG charts, legend swatches, and hover-label borders, including comparisons beyond the explicit palette length.

Build and preview integration tests MUST use isolated token inputs and MUST NOT edit the project's authored artifact.
Browser coverage MUST verify representative computed styles, responsive behavior, and chart presentation from token values, alongside the existing dashboard interaction checks.
It MUST verify that styling is available after application initialization without a separate stylesheet request.
Invalid token input MUST fail the build rather than silently reuse previous generated values.

### Local preview

Browser integration tests MUST exercise the supported preview launcher as well as release assets.
They MUST verify explicit history selection, populated charts, and complete JSON downloads.
A history-only change outside the dashboard crate MUST trigger a rebuild and update the running page without restarting the preview.
A token-only change outside the dashboard crate MUST also rebuild the preview and update both CSS and chart presentation without restarting it.
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
Test reports MUST identify failed cases and the relevant local inputs without relying on responses from the original statistics service.

Live source investigation and GitHub deployment or notification verification MUST remain separate operational activities outside test commands and recurring test jobs.
Those activities MAY contact live services when needed for an approved delivery step; they MUST NOT become test-suite prerequisites or automatic fixture-refresh hooks.
Controlled failure-notification verification MUST use a local invalid fixture or simulated collector failure rather than repeated requests to the statistics page.
