# Metrics source analysis

## Inspection

The public [metrics page](https://account.monstersandmemories.com/metrics) and its [browser script](https://account.monstersandmemories.com/assets/app.js?v=2) were inspected on 2026-10-02.
These are observations of an evolving source, not promises about future data availability.
The resulting behavioral requirements are in [requirements.md](../specs/requirements.md), and acquisition decisions are in the [architecture proposal](../specs/architecture.md).

## Source observations

- Active Subscriptions — one global count; this is not documented as a count of distinct subscribers.
- Total Online — one global concurrent population count.
- Server cards — six cards with stable-looking IDs and human-readable names.
- DAILY ACTIVE / MONTHLY ACTIVE — per-server fields, all zero in the fetched HTML; the source does not explain their counting unit, window boundaries, or whether these zeros are placeholders.
- Online — concurrent population for each server.
- Starting Zones — per-server current populations, a starting-zone total, and individual named zones with IDs.
- Last 24 Hours — an embedded chart array with timestamps and online counts; each fetched array contained two identical points, not a recoverable day of history.
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

The page uses Phoenix LiveView, so the initial HTML alone does not prove equivalence with the connected browser view.
Equivalence remains unresolved, particularly for activity counts and chart history.
The [collection contract](../specs/requirements.md#history-and-collection) defines the verification and approval gate for acquisition.
