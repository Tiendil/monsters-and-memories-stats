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

The masthead MUST use the heading “Statistics for Monsters & Memories” and the sentence-case credit “Made with love and curiosity by Tiendil” above it.
The browser page title MUST use the same wording as the heading.
The name “Tiendil” MUST link to `https://tiendil.org`, open in a new tab, and be visibly identifiable as a link.
Its action group MUST provide “Download JSONL,” “Star on GitHub,” and “Request a feature.”
It MUST remain compact rather than use a promotional hero layout.
On wide screens, the action group MUST sit beside the title; on narrow screens, the title MUST precede a compact action row.
On wide screens, the actions MUST share a control height, neutral surfaces, and subtle borders.
On narrow screens, “Request a feature” MUST use a compact text link below “Star on GitHub” to preserve space for the metrics.
The actions MUST wrap when needed for narrow widths or enlarged text.
The repository action required by [requirements.md](requirements.md#dashboard-behavior) MUST retain link navigation and pair its text with a decorative scalable star icon.
The download action MUST retain native link behavior so users can copy and share the static archive address.
All masthead actions MUST remain available from every content section.

One visible summary near the masthead MUST show the first observation's date and the latest observation's date and time in UTC.
It MUST also show the total number of records and describe collection as approximately hourly, without labeling the data “live.”
The collection phrase MUST link “M&M’s public statistics” to `https://account.monstersandmemories.com/metrics` using the normal link styling.
This link MUST open in a new tab or window.
The dates and record count MUST represent the complete loaded history, independently of selected filters or comparisons; one record means one collected observation.
The dates and record count MUST use a medium font weight in the normal dark text color; the surrounding text and collection frequency MUST retain normal weight and muted color.
The summary MUST wrap naturally on narrow screens without requiring an archive-details disclosure.
Empty history MUST show a clear no-statistics message and zero records without inventing dates.

**Example:** “Data from 2 Oct 2026 to 4 Oct 2026, 20:00 UTC · 42 records · collected roughly hourly from [M&M’s public statistics](https://account.monstersandmemories.com/metrics).”

The summary's latest collection time MUST communicate data age without a separate freshness warning or status message.
Synthetic-data notices MUST remain visible without opening another view or disclosure.
Human-readable UTC dates SHOULD be used in the summary to reduce scanning effort; point details MUST show the original observation time to the minute, and downloads MUST retain full timestamp precision.

### Exploration controls

A shared control area MUST follow “Now” and precede the active content.
It MUST expose a multi-select labeled “Servers,” the time range, and period comparison.
The Servers control MUST provide independently toggleable checkboxes for “All Servers” and every server discovered in the complete history.
Server and zone names throughout the dashboard MUST use their display name when available, falling back to the ID for a missing or blank name without appending an ID to a display name.
Only “All Servers” MUST be selected initially; selecting it MUST NOT select the individual-server checkboxes.
The closed control MUST identify a single selected entity or summarize multiple selections, including whether “All Servers” is included; chart legends MUST show the selected identities.
The dropdown MUST support mouse, touch, and keyboard use without modifier keys, and MUST remain open while toggling choices.
Escape MUST close the dropdown and restore focus to its trigger; clicking or moving focus outside MUST close it.
Option labels MUST wrap long names, and long option lists MUST scroll within the dropdown.
The closed control MAY truncate a long name visually when its full label remains accessible in the open dropdown and to assistive technology.
An empty selection MUST show an explicit prompt to choose servers and a way to restore “All Servers,” without silently selecting an entity or plotting data.
The selected time range MUST remain apparent when its control is closed.
The all-servers scope MUST be labeled “All Servers” in controls, chart labels, and value details.
All time ranges MUST remain available, with their boundaries and default defined by [requirements.md](requirements.md#dashboard-behavior).
The control area MUST NOT repeat the selected interval and observation count in a separate summary row; the selected range MUST remain apparent from the controls and chart ticks.

On wide screens, controls SHOULD share a compact row to bring the first chart into view sooner.
At narrow widths they MUST wrap or stack with visible labels rather than require horizontal page scrolling.
The control area MAY remain sticky where sufficient vertical space exists, but it MUST NOT obscure focused controls, chart details, or section headings.
Expanded comparison editors MUST NOT become a large persistent sticky overlay.

Selecting a preset, comparison mode, date-matching option, or server MUST update the affected content immediately without reloading the page.
Custom date fields MUST apply together through “Apply range” or “Add period” after validation.
Changing the content section MUST preserve server, range, and comparison selections.

### Date and comparison menus

The time controls MUST use a compact primary-range button labeled “Time range” and a comparison button labeled “Comparison.”
Each label MUST appear above its own button and contribute to its accessible name together with the current selection.
The controls MUST NOT include “vs” or another connector between them.
The primary menu MUST begin with Today and Yesterday, followed by the rolling ranges, All time, Custom range, and an Enable comparison or Disable comparison action.
The comparison button MUST read “Compare” while disabled and identify the active mode while enabled; multiple custom periods MUST be reflected in its label.
Its menu MUST offer Disable comparison, Previous period, Year over year, and Custom period, followed by a visually separated choice between Match day of week and Match exact date.
The default matching rule and period calculations MUST follow [requirements.md](requirements.md#time-frame-comparison).

Custom UTC date inputs, validation messages, Add period, and removable custom selections MUST live inside their menus.
The comparison menu MUST show the resolved dates so weekday adjustments can be inspected before closing it.
A separate comparison-information section or persistent editor above the charts MUST NOT be shown; chart legends identify the plotted periods and entities.
The primary range MUST remain visible during comparisons.
The controls MUST NOT add keyboard shortcut bindings or shortcut badges.

The menus MUST support mouse, touch, and standard keyboard activation and focus navigation.
Escape MUST close a menu and restore focus to its trigger; clicking or moving focus outside MUST close it.
Selecting a preset, automatic mode, or matching option MUST close its menu; custom editing MUST remain open while adding or removing periods.
Menus MUST stay within the page width, scroll long content, and avoid a persistent sticky overlay.
Opening one menu MUST close any other open selection menu.

### Content navigation

The dashboard MUST provide three clearly named content sections:

- Overview — trends for online population, daily activity, monthly activity, and global subscriptions.
- Player activity — starting-zone populations, server population shares, and activity by weekday and hour.
- Engagement — daily participation, online presence, and activity relative to subscribers.

Overview MUST be the initial section when the URL has no recognized tab or plot fragment.
One section MUST be presented at a time so visitors do not need to scan every metric to find a relevant chart.
Section navigation MUST be visible near the shared controls and MUST have a clear selected state.
All sections MUST remain directly reachable on mobile without an icon-only navigation menu.
Section changes MUST NOT reset an open comparison or silently change its meaning.

#### Direct links

The section links MUST use these URL fragments:

- Overview: `#overview`.
- Player activity: `#player-activity`.
- Engagement: `#engagement`.

Each plot MUST have a stable `#chart-…` fragment, retaining existing plot identifiers independently of display-title changes.
Opening a plot fragment MUST select its owning section before scrolling to and focusing the plot.
Opening a section fragment MUST select that section.
Initial page visits, refreshes, fragment changes, and browser Back/Forward MUST restore both the destination and all applied selections.
An empty or unknown destination MUST select Overview without an error while retaining valid settings from the fragment.
The content skip link MUST focus dashboard content without changing the current URL or selections; incoming legacy `#content` navigation MUST preserve the current selections.
Navigation within the page MUST preserve all shared and chart-local selections, including those in inactive sections.

Each plot heading MUST have an adjacent native link displaying `#`, with an accessible name and tooltip identifying the plot, such as “Link to Online.”
The link MUST remain subtly visible on desktop and mobile, with clear hover and keyboard-focus states and an adequate touch target.
Activating it MUST set the plot fragment in the address bar and navigate to that plot.
It MUST support native link actions, including copying the link address and opening it in a new tab, without replacing them with clipboard behavior.
Section navigation and “Now” links MUST use the same behavior and include the current selections in their addresses.

##### Applied view settings

The fragment MUST use `#destination?key=value&key=value` with URL-encoded keys and values.
Parameters MUST encode applied view settings with these names:

- `range`: `today`, `yesterday`, `7`, `30`, `90`, `180`, `365`, `all`, or `custom`.
- `from` and `to`: inclusive UTC dates in `YYYY-MM-DD` form for a custom primary range.
- Repeated `scope`: `all` or `server:<source ID>`.
- `compare`: `disabled`, `previous`, `year-over-year`, or `custom`.
- `match`: `date` or `weekday`.
- Repeated `period`: custom comparison date pairs in `YYYY-MM-DD/YYYY-MM-DD` form, with both dates inclusive.
- Repeated `zone`: `all` or `zone:<source ID>`.
- Repeated `online-metric`: `online-daily` or `online-monthly`.
- Repeated `subscriber-metric`: `daily-subscriptions`, `monthly-subscriptions`, or `online-subscriptions`.

Server and zone IDs MUST be used instead of display names so renaming an entity does not invalidate its links.
Missing entity IDs MUST remain selected and appear as removable options labeled with their ID and “unavailable.”
Generated links MUST omit default settings, retain selection order, and use a consistent parameter order.
An explicit empty value for a selection parameter MUST represent no selections; an omitted parameter MUST use that control's defaults.
Simple tab and plot fragments without parameters MUST remain supported and use default selections on a fresh visit.
Relative presets MUST remain relative to the current UTC time; custom ranges and comparison periods MUST retain their fixed dates.
Saved custom periods and chart-local selections MUST survive temporary mode or section changes.
Unknown parameters MUST be ignored; malformed values MUST fall back only for the affected setting.
Valid entries in repeated parameters MUST be preserved and duplicates removed; if no valid selection remains, use its default unless an explicit empty value was supplied.
Invalid comparison periods MUST be skipped.

Each change to applied selections or destination MUST add one browser history entry.
Restoring a URL MUST NOT add another history entry or discard Forward history.
Opening menus, editing unapplied dates, validation errors, hover, and clock updates MUST NOT change the URL or browser history.
Changing filters MUST NOT scroll to the fragment's plot again; following a plot link MUST scroll to and focus its plot, including repeated activation of the current link.

**Example:** `#chart-online-presence?range=30&scope=server%3Aa&compare=previous` opens Online presence for server `a`, with the last 30 days compared with the previous period.
Copying the chart link preserves that configuration; selecting another time range and pressing Back restores the previous range.

### Now

A section titled “Now” MUST appear after the collection-status summary and before the view controls.
It MUST remain visible in every content section, including during comparisons and empty chart selections.
It MUST show four counts from the latest collected snapshot in the complete loaded history:

- Online.
- Daily active (DAU).
- Monthly active (MAU).
- Subscribers.

Online, daily, and monthly counts MUST sum all servers in that snapshot; the subscriber count MUST use its global active-subscription value.
Server, time-range, and comparison controls MUST NOT change these counts or hide the section.
“Now” MUST mean the latest collected state, not a live measurement; the collection-status summary supplies its timestamp.
Empty history MUST show “Not available” for all four values, and published zeros MUST remain zero.
The UI MUST NOT retrieve older values or sum successive daily/monthly observations to fill these counts.
The section MUST NOT repeat the observation timestamp or an all-server aggregation explanation above the counts.

**Example:** A visitor selects a retired server and a historical month for the plots.
“Now” continues to show all-server totals from the latest collected snapshot, even when the selected charts have no observations.

Counts MUST be exact, readable integers with digit grouping, rather than abbreviated values or unexplained growth percentages.
They MUST be visually stronger than supporting text.
Cards MUST keep a small token-based gap between each link and its count; desktop labels MUST use their text height so the cards remain compact.
Each card's link text MUST equal its corresponding plot title and MUST scroll to and focus that plot.
Activating a link from another content section MUST open Overview before scrolling, while preserving the view controls.
A separate comparison-information section MUST NOT replace the cards.

### Overview

Overview MUST show four separate charts in this order:

1. Online.
2. Daily active (DAU).
3. Monthly active (MAU).
4. Subscribers.

Each chart MUST occupy the full content width, including without comparisons.
Online population MUST be labeled “Online” in the cards and chart headings.
The active-subscription count MUST use the chart title “Subscribers” and the legend label “Global subscribers.”
Daily and monthly activity MUST retain their distinct source labels, and subscriber chart legends MUST visibly retain global scope.

### Detailed sections

The Player activity section MUST show full-width views in this order: Starting-zone population, Server population share, and Activity heatmap.
The Starting-zone population chart MUST combine the selected starting-zone totals and individual zones.
Online MUST remain in Overview without a duplicate chart in Player activity.
A checkbox dropdown labeled “Show zones” MUST appear inside the Starting-zone population card, after its explanation and before its legend, and provide “All Zones” plus every zone discovered in the complete history.
“All Zones” MUST plot the total across all reported starting zones; it MUST NOT select individual-zone checkboxes or sum only checked zones.
Only “All Zones” MUST be selected initially.
Every option MUST toggle independently, allowing the total and more than two individual zones on the same chart.
The dropdown MUST follow the same interaction, layout, and accessibility requirements as the Servers control.
Its closed label MUST name a single selection or summarize multiple selections, including whether “All Zones” is selected.
Clearing all zones MUST show a prompt and an explicit “Show All Zones” recovery action inside that chart card, without silently restoring a selection.
Changing zones MUST preserve the current servers, time range, and comparisons; zone selections MUST persist across content sections.
Historical zones MUST remain selectable, and missing zone values MUST follow the unavailable-value rules in [requirements.md](requirements.md#metric-interpretation).
Each selected zone or total MUST produce a series for each selected server scope and comparison period.
Legends and hover details MUST identify the zone or “All Zones,” server scope, and period when applicable.
Zone/server/period combinations MUST have distinct, stable series identities, preserving their colors when other selections change.

Server population share MUST use the time-series chart anatomy and a 0–100 percent axis.
A short note MUST explain that “All Servers” shows individual shares of the complete observed total.
Activity heatmap MUST use separate labeled panels for each selected server scope and period, arranged vertically at full width.
Each panel MUST show weekday rows ordered Monday through Sunday and hour columns ordered 00 through 23, with the hour axis explicitly labeled UTC.
A labeled sequential color scale MUST show mean online counts, with independent bounds for all-servers totals and individual servers as defined in [requirements.md](requirements.md#population-insights).
The scale MUST use the published Inferno sequential palette, progressing from near-black through purple and orange to bright yellow as activity increases.
The scale MUST label its lower and upper bounds, with intermediate labels where space permits.
Labels MUST use ordinary numbers with thousands separators rather than scientific notation, retaining decimals when needed to distinguish small averages.
Scale labels MUST remain readable without overlap or clipping at narrow widths.
Unobserved cells MUST retain the neutral chart surface and remain distinguishable from measured zero.
The heatmap MUST NOT use categorical line swatches to represent its intensity scale.
A short note MUST explain weekday/hour averages and increasing brightness for higher values.
It MUST identify blank cells as missing observations and explain that individual servers share a scale including unchecked servers, while “All Servers” uses a separate scale.
It MUST briefly explain that separate scales keep larger totals from hiding differences between individual servers.
It MUST explain that both scales cover the selected periods.
Cell hover MUST expose the aggregate details defined in [requirements.md](requirements.md#population-insights).
An unavailable panel MUST identify its scope and period and show an empty-state message.

Engagement MUST show three full-width charts in this order:

1. Daily participation — daily active divided by monthly active, without a local metric selector.
2. Online presence — online population divided by daily or monthly active counts from the same snapshot.
3. Activity relative to subscribers — daily active, monthly active, or online population divided by global subscriptions from the same snapshot.

The latter two charts MUST each have a checkbox dropdown labeled “Show metrics,” using the Servers control's interaction and accessibility behavior.
Online presence MUST initially select both Online / daily active and Online / monthly active.
Activity relative to subscribers MUST initially select daily activity, monthly activity, and online.
Each metric MUST remain independently toggleable, and restoring defaults MUST select all metrics offered by that chart.
Selections MUST persist across content sections and changes to the shared server, time-range, and comparison controls.
An empty metric selection MUST plot no series and offer an explicit action to restore that chart's defaults.
Every selected metric MUST combine with every selected server scope and period, with stable distinct series identities and labels identifying the metric, scope, and period when applicable.
The two metric selectors MUST affect only their own charts.
Ratio axes MUST remain percentages without a 100-percent ceiling.
The tab MUST NOT have a redundant “Ratios of reported counts” heading or additional summary cards.

### Data explanations

Short interpretation labels MUST remain close to affected values, including “Global subscribers.”
Every chart MUST show a concise plain-language explanation below its title, stating what the metric measures and how its values are interpreted.
Descriptions MUST use neutral, factual game-design language with familiar terms and short sentences.
Descriptions MUST omit redundant source-attribution and data-collection qualifiers already established by the page context.
They MUST NOT address the reader with invitations such as “follow”, “discover”, or “see how”, or use playful or promotional language.
Direct instructions MAY explain a chart control when needed.
General caveats about unconfirmed counting methods and source windows MUST remain in the README rather than visible or accessible chart descriptions.
Formulas MUST appear as compact blocks below the chart title and above their explanatory prose, using medium-weight monospace text in the dashboard's burnt-orange accent color.
Each formula block MUST have a faint warm background, a subtle thin border, small rounded corners, and compact padding.
Blocks MUST fit their text within the available width, with formulas wrapping naturally on narrow screens.
The chart title MUST remain visually stronger than the formula, and explanatory prose MUST use muted text.
Chart descriptions MUST contain no reference links.
Explanations MUST remain visible and accessible without hover or opening a disclosure.
Online-presence explanations MUST identify the online population relative to daily or monthly active players.
Subscriber-activity explanations MUST identify the global subscriber count and state that ratios can exceed 100 percent.
Detailed calculation rules, aggregation scope, and missing-data handling MUST remain in the README rather than repeated in chart explanations or time-series tooltips.
Detailed methodology, source limitations, all-server aggregation semantics, and archive coverage MUST be documented in the README.
The dashboard MUST NOT repeat all-server aggregation explanations above summaries or chart groups.
Active selections and qualifications needed to interpret a displayed number MUST remain visible.
Repeated paragraphs about the same source limitation MUST NOT push every chart below its own wall of prose.

The footer MUST contain only a plain link labeled “Charts by Plotly,” separated from the dashboard content by one subtle divider.
The credit MUST remain available from every content section and align with the right edge of the content at all supported widths.
The dashboard MUST NOT include an “About the data” disclosure.

## Comparisons

Server comparisons MUST use the shared Servers multi-select without a separate server-comparison mode or duplicate selection controls.
The comparison entry point MUST distinguish ordinary chronological viewing from period comparison.
Period comparison MUST offer Previous period, Year over year, and Custom period through the date menus.
It MUST reveal date fields only when editing a custom range or custom comparison.

The controls MUST identify the active comparison type and selected periods; chart legends MUST identify the selected series.
A separate comparison heading or information section MUST NOT repeat these selections above the charts.
Users MUST be able to add and remove more than two series without losing the other selections.
Duplicate entries MUST NOT produce indistinguishable duplicate series.
Invalid input MUST have a local explanation associated with the relevant field.
An incomplete comparison MUST explain the next action, such as adding a period, instead of presenting an unexplained blank chart.

The selected servers MUST apply to every chart in both ordinary viewing and period comparisons, preserving selections across section and mode changes.
Period comparisons MUST identify each selected entity and period in the chart legend or heatmap panel label.
Server population share MUST expand All Servers into individual shares as defined in [requirements.md](requirements.md#population-insights).
Global subscriptions MUST appear once per period, independently of the number of selected servers, and MUST disappear when no entities are selected.
The primary range MUST control the primary chart series in every comparison mode.
These stored selections MUST remain available when returning to ordinary mode.
Controls MUST NOT appear to filter charts that do not use them.

Comparison charts MUST use the full available content width.
Selected-series labels MUST identify the entity or period and distinguish sums from individual servers.
Global-only metrics MUST retain the scope rules defined in [requirements.md](requirements.md#entity-comparison).

**Example:** Comparing three months for one server shows three period labels and a calendar-aligned chart.
Its legend identifies the selected months.

## Charts

### Hierarchy and labeling

Each time-series chart MUST present its content in this order:

1. Short metric title.
2. Formula, when applicable.
3. Short explanation.
4. Selector affecting only that chart, when present.
5. Series legend.
6. Visualization.

A selector affecting only one chart MUST remain inside that chart's card rather than above the section's charts.
Its label MUST use semibold dark text, distinct from muted explanatory prose and smaller than the chart title.
The label MUST sit directly above its dropdown, with a smaller gap than the space separating the control group from the explanation above it.
Chart selectors MUST share this presentation and MUST NOT add an enclosing border or background beyond the dropdown's own control styling.
Their labels MUST contribute to their accessible names.
Heatmaps MUST use the panel labels and color scale defined under Detailed sections.
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
Time-series count and ratio charts SHOULD begin at zero to make magnitude comparisons straightforward.
A nonzero lower bound MAY be used for a line-only chart when needed to reveal variation, provided that the scale is clearly visible and the same scale applies to every compared series.
Activity ratios MUST remain able to exceed 100 percent; server population shares MUST use their bounded percentage scale.

### Series presentation

Time series MUST use unsmoothed lines so interpolation does not imply measured peaks or dips.
Missing observations and gaps MUST retain the behavior defined in [requirements.md](requirements.md#dashboard-behavior).
Markers SHOULD be reserved for isolated or sparse observations so dense hourly series remain readable.

Every time-series chart MUST have a legend, including single-series charts.
The legend MUST be a vertical list with one series per row at every viewport width.
Each legend entry MUST show a fixed-width colored line sample followed by the entity or period name, without a numeric prefix.
Line samples MUST align in one column, with a consistent gap before left-aligned names.
Wrapped names MUST align with their first text line, and line samples MUST remain beside that first line.
Legends MUST appear consistently near the chart and wrap readable labels rather than truncate identities.
Line samples, labels, and chart hover styling MUST use matching series encodings.
All data-series lines and legend line samples MUST be solid.
Connections across sparse collection intervals MUST use a subdued gray with reduced opacity according to the observation-interval rules in [requirements.md](requirements.md#dashboard-behavior).
These connections MUST retain the ordinary line width; observation markers, legends, and hover details MUST retain their series colors.
Legends and hover details MUST identify each series by its stable label alongside its color.
An entity or period MUST retain its visual encoding across the active charts and when other selections are added or removed during the session.
Entity identities and their style assignment MUST remain application data rather than become hardcoded design tokens.

Chart and legend space MUST grow with comparison content so every selected series remains identifiable.
The default palette MUST NOT impose a maximum comparison count.
The design MUST be reviewed with at least seven simultaneous series, including overlapping values and long labels.

### Value inspection

Native chart hover MUST retain the exact-value and original UTC timestamp behavior defined in [requirements.md](requirements.md#dashboard-behavior).
The first tooltip line MUST show the value followed by the series name; the second MUST show a human-readable date and time ending in “UTC,” with minute precision.
Time-series tooltips MUST contain only the value and series name followed by the timestamp, without calculation breakdowns or sample-coverage text.
Tooltip widths MUST follow their content without an authored width or character-count limit, and series names MUST remain on one line, including in heatmap tooltips.
Chart height MUST accommodate simultaneous two-line time-series labels.

**Example:** “429 All Servers” on the first line and “02 Oct 2026, 10:00 UTC” on the second.

The complete-history JSONL download MUST remain available independently of chart hover, including to touch and keyboard users.

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
Body copy, controls, chart labels, and metric values MUST use a legible sans-serif family, except for the monospace formulas defined above.
Metric values MUST use tabular numerals where supported.
Display lettering MUST NOT be used for dense numbers or chart axes.

Body and control text SHOULD use a 1rem base; compact metadata and chart labels SHOULD remain at least 0.875rem at the default text size.
Exceptions MAY be used for secondary annotations when rendered inspection establishes equivalent readability.
Hierarchy MUST come from a small shared type scale, weight, and spacing rather than extensive uppercase text or letter spacing.
The title MUST remain subordinate to the dashboard's data on the initial screen.

### Layout and density

The page MUST use one centered content region with aligned controls, summary values, and chart edges.
Wide-screen summaries MUST form one row of four values; narrow-screen summaries MUST use a two-by-two layout when labels fit and a single column when needed.
All charts in Overview, Player activity, and Engagement MUST use the full content width, including comparison plots.
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
