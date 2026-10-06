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
Collected observations MUST have an independent history on a dedicated `data` branch, separate from application commits on the default branch.
Generated site assets MUST be delivered as Pages artifacts rather than committed to either branch.

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

The dashboard MUST show useful ratios and relationships between activity and subscriptions without inventing unavailable statistics.

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

The dashboard MUST provide a static JSONL file alongside its published assets containing all historical data included in that dashboard build.
The download MUST preserve the original versioned JSONL records, independently of the selected time range, entity scope, or comparisons.
Each published dashboard MUST provide a static `build-info.json` identifying the exact source and data commit IDs used for that deployment.

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
Chart descriptions MUST NOT contain reference links.
Background definitions MAY be linked in the README.
Custom ratios and source-specific counts MUST use explicit descriptions rather than borrowed KPI names such as conversion or retention.
DAU/MAU MAY be identified as stickiness.
The README MUST explain the unverified source windows and the distinction from returning-player retention; chart descriptions MUST focus on interpreting the metric rather than repeating these qualifications.

The README MUST explain that DAU/MAU are the source's daily/monthly active fields with unverified counting semantics.
The UI MUST label subscriptions as subscriptions, without equating them with unique people.
Missing fields MUST NOT become zero; a literal published zero MUST remain zero, with source limitations documented in the README.
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
Tabs, plots, and their applied selections MUST be directly addressable through URL fragments, with navigation and plot-link controls as defined in [dashboard-design.md](dashboard-design.md#direct-links).
Shared links, refresh, and browser Back/Forward MUST restore the time-zone choice, server selections, time ranges, comparison settings and custom dates, starting-zone selections, and both Engagement metric selections.
They MUST also restore the Trends busiest-hours grouping.

The dashboard footer MUST credit Plotly with a link to its JavaScript charting library.
The dashboard MUST provide a visible link labeled “Star on GitHub” to the [project repository](https://github.com/Tiendil/monsters-and-memories-stats), inviting visitors to star it on GitHub.
The dashboard MUST provide a visible link labeled “Feedback” that opens one repository issue form directly, without an intermediate template chooser.
The form MUST contain the standard issue title, a required Type dropdown, and one required Description field.
The Type options MUST be ordered “Feature suggestion,” “Bug report,” and “Other”; this selector MUST appear on GitHub's issue page rather than in the dashboard.
The description guidance MUST ask for a proposed change and its purpose for suggestions, and actual behavior, expected behavior, and reproduction steps when possible for bugs.
Links and screenshots MUST be optional additions within that description, without separate fields or checklists.
“Feedback” and “Star on GitHub” MUST open in a new tab or window without giving the destination access to the dashboard's opener.

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

All plots and ratio series MUST share one visible primary time-range selector.
Time-frame comparisons MUST add periods alongside that primary range as defined under Comparisons below.
The available ranges MUST be:

- today.
- yesterday.
- last 7 days.
- last 30 days.
- last 90 days.
- last 180 days.
- last year.
- all time.
- a custom inclusive range of dates in the selected time zone.

Custom dates MUST be valid, with the end on or after the start. Both boundary dates MUST be included in full.
Invalid inputs MUST leave the applied range unchanged and show a local validation message.

The default range MUST be the last 7 days.
Today MUST run from midnight in the selected time zone through the current time.
Yesterday MUST include the complete preceding calendar day in that zone and exclude the current midnight.
These day presets MUST follow the current date in the selected time zone when its clock crosses midnight.
Rolling ranges MUST use elapsed days of 24 hours each.
The rolling ranges and All time MUST end at the current instant; “last year” MUST mean the previous 365 days, and “all time” MUST include all collected history.
The UI MUST state the selected interval and show when it contains no observations.

The UI MUST show the latest collection time and the available history interval.
The displayed latest collection time MUST indicate data age without a separate freshness warning or status label.
Displayed times MUST identify the selected time zone as defined below.
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
Users MUST be able to inspect exact plotted values through point details and download all recorded observations as JSONL.
Hovering a plotted observation with the mouse MUST show its exact value followed by its series name on the first line and its collection date and time on the second line.
The displayed time MUST convert the original observation instant to the selected time zone, omitting seconds and fractional seconds.
It MUST use a readable date without an ISO `T` separator or `Z` suffix.
Tooltip timestamps MUST end with the selected zone’s name, using `UTC` or the browser’s IANA name such as `Europe/Berlin`, without a numeric UTC-offset suffix.
The download MUST preserve each original timestamp at its full stored precision.
This MUST work for every observation time-series chart and comparison mode, including dense series and overlapping comparison points.
Aggregate heatmap details MUST follow Population insights below.
Ratio details MUST show a percentage rounded to two decimal places, without a numerator/denominator breakdown or sample-coverage text.
Series names MUST remain on one line without an authored tooltip-width or character-count limit.
Hover details MUST refer only to collected observations, MUST NOT invent points inside gaps, and MUST clear when the pointer leaves the plot or the selection changes.
The following states MUST be understandable:

- initialization.
- empty data.
- invalid data.

### Time zones

The dashboard MUST offer UTC and Local time for all visible dates and times, with Local time selected by default.
An explicit UTC or local setting in the URL MUST override the default.
Local time MUST use the browser's IANA time zone and show its name followed by “(local)” on the local-time button in both modes.
The choice MUST update the history summary, date controls, comparison labels, chart axes, tooltips, and heatmap aggregation together.
Offsets MUST follow the time-zone rules at each observation instant, including daylight-saving changes and fractional-hour offsets.
If the browser zone is unavailable or unsupported, the dashboard MUST use UTC and disable the Local time action.
A shared URL requesting unavailable local time MUST explain the UTC fallback.

Today, Yesterday, custom dates, calendar comparisons, and weekday matching MUST use the selected zone's calendar.
Switching zones MUST retain entered calendar dates while recalculating their instant boundaries.
Calendar-day ranges MAY span 23 or 25 elapsed hours across daylight-saving changes.
An ambiguous calendar boundary MUST use its first occurrence; a skipped boundary MUST advance to the first valid instant after the gap.
Chronological charts MUST retain original instants and actual collection intervals.
Calendar-aligned comparison lines MUST break when repeated local clock times make their aligned coordinates stop increasing.

The time-zone choice MUST be encoded in shared links and restored on reload and browser Back/Forward.
Local links MUST use each recipient's browser zone rather than the sender's zone.
Stored observations and downloaded timestamps MUST remain unchanged in UTC.

### Population insights

The Player activity section MUST include server population share over time and an activity heatmap derived from collected online counts.
Server population share MUST divide a server's online count by the sum of all server online counts in the same snapshot and display a percentage on a 0–100 percent axis.
Selecting individual servers MUST NOT change that denominator.
For this chart, “All Servers” MUST expand to all discovered individual servers, without a redundant 100-percent total or duplicate lines when individual servers are also selected.
An absent server or zero all-server total MUST yield an unavailable share; a published zero with a positive total MUST remain zero.
Share lines MUST retain ordinary observation gaps, percentage hover values, and server/period identities.

The activity heatmap MUST group available online observations by their original instant’s weekday and hour in the selected time zone, with Monday through Sunday and hours 00 through 23.
Repeated local hours MUST contribute all their observations to the same weekday/hour cell; hours with no observations MUST remain blank.
Each cell MUST show the arithmetic mean of the available observations for that bucket within the selected range or comparison period.
Every available observation MUST have equal weight; gaps MUST NOT be interpolated or treated as zero, and an absent server MUST NOT contribute a sample.
An empty bucket MUST remain blank and distinct from a measured zero.
Details MUST identify:

- The server scope and period when applicable.
- The weekday, hour interval, and selected time zone.
- The rounded mean.
- The exact sum and observation count.

Heatmap details describe aggregates rather than individual observation timestamps.
The UI MUST describe the heatmap values as averages of online populations.
The README MUST explain that these are sampled averages whose coverage can be uneven.

The shared Servers and time controls MUST govern both views, including more than two entities or periods.
The heatmap MUST show a separate labeled panel per server scope and period, with “All Servers” representing the summed online count in each observation.
Heatmaps MUST use two independent linear color scales, each spanning the smallest to the largest available cell mean in its group:

- All-servers totals, shared across the selected periods.
- Individual servers, shared across every discovered server and the selected periods, including unchecked servers.

The total MUST NOT influence the individual-server scale, and individual-server means MUST NOT influence the total scale.
Server selection MUST control panel visibility without changing either scale for the same selected periods.
Missing cells MUST NOT affect these bounds; a measured zero MUST be included.
If every available cell in a scale group has the same mean, that scale MUST use a nonnegative, nonzero span containing and labeling that value so constant selections remain readable.
Each displayed scale range MUST be labeled and recomputed when the selected time zone, time range, or comparison periods change.
Period comparison MUST group by each observation's actual weekday and hour in the selected time zone rather than shift observations onto the primary period's calendar.
Starting-zone selections MUST affect only the starting-zone chart.

**Example:** One server has 20 of 80 online players, giving a 25-percent share even when it is the only selected server.
Two Monday 10:00–11:00 UTC observations of 20 and 40 give a heatmap mean of 30 from two records; an unobserved Monday hour remains blank.

**Example:** Total cell means range from 420 to 528, while individual-server cell means range from 100 to 216.
Total panels use 420–528 and individual-server panels use 100–216.
Unchecking the server with the largest mean hides its panel but leaves the individual-server scale at 100–216.

### Player trends

Trends MUST provide server growth, busiest recurring hours, and starting-area activity rankings from the existing observations.
It MUST use the shared server and time-zone selections and show Week, Month, and Year columns together.
These columns MUST compare the last 7, 30, and 365 complete calendar dates in the selected zone with the preceding equal number of dates, respectively.
The current calendar date MUST be excluded; both resolved date ranges MUST be available from each column header.
Chart range, comparison, zone, and engagement selections MUST remain preserved but MUST NOT affect these rankings.
“All Servers” MUST expand into individual servers without duplicate rows or an aggregate competing with its components.
Missing servers MUST remain unavailable rather than zero, including servers absent for an entire period.

Typical online MUST be the median of qualifying daily median online counts, giving each qualifying date equal weight.
A daily summary MUST require at least half of that date's elapsed hours to have observations, rounded upward, including 23- and 25-hour dates.
A period summary MUST require qualifying daily summaries on at least half of its calendar dates, rounded upward.
These minimum-coverage rules MUST apply independently to each server’s online population and combined starting-area population.
They establish minimum data coverage rather than a statistical confidence interval.
Missing observations MUST NOT be interpolated, carried forward, or treated as zero.

Server growth MUST order rows by descending weekly absolute change in typical online.
Each period column MUST show both absolute and relative change for that server, matching identities independently of the period’s rank order.
Changes MUST require qualifying summaries for both periods; percentage change MUST be unavailable when the prior value is zero.
Starting-area activity MUST contain one row per selected server and order rows by the latest week’s typical combined starting-area population.
Starting-area counts MUST be summed within each server observation before calculating daily and period medians, using a wide checked sum.
A present server with an empty starting-area list MUST contribute zero, consistent with the collector’s validated total; an absent server MUST contribute no sample.
Each period column MUST show absolute and relative change for the same server, regardless of its rank in that period.
Servers without qualifying weekly values MUST remain included so their longer-period values can still be shown.
Available ranking values MUST precede unavailable values, and equal values MUST use stable identity ordering.
These rankings MUST NOT imply measured migration, new-player acquisition, group availability, or onboarding success.

Busiest hours MUST show the three highest qualifying, non-overlapping three-hour clock windows per selected server.
Windows MUST begin at multiples of three hours, from 00:00–03:00 through 21:00–24:00 in the selected time zone.
An independent All days or By weekday control MUST default to All days.
All days MUST combine each window across calendar dates; By weekday MUST retain separate weekday/window combinations.
Each date/window summary MUST require observations in at least two distinct local clock hours and use their median online count.
The typical window population MUST be the median of those date/window summaries, with at least half of the applicable dates represented, rounded upward, and at least two distinct dates.
By weekday MUST use the number of occurrences of that weekday in the period when determining coverage.
Repeated local hours MUST retain their observations without counting as two distinct clock hours; skipped hours MUST NOT create samples.
Each server row MUST show its top three qualifying windows independently in each period column.
Each window MUST show its clock interval and weekday when applicable, typical online population, and absolute change for the same window in the previous period when available.
Windows MUST rank independently within each server, breaking ties by weekday and starting hour.
Unavailable window cells MUST explain insufficient history and identify All days or the longer-period columns as recovery options when repeated weekdays are insufficient.

**Example:** Daily medians of 10, 20, 30, and 40 give typical online 25 when those four dates meet a seven-day period's coverage requirement.
Additional observations on one qualifying date do not give that date more weight.

### Ratios

Ratios MUST include daily/monthly activity and activity/global-subscriptions for the available daily and monthly counts.
They MUST also include online population divided by daily activity, monthly activity, and global subscriptions.
In per-server views, the subscription denominator MUST be explicitly labeled global.
Values that compare activity with subscriptions MUST be described as ratios of reported counts, not as proven fractions of subscribers playing.
They MUST NOT be clamped to 100 percent.
A zero denominator or unavailable value MUST yield “not available,” never infinity or a fabricated zero.

Online ratios MUST use the online count and denominator from the same collected snapshot, independently for each selected server scope and period.
Every selected observation MUST remain a separate point at its original timestamp, without daily aggregation.
Missing hours and absent servers MUST NOT contribute zeros, interpolated values, or carried-forward observations.
All-server ratios MUST divide the snapshot's summed online count by its summed daily or monthly active count, or by its global subscriber count.
Period comparisons MUST preserve every original observation and its ratio while aligning timestamps using the ordinary comparison rules.
Online-ratio hover details MUST use the ordinary two-line format with the rounded percentage, series name, and observation timestamp.
These points MUST retain the ordinary connection-interval rules.
The README MUST explain same-snapshot calculations, aggregation scope, and missing-data handling, with a calculation example.
These ratios MUST be described as online presence, without claiming measured playtime, session length, retention, or subscriber conversion.

**Example:** A snapshot at 08:00 reports 10 online and 40 daily active, producing a 25-percent point at 08:00.
A snapshot at 20:00 reports 30 online and 100 daily active, producing a separate 30-percent point at 20:00.
Missing hours contribute no values; a range starting at noon includes only the 20:00 point.

### Comparisons

Comparison capabilities MUST apply to every time-series plot family where the selected data has compatible meaning, including source counts and derived ratios.
Compared series MUST use a common value scale and consistent calculation rules.
Each series MUST remain distinguishable, with its entity scope and selected period visible.
The existing rules for aggregation labels and unavailable values MUST also apply in comparisons.

#### Time-frame comparison

The selected primary range MUST remain visible and included in every time-frame comparison.
Comparison MUST be disabled initially, and users MUST be able to enable or disable it without changing the primary range or selected servers.
The comparison choices MUST be:

- **Previous period:** the same duration immediately preceding a rolling primary range when matching exact dates.
  For calendar-date ranges, use the same number of preceding calendar dates.
  For Today, compare midnight through the same local clock time on the preceding day.
- **Year over year:** the primary range's dates shifted back one calendar year when matching exact dates; a February 29 boundary MUST clamp to February 28 where necessary.
- **Custom period:** an inclusive range of dates in the selected time zone. Users MUST be able to add and remove any number of custom periods, including periods longer or shorter than the primary range.

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

Calendar-date comparisons MUST preserve civil date and clock spans when matching weekdays; rolling ranges MUST preserve elapsed duration.
The menu and chart legends MUST identify the resolved dates, including any weekday adjustment.
Switching back to exact dates MUST restore the original custom dates.

##### Plot alignment

With exact-date matching, comparisons containing only complete calendar months MUST align by day of month and time of day; comparisons containing only complete calendar years MUST align by month, day, and time of day.
Year-over-year comparisons with exact-date matching MUST align by month, day, and time of day, including when the primary range covers only part of a year or crosses a year boundary.
Other ranges and weekday-matched periods MUST align by elapsed time from their respective starts.
Different durations MUST retain their actual lengths; the common axis MUST accommodate the longest period without stretching observations.
Calendar boundaries MUST use the selected time zone, and exact-value inspection MUST display each original observation instant in that zone to the minute rather than the aligned comparison coordinate.
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

The UI MUST provide a native link labeled “Download JSONL” to the static `history.jsonl` file alongside the dashboard.
The link MUST support downloading the file and copying its address for sharing.
The file MUST be directly accessible without running the dashboard or opening a repository website.
Each successful build MUST package the complete validated input file unchanged, including in local previews and under repository subpaths.
The stable URL MUST serve the archive from the latest published build; an already open dashboard can retain older embedded data until reloaded.
Initialization and plot interactions MUST continue using embedded data without fetching the archive; an explicit download or direct file visit MAY request it.
A valid empty history MUST remain downloadable as an empty file.
The complete history means all project-collected snapshots, not the rolling historical series exposed by the source.

**Example:** Selecting one server and the last seven days changes the charts but leaves the download containing all servers and all recorded times in the published dashboard build.

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

Browser verification of all presets and custom dates in UTC and local time on every chart family, including boundary timestamps, validation, and an empty interval.

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

- Downloaded JSONL is byte-for-byte identical to the build input under different filters and comparison selections.
- Downloads work for both populated and valid empty history.
- The file is directly accessible through a shareable static URL in local preview and under the GitHub Pages repository subpath.
- History-only rebuilds update the static archive alongside the embedded data; failed builds preserve both previous outputs.

### Tests without source access (R20)

- Passing automated suites for the components delivered in the current implementation step.
- Local source fixtures and synthetic history cover success, failure, and boundary behavior.
- No requests to the original statistics service or fixture downloads from test setup, execution, or teardown.

### Design tokens (R21)

- CSS and chart presentation derive reusable visual values from the same token artifact.
- Token-only changes update both outputs with reused build caches and in the running preview.
- Invalid token data fails the build with a useful diagnostic.
- Browser verification covers responsive layouts, control states, chart labels, series colors, and hover details after style changes.
