# Dashboard design tokens

## Goal of the document

This document defines the dashboard's design token source, naming conventions, supported data format, and use in CSS and chart configuration.

## Scope

This specification covers reusable presentation values and their translation into dashboard styles.
Screen composition, metric semantics, and collection behavior are outside its scope.

## Token source

Shared token terminology is defined in the [project dictionary](dictionary.md#presentation-terms).

`specs/design-tokens.tokens.json` MUST be the single authored source for reusable dashboard presentation values.
It MUST use the [Design Tokens Format Module 2025.10](https://www.designtokens.org/tr/2025.10/format/), with the project profile defined below.
The dashboard MUST consume generated representations of that artifact in both CSS and chart configuration.
Generated representations MUST NOT be edited by hand or committed as another source of token values.
Visual roles and their use in the dashboard MUST follow [dashboard-design.md](dashboard-design.md); the token artifact supplies their reusable values.

Specifications SHOULD refer to semantic token names when they need to identify presentation roles, so changes to a value do not require repeating that value in prose.
Token descriptions MUST explain their intended use.
Descriptions MUST identify the presentation role rather than merely repeat the token name, and MUST explain unit interpretation when a numeric value is combined with a CSS unit.
Changing a value MUST preserve its named meaning; renaming or removing a token MUST update its consumers and specification references together.

## Token organization

Tokens MUST use nested groups with lowercase names.
Name segments MUST contain only lowercase ASCII letters, digits, and hyphens.
Dots MUST separate group names in references, not occur inside a name segment.
The build MUST reject names that collide after conversion to CSS or Rust identifiers.

### Naming

General UI tokens MUST be grouped by presentation family, with progressively more specific role, property, and state groups where those distinctions apply.
Related properties and interaction states MUST use the same nesting order within a family.
Names MUST identify the property when a role has several properties, and MUST distinguish a preferred size from a minimum or maximum constraint.
Hyphens MUST join words within a concept; independently selectable properties or states MUST use nested groups.

**Example:** `color.control.primary.background.default` and `color.control.primary.background.hover` identify the same property in different states, while `color.control.primary.text` identifies the foreground.
`layout.period-input.preferred-width` describes a flexible starting width without promising a minimum width.

Chart rendering and series-specific roles MUST be grouped under `chart`, with related decisions organized by their chart element or behavior.
Surrounding cards and descriptive content MUST use the general UI families.
Axis label properties MUST share `chart.axis.label`; series palette and fallback-color parameters MUST share `chart.series`.
Chart dimensions and pointer hit distances MUST identify their units in displayed CSS pixels; chart width MUST follow the available container width.
Chart height MUST combine a minimum viewport height with per-series hover space so larger comparisons retain their labels.
Numbered palette entries MUST denote series order without implying metric meaning or server identity.

### Shared values

Components MUST use semantic tokens for reusable visual roles.
Shared spacing and font-size scales MUST be organized under `scale`, with semantic tokens referring to them through aliases.
Consumers MUST use semantic roles rather than reference scale entries directly.
Scales SHOULD use a small, ordered set of values to make spacing and typography consistent; a distinct value MAY be retained when its presentation role requires it.
Text colors SHOULD express shared emphasis roles, with separate roles for interaction and data states when needed for clarity.
Additional primitive palettes MAY be introduced for intentionally shared color decisions.
Equal values with unrelated meanings MAY remain separate tokens; numerical equality alone MUST NOT make two roles share one token.
Component-specific tokens MUST NOT be reused for unrelated components solely because their current values match.
Independent roles that share a scale value MUST alias the common scale entry rather than alias one another.
Descriptions of shared semantic roles MUST state their intended reusable pattern.

**Example:** Input and table-cell padding can alias the same spacing scale entry through separate semantic tokens.
Changing the input-padding alias affects inputs only; changing their common scale entry affects both roles.

### Coverage

Token categories MUST cover the reusable presentation decisions used by the dashboard:

- surfaces, text, links, borders, and control states.
- focus, error, stale-data, and unavailable-data presentation.
- spacing, radii, and border widths.
- font families, sizes, weights, line heights, and tracking.
- opacity and any shadows or stacking layers used by authored presentation.
- layout size limits and responsive breakpoints.
- chart palettes, axes, grid lines, labels, and presentation dimensions.

Categories MUST describe dashboard roles rather than source server names or a fixed entity roster.
Unused categories or tokens MUST NOT be added solely for hypothetical features.

## Format profile

Each token MUST contain `$value` and `$description`, with its type supplied by an explicit token or inherited group `$type`.
Groups whose descendants share a type SHOULD declare that `$type` once so repeated declarations do not obscure the token values.
Mixed groups MAY override an inherited type where needed.
The supported value types MUST be:

- `color`, using structured sRGB components and optional alpha under the [Color Module 2025.10](https://www.designtokens.org/tr/2025.10/color/).
- `dimension`, using numeric `value` and `unit`, limited to `px` or `rem`.
- `fontFamily`, using an ordered array of family names.
- `fontWeight`, using a numeric weight.
- `number`, for unitless presentation values.
- `shadow`, using structured shadow values.

Colors, dimensions, and shadows MUST NOT be encoded as CSS strings.
Aliases MUST use complete, same-document token references such as `{color.surface.panel}`.
The build MUST resolve chained aliases and reject missing targets, cycles, and type mismatches.
The artifact MUST NOT use external references, group inheritance through `$extends`, or other reference forms outside this profile.

This profile limits the project's authored token documents; the build adapter is not required to be a general-purpose implementation of every DTCG feature.
Unsupported format features MUST fail with a diagnostic rather than be silently omitted or emitted as invalid CSS.

**Example:** A scale entry is written as `{"$type":"dimension","$value":{"value":1.5,"unit":"rem"},"$description":"Sixth step of the shared spacing scale."}`.
A semantic alias can use `{"$type":"dimension","$value":"{scale.spacing.6}","$description":"Inner padding of panels containing controls or explanatory content."}`.

Unitless ratios MAY be combined with CSS units in consumer expressions when the intended CSS unit is outside the dimension profile.
Such expressions MUST preserve the token's meaning.

**Example:** A tracking token of type `number` can be used in `calc(var(--mnm-font-tracking-eyebrow) * 1em)`.
It does not require inventing a DTCG dimension with an unsupported `em` unit.

## CSS consumption

Generated CSS MUST expose tokens as custom properties with the `--mnm-` prefix and hyphen-separated paths.
For example, `color.surface.panel` MUST map to `--mnm-color-surface-panel`.
Authored CSS MUST reference those properties for reusable presentation values.
Selectors and layout relationships MUST remain in authored CSS.

Structural values MAY remain literal where they express layout behavior rather than a reusable design choice.
Examples include `display: grid`, `width: 100%`, and a zero-margin reset.
Component-local geometry derived from observations, such as an observation marker's position, MUST remain calculated rather than become a token.

Values used where CSS custom properties are unavailable, including media-query conditions, MUST be resolved from tokens during the build.
They MUST NOT require manually duplicating breakpoint values in CSS.
Generated styles MUST be embedded in the compiled frontend and applied when the WASM application mounts.
The loading state before WASM initialization MAY use browser-default styling.

**Rationale:** CSS `var()` is limited to property values and cannot supply a media-query condition, as described in [MDN's custom property guide](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascading_variables/Using_custom_properties).
Build-time resolution keeps breakpoints in the same authored source as other presentation values.

## Chart consumption

Rust chart configuration MUST consume typed presentation values generated from the same resolved token artifact as CSS.
It MUST NOT maintain a separate handwritten palette or copies of token values.
Chart lines, legend swatches, and hover-label borders MUST use the same series-color selection for the same series.
Chart font and dimension conversion MUST preserve the declared units; CSS-relative dimensions MUST NOT silently become fixed pixels.
Plotly configuration MUST receive resolved colors and pixel dimensions from generated Rust values; it need not read CSS custom properties at runtime.
Dimensions consumed through whole-pixel Plotly.rs APIs MUST reject fractional pixel values rather than truncate them.
Native chart-library layout details without an authored override MUST NOT have unused tokens.

The number of palette entries MUST NOT limit the number of comparison series.
Any additional series-color generation MUST remain deterministic, with its adjustable color parameters supplied by tokens and its selection algorithm owned by Rust chart configuration code.
Token names MUST NOT encode collected server identities or imply that colors change metric meaning.

## Build behavior

Token generation MUST be deterministic and MUST run as part of the supported dashboard build and preview commands.
Token-only changes MUST update CSS and chart presentation even when build caches are reused.
The running local preview MUST rebuild after token-only changes, including atomic replacement of the artifact outside the dashboard crate.
The frontend MUST NOT fetch the token artifact at runtime.

Invalid tokens or values that cannot be represented by their consumers MUST fail the build with the affected token path and reason.
Generation MUST reject unresolved aliases and output-name collisions before packaging dashboard assets.
CSS and Rust outputs in one dashboard build MUST derive from the same token input.

## Accessibility

Token values MUST support readable text, visible focus indicators, and distinguishable controls and chart series at supported viewport sizes.
Color MUST NOT be the only indication of errors, stale data, unavailable observations, or series identity.
The labels and exact-value inspection required by [requirements.md](requirements.md) MUST remain available alongside color cues.
Token changes MUST be reviewed in the rendered dashboard, including control states, chart labels, and hover details.

## Boundaries

Tokens MUST define presentation only.
They MUST NOT encode source values, metric calculations, collection rules, or history validation.
Chart data ranges, gap detection, and time alignment MUST retain their domain rules independently of styling.
Test coverage for token processing and presentation integration is defined in [tests.md](tests.md#design-tokens).
