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
- Combined starting-zone series for each selected zone/total, server scope, and period, with consistent axes, stable distinct identities, and no observations for empty selections.
- Zero starting-zone totals for validated empty lists, historical zone discovery, and complete JSON exports preserving differing per-server zone membership.
- Server population shares with the complete snapshot denominator, individual selections, All Servers expansion without duplicate lines, missing servers, and zero denominators.
- Weekday/hour UTC means with exact sums and sample counts, uneven sampling, missing cells, published zeros, boundary timestamps, and absent servers.
- Separate heatmaps for selected entities and periods, using original UTC buckets and common displayed color bounds, including more than two selections and empty ranges.
- Ratio values, including zero denominators and values above 100 percent.
- Correlation with known expected results and selection of the last jointly available sample per UTC day.
- Unavailable correlation for insufficient paired days or zero variance.
- All range presets and custom inclusive UTC date boundaries, including invalid or reversed inputs.
- Today and Yesterday at UTC midnight, month/year rollover, and leap-day boundaries; Today comparisons MUST use the same elapsed part of the prior day, and Yesterday comparisons MUST retain the full day.
- Series-colored connections below 3 hours, subdued gray connections from 3 hours to less than 24 hours, and no connections at 24 hours or more, including values immediately below and at both boundaries.
- Interrupted connections for unavailable metric values and absent calendar dates, independently of interval styling.
- Automatic previous-period and year-over-year calculation when the primary range changes; exact-date and weekday matching, including leap-day boundaries and ranges crossing a year.
- Elapsed and calendar-period alignment, including unequal month lengths, different custom durations, leap-day gaps, and original observation timestamps.
- Stable automatic series identities across clock updates, deduplication of resolved periods, and disabled or empty custom comparisons.
- Comparisons with more than two periods or entities and with missing observations.
- Combined server-and-period selections, distinct identities for each pair, global subscriptions without duplicate series, and empty server selections.

### Dashboard integration

Automated browser tests MUST run against a locally built dashboard with embedded test history.
They MUST use the same chart-engine URL and asset-loading behavior as ordinary builds and previews.

Browser coverage MUST include:

- Rendering and exact-value inspection for the supported metric families.
- Mouse hover details with exact counts or ratios followed by the series name on the first line and the original UTC date and time to the minute on the second, including comparison plots, overlapping points, and resized or horizontally scrolled charts; ratio numerators and denominators MUST remain available.
- Native chart hover on dense series without visible point markers.
- Mixed series-colored and subdued gray solid connections with matching widths, without duplicate hover details at shared endpoints or invented values along connections.
- Chart-engine loading from the pinned CDN URL at both root and subpath URLs, and successful chart initialization after selection changes.
- Clearing hover details when leaving a plot or changing selections, without showing values inside gaps or for unavailable observations.
- The default last-7-days range, range selection, and independent server checkbox toggles, including historical servers, “All Servers,” and clearing/restoring the selection.
- Unavailable metrics and empty or invalid data states.
- Month-to-month and year-to-year comparisons with at least three periods.
- Server-to-server and all-servers-to-server comparisons with at least three series.
- Enabling and disabling comparison from both menus, changing the primary range, choosing Previous period or Year over year, and switching between exact dates and weekdays.
- Applying custom primary dates, adding and removing multiple custom comparisons, and validation without changing the applied range.
- Closing menus through Escape and outside interaction, focus restoration, section persistence, and usable date menus at narrow widths.
- Complete-history JSON downloads independent of filters, including valid empty history.
- Operation under a repository subpath.
- Absence of separate runtime requests for metrics data.
- Charts and downloads containing only the collected current-state snapshots, with interval styling for missed collections and no synthetic records.

Build integration tests MUST verify that a history-only change updates the embedded dataset even when caches are reused.
They MUST verify that empty and populated JSONL fixtures become equivalent embedded history available through the dashboard's shared Rust snapshot types.
These tests MUST verify preserved observations without depending on a particular internal embedded representation.
Invalid JSONL history MUST fail the build without silently omitting records.
The downloaded `history.json` MUST be a valid JSON document matching the complete embedded history and preserving every observation from the build's JSONL input.

### Dashboard presentation

Presentation coverage MUST verify [dashboard-design.md](dashboard-design.md) alongside the metric and comparison contracts.
Automated tests MUST cover:

- A visible coverage summary with the complete history's first date, latest UTC date and time, and record count, independently of filters and comparisons, including empty history and a single record.
- Navigation between Overview, Player activity, and Relationships while retaining the selected scope, range, and comparisons.
- Four full-width Overview charts ordered Online, Daily active (DAU), Monthly active (MAU), and Subscribers, including working summary links to their corresponding charts.
- A visible “Now” section above the view controls in every content section, with the latest complete-history all-server totals and global subscriber count.
- Unchanged “Now” values across server, range, and comparison selections, including a retired server and empty selections; published zeros and empty history MUST remain distinguishable.
- Card link text matching the plot titles, with mouse and keyboard activation opening Overview and scrolling to and focusing the corresponding plot without changing filters.
- Persistent cards during comparisons, without a duplicate comparison-information section.
- A single full-width starting-zone plot with independently selectable “All Zones” and discovered zones, including historical and unavailable zones, without a duplicate Online chart.
- Multiple zones combined with server and period comparisons, exact hover values, stable colors, and persistent choices across content sections.
- Full-width server population shares and heatmaps in Player activity, correct percentage and aggregate hover details, independent zone filtering, entity/period changes, blank buckets versus zero, and narrow-screen labels and color scales.
- Zone checkbox keyboard operation, Escape and outside dismissal, long labels at narrow widths, empty-selection recovery, and independence from the shared view controls.
- Clicking server and zone option text as well as checkboxes, including repeated toggles in release builds and demo previews at desktop and narrow widths.
- Keeping a dropdown open through native label activation without crashes, while preserving dismissal when keyboard focus moves outside.
- Shared server selections that persist across sections and period modes, with correlations grouped per selected entity over the shared range.
- Stable series encodings when adding or removing other selections, with at least seven series and overlapping points.
- Display names without appended technical IDs, with ID fallback for missing or blank names.
- Keyboard operation of navigation, comparison editing, and downloads.
- Keyboard toggling in the Servers dropdown, Escape and outside dismissal, focus restoration, and long server names at narrow widths.
- Accessible names, selected states, useful focus retention, and local validation messages.
- Empty-range recovery through all time, preserving server scope.
- Chart-engine failure leaving summary values and history download usable.

Rendered review MUST cover:

- The initial-view hierarchy at 1440 by 900 and 375 by 812 CSS pixels, including the synthetic-data notice and long source names.
- Page reflow at 320 CSS pixels, intermediate widths, and 200-percent text enlargement.
- Readable value-axis titles and vertical legends on single-series and comparison charts, with one series per row and each fixed-width solid line sample followed by its name; include aligned swatches, wrapped names aligned beneath their first text line, dense comparisons, and any confined horizontal scrolling.
- Text and essential-graphic contrast from the actual token colors and rendered backgrounds.
- Stable series labels in legends and hover details, matching series colors, and series-colored or subdued gray solid connections according to collection intervals.
- Visible focus, touch target sizes, control states, and unobscured content when controls are sticky.
- Empty, unavailable, initialization, and chart-failure states.
- Reduced-motion behavior and preservation of chart information without decorative animation.

Synthetic fixture values MUST establish expected summary observations and missing-entity cases independently of the presentation implementation.
Visual inspection MUST assess hierarchy and readability, not merely the presence of expected DOM elements.
Passing preexisting browser checks MUST NOT be reported as coverage of presentation behavior that has not been implemented.

### Design tokens

Token tests MUST use local token fixtures and the project's authored artifact, following [design-tokens.md](design-tokens.md).
They MUST cover:

- Imports for every supported Tailwind reference family, using the installed theme and isolated CSS fixtures.
- Changes to theme references or upstream fixture values reaching CSS and Rust output together.
- Rejection of missing references, unsupported imports, type mismatches, and shadowing the reserved namespace, with the affected semantic path.
- Parsing theme declarations without importing values from animation keyframes or unrelated CSS rules.
- A deterministic, self-contained resolved DTCG document.
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
The default demo range MUST cover all three interval styles, including the exact 3-hour and 24-hour boundaries.
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
