# Historical metrics requirements

## Goal of the document

This document defines collector and dashboard behavior and acceptance evidence.

## Scope

This specification covers the externally observable behavior of historical reporting for public Monsters & Memories statistics.
It does not cover private account data or statistics that the public source does not expose.

## User requirements

### R1: GitHub infrastructure

GitHub MUST provide data storage, scheduled collection, and dashboard hosting.

### R2: Scheduled collection

A GitHub Actions collector MUST be scheduled once per hour and collect the public [metrics page](https://account.monstersandmemories.com/metrics).

### R3: History storage

Collected history MUST be stored in a JSON Lines (JSONL) file in this repository and retained across collection runs.
Each line MUST contain one versioned snapshot.

### R4: Dashboard hosting

The dashboard MUST be a static website hosted on GitHub Pages.

### R5: Application technology

Collector and dashboard application logic MUST be implemented in Rust, using Leptos for the frontend if feasible.
Third-party chart engines MAY use another language; a Rust API is useful for integration but is not required.

### R6: Metric coverage

The dashboard MUST present the following metric families:

- daily active counts.
- monthly active counts.
- active subscriptions.
- starting-zone statistics.

### R7: Server scope

The dashboard MUST support totals and per-server views where the source permits them.

### R8: Metric relationships

The dashboard MUST show useful ratios, relationships, and correlations between activity and subscriptions without inventing unavailable statistics.

### R9: Time ranges

Every plot MUST support the time ranges listed under Dashboard behavior.

### R10: Source changes

A source change that breaks the collection contract MUST fail collection and notify the maintainer.

### R11: Additional statistics

Other useful exposed statistics SHOULD be included where they do not substantially complicate the project.

### R12: Development tooling

Work MUST use the following development artifacts, drawing inspiration from the named Feeds Fun project:

- `AGENTS.md`.
- Donna workflows.
- Depmesh configuration.
- project specifications.

Depmesh MUST expose only `governs` and `governed_by`, and specification work MUST start with `specs/meta/general.md`.

### R13: Design approval

Major architecture, library, and filesystem decisions MUST be discussed and approved before implementation.

### R14: Delivery checkpoints

Work MUST proceed in consistent, working steps, each ending with a request for user review and a commit before the next step.

### R15: Simplicity

The project MUST remain simple and SHOULD use existing libraries rather than recreate their functionality.
Using existing libraries under R15 reduces code the project must maintain.
A direct implementation MAY be used when available libraries would add disproportionate complexity for the required behavior.

### R16: Server discovery

During source parsing, the collector MUST discover all server identities and names from the page without a hardcoded server list.
Server additions or removals MUST be accepted without code or configuration changes when the source contract is otherwise satisfied.
Each snapshot MUST reflect the server set present at collection time.
Each discovered server MUST satisfy the same metric and identity validation rules.

**Example:** A valid new server card appears in the next collected snapshot automatically.
If another server disappears from the page, the next snapshot omits it while earlier snapshots retain its observations.

### R17: Plot comparisons

The dashboard MUST support comparisons between plots of similar nature, meaning series with compatible metric semantics, units, and calculation rules.
It MUST provide both comparison types:

- Time-frame comparison — the same metric and entity scope over a primary range and previous, year-over-year, or custom periods, including month-to-month and year-to-year comparisons.
- Entity comparison — the same metric and time interval across entities, including server-to-server and all-servers-to-server comparisons.

Both comparison types MUST support more than two series in one comparison.
Users MUST be able to add and remove comparison series.

### R18: Embedded history

The frontend build MUST validate and embed the complete collected history in the compiled frontend.
The internal embedded representation is an implementation choice.
The dashboard MUST use this embedded history without a separate runtime request for metrics data.
Publishing newly collected observations MUST include rebuilding and deploying the frontend with the updated history.

### R19: History download

The dashboard MUST provide a downloadable JSON file containing all historical data included in that dashboard build.
The download MUST use the JSON export schema and preserve all observations, independently of the selected time range, entity scope, or comparisons.

### R20: Tests without source access

The project MUST include automated tests for the collector, shared data contract, and dashboard as those components are implemented.
Tests MUST NOT contact the original statistics service, including its public page and HTTP or WebSocket endpoints.
Third-party runtime assets MAY be loaded from the internet.
Tests MUST use local fixtures or synthetic data, with coverage and execution rules defined in [tests.md](tests.md).

### R21: Design tokens

The dashboard MUST use one machine-readable design token source for reusable styles, including CSS and chart presentation.
Token naming, data format, and consumption MUST follow [design-tokens.md](design-tokens.md).
Changing a token MUST update every consumer of that role through the supported build and preview commands.

## Metric interpretation

Shared metric and collection terms are defined in the [project dictionary](dictionary.md).
Source investigation evidence is recorded in the [source analysis](../docs/source-analysis.md).

The UI MUST distinguish source-reported counts from derived values.
Every chart MUST have a concise explanation of what it measures and how to interpret it.
Recognized metrics MUST link to a suitable definition, preferring GameAnalytics' [engagement metric dictionary](https://docs.gameanalytics.com/events-metrics-and-filtering/metrics/#engagement) for DAU, MAU, and DAU/MAU.
General concepts MAY use Wikipedia definitions, including [concurrent users](https://en.wikipedia.org/wiki/Concurrent_user) and [Pearson correlation](https://en.wikipedia.org/wiki/Pearson_correlation_coefficient).
Links MUST describe the referenced concept and MUST NOT imply that M&M's unverified counting semantics conform to an external definition.
Custom ratios and source-specific counts MUST use explicit descriptions rather than borrowed KPI names such as conversion or retention.
DAU/MAU MAY be identified as stickiness, with a qualification about unverified source windows and its distinction from returning-player retention.

The README MUST explain that DAU/MAU are the source's daily/monthly active fields with unverified counting semantics.
The UI MUST label subscriptions as subscriptions, without equating them with unique people.
Missing fields MUST NOT become zero; a literal published zero MUST remain zero, with the source limitation visible.
The dashboard MUST NOT infer MAU by summing observations.

Starting-zone counts MAY be summed by zone ID across servers, and across zones within a server.
An individual zone absent from a server's observation MUST remain unavailable, not zero.
An all-server count for an individual zone MUST be unavailable when any observed server omits that zone; a partial sum MUST NOT be presented as a complete total.
Starting-zone totals MUST sum each server's reported zone rows, including a zero total for a validated empty list.
Daily/monthly activity MAY be summed across servers; the README and accessible chart descriptions MUST explain that these are sums without deduplication.
Such sums MUST NOT be described as game-wide unique active users, since one account may use multiple servers.
Global subscriptions MUST remain global in a per-server view; per-server subscription numbers MUST NOT be invented.

**Example:** Daily active counts of 10 and 20 on two servers yield a reported sum of 30, but do not establish that 30 different players were active.

The dashboard SHOULD plot online populations as the additional statistic under R11.
Starting-zone population MUST NOT be described as new players or character creation counts.
Historical values MUST retain server and zone identities even if those entities later disappear from the source.

## History and collection

This section records the collection contract for R2, R3, and R10.

Implementation verification MUST compare collected current-state values with the connected public browser view.
This comparison is a separate source investigation and MUST NOT run as part of the test suite.
If the acquisition mechanism cannot obtain required current-state metrics, its design MUST be revised and approved rather than silently dropping those metrics or introducing an unapproved client.

History MUST consist only of snapshots of the source's current reported state obtained during successful collections.
The collector MUST NOT copy source-provided historical windows into snapshots or import their points as additional observations.
Hourly sampling MUST be described as observations, not an exhaustive record of every change within the hour.
The final point of a source historical series MAY supply a metric value only when the reviewed source contract establishes that it represents the last known state of that metric.
Being the point with the latest timestamp MUST NOT by itself establish that meaning; a historical bucket or aggregate MUST NOT be substituted for a current-state value.
Selecting such a point MUST NOT backdate the snapshot or revise earlier snapshots.
Required current fields MUST NOT be silently replaced with historical values when validation fails.
Unfinished activity fields MUST NOT be accepted as published zero values.
Existing history MUST NOT be discarded because collection fails at any stage.

Each successful observation MUST carry its actual collection timestamp in UTC.
The collector MUST preserve source values and identities, including unchanged values in a new hour.
Names MUST be preserved from the source rather than reconstructed from IDs.
Repeating a run within an already recorded UTC hour MUST be a successful no-op without replacing the recorded sample.
It MUST validate the existing history before changing it and reject corrupt or unsupported data.
Writes MUST be atomic, and overlapping collection runs MUST NOT lose or overwrite existing observations.
Missing intervals MUST remain missing; collection MUST NOT create synthetic catch-up samples.
Later source responses MUST NOT backfill missed collection intervals.

**Example:** After an observation is saved at 10:17 UTC, a run at 10:45 UTC leaves that observation unchanged.
If collection fails during the next hour, history contains no invented observation for that hour.

**Example:** A completed source response contains current online population 100 and 50 chart points.
The snapshot stores online population 100 once; it does not store the chart points.
If an approved source field instead exposes its last known state as a final chart point, that one value can supply the snapshot's metric without importing the rest of the series.

Current metric fields MUST be validated after the asynchronous load completes.
Temporary loading placeholders before completion MUST NOT be treated as completed metric values or as source-contract failures.
The identities and protocol fields needed to establish readiness MUST remain valid throughout collection.
An invalid completed rendering MUST fail collection without waiting for a later response to conceal the problem.

The parser MUST check:

- required sections.
- metric labels.
- unique identities.
- numeric formats.
- the structure and unique identities of reported starting-zone rows within each discovered server.

The collector MUST discover starting-zone identities and names from each server's completed list without a hardcoded zone roster.
Zone additions, removals, and reordering MUST be accepted without code or configuration changes when the source contract is otherwise satisfied.
Each snapshot MUST retain exactly the zones reported for each server, without filling omitted zones from earlier observations or assigning them zero counts.
An empty list MUST be accepted only when the required published starting-zone total is zero.
An absent list, malformed row, or inconsistent total MUST still fail collection.

**Example:** One server reports four valid zones whose counts sum to its published total while another reports five.
Both are collected with their respective zone lists; the omitted zone remains available in earlier observations but has no value for that server in this snapshot.

Collection MUST fail with a useful diagnostic when required metric fields or structural elements are:

- missing.
- duplicated.
- renamed.
- unexpected under the approved source contract.

Counts MUST be nonnegative integers; malformed counts MUST NOT be silently coerced.
The published online total MUST equal the sum of the server counts, and each published starting-zone total MUST equal its zone counts.
Ordinary population changes and cosmetic HTML changes that preserve this contract MUST remain collectable.
No parser can reliably detect an upstream semantic change that leaves all observable structure and labels unchanged; that limitation MUST be documented.

Failure MUST produce a failing Actions run with a diagnostic that identifies the source-contract problem.
Diagnostics for unexpected elements in a starting-zone list MUST include:

- server identity.
- containing list.
- element tag.
- ID when present.
- a bounded excerpt of visible text.

Duplicate-zone diagnostics MUST identify the duplicated zone and server.
Starting-zone total mismatches MUST report the published total, computed sum, and observed zone identities.

Maintainer notification setup and a demonstrated failure notification MUST be part of deployment acceptance.
The last valid dashboard data MUST remain usable after collection failure.

## Dashboard behavior

This section records the presentation contract for the collected current-state snapshots.
The dashboard's information hierarchy, layout, and visual interaction MUST follow [dashboard-design.md](dashboard-design.md).

The dashboard footer MUST credit Plotly with a link to its JavaScript charting library.
The dashboard MUST provide a visible link labeled “Star on GitHub” to the [project repository](https://github.com/Tiendil/monsters-and-memories-stats), inviting visitors to star it on GitHub.
The dashboard MUST provide a visible link labeled “Request a feature” to the repository's new-issue page so visitors can suggest dashboard improvements.

The dashboard MUST have an all-servers view and derive its server selector from collected history, including entities present only in historical data.
Visible server and zone labels MUST use the source display name when it is nonblank, otherwise the source ID.
Labels MUST NOT append technical IDs to available display names; internal identities and downloaded data MUST remain unchanged.
It MUST plot the following over time:

- source-reported daily/monthly activity.
- global active subscriptions.
- online population.
- starting-zone populations.

Starting-zone totals and individual zones MUST be selectable together in one plot, with independent choices for “All Zones” and each discovered zone.
These selections MUST combine with the selected server scopes and comparison periods without changing the aggregation and missing-value rules.

All plots, ratio series, and correlation calculations MUST share one visible primary time-range selector.
Time-frame comparisons MUST add periods alongside that primary range as defined under Comparisons below; correlations MUST continue to use the primary range.
The available ranges MUST be:

- today.
- yesterday.
- last 7 days.
- last 30 days.
- last 90 days.
- last 180 days.
- last year.
- all time.
- a custom inclusive range of UTC dates.

Custom dates MUST be valid, with the end on or after the start. Both boundary dates MUST be included in full.
Invalid inputs MUST leave the applied range unchanged and show a local validation message.

The default range MUST be the last 7 days.
Today MUST run from the current UTC midnight through the current time. Yesterday MUST include the complete preceding UTC calendar day and exclude the current midnight.
These day presets MUST follow the current UTC date when the clock crosses midnight.
The rolling ranges and All time MUST end at the current UTC time; “last year” MUST mean the previous 365 days, and “all time” MUST include all collected history.
The UI MUST state the selected interval and show when it contains no observations.

The UI MUST show the latest collection time and the available history interval.
The displayed latest collection time MUST indicate data age without a separate freshness warning or status label.
Times MUST be labeled UTC.
Missing samples MUST NOT be plotted as zeros or added as synthetic observations.
Connections between consecutive available observations MUST use their actual collection interval:

- Less than 3 hours: a solid line in the series color.
- At least 3 hours and less than 24 hours: a subdued gray solid line.
- At least 24 hours: no line.

Unavailable metric values and absent calendar dates introduced by comparison alignment MUST interrupt connections regardless of the collection interval.
Connections MUST be straight visual guides between observations, without adding values to hover details, calculations, or downloads.

**Example:** Observations exactly 3 hours apart are connected with a subdued gray solid line; observations exactly 24 hours apart are not connected.

Charts MUST identify their series, units, and aggregation scope.
They MUST remain usable at narrow viewport widths.
Users MUST be able to inspect exact plotted values through point details and download all recorded observations as JSON.
Hovering a plotted observation with the mouse MUST show its exact value followed by its series name on the first line and its collection date and time on the second line.
The displayed time MUST use the original observation's UTC date, hour, and minute, omitting seconds and fractional seconds.
It MUST use a readable date and end with `UTC`, without an ISO `T` separator or `Z` suffix.
The download MUST preserve each original timestamp at its full stored precision.
This MUST work for every observation time-series chart and comparison mode, including dense series and overlapping comparison points.
Aggregate heatmap details MUST follow Population insights below.
Daily online-average ratios MUST identify their sample coverage as defined under Ratios and correlation below.
Ratio details MUST include the exact numerator and denominator alongside the rounded percentage.
Hover details MUST refer only to collected observations, MUST NOT invent points inside gaps, and MUST clear when the pointer leaves the plot or the selection changes.
The following states MUST be understandable:

- initialization.
- empty data.
- invalid data.

### Population insights

The Player activity section MUST include server population share over time and an activity heatmap derived from collected online counts.
Server population share MUST divide a server's online count by the sum of all server online counts in the same snapshot and display a percentage on a 0–100 percent axis.
Selecting individual servers MUST NOT change that denominator.
For this chart, “All Servers” MUST expand to all discovered individual servers, without a redundant 100-percent total or duplicate lines when individual servers are also selected.
An absent server or zero all-server total MUST yield an unavailable share; a published zero with a positive total MUST remain zero.
Share lines MUST retain ordinary observation gaps, exact numerator/denominator details, and server/period identities.

The activity heatmap MUST group available online observations by their original UTC weekday and hour, with Monday through Sunday and hours 00 through 23.
Each cell MUST show the arithmetic mean of the available observations for that bucket within the selected range or comparison period.
Every available observation MUST have equal weight; gaps MUST NOT be interpolated or treated as zero, and an absent server MUST NOT contribute a sample.
An empty bucket MUST remain blank and distinct from a measured zero.
Details MUST identify:

- The server scope and period when applicable.
- The weekday and UTC hour interval.
- The rounded mean.
- The exact sum and observation count.

Heatmap details describe aggregates rather than individual observation timestamps.
The UI and README MUST explain that these are sampled averages, whose coverage can be uneven.

The shared Servers and time controls MUST govern both views, including more than two entities or periods.
The heatmap MUST show a separate labeled panel per server scope and period, with “All Servers” representing the summed online count in each observation.
Every displayed panel MUST use the same color scale from zero to the largest displayed cell mean so comparisons remain meaningful.
An all-zero selection MAY use a positive upper bound to keep measured zero cells visible.
Period comparison MUST group by each observation's actual UTC weekday and hour rather than shift observations onto the primary period's calendar.
Starting-zone selections MUST affect only the starting-zone chart.

**Example:** One server has 20 of 80 online players, giving a 25-percent share even when it is the only selected server.
Two Monday 10:00–11:00 UTC observations of 20 and 40 give a heatmap mean of 30 from two records; an unobserved Monday hour remains blank.

### Ratios and correlation

Ratios MUST include daily/monthly activity and activity/global-subscriptions for the available daily and monthly counts.
They MUST also include sampled average online divided by daily activity, monthly activity, and global subscriptions.
In per-server views, the subscription denominator MUST be explicitly labeled global.
Values that compare activity with subscriptions MUST be described as ratios of reported counts, not as proven fractions of subscribers playing.
They MUST NOT be clamped to 100 percent.
A zero denominator or unavailable value MUST yield “not available,” never infinity or a fabricated zero.

Online-average ratios MUST produce one aggregate per original UTC day, independently for each selected server scope and period.
The numerator MUST be the arithmetic mean of available online observations inside that day and selected period, with equal weight per sample.
The denominator MUST come from the last observation contributing to that mean; the plotted timestamp MUST be that observation's original timestamp.
Missing hours and absent servers MUST NOT contribute zeros, interpolated values, or carried-forward observations.
The current day and days clipped by a selection MUST use only the available samples inside the selection.
Period alignment MUST occur after grouping by original UTC dates.
All-server means MUST average the per-snapshot totals, without dividing by the number of servers.
Hover details MUST show the exact online sum, sample count, denominator, first sample time, and final sample timestamp so the arithmetic and temporal coverage can be inspected.
Aggregate points MUST retain the ordinary connection-interval rules; daily aggregation MUST NOT imply that missing hours were continuously measured.
These ratios MUST be described as sampled online presence, without claiming measured playtime, session length, retention, or subscriber conversion.
Average online divided by MAU MUST remain a daily indicator, not a monthly playtime estimate.

**Example:** Online samples of 10 and 30 at 08:00 and 20:00 UTC give a sampled mean of 20.
With a daily activity count of 100 in the 20:00 observation, the daily ratio is 20 percent and is plotted at 20:00, with two samples and their coverage exposed in its details.
Missing hours contribute no values; a range starting at noon includes only the sample of 30.

Correlation MUST use Pearson's r for available pairs among daily activity, monthly activity, and global subscriptions over the selected range, separately for each selected entity.
To avoid treating repeated hourly daily/monthly counts as independent days, it MUST use the last jointly available observation per UTC day inside that range.
It MUST show the paired-day count and return “not available” for fewer than three paired days or zero variance.
Missing data MUST NOT be interpolated for correlation.
The UI MUST explain that correlation does not establish causation and that overlapping source activity windows limit interpretation.

**Example:** Twenty-four hourly observations within one UTC day contribute at most one daily/monthly pair to correlation, not 24 independent pairs.

### Comparisons

Comparison capabilities MUST apply to every time-series plot family where the selected data has compatible meaning, including source counts and derived ratios.
Compared series MUST use a common value scale and consistent calculation rules.
Each series MUST remain distinguishable, with its entity scope and selected period visible.
The existing rules for aggregation labels and unavailable values MUST also apply in comparisons.

#### Time-frame comparison

The selected primary range MUST remain visible and included in every time-frame comparison.
Comparison MUST be disabled initially, and users MUST be able to enable or disable it without changing the primary range or selected servers.
The comparison choices MUST be:

- **Previous period:** the same duration immediately preceding the primary range when matching exact dates. For Today, compare the same elapsed part of the preceding UTC day, beginning at midnight.
- **Year over year:** the primary range's dates shifted back one calendar year when matching exact dates; a February 29 boundary MUST clamp to February 28 where necessary.
- **Custom period:** an inclusive range of UTC dates. Users MUST be able to add and remove any number of custom periods, including periods longer or shorter than the primary range.

Changing the primary range MUST recalculate automatic comparisons and retain the selected custom periods.
The dashboard MUST retain the primary chart while a custom comparison has no periods yet.
Identical resolved periods MUST appear only once.
Automatic series identities and colors MUST remain stable when the current-time clock advances.

##### Date matching

The comparison menu MUST offer **Match exact date** and **Match day of week**.
Exact-date matching MUST be the default and MUST preserve explicitly selected custom dates.
Weekday matching MUST align each secondary start to the primary start's weekday:

- For Previous period, move the exact-date candidate backward by zero to six days so it cannot overlap the primary range; retain its duration.
- For Year over year, choose the nearest matching weekday to the prior-year start, within three days, and use the primary duration.
- For Custom period, choose the nearest matching weekday to the selected start, within three days, and retain the selected duration.

The menu and chart legends MUST identify the resolved dates, including any weekday adjustment.
Switching back to exact dates MUST restore the original custom dates.

##### Plot alignment

With exact-date matching, comparisons containing only complete calendar months MUST align by day of month and time of day; comparisons containing only complete calendar years MUST align by month, day, and time of day.
Year-over-year comparisons with exact-date matching MUST align by month, day, and time of day, including when the primary range covers only part of a year or crosses a year boundary.
Other ranges and weekday-matched periods MUST align by elapsed time from their respective starts.
Different durations MUST retain their actual lengths; the common axis MUST accommodate the longest period without stretching observations.
Calendar boundaries MUST use UTC, and exact-value inspection MUST display each observation's original UTC date and time to the minute rather than the aligned comparison coordinate.
Calendar-year comparison MUST be distinguished from the standard rolling “last year” range of 365 days.
Dates absent from a compared period, such as a leap day in a non-leap year, MUST remain absent.
Connections across missing collection intervals MUST follow the same interval rules as ordinary charts.
Absent calendar dates and incomplete periods MUST remain visible as gaps or partial coverage, without invented zeros or extrapolated values.

**Example:** Selecting February as the primary range and adding March and April with exact-date matching produces three series aligned by day of month.
March's day 31 has no corresponding value in the February or April series.

**Example:** Comparing January 1–7, 2023 with the previous year uses January 1–7, 2022 for exact dates, or January 2–8, 2022 for matching weekdays.

#### Entity comparison

Entity comparisons MUST use the same selected time interval for every series.
Entity choices MUST come from collected history, including historical servers, and MUST support selecting the all-servers view alongside individual servers.
One shared multi-select labeled “Servers” MUST control the visible entities across charts and period comparisons.
Each selected entity MUST have its own series; when periods are compared, each selected entity and period MUST have its own series.
Server shares and heatmaps MUST use the expansion and panel rules under Population insights.
Users MUST be able to turn individual entities on or off independently, including clearing the selection.
An empty selection MUST show no data series and MUST offer an explicit selection prompt.
The all-servers series MUST retain its ordinary aggregation semantics, including contributions from the individually selected servers where applicable.
Server population share MUST use individual shares in place of an all-servers series.
Metrics available only globally MUST NOT be offered as if they had server-specific values.
Global subscriptions MUST appear once in ordinary viewing or once per comparison period when at least one entity is selected.
Periods when a selected entity has no observations MUST remain gaps.

**Example:** An online-population comparison of all servers, Server A, and Server B displays three series.
The all-servers series includes A and B when they are present in the corresponding observation.

### Embedded history

The committed JSONL history MUST be the source for the embedded history.
Embedding MUST preserve every observation and its order without changing metric values or identities.
Malformed or unsupported JSONL records MUST fail the build rather than be skipped.
Displayed data MUST remain consistent with the history embedded in the loaded frontend build.
The displayed latest collection time MUST describe that embedded history, even if newer observations have since been collected.

### History download

The UI MUST provide a clearly labeled control for downloading the complete history as `history.json`.
The download MUST be produced from the complete embedded history and MUST NOT request metrics data from a separate endpoint.
A valid empty history MUST remain downloadable as a valid history document with no snapshots.
The complete history means all project-collected snapshots, not the rolling historical series exposed by the source.

**Example:** Selecting one server and the last seven days changes the charts but leaves the download containing all servers and all recorded times in the loaded dashboard build.

## Acceptance evidence

Collection and deployment are not complete merely because local tests pass.
The README MUST explain the unavailability of:

- pre-collection history and missed collection intervals.
- deduplicated global activity.
- per-server subscriptions.

The following evidence is required before final completion.
Live source and deployment evidence MUST be gathered through separate operational verification; automated tests MUST follow [tests.md](tests.md).

### Collection and storage (R1–R3)

- A successful manual Actions run.
- A successful scheduled run.
- A resulting JSONL commit that adds a snapshot line and preserves earlier history.

### Application and hosting (R4–R5)

- Rust sources for collector and dashboard.
- A release WASM build.
- A working GitHub Pages URL with the repository subpath.

### Metric coverage (R6–R8, R11)

- Source/browser comparison.
- Parser fixture tests.
- Completed current-state extraction without importing source-provided history.
- Calculation tests.
- Inspection of every supported metric, scope, and unavailable-data case in the dashboard.

### Time ranges (R9)

Browser verification of all presets and custom UTC dates on every chart family, including boundary timestamps, validation, and an empty interval.

### Failure handling (R10)

- Changed-source fixtures fail without altering history.
- A controlled failed workflow delivers a maintainer notification.
- The last good page remains available.

### Development tooling (R12)

- `AGENTS.md`.
- Readable specs.
- A successful Donna polish run.
- Depmesh results containing only the two required relations with correct reverse links.

### Approval checkpoints (R13–R14)

Actual user approvals and review/commit checkpoints before subsequent implementation steps.

### Simplicity (R15)

Review of dependencies and a subtraction pass over each component; no unnecessary service or abstraction.

### Server discovery (R16)

- Fixtures with added, removed, or reordered valid server cards parse without code or configuration changes.
- Missing or duplicate server identities and malformed metrics on any discovered server fail collection.
- Historical observations remain intact, and the dashboard offers both newly discovered and historical servers.

### Plot comparisons (R17)

- Browser verification of month-to-month and year-to-year comparisons with at least three periods in every compatible plot family.
- Browser verification of server-to-server and all-servers-to-server comparisons with at least three series, including adding and removing selections.
- Calculation and browser checks for Previous period, Year over year, Custom period, enabling/disabling comparisons, and changes to the primary range.
- Calculation and boundary checks for exact-date and weekday matching, elapsed and calendar alignment, unequal lengths, leap days, and stable automatic series identities.
- Correct aggregation labels and visible gaps for incomplete periods or missing entity observations.
- Global-only metrics are unavailable for server-specific comparison rather than presented as invented per-server data.

### Embedded history (R18)

- Empty and populated JSONL histories produce equivalent embedded history with all observations preserved in order.
- Browser network inspection confirms that initialization and plot interactions make no separate metrics-data request.
- Changing only the history input updates the embedded dataset on rebuild, including when build caches are reused.
- Invalid history fails the frontend build without replacing the last valid published dashboard.

### History download (R19)

- Downloaded JSON follows the export schema and preserves all embedded observations under different filters and comparison selections.
- Downloads work for both populated and valid empty history.
- Downloads work in local preview and under the GitHub Pages repository subpath without requesting metrics data separately.

### Tests without source access (R20)

- Passing automated suites for the components delivered in the current implementation step.
- Local source fixtures and synthetic history cover success, failure, and boundary behavior.
- No requests to the original statistics service or fixture downloads from test setup, execution, or teardown.

### Design tokens (R21)

- CSS and chart presentation derive reusable visual values from the same token artifact.
- Token-only changes update both outputs with reused build caches and in the running preview.
- Invalid token data fails the build with a useful diagnostic.
- Browser verification covers responsive layouts, control states, chart labels, series colors, and hover details after style changes.
