# Project dictionary

## Goal of the document

This document defines terminology shared by project specifications.

## Scope

The dictionary covers domain terms used across the project's collection and dashboard contracts.
It does not define the upstream counting semantics that the public source leaves unspecified.

## Collection terms

- `source` — the public [Monsters & Memories metrics page](https://account.monstersandmemories.com/metrics) from which the project obtains metrics.
- `source contract` — the reviewed expectations that determine whether source data can be interpreted as a valid project observation.
- `observation` — the source's current reported metric values obtained during one successful collection, associated with the actual collection timestamp.
- `snapshot` — the stored representation of one observation.
- `history` — the retained chronological sequence of observations.
- `last known state` — the latest state reported by the source for a metric, as established by its source contract; a final historical bucket does not qualify solely because it is the newest point.

## Metric terms

- `DAU` — the project's shorthand for the source's daily active count, without asserting counting semantics that the source has not established.
- `MAU` — the project's shorthand for the source's monthly active count, with the same limitation on counting semantics.
- `active subscriptions` — the global subscription count reported by the source, which is not assumed to count distinct people.
- `online population` — the concurrent population reported by the source within a stated scope.

## Presentation terms

- `design token` — a named presentation decision stored as data for use by specifications and implementation.
- `semantic token` — a token named for a stable UI role, such as panel background or warning text.
- `primitive token` — a palette or scale value that can supply semantic tokens.
- `token artifact` — the machine-readable source of design token values.
