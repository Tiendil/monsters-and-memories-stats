# Dashboard design research

## Investigation

Inspected on 2026-10-04.
This document separates observed evidence from design recommendations for the [dashboard presentation specification](../specs/dashboard-design.md).
The public marketing site and media kit were inspected separately from application tests.
The statistics service was not requested; the dashboard review used the local synthetic demo.

## Current dashboard

The application exposes its features, but gives archive metadata and controls more initial space than the actual metrics.
In the local demo at a 1440 by 900 viewport, the first plot began about 968 pixels below the page top.
At 375 by 900, it began about 1,408 pixels down.
The full page was approximately 5,269 and 9,179 pixels tall respectively, with ten charts for the demo's two zones.
These are measurements of one rendering, not fixed properties of every dataset.

The main issues observed in the browser and source are:

- A large promotional heading, separate archive panel, and large control panel precede any plotted data.
- No headline metric values answer a quick population or activity question.
- Every zone gets another chart in the same long sequence, so additional source identities increase page length.
- Repeated metric qualifications, numbered single-series legends, and sample counts compete with the plots.
- The “View” control combines ordinary exploration with several comparison configurations.
- Shared controls remain visible even when a comparison changes which controls govern the charts.
- Dark teal panels have similar visual weight throughout; the page does not establish a strong primary and secondary hierarchy.
- Mobile retains the same long reading sequence and horizontal plot scrolling.

The existing data semantics, dynamic identities, UTC details, downloads, and comparison capabilities are useful and should be preserved.
The redesign is primarily information architecture and presentation, not a new collection or storage system.

## Official Monsters & Memories resources

### Available media

The official [Media page](https://monstersandmemories.com/media) invites content creators and press to use its images and links to the [Media Kit](https://monstersandmemories.com/media-kit).
The kit presents logos, B-roll and trailers, and soundtrack links.
The media navigation also leads to concept art, screenshots, and illustrations.
These are the appropriate starting points for a small game-identity element.

### Styles and typography

Browser inspection of the media kit found a Squarespace website with site-specific generated stylesheets and component CSS.
The rendered design uses a pale textured background, dark text, warm orange-brown accents, and decorative serif typography.
Computed styles on its headings and paragraphs identified IM Fell English.
This is an observation of the public site, not a published typography standard for third parties.

The media pages and kit did not expose a reusable developer stylesheet, component library, design-token package, or documented brand-system download.
Targeted searches of the official domain for stylesheet, style guide, brand guidelines, fan kit, and fan-content resources did not locate one.
This establishes that none was found in the inspected public resources; it does not prove that private or unindexed resources do not exist.

A generated stylesheet that happens to be publicly loaded by Squarespace is not a supported integration contract for this dashboard.
The recommendation is to create our own tokens and styles, drawing visual cues from the game and using selected published media assets where appropriate.
The site's broad visual direction can be retained without copying its marketing layout, textures, or dense display-font usage into charts.

IM Fell English is listed in the [Google Fonts catalog](https://fonts.google.com/specimen/IM+Fell+English).
A font used in implementation should be sourced from its distribution and retain that distribution's license information, rather than be extracted from the site's CSS.
No artwork, logo, or font has been added to the application during this research.

## Dashboard guidance

### Hierarchy and exploration

[Carbon's dashboard guidance](https://www.carbondesignsystem.com/building-blocks/data-visualization/dashboards) distinguishes quick status presentation from exploratory analysis and recommends clear hierarchy, consistent chart organization, and restrained metric density.
For this project, that supports a compact Overview with headline counts, followed by explicit Activity, Population, and Relationships sections.
It does not require a separate analytics platform or a user-configurable widget system.

[Nielsen Norman Group's progressive-disclosure guidance](https://www.nngroup.com/articles/progressive-disclosure/) recommends deferring advanced or less frequently used options until they are needed.
Here that applies to period editors, archive metadata, and detailed methodology.
It does not justify hiding active scope, stale-data status, or important qualifications about the numbers.

[Grafana's dashboard guidance](https://grafana.com/docs/grafana/latest/visualizations/dashboards/build-dashboards/best-practices/) emphasizes directed exploration, shared variables, and comparing like with like.
Its relevant lesson is to preserve one understandable selection context across sections and make comparison scope explicit.
Operational alerting conventions such as coloring ordinary changes as good or bad do not fit this community archive.

### Chart readability

[ONS line-chart guidance](https://service-manual.ons.gov.uk/data-visualisation/chart-types/line-chart) supports line charts for trends, sparse use of markers, and visible gaps in missing data.
Our hourly observations fit that approach; the chart should not smooth or fill gaps to look more continuous.

[ONS axis guidance](https://service-manual.ons.gov.uk/data-visualisation/guidance/axes-and-gridlines) advises consistent comparison scales and avoiding misleading dual axes.
Daily and monthly activity should remain distinct charts; more compact presentation should not combine incompatible interpretations into one scale.
Simple line charts can use a clearly labeled nonzero bound when variation would otherwise disappear, while count magnitudes generally benefit from a zero baseline.

[ONS chart-text guidance](https://service-manual.ons.gov.uk/data-visualisation/guidance/chart-text) supports concise labels and avoiding repeated footnotes across charts.
That supports short local qualifiers and one accessible data-explanation area, while preserving exact UTC timestamps in details.

[Carbon's legend guidance](https://www.carbondesignsystem.com/building-blocks/data-visualization/legends) recommends omitting unnecessary single-category legends and maintaining clear labels for multiple series.
Our implementation should retain entity or period identity without numbering a one-line chart, and reserve fuller legends for comparisons.

### Accessibility and small screens

The relevant W3C guidance establishes concrete acceptance criteria:

- [Text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): normal text needs 4.5:1 contrast; qualifying large text needs 3:1.
- [Non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): essential control states and meaningful graphics need 3:1 contrast against adjacent colors.
- [Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html): ordinary content must remain usable at 320 CSS pixels, with exceptions for content that needs two-dimensional layout.
- [Target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html): the AA minimum is 24 CSS pixels or a permitted exception, including sufficient spacing; 44-pixel primary controls are our comfort target, not a claim about the AA minimum.
- [Unobscured focus](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html): sticky UI must not hide the focused control.
- [Hover or focus content](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html): supplementary content needs suitable dismissal and persistence behavior; exact data must also be reachable without hover.

The complete-history JSON download provides access to recorded observations independently of chart pointer interaction.
The download control should remain usable with touch and keyboard input.

## Recommended composition

The [layout illustration](dashboard-layout.svg) shows a desktop Overview and its narrow-screen arrangement with explicitly synthetic numbers.
It is a composition study, not an implemented dashboard or a second machine-readable style specification.

The intended hierarchy is:

1. Compact community identity, dataset freshness, and a secondary history download.
2. Shared server and time selection, with comparison settings revealed on request.
3. Visible section navigation.
4. Four exact headline counts from the last selected snapshot.
5. A full-width population trend.
6. Detailed section exploration, exact values, and data explanations as needed.

Activity holds the daily, monthly, and subscription charts.
Population provides population trends and a dynamic selector for individual zones, preventing the first screen from growing with the zone roster.
Relationships groups ratios and clearly scoped correlations.
Comparisons remain available across every applicable section, with larger full-width charts and removable selected-series labels.

The proposed visual treatment uses warm ivory, near-white plotting surfaces, dark ink, and restrained orange-brown accents.
A serif title supplies game character while sans-serif controls and numerical labels keep the analytical content legible.
Color values and reusable dimensions belong in the existing token artifact during implementation, with contrast verified in the rendered result.

## Boundaries and tradeoffs

The light presentation is a project design choice informed by the official site's visual language, not a universal rule that dashboards should be light.
A second theme would add another contrast and rendering surface to maintain and is not needed for this design.
The small section navigation adds one interaction to reach some charts, in exchange for reducing the initial wall of metrics and repeated explanation.
Selections persist across sections to keep that interaction inexpensive.

Headline summaries add presentation calculations, so their snapshot and missing-entity semantics are specified explicitly.
They do not introduce growth claims, inferred unique-user counts, new stored fields, or source requests.
The full archive download remains independent of the visible section and filters.

The Rust, Leptos, Plotly, embedded-history, and design-token architecture can support this design.
No UI framework migration, dashboard backend, or chart-library replacement is required by it.
Implementation will need new layout and interaction coverage; passing today's tests does not validate those unimplemented behaviors.
