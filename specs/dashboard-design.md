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
The interface MUST identify itself as an independent community statistics archive and MUST NOT imply that it is the official account service.

The [design research](../docs/dashboard-design-research.md) records supporting evidence and the investigation of official media resources.
The [layout illustration](../docs/dashboard-layout.svg) demonstrates the hierarchy with synthetic values; its dimensions, artwork, and colors are not an additional token source.

## Page composition

### Masthead and collection status

The masthead MUST contain the game name and the dashboard's community identity.
Its action group MUST provide “Download JSON,” “Star on GitHub,” and “Request a feature.”
It MUST remain compact rather than use a promotional hero layout.
On wide screens, the action group MUST sit beside the title; on narrow screens, the title MUST precede a compact action row.
On wide screens, the actions MUST share a control height, neutral surfaces, and subtle borders.
On narrow screens, “Request a feature” MUST use a compact text link below “Star on GitHub” to preserve space for the metrics.
The actions MUST wrap when needed for narrow widths or enlarged text.
The download action MUST explain that it contains the complete archive, independently of the current selection.
The repository action required by [requirements.md](requirements.md#dashboard-behavior) MUST retain link navigation and pair its text with a decorative scalable star icon.
All masthead actions MUST remain available from every content section.

The latest collection time and freshness MUST appear near the masthead.
The status MUST distinguish the age of the loaded dataset from the selected historical period.
The UI MUST describe collection as hourly observations and MUST NOT label the data “live.”
Stale and synthetic-data notices MUST remain visible without opening another view or disclosure.

The available history interval and observation count MUST be accessible through a labeled archive-details disclosure near this status.
These details MUST NOT occupy a large, separate introductory panel.
Human-readable UTC dates SHOULD be used in summaries to reduce scanning effort; exact original timestamps MUST remain available in point details, data tables, and downloads.

### Exploration controls

A shared control area MUST precede the active content and expose the server scope, time range, and comparison entry point.
The selected server and time range MUST remain apparent when a control is closed.
The six existing time ranges MUST remain available, with the default defined by [requirements.md](requirements.md#dashboard-behavior).
The selected interval and its observation count MUST be summarized concisely below the controls.

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

The counts MUST use the last snapshot within the shared interval, with its observation time visible.
Server-scoped values MUST come from that same snapshot; if the chosen server is absent, its counts MUST show “Not available.”
The UI MUST NOT silently retrieve an earlier server observation or sum successive daily/monthly observations to fill these counts.
All-server activity MUST be labeled as a sum without deduplication, and subscriptions MUST be labeled global even when a server is selected.
An empty selected interval MUST produce an explicit empty state rather than display the latest values from outside it.

**Example:** The last snapshot in the selected range contains Servers A and B, but the user selects historical Server C.
The activity and online summary values are unavailable for C; the global subscription count still comes from that snapshot and retains its global label.

Headline counts MUST be exact, readable integers with digit grouping, rather than abbreviated values or unexplained growth percentages.
They MUST be visually stronger than timestamps, sample counts, and helper text.
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

Short interpretation labels MUST remain close to affected values, including “Global subscriptions” and “Sum across servers; not deduplicated.”
Detailed methodology, source limitations, and archive coverage MUST live in a labeled “About the data” disclosure accessible from every section.
Disclosures MUST NOT hide stale-data warnings, active selection, or qualifications needed to interpret a displayed number.
Repeated paragraphs about the same source limitation MUST NOT push every chart below its own wall of prose.

The “About the data” disclosure MUST sit immediately above the footer, with one divider separating this area from the dashboard content.
The footer MUST form two compact groups: independent community attribution with the UTC convention, and plain links labeled “Data source,” “GitHub,” and “Charts by Plotly.”
Footer links MUST remain outside disclosures and available from every content section.
The groups and links MUST wrap on narrow screens with restrained spacing.

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

Each chart MUST present a short metric title, a concise scope or unit label, and the visualization before secondary implementation or sampling detail.
The same chart anatomy MUST be used across sections.
Observation counts and detailed qualifications SHOULD appear in captions or disclosures to reduce repeated visual clutter, except when a qualification is necessary beside the title.
Axis labels MUST remain readable at narrow widths; reduce tick density before reducing text size.
Counts MAY use compact axis labels when their scale is clear, while hover and exact-value tables MUST retain exact values.

Normal time axes MUST run chronologically from left to right and identify UTC.
Comparison axes MUST describe their alignment in human terms, retaining original timestamps in details.
Gridlines MUST be visually subordinate to series and use a small number of labeled, meaningful intervals.
Charts MUST use a linear value scale and MUST NOT combine unrelated units with dual axes.
Count and ratio charts SHOULD begin at zero to make magnitude comparisons straightforward.
A nonzero lower bound MAY be used for a line-only chart when needed to reveal variation, provided that the scale is clearly visible and the same scale applies to every compared series.
Ratios MUST remain able to exceed 100 percent.

### Series presentation

Time series MUST use unsmoothed lines so interpolation does not imply measured peaks or dips.
Missing observations and gaps MUST retain the behavior defined in [requirements.md](requirements.md#dashboard-behavior).
Markers SHOULD be reserved for isolated or sparse observations so dense hourly series remain readable.

A single-series chart MUST identify its scope without a redundant numbered legend.
Multiple-series legends MUST appear consistently near the chart and wrap readable labels rather than truncate identities.
Line samples, labels, and chart hover styling MUST use matching series encodings.
Color MUST NOT be the only series distinction; comparison series MUST also have stable labels and distinguishable line styles or marker shapes.
An entity or period MUST retain its visual encoding across the active charts and when other selections are added or removed during the session.
Entity identities and their style assignment MUST remain application data rather than become hardcoded design tokens.

Chart and legend space MUST grow with comparison content so every selected series remains identifiable.
The default palette MUST NOT impose a maximum comparison count.
The design MUST be reviewed with at least seven simultaneous series, including overlapping values and long labels.

### Value inspection

Native chart hover MUST retain the exact-value and original UTC timestamp behavior defined in [requirements.md](requirements.md#dashboard-behavior).
Every chart MUST also provide a clearly labeled “View data” disclosure containing accessible exact values.
This MUST support touch and keyboard users without requiring pointer hover.
Table headers MUST identify series, timestamp, and value; numeric values MUST align consistently and use tabular digits.
Table pagination and its current position MUST remain visible while inspecting a page of values.

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
Body copy, controls, chart labels, tables, and metric values MUST use a legible sans-serif family.
Metric values and tabular data MUST use tabular numerals where supported.
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
When a dense comparison or exact-value table requires two-dimensional space, horizontal scrolling MUST be confined to a labeled, keyboard-accessible region.
Axis labels and value details MUST remain reachable within that region.
Narrow layouts MUST preserve metric coverage, comparison controls, downloads, and data explanations.

All actions MUST be operable by keyboard with visible focus and meaningful accessible names.
Section navigation, disclosures, and selected controls MUST communicate their current state to assistive technology.
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
Chart-engine failure MUST leave the surrounding values, exact-value tables, and history download usable.
State messages MUST explain what the user can do without suggesting that reloading fetches fresh metrics from a backend.
The initial pre-WASM loading state MAY retain the browser-default styling permitted by [design-tokens.md](design-tokens.md#css-consumption).

## Verification

Required automated and rendered acceptance coverage is defined in [tests.md](tests.md#dashboard-presentation).
Visual review MUST assess the hierarchy and readability with synthetic populated history, sparse history, empty selections, and large comparisons.
Passing the existing calculation and rendering tests alone MUST NOT establish conformance to this presentation specification.
