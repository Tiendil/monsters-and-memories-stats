# Dashboard presentation

## Goal of the document

This document defines the dashboard's information hierarchy, visual identity, layouts, and interaction behavior.

## Scope

This specification covers the user-facing presentation of the historical statistics dashboard.
Metric definitions, collection, storage, and chart-engine selection are outside its scope.

## Design direction

The dashboard MUST support a quick assessment of the recorded population and activity, followed by exploration of trends and comparisons.
The initial view MUST emphasize metric values and trends over archive administration or explanatory prose.
All existing metric families and comparison capabilities defined in [requirements.md](requirements.md) MUST remain discoverable.

The visual identity MUST use a restrained fantasy editorial style: warm paper-colored surfaces, dark ink text, and a limited amber or burnt-orange accent.
Game identity MUST come primarily from the masthead and display typography.
Charts and controls MUST use plain surfaces and clear functional styling.
The interface MUST credit its creator and MUST NOT imply that it is the official account service.

The [design research](../docs/dashboard-design-research.md) records supporting evidence and the investigation of official media resources.
The [layout illustration](../docs/dashboard-layout.svg) demonstrates the hierarchy with synthetic values; its dimensions, artwork, and colors are not an additional token source.

## Page composition

### Masthead and collection status

The masthead MUST contain the game name and the sentence-case credit “Made with love and curiosity by Tiendil” above it.
The name “Tiendil” MUST link to `https://tiendil.org`, open in a new tab, and be visibly identifiable as a link.
Its action group MUST provide “Download JSON,” “Star on GitHub,” and “Request a feature.”
It MUST remain compact rather than use a promotional hero layout.
On wide screens, the action group MUST sit beside the title; on narrow screens, the title MUST precede a compact action row.
On wide screens, the actions MUST share a control height, neutral surfaces, and subtle borders.
On narrow screens, “Request a feature” MUST use a compact text link below “Star on GitHub” to preserve space for the metrics.
The actions MUST wrap when needed for narrow widths or enlarged text.
The repository action required by [requirements.md](requirements.md#dashboard-behavior) MUST retain link navigation and pair its text with a decorative scalable star icon.
All masthead actions MUST remain available from every content section.

One visible summary near the masthead MUST show the first observation's date and the latest observation's date and time in UTC.
It MUST also show the total number of records and describe collection as approximately hourly, without labeling the data “live.”
The collection phrase MUST link “M&M’s public statistics” to `https://account.monstersandmemories.com/metrics` using the normal link styling.
This link MUST open in a new tab or window.
The dates and record count MUST represent the complete loaded history, independently of selected filters or comparisons; one record means one collected observation.
The dates and record count MUST use a medium font weight in the normal dark text color; the surrounding text and collection frequency MUST retain normal weight and muted color.
The summary MUST wrap naturally on narrow screens without requiring an archive-details disclosure.
Empty history MUST show a clear no-statistics message and zero records without inventing dates.

**Example:** “Statistics from 2 Oct 2026 to 4 Oct 2026, 20:00 UTC · 42 records · collected roughly hourly from [M&M’s public statistics](https://account.monstersandmemories.com/metrics).”

Stale-data warnings MUST remain visible near the summary according to [requirements.md](requirements.md#dashboard-behavior); fresh data MUST NOT require a separate positive-status message.
Synthetic-data notices MUST remain visible without opening another view or disclosure.
Human-readable UTC dates SHOULD be used in the summary to reduce scanning effort; exact original timestamps MUST remain available in point details and downloads.

### Exploration controls

A shared control area MUST precede the active content and expose the server scope, time range, and comparison entry point.
The selected server and time range MUST remain apparent when a control is closed.
The all-servers scope MUST be labeled “All Servers” in controls, chart labels, and value details.
The six existing time ranges MUST remain available, with the default defined by [requirements.md](requirements.md#dashboard-behavior).
The control area MUST NOT repeat the selected interval and observation count in a separate summary row; the selected range MUST remain apparent from the controls and chart ticks.

On wide screens, controls SHOULD share a compact row to bring the first chart into view sooner.
At narrow widths they MUST wrap or stack with visible labels rather than require horizontal page scrolling.
The control area MAY remain sticky where sufficient vertical space exists, but it MUST NOT obscure focused controls, chart details, or section headings.
Expanded comparison editors MUST NOT become a large persistent sticky overlay.

Changing a selection MUST update the affected content without a separate Apply step or a page reload.
Changing the content section MUST preserve server, range, and comparison selections.

### Content navigation

The dashboard MUST provide four clearly named content sections:

- Overview — headline counts and the online-population trend.
- Activity — daily activity, monthly activity, and global subscriptions over time.
- Population — online population, total starting-zone population, and individual starting zones over time.
- Relationships — the three activity ratios and the corresponding correlations.

Overview MUST be the initial section.
One section MUST be presented at a time so visitors do not need to scan every metric to find a relevant chart.
Section navigation MUST be visible near the shared controls and MUST have a clear selected state.
All sections MUST remain directly reachable on mobile without an icon-only navigation menu.
Section changes MUST NOT reset an open comparison or silently change its meaning.

### Overview

In ordinary mode, Overview MUST show four headline counts:

- Online population.
- Daily active count.
- Monthly active count.
- Active subscriptions.

The counts MUST use the last snapshot within the shared interval.
Server-scoped values MUST come from that same snapshot; if the chosen server is absent, its counts MUST show “Not available.”
The UI MUST NOT silently retrieve an earlier server observation or sum successive daily/monthly observations to fill these counts.
Subscriptions MUST be labeled global even when a server is selected.
The summary MUST NOT repeat the observation timestamp or an all-server aggregation explanation above the counts.
An empty selected interval MUST produce an explicit empty state rather than display the latest values from outside it.

**Example:** The last snapshot in the selected range contains Servers A and B, but the user selects historical Server C.
The activity and online summary values are unavailable for C; the global subscription count still comes from that snapshot and retains its global label.

Headline counts MUST be exact, readable integers with digit grouping, rather than abbreviated values or unexplained growth percentages.
They MUST be visually stronger than supporting text.
An online-population chart MUST follow the summary and occupy the full content width.
The summary MUST offer clear links to the relevant detailed sections without making hover the only way to discover navigation.

In comparison mode, the single-snapshot summary MUST be replaced by a concise summary of the compared entities or periods.
It MUST NOT present one entity's or period's counts as the result of the whole comparison.

### Detailed sections

Activity MUST keep daily and monthly activity in separate charts with their distinct source labels.
Its subscriptions chart MUST visibly retain global scope.

Population MUST present the online and total starting-zone trends before individual-zone detail.
Individual-zone detail MUST use a labeled zone selector derived from the complete history, with one selected zone chart visible at a time.
The selector MUST preserve historical zone identities and MUST NOT use a hardcoded roster.
Every available zone MUST remain reachable; absence in the current selection MUST produce an explicit unavailable state.
Changing zone MUST preserve the current server, range, and comparison selections.

Relationships MUST group ratios separately from correlations.
Correlation results MUST show their paired-day counts and actual shared range and server scope next to the coefficients.
When chart comparisons use other entities or periods, the correlation block MUST explicitly state that it still uses the shared range and server scope; it MUST NOT appear to calculate a correlation between the compared series.

### Data explanations

Short interpretation labels MUST remain close to affected values, including “Global subscriptions.”
Detailed methodology, source limitations, all-server aggregation semantics, and archive coverage MUST be documented in the README.
The dashboard MUST NOT repeat all-server aggregation explanations above summaries or chart groups.
Stale-data warnings, active selections, and qualifications needed to interpret a displayed number MUST remain visible.
Repeated paragraphs about the same source limitation MUST NOT push every chart below its own wall of prose.

The footer MUST contain only a plain link labeled “Charts by Plotly,” separated from the dashboard content by one subtle divider.
The credit MUST remain available from every content section and align with the right edge of the content at all supported widths.
The dashboard MUST NOT include an “About the data” disclosure.

## Comparisons

The comparison entry point MUST distinguish ordinary viewing, server comparison, and period comparison.
Server comparison MUST expose dynamic entity choices, including the all-servers sum, and show selected entities as removable labeled items.
Period comparison MUST distinguish calendar months, calendar years, and equal-duration intervals.
It MUST reveal only the inputs relevant to the chosen comparison type.

The active comparison type and its selected series MUST remain visible above the charts.
Users MUST be able to add and remove more than two series without losing the other selections.
Duplicate entries MUST NOT produce indistinguishable duplicate series.
Invalid input MUST have a local explanation associated with the relevant field.
An incomplete comparison MUST explain the next action, such as adding a period, instead of presenting an unexplained blank chart.

In server-comparison mode, the ordinary server selector MUST be identified as the correlation scope or omitted when it has no effect in the active section.
In period-comparison mode, the shared range MUST be identified as the correlation range or omitted when it has no effect in the active section.
These stored selections MUST remain available when returning to ordinary mode.
Controls MUST NOT appear to filter charts that do not use them.

Comparison charts MUST use the full available content width.
Selected-series labels MUST identify the entity or period and distinguish sums from individual servers.
Global-only metrics MUST retain the unavailable-state behavior defined in [requirements.md](requirements.md#entity-comparison).

**Example:** Comparing three months for one server shows three period labels and a calendar-aligned chart.
Its caption identifies the selected months; a separate correlation block identifies the shared rolling interval instead of borrowing the comparison caption.

## Charts

### Hierarchy and labeling

Each chart MUST present a short metric title, a series legend, and the visualization.
The same chart anatomy MUST be used across sections.
The value-axis title MUST sit beside its axis and read “Count” for counts or “Percent (%)” for ratios.
Charts MUST NOT repeat units beside the metric heading or display plotted-observation counts or a separate “Observation time (UTC)” caption.
Qualifications necessary to interpret a chart MUST remain close to the affected values.
Axis labels MUST remain readable at narrow widths; reduce tick density before reducing text size.
Counts MAY use compact axis labels when their scale is clear, while hover details MUST retain exact values.

Normal time axes MUST run chronologically from left to right; the page's collection summary and point details MUST identify UTC.
Period-comparison axes MUST describe their alignment in human terms beside the horizontal axis, retaining original timestamps in details.
Gridlines MUST be visually subordinate to series and use a small number of labeled, meaningful intervals.
Charts MUST use a linear value scale and MUST NOT combine unrelated units with dual axes.
Count and ratio charts SHOULD begin at zero to make magnitude comparisons straightforward.
A nonzero lower bound MAY be used for a line-only chart when needed to reveal variation, provided that the scale is clearly visible and the same scale applies to every compared series.
Ratios MUST remain able to exceed 100 percent.

### Series presentation

Time series MUST use unsmoothed lines so interpolation does not imply measured peaks or dips.
Missing observations and gaps MUST retain the behavior defined in [requirements.md](requirements.md#dashboard-behavior).
Markers SHOULD be reserved for isolated or sparse observations so dense hourly series remain readable.

Every chart MUST have a legend, including single-series charts.
Each legend entry MUST show the entity or period name followed by a colored line sample, without a numeric prefix.
Legends MUST appear consistently near the chart and wrap readable labels rather than truncate identities.
Line samples, labels, and chart hover styling MUST use matching series encodings.
Color MUST NOT be the only series distinction; comparison series MUST also have stable labels and distinguishable line styles or marker shapes.
An entity or period MUST retain its visual encoding across the active charts and when other selections are added or removed during the session.
Entity identities and their style assignment MUST remain application data rather than become hardcoded design tokens.

Chart and legend space MUST grow with comparison content so every selected series remains identifiable.
The default palette MUST NOT impose a maximum comparison count.
The design MUST be reviewed with at least seven simultaneous series, including overlapping values and long labels.

### Value inspection

Native chart hover MUST retain the exact-value and original UTC timestamp behavior defined in [requirements.md](requirements.md#dashboard-behavior).
The complete-history JSON download MUST remain available independently of chart hover, including to touch and keyboard users.

## Visual language

### Surfaces and color

The default presentation MUST use a warm light background, near-white chart surfaces, and dark neutral text.
Amber or burnt orange MUST be used sparingly for active navigation, selected controls, and small identity accents.
The chart palette MUST use distinct categorical colors with sufficient contrast against chart surfaces rather than repeat the brand accent for every series.
Status colors MUST remain distinguishable from ordinary selected-state styling and MUST be accompanied by text or symbols.
Ordinary population increases and decreases MUST NOT automatically be styled as success or failure.

Panels SHOULD be separated primarily by spacing, alignment, and subtle borders to keep the display calm.
Strong shadows, ornamental frames, and decorative gradients MUST NOT compete with chart data.
Textures or game artwork MUST NOT appear behind plots, controls, or body text.

### Typography

The masthead and major section headings MUST use an old-style serif display treatment compatible with the game's visual identity.
IM Fell English SHOULD be used for this limited display role because it matches the inspected official site; a readable serif fallback MAY be used where font loading is unavailable.
Body copy, controls, chart labels, and metric values MUST use a legible sans-serif family.
Metric values MUST use tabular numerals where supported.
Display lettering MUST NOT be used for dense numbers or chart axes.

Body and control text SHOULD use a 1rem base; compact metadata and chart labels SHOULD remain at least 0.875rem at the default text size.
Exceptions MAY be used for secondary annotations when rendered inspection establishes equivalent readability.
Hierarchy MUST come from a small shared type scale, weight, and spacing rather than extensive uppercase text or letter spacing.
The title MUST remain subordinate to the dashboard's data on the initial screen.

### Layout and density

The page MUST use one centered content region with aligned controls, summary values, and chart edges.
Wide-screen summaries MUST form one row of four values; narrow-screen summaries MUST use a two-by-two layout when labels fit and a single column when needed.
Ordinary charts MAY use two columns where each plot remains readable.
The primary Overview trend and all comparison plots MUST use the full content width.
Spacing MUST distinguish content sections more strongly than elements inside one chart or control group.

At a 1440 by 900 CSS-pixel viewport, the default populated view MUST show the collection status, shared controls, four summary values, and plotted data without scrolling.
At a 375 by 812 CSS-pixel viewport, the initial screen MUST show the summary values and the start of the primary chart, with additional content available through vertical scrolling.
These checks MUST include a synthetic-data notice and representative long source names, rather than rely on unusually short labels.

### Tokens and assets

All reusable presentation values MUST follow [design-tokens.md](design-tokens.md).
The token artifact MUST remain the single authored source for the palette, typography, spacing, chart styling, and responsive values consumed by implementation.
Changes to visual roles MUST update CSS and Plotly presentation together.
The implementation MUST NOT copy proposed numeric values from a layout illustration into an independent handwritten palette.

Any game artwork or logo MUST come from the game's published media resources, with its source and applicable usage terms recorded alongside the asset.
Fonts MUST be obtained from their distributable source with required license information retained.
The dashboard MUST use its own styles and MUST NOT depend on the official site's generated Squarespace stylesheet as a third-party UI library.

## Responsive and accessible interaction

The interface MUST support WCAG 2.2 AA requirements applicable to its content and controls.
Normal text MUST meet at least 4.5:1 contrast, large text at least 3:1, and essential control or chart graphics at least 3:1 against adjacent colors.
Decorative borders and nonessential gridlines need not have the prominence of meaningful graphics.

Page content MUST reflow at 320 CSS pixels without horizontal document scrolling.
Ordinary charts SHOULD fit their container by adapting tick density and margins.
When a dense comparison requires two-dimensional space, horizontal scrolling MUST be confined to a labeled, keyboard-accessible region.
Axis labels and value details MUST remain reachable within that region.
Narrow layouts MUST preserve metric coverage, comparison controls, downloads, and data explanations.

All actions MUST be operable by keyboard with visible focus and meaningful accessible names.
Section navigation and selected controls MUST communicate their current state to assistive technology.
The page MUST provide a skip link to its main content and a logical heading hierarchy.
Primary controls SHOULD have at least 44 by 44 CSS-pixel hit areas for comfortable touch use; compact controls MUST meet WCAG's 24-pixel minimum or spacing exception.
Text enlargement to 200 percent MUST NOT hide controls, values, or labels.

Selection updates MUST preserve a useful focus location rather than move focus unexpectedly into a chart.
Errors and result-state changes MUST be announced without repeatedly announcing every chart redraw.
Authored help popovers MUST support keyboard opening and dismissal and MUST NOT make hover the sole access path.
Nonessential animation MUST respect reduced-motion preferences; charts MUST NOT animate a sweep through fabricated intermediate observations.

## Data and loading states

The interface MUST distinguish:

- Application initialization.
- No collected history.
- No observations in the selected interval.
- An entity or metric unavailable for the selected observation or comparison.
- Stale collected data.
- Chart-engine failure.
- Invalid comparison input.

Empty or unavailable values MUST use a clear label rather than a misleading zero, flat line, or unlabeled dash.
An empty interval MUST offer a direct way to select all time while retaining the server scope.
Chart-engine failure MUST leave the surrounding values and history download usable.
State messages MUST explain what the user can do without suggesting that reloading fetches fresh metrics from a backend.
The initial pre-WASM loading state MAY retain the browser-default styling permitted by [design-tokens.md](design-tokens.md#css-consumption).

## Verification

Required automated and rendered acceptance coverage is defined in [tests.md](tests.md#dashboard-presentation).
Visual review MUST assess the hierarchy and readability with synthetic populated history, sparse history, empty selections, and large comparisons.
Passing the existing calculation and rendering tests alone MUST NOT establish conformance to this presentation specification.
