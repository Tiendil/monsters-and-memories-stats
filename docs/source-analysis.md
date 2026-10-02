# Metrics source analysis

## Inspection

The public [metrics page](https://account.monstersandmemories.com/metrics) and its [browser script](https://account.monstersandmemories.com/assets/app.js?v=2) were inspected on 2026-10-02.
These are observations of an evolving source, not promises about future data availability.
The resulting behavioral requirements are in [requirements.md](../specs/requirements.md), and acquisition is defined in [architecture.md](../specs/architecture.md#acquisition).

## Source observations

- Active Subscriptions — one global count; this is not documented as a count of distinct subscribers.
- Total Online — one global concurrent population count.
- Server cards — six cards with stable-looking IDs and human-readable names.
- DAILY ACTIVE / MONTHLY ACTIVE — per-server fields, all zero in the initial HTML and nonzero after the connected page loads its data; the initial zeros were placeholders in this inspection, while the counting unit and window boundaries remain undocumented.
- Online — concurrent population for each server.
- Starting Zones — per-server current populations, a starting-zone total, and individual named zones with IDs.
- Last 24 Hours — the initial HTML contained two identical current points per server; after LiveView updates, each server exposed 50 distinct points spanning roughly a day.
- Weekly activity — no weekly-active field found.

## Observed identities

The server IDs observed on that date were:

- `kravvin`.
- `nunavoth`.
- `vespyra`.
- `tilustra`.
- `trem`.
- `estaire`.

The zone IDs observed on that date were:

- `ailvorith`.
- `evergrove`.
- `nightharbore`.
- `nightharborw`.
- `underdocks`.

These identities describe the inspected page; ongoing server discovery is defined by [R16](../specs/requirements.md#r16-server-discovery).

## Browser verification

One browser navigation on 2026-10-02 captured both the initial HTTP response and the connected DOM.
The initial chart timestamps were 14:56:03 UTC; the connected chart endpoints were 14:56:05 UTC.
The page used Phoenix LiveView and delivered six successive metric diffs after connecting, so connection itself did not establish that every server had finished loading.
This was a separate live investigation, not a test run.
The inspection saved local data and did not repeatedly request the statistics page.

The initial response reported zero daily/monthly activity for every server.
The connected page reported the following daily/monthly values:

- Kravvin: 1,172 / 1,249.
- Nunavoth: 745 / 797.
- Vespyra: 2,003 / 2,106.
- Trem: 5,566 / 6,011.
- Estaire: 4,156 / 4,387.
- Tilustra: 3,211 / 3,300.

Both stages reported 18,866 active subscriptions and 4,488 online across the six servers.
Each connected chart contained 49 half-hour points from 2026-10-01 14:00 through 2026-10-02 14:00, followed by the current endpoint.
The historical timestamp strings omitted offsets, while the endpoint used `Z`.
The `MetricsCcuChart` implementation in the public browser script appends `Z` when an offset is absent, establishing the source UI's UTC interpretation.

[Sanitized HTML fixtures](../crates/mnm-stats-collector/tests/fixtures/README.md) preserve the two stages without session credentials or executable scripts.
The initial response is not equivalent to the connected view and is unsuitable as the collector's sole input.
The [acquisition design](../specs/architecture.md#acquisition) accounts for this under the [collection contract](../specs/requirements.md#history-and-collection).

## Direct access verification

On 2026-10-02, a separate direct-client investigation received the completed metrics without launching a browser.
The successful session received one join response followed by six LiveView diffs, including the daily/monthly counts and current online values for all six dynamically discovered servers.
Its current chart endpoints were timestamped 15:19:48 UTC.
This establishes transport feasibility, not a complete collector or a tested readiness algorithm.

The working flow was:

1. Request `/metrics` over HTTPS.
2. Retain its anonymous session cookie and extract the CSRF token, signed LiveView session information, and generated view ID from the response.
3. Open `/live/websocket` with the session context and join the channel identified by that view ID.
4. Receive the page's asynchronous rendering updates containing completed metric values.

The initial page request obtains the session information that the subsequent connection uses.
This is part of the site's LiveView protocol, not a universal requirement to fetch a page before using WebSockets.
No account login was used.
A standalone metrics REST endpoint has not been identified; the verified interface carries LiveView rendering updates rather than a dedicated metrics document.
Phoenix documents this [HTTP-to-connected-view lifecycle](https://phoenix-live-view.hexdocs.pm/Phoenix.LiveView.html#life-cycle).

## Acquisition and storage assessment

The verified direct connection supports a Rust HTTP and WebSocket collector without a browser runtime.
The client still needs to interpret the source's LiveView updates and validate completed current-state values.

The user selected snapshots of current state only.
Although the source sends online-history windows, those windows are not imported into project history.
The captured pages provide explicit current Online fields; a final historical point would be an alternative only if its last-known-state meaning were established under the source contract.

Serializing the captured current values into the snapshot schema produced an illustrative 2,640-byte JSONL record.
At the observed six servers and five zones per server, hourly records of that size would total roughly 23 MB per year before compression.
This excludes the source's chart windows entirely and is an extrapolation, not a measured future dataset or WASM size.

## Remaining verification

The collector still needs a reliable per-server completion check, including legitimate zero counts and servers with no historical points.
The browser investigation used a three-second wait, and the direct prototype checked for richer chart arrays to observe the completed response.
Neither is a production readiness algorithm.
Initial placeholder data must remain distinguishable from a completed response without requiring positive counts or a fixed number of historical points.
This must be verified before claiming the collector works.

The counting semantics of DAILY ACTIVE and MONTHLY ACTIVE remain unverified.
The source's UTC treatment of historical timestamps is supported by its browser code; whether historical buckets represent instantaneous samples or aggregates remains undocumented.
Their position at the end of a chart alone does not establish last-known-state semantics.
