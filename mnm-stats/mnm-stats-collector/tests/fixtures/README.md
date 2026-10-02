# Public metrics fixtures

Source: [Monsters & Memories metrics](https://account.monstersandmemories.com/metrics), inspected on 2026-10-02 through one browser navigation.
These captures support offline parser and acquisition tests.

- `metrics-initial.html` preserves the initial response's main content. Its daily/monthly counts are placeholders and its six charts each contain two identical current points at 14:56:03 UTC.
- `metrics-connected.html` preserves the same page after its LiveView updates. Each server has daily/monthly values and 50 online-history points, ending at 14:56:05 UTC.

Both files retain source identities, display names, labels, metric values, and chart attributes.
Each contains 18,866 global subscriptions, 4,488 total online, six server cards, and five zones per server.
Expected per-server activity values and the source-timestamp interpretation are recorded in the [source analysis](../../../../docs/source-analysis.md#browser-verification).
These observed identities are fixture data, not a production server roster.
The source's chart arrays remain in the fixtures so parser tests can verify that snapshots contain only current-state values and do not import those arrays.
`liveview.json` records the direct WebSocket investigation at 15:19:48 UTC on the same date: one join response and six successive metric updates.
It contains 18,910 subscriptions and 4,720 online, so its current counts differ from the earlier HTML captures.
Its channel identity and join/message references are replaced with fixture values; it contains no cookies, CSRF tokens, signed session values, or outgoing join credentials.
HTTP initialization in transport tests uses synthetic session values and a loopback server.
The captured protocol data is never refreshed by tests.

Sanitization keeps the original main-content structure, replaces the generated LiveView root ID with a fixed fixture ID, removes session-specific `data-phx-*` attributes, and wraps the content in a minimal local HTML document.
The original head, external scripts, navigation, cookies, CSRF token, and LiveView session credentials are not included.
The captures do not replay LiveView and must not load upstream assets.
The HTML files establish before/after parser inputs; the protocol capture also exercises the observed update sequence and readiness checks.

Fixture capture and refresh are separate reviewed source-investigation activities.
Tests must read committed local data and must never refresh these files or request their source URL.
