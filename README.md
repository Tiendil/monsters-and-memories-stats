# Monsters & Memories historical statistics

A Rust collector and static dashboard for the public [Monsters & Memories metrics](https://account.monstersandmemories.com/metrics), using repository JSONL storage, GitHub Actions, and GitHub Pages.

The workspace provides the Rust HTTP/WebSocket collector, shared history model, local history-validation CLI, and a Leptos dashboard. The dashboard embeds validated history in WASM and presents metric plots, comparisons, ratios, correlations, and complete-history JSON downloads. Workflows provide code checks, hourly collection, and Pages deployment; repository setup and live acceptance are described below. The committed history starts empty.

- [Specification index](specs/intro.md)
- [Requirements](specs/requirements.md)
- [Architecture](specs/architecture.md)
- [Development workflow](specs/development.md)
- [Tests and network access](specs/tests.md)
- [Source analysis](docs/source-analysis.md)
- [Sanitized source fixtures](mnm-stats/mnm-stats-collector/tests/fixtures/README.md)
- [Agent instructions](AGENTS.md)

## Setup

Install Docker Engine and Docker Compose, then run:

```bash
./bin/dev.sh setup
```

Setup builds the Linux x86_64 development image, fetches locked Cargo dependencies, and pulls the pinned Playwright MCP image. The image includes Rust 1.98.0, rustfmt, Clippy, the WASM target, Trunk 0.21.14, wasm-bindgen 0.2.129, Python, actionlint 1.7.12, and matching Chrome/ChromeDriver 154.0.8037.92. Host Rust, Node, and browser installations are not required. Browser tooling is for tests and inspection; collection uses Rust HTTP/WebSocket clients.

`rust-toolchain.toml`, the dashboard's `Trunk.toml`, `Cargo.lock`, and the Dockerfile record the corresponding pinned versions. Update the Dockerfile alongside toolchain/build-tool upgrades. Setup may access package registries and tool downloads; test execution uses local fixtures and never requests source metrics.

Build caches live in ignored `.cache/docker/` and `target/docker/`. Container outputs use your user/group IDs. Existing host build artifacts are kept separate. Use `./bin/dev.sh -- COMMAND...` for additional commands inside the same image.

GitHub jobs reuse `.github/actions/setup/` to build this image with a Docker layer cache and restore Cargo caches. `./bin/dev.sh setup --image-ready` fetches dependencies after that image has been loaded; it skips rebuilding the image and pulling the interactive MCP service. Application commands are the same locally and in CI.

## Dashboard views

“Now” appears above the view controls and shows the latest collected totals across all servers, independently of filters and comparisons. Subscribers is the global active-subscription count. Its card links open Overview and jump to the corresponding plots. Overview contains full-width plots labeled Online, Daily active (DAU), Monthly active (MAU), and Subscribers, in that order. Player activity and Engagement expose the remaining detailed charts while preserving the selected server, range, and comparison. Player activity combines starting-zone totals and individual zones in one full-width chart. Its Starting zones dropdown independently toggles All Zones and zones discovered across the full archive, using the same checkbox interaction as Servers. All Zones is the total across all reported zones and can appear alongside individual zones. Each line identifies its zone, server scope, and comparison period. Zone selections persist across sections; clearing them shows an explicit recovery action.

Player activity also includes **Server population share** and an **Activity heatmap**. Shares divide each server's online count by the complete snapshot total. In this chart, All Servers shows every individual server; selecting a subset keeps the same denominator. Missing servers and zero totals have no share value.

The heatmap shows mean observed online counts for each weekday/hour UTC bucket in the selected interval. Each selected server scope and comparison period has its own panel, using a shared color scale. All Servers uses summed online counts. Hover shows the mean, exact sum, and sample count. These are equally weighted sampled averages with potentially uneven coverage, not continuous or time-weighted population measurements. Missing cells stay blank, measured zeros stay zero, and period comparisons retain each sample's actual UTC weekday and hour. Starting zones only controls the starting-zone chart.

Use Servers to toggle individual servers and All Servers independently. Use the date menus to compare the primary range with the previous period, previous year, or custom periods. Chart colors stay attached to the selected series during the session, with each legend entry on its own row. Series-colored solid lines connect observations less than 3 hours apart; subdued gray solid lines connect intervals from 3 hours to less than 24 hours. Longer intervals remain empty. Chart hover provides exact values and UTC observation times to the minute; “Download JSON” always includes the complete archive.

Presentation follows [the dashboard design](specs/dashboard-design.md) and [design tokens](specs/design-tokens.md). IM Fell English is distributed locally with its [SIL Open Font License](mnm-stats/mnm-stats-dashboard/fonts/OFL.txt) and [source attribution](mnm-stats/mnm-stats-dashboard/fonts/README.txt).

## Local use

For a populated preview, from bash or fish:

```bash
./bin/serve-dashboard.sh --demo
```

Open `http://127.0.0.1:8080/`. The page identifies synthetic data, with colored sections, subdued gray sections, and day-long breaks visible in the default last-7-days range, plus older samples for comparisons. Demo history is generated by Rust under `.session/preview/`; it never changes `data/history.jsonl` or contacts the public source. Ctrl+C stops the preview container.

To preview existing history:

```bash
./bin/serve-dashboard.sh --history /path/to/history.jsonl
./bin/serve-dashboard.sh --history data/history.jsonl --port 8081
```

Pass any existing JSONL file, including one outside the repository. The launcher mounts its parent directory read-only, prints the selected container path and observation count, and watches history changes. `MNM_STATS_HISTORY` remains supported. Without an override, the input is `data/history.jsonl`, which starts empty. Old samples require **All time** if they fall outside the default last 7 days. Missing or invalid inputs fail with diagnostics.

Source, history, and design-token changes rebuild the preview and reload the browser. Preview output is under `.session/preview/dist/`, separate from release output. To validate history or build release assets:

```bash
./bin/validate-history.sh
./bin/build-dashboard.sh
./bin/build-dashboard.sh --public-url /monsters-and-memories-stats/
```

Release assets go to ignored `dist/`. `MNM_STATS_HISTORY` also selects a different build input. Build output and JSON downloads preserve all observations, with no separate runtime metrics request. Paths passed to general container commands should be repository-relative; preview history paths may be outside the repository.

## Browser inspection

The checked-in `.codex/config.toml` configures Playwright MCP using `bin/playwright-mcp.sh`. After setup, restart the Codex client if necessary and check `/mcp` for `playwright`; project configuration requires a trusted project. The launcher starts a temporary stdio browser container on the same Compose network as the preview. It does not publish an MCP HTTP port or start the dashboard automatically.

Start the preview, then navigate the MCP browser to `http://dashboard:8080/`. This serves the same application as the host URL, including when the host port is overridden. Use MCP to inspect controls, take desktop/mobile screenshots, inspect console errors and network requests, and download the embedded history. Evidence is mounted at `/workspace/.session/playwright` inside the browser and `.session/playwright/` on the host. Stop temporary previews after inspection.

## Styling

[Design tokens](mnm-stats/mnm-stats-dashboard/design-tokens.tokens.json) are the shared source for CSS and chart presentation, following the project's [DTCG format profile](specs/design-tokens.md). Author semantic values with Tailwind references such as `{tailwind.color.orange.800}`, `{tailwind.spacing.4}`, and `{tailwind.radius.md}`; keep selectors and layout rules in `mnm-stats/mnm-stats-dashboard/style.css`. CSS uses `var(--mnm-...)`; media-query conditions use `token(breakpoint.medium)` so the build can substitute a concrete dimension.

The development image installs Tailwind CSS 4.3.3's official `theme.css` and license from a checksum-pinned npm archive; run `./bin/dev.sh setup` after the image changes. Tailwind supplies the values without a utility-class compiler or Node runtime.

The Rust build reads that theme, resolves its references and semantic aliases, and generates CSS, typed chart values, and a self-contained `tokens.resolved.json` under Cargo's ignored build output. Its OKLCH colors are converted once to sRGB for matching CSS and Plotly output; do not copy those generated numbers back into the authored palette.

Generated CSS is compiled into WASM and inserted into the page when the application mounts, with no separate stylesheet request. The initial loading message uses browser-default styling until WASM starts. Token data is never fetched at runtime, and generated styles should not be edited. Plotly chart dimensions and font sizes use whole CSS pixels; other CSS dimensions retain their declared `px` or `rem` units.

Rust constructs Plotly figures and exact hover text through Plotly.rs. The page loads Plotly.js cartesian 3.0.1 from [Plotly's official CDN](https://cdn.plot.ly/plotly-cartesian-3.0.1.min.js) and credits Plotly in the footer. The script URL in the dashboard's `index.html` pins the version compatible with Plotly.rs 0.14.1; update both together when upgrading. Ordinary previews and deployed charts require access to that CDN.

Browser tests load the same CDN script as previews and deployed pages, so they require access to that CDN. All metric inputs remain local fixtures or synthetic data; tests never contact the original statistics service or refresh source fixtures. The Plotly bundle is not committed or packaged with the site.

Both build and preview commands use the authored token file by default. For an isolated local copy, use `env MNM_STATS_TOKENS=/path/to/tokens.json ./bin/serve-dashboard.sh --demo` (works in Bash and Fish). The launcher mounts its parent directory read-only and watches atomic file replacements. The same variable works with `./bin/build-dashboard.sh`.

## Dashboard

History contains hourly current-state snapshots only. Pre-collection history and missed intervals are unavailable. The source does not provide deduplicated global activity or per-server subscription counts. Daily/monthly activity fields have unverified counting semantics; subscriptions are not assumed to count distinct people.

“All Servers” adds each observed server's daily/monthly counts without deduplication. One account may contribute on multiple servers, so these sums are not counts of unique game-wide players.

The dashboard plots daily and monthly activity, global subscriptions, online population, starting-zone totals and individual zones, and six activity ratios. The shared time range applies to ordinary charts, entity comparisons, and correlations. Server choices and zone charts come from the complete embedded history, including historical entities. Labels show display names when available and IDs otherwise.

Engagement stacks three full-width charts: **Daily participation** (DAU/MAU), **Online presence** (average online / DAU or MAU), and **Activity relative to subscribers** (DAU, MAU, or average online / global subscriptions). The latter two have independent Metrics checkbox selectors, following Servers. Online presence starts with average online / DAU; subscriber activity starts with DAU and MAU. Choices persist across tabs and filters, and empty selections offer an explicit reset. Legends identify each metric, server, and period.

Average online uses an equally weighted arithmetic mean of available samples within each original UTC day and selected period. It is paired with the denominator from the last contributing observation and plotted at that observation's timestamp. All Servers averages per-snapshot online totals. Absent servers and missing hours contribute no samples; partial days remain partial, and comparisons aggregate their own original dates. For example, samples of 10 and 30 give a mean of 20; dividing by a final DAU count of 100 gives 20%. A zero final denominator is unavailable. This is sampled online presence, not measured playtime; average online / MAU is a daily indicator, not monthly playtime. Daily points follow the same connection-interval rules as other observations.

Every chart includes a short explanation. [GameAnalytics](https://docs.gameanalytics.com/events-metrics-and-filtering/metrics/#engagement) supplies definitions for DAU, MAU, and DAU/MAU (often called stickiness). Those definitions explain common terminology; M&M's counting windows and deduplication semantics remain unverified. DAU/MAU does not establish cohort retention, and ratios to subscriptions do not establish subscriber conversion or the fraction of subscribers playing. Wikipedia explains the general concepts of [concurrent users](https://en.wikipedia.org/wiki/Concurrent_user) and [Pearson correlation](https://en.wikipedia.org/wiki/Pearson_correlation_coefficient). Definition links open in new tabs.

An omitted individual zone is unavailable for that server and observation, leaving a chart gap. Its all-server value is also unavailable if any observed server omits it, so a partial sum is not shown as a complete total. Starting-zone totals sum the reported rows; an empty list validated against a published zero total contributes zero.

Use **Servers** to toggle any number of servers and the independent **All Servers** sum. The selection persists across sections and period comparisons. Clear every checkbox to hide all series; **Show All Servers** restores the default. The dropdown supports keyboard selection, Escape, and clicking outside to close.

Choose a primary **Time range** preset or apply a **Custom range** of inclusive UTC dates. **Today** starts at UTC midnight and ends now; **Yesterday** covers the complete preceding UTC day. Use **Compare** beside it to choose:

- **Previous period:** the same duration immediately before the primary range; Today compares with the same elapsed part of yesterday.
- **Year over year:** the corresponding dates one calendar year earlier.
- **Custom period:** add one or more UTC date ranges; their lengths may differ.
- **Disable comparison:** return to the primary range alone. The primary menu can also enable or disable comparison.

**Match exact date** is the default. **Match day of week** moves secondary starts onto the primary start's weekday: backward for the previous period to avoid overlap, or to the nearest matching weekday for yearly/custom periods. The menu and legends show the resulting dates. Switching back restores custom dates as entered.

Changing the primary range updates automatic comparisons; custom selections persist. Complete months align by day of month, complete years and exact year-over-year comparisons by calendar date, and other ranges by elapsed time. Missing leap days remain gaps. Custom dates apply with **Apply range** or **Add period**; remove secondary periods inside the comparison menu. There is no two-period limit and no shortcut bindings.

Period comparisons plot each selected server for each period, with global subscriptions once per period. Correlations use the primary range separately for each selected entity. Incomplete periods are not extrapolated. Connections use the same interval styles as ordinary charts, breaking at unavailable values, absent calendar dates, and intervals of 24 hours or more. Lines are visual guides; they add no observations to hover details, calculations, or downloads.

Hover a plotted point to see its exact value and series name on the first line, with its original UTC collection date and time to the minute on the second, including in comparisons. Overlapping points show their individual details. The **Download JSON** action provides all recorded observations with full timestamp precision. Counts are exact; ratios show percentages rounded to two decimal places. Time-series tooltips contain only the value, series name, and timestamp, without calculation or sample-coverage details. Tooltip names stay on one line without an authored width or character limit, including on heatmaps. Zero denominators produce gaps, and ratios may exceed 100 percent. Narrow screens can scroll comparison charts horizontally when needed.

Correlations use Pearson's r on the last jointly available observation per UTC day, show the paired-day count, and require at least three days with variation in both metrics. Correlation does not establish causation, and overlapping source activity windows limit interpretation.

**Download JSON** always includes every observation in the loaded build, regardless of filters or comparisons. The header summarizes the complete history's first date, latest UTC date and time, and record count, with a link to M&M’s public statistics and collection described as roughly hourly. An empty repository history produces an empty dashboard until observations are collected.

## Collection

Collect the current public state into the repository history:

```bash
./bin/collect.sh --history data/history.jsonl
```

Each run initializes an anonymous HTTP session and receives the page's LiveView WebSocket updates. The collector discovers servers and each server's starting zones from the rendered page and validates required metrics, names, unique identities, counts, and published totals. Zone lists may differ between servers and change between observations; omitted zones remain absent rather than becoming zero. An empty zone list is accepted only with a published zero total. HTTP, WebSocket, readiness, and lock waits are bounded. A failed collection exits with a diagnostic and leaves history unchanged.

Both requests identify the collector with a User-Agent such as `mnm-stats-collector/abc123def456 (branch=main; +https://github.com/Tiendil/monsters-and-memories-stats)`. Build-time `MNM_STATS_BUILD_REVISION` and `MNM_STATS_BUILD_BRANCH` supply the commit hash and branch; CI passes the actual checked-out revision, and Compose forwards these values into the build. The displayed revision uses 12 characters. Local builds without metadata use `mnm-stats-collector/dev` and omit the branch, while retaining the project URL. Changing build metadata updates cached builds automatically.

Readiness requires a complete server batch and replacement of every server's initial chart payload. This is evidence from the observed source protocol, not an explicit upstream completion flag. Zero activity and empty chart payloads are accepted when that replacement is observable. If a server's completed output is indistinguishable from its initial placeholder, collection fails rather than storing an uncertain zero. An upstream semantic change with unchanged observable structure cannot be detected reliably. See the [source analysis](docs/source-analysis.md#collector-verification) for the evidence and limitation.

Only current-state fields enter the snapshot; chart history is not imported. One new UTC hour adds one JSONL record. A previously recorded hour is a successful no-op, and missed hours remain gaps. A stable `.jsonl.lock` sidecar serializes local writers while `.jsonl.tmp` stages an atomic replacement. These files are ignored by Git; leave the lock file in place while collectors may be running. An interrupted staging file is overwritten by the next successful write.

For a local source fixture server, `--source URL` replaces the public URL. To exercise the full rendering, readiness, parsing, and storage path without network access, replay the committed protocol fixture into an isolated file:

```bash
mkdir -p .session/manual-replay
./bin/collect.sh --history .session/manual-replay/history.jsonl \
  --replay mnm-stats/mnm-stats-collector/tests/fixtures/liveview.json \
  --observed-at 2026-10-02T15:20:00Z
```

`--observed-at` is available only with fixture replay. Live collection always uses its actual collection time. A replay never falls back to the public source, and its synthetic timestamp must not be used for repository history.

## Checks

```bash
./bin/check-environment.sh
./bin/check-actions.sh
./bin/check-format.sh
./bin/check-lints.sh
./bin/test.sh
./bin/test-build-metadata.sh
./bin/test-browser.sh
./bin/build-dashboard.sh
```

Native tests cover JSONL validation and exports, collection and history preservation, dashboard aggregation, ratios, daily correlation sampling, range boundaries, calendar alignment, and missing observations. Browser tests exercise every chart family, all range presets, exact values, historical servers, comparisons with more than two series, and complete downloads under filters. They also verify root/subpath hosting, runtime asset requests, history-only cached rebuilds, failed builds preserving the last site, and the development preview with watched history updates. The browser clock is fixed by a small test-only clock stub; application logic and test assertions are Rust. Tests use loopback servers and keep scratch inputs, browser profiles, and logs under ignored `.session/tests/`. They never modify `data/history.jsonl` or captured source fixtures.

The notification-probe test uses a local malformed fixture. `check-actions.sh` uses actionlint to check workflow syntax, expressions, and action inputs without running a workflow.

Token tests cover Tailwind theme imports, reference changes, structured values, aliases, invalid inputs, name collisions, self-contained DTCG output, and equivalent CSS/Rust output. Browser tests also check computed styles, breakpoints, matching chart/legend/hover colors beyond the explicit palette, cached token-only builds, and preview reloads after token-only atomic replacements. They use isolated token copies and never modify the authored token file.

Donna separates regular code checks from build validation. Both workflows run locally with focused repair actions, without project Git operations or hosted workflows. Normal build dependency resolution and downloads are allowed. Tests use local metric inputs and must not contact the original statistics service; third-party runtime assets may load from the internet.

For regular code changes, run configuration checks, formatting, linting, and native tests:

```bash
donna -p llm status
donna -p llm list
depmesh -p llm relations
donna -p llm run @/workflows/polish.donna.md
```

When changing build logic, also run collector build-metadata checks, browser/build integration (including repeated history/token rebuilds and preview checks), and the release WASM build:

```bash
donna -p llm run @/workflows/polish-build.donna.md
```

Build logic includes build scripts, packaging and embedding, preview rebuilding, build metadata, toolchain/dependency configuration, and their verification commands and workflows. Ordinary application code, UI copy, styles, token values, and history-data changes use regular polish; frontend changes still receive Playwright MCP review. Test compilation and linting may compile code during regular polish. CI retains the complete check coverage through the same commands.

Depmesh exposes only `governs` and `governed_by`. Agents use those relationships to review changes against the specifications. Each implementation step ends with user review and a commit before the next begins; task plans and approval records stay under `.session/`.

## GitHub operation

### Workflows

- **Code checks** (`code-checks.yml`) runs the shared checks on pull requests and manual dispatch. It has read-only repository access and does not publish a site or collect metrics.
- **Collect metrics** (`collect.yml`) runs at minute 17 of every UTC hour and supports manual dispatch on the default branch. It checks out the latest default branch, collects one current-state observation, validates history, and commits only `data/history.jsonl` using `GITHUB_TOKEN`. Unchanged history produces no commit. Running collectors are serialized and are not canceled by another collection trigger.
- **Publish dashboard** (`pages.yml`) runs on pushes to `main`, successful completion of **Collect metrics** on `main`, and manual dispatch on the default branch. It checks out the latest default branch, builds the complete history into WASM using the configured Pages path, and uploads/deploys through the official Pages actions. Only the deployment job has `pages: write` and `id-token: write` permissions.

The default branch is `main`; update the two branch filters in `pages.yml` if it is renamed. Code and history stay on the same branch. Generated site assets are published as a Pages artifact, not committed to a deployment branch.

Collector commit subjects use `collector: record snapshot at <observed_at>`, for example `collector: record snapshot at 2026-10-02T20:39:21.494914513Z`. The timestamp comes from the latest stored observation in UTC, preserving fractional seconds, so publication retries still identify the collected data.

The collection-completion trigger is essential: a push using `GITHUB_TOKEN` does not start another push workflow. The `workflow_run` event instead starts Pages after the collector has published its data commit. Both workflow files must exist on the default branch. [GitHub token behavior](https://docs.github.com/en/actions/concepts/security/github_token), [workflow-run events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run).

Pages serializes the entire build/deploy cycle without canceling a running deployment. Every queued run checks out the latest branch after the preceding cycle finishes, so delayed older triggers cannot publish an older checkout over a newer deployment. Invalid history or a failed build prevents deployment and leaves the last published site in place. Failed-collection runs use separate concurrency groups so they cannot replace a pending successful collection's deployment. GitHub may replace other pending runs; the surviving run still builds the latest branch. [Concurrency behavior](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).

### Setup

1. Review and publish the workflows to `main`.
2. In repository **Settings → Pages → Build and deployment**, select **GitHub Actions** as the source. The workflow reads this setting; it does not enable Pages automatically. Allow deployments from `main` in the `github-pages` environment. [Custom Pages workflow setup](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
3. Ensure Actions can run and repository rules permit the collector's `GITHUB_TOKEN` to push the history commit. The workflow explicitly requests `contents: write` for collection. No personal access token is needed. If a branch rule rejects the bot, review that rule with the maintainer; the workflow does not bypass it.
4. Configure native failure notifications as described below, then perform the acceptance checks.

After a successful deployment, the expected default URL is [tiendil.github.io/monsters-and-memories-stats](https://tiendil.github.io/monsters-and-memories-stats/). The **Publish dashboard** run reports the actual URL, including any configured custom domain.

### Failure notifications

The responsible maintainer must enable GitHub Actions email or web notifications in their account's notification settings; failure-only notifications are sufficient. A repository watch alone does not establish delivery. Manual-run notifications go to the person who triggers that run. Scheduled notifications go to the workflow's initial creator, then to the most recent cron editor, or to the person who re-enables a disabled schedule. If ownership changes, the intended recipient should re-enable the schedule and verify their settings. [GitHub notification rules](https://docs.github.com/en/actions/concepts/workflows-and-actions/notifications-for-workflow-runs).

To verify delivery, that maintainer should manually run **Collect metrics** on `main` with **verify_failure_notification** selected. This deliberately replays a malformed local fixture and fails; it never contacts the metrics page, changes repository history, or builds/deploys Pages. Confirm receipt of the failure notification and that the existing dashboard remains available. Ordinary collection leaves the option unchecked. A failed run badge is not proof that a notification arrived.

### Recovery and scheduling limits

Schedules are best-effort: runs can be delayed or dropped, and public repositories' schedules may be disabled after 60 days without repository activity. The minute-17 offset avoids the documented start-of-hour load peak but does not guarantee timely execution. Re-enable a disabled **Collect metrics** workflow from the Actions tab and run it manually once; review the notification recipient after re-enabling. Missed intervals remain gaps. [Schedule limits and recovery](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule).

For source or validation failures, inspect the failing step, review the source contract and fixtures, and ship the fix through ordinary code review. Repeated failures stay visible until fixed. Do not bypass validation or replace earlier observations. The existing dashboard remains usable and shows the latest collection time from its embedded history.

For a rejected history push, the run fails without force-pushing and retains `collection-history-RUN_ID-ATTEMPT` for 30 days. The artifact contains the complete local history, including the unpublished observation. Download it before retrying. Compare it with the latest branch history; preserve any missing observation in chronological order, retain existing records, reject duplicate UTC hours, and validate the result with `./bin/validate-history.sh PATH` before a reviewed recovery commit. Never replace the branch's history wholesale with the artifact. An ordinary retry collects the current hour and cannot recreate a lost earlier sample.

If collection succeeds but Pages fails, fix the build or Pages configuration and manually run **Publish dashboard**. Its fresh checkout rebuilds all committed history. Invalid data cannot replace the last valid dashboard.

### Deployment acceptance

Local checks do not establish hosted operation. Before leaving collection unattended, retain evidence of:

- A passing manual **Code checks** run.
- A passing manual **Collect metrics** run and its history-only commit.
- A passing scheduled collection and preservation of earlier observations.
- An automatically triggered **Publish dashboard** run containing the bot's committed observation.
- The hosted dashboard's latest collection time and downloaded `history.json` matching the complete committed history used by that build, including after a history-only update.
- A received notification from the controlled failure probe, with the last valid page still available.
