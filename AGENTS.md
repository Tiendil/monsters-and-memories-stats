# Agent instructions

## Working agreement

- Read `specs/meta/general.md` and the [specification index](specs/intro.md), then the specifications relevant to the task in full.
- Prefer subsections and lists instead of tables in specifications.
- Use `specs/requirements.md` for product requirements, `specs/architecture.md` for architecture, and `specs/development.md` for the development workflow and review checkpoints.
- State current contracts directly in specifications; keep revision history and task-specific approval status in `.session/`.
- Follow [tests.md](specs/tests.md) when adding or running tests. Tests must make no internet requests and must use local fixtures or synthetic data; never request the public statistics page or refresh fixtures from a test run.
- Discuss major decisions with the user and obtain approval before implementing them. Writing a specification does not grant implementation approval.
- Work in bounded steps that leave the repository consistent. At the end of each step, report changes and checks, and ask the user to review and commit before proceeding.
- Do not stage, commit, push, deploy, or change repository settings unless the user authorizes that action. Automated collector data commits follow the application's collection contract.
- Keep the design small. Prefer existing libraries, one source of truth, and direct code over frameworks or abstractions for hypothetical needs.
- Collector and dashboard application logic must be Rust. Workflow YAML, minimal shell orchestration, HTML/CSS, and generated WebAssembly JavaScript bindings are supporting artifacts.

## Tools

Use the installed Donna and Depmesh tools from the repository root.
Their built-in documentation describes their command syntax.

At the start of a session, read `donna -p llm skill usage` and `depmesh -p llm skill usage`, then run:

```bash
donna -p llm status
donna -p llm list
depmesh -p llm relations
```

Continue relevant pending Donna work before starting another workflow.
Do not reset Donna with `new-session` without an explicit user request.
Before editing an existing artifact, query `depmesh -p llm dependencies @/path/to/artifact` and read the returned governing specifications.
When editing a specification, also inspect its `governs` results for affected artifacts.
Maintain both governance directions, using only `governs` and `governed_by`.
Add implementation coverage to Depmesh when approved implementation files are introduced.

Run `donna -p llm run @/workflows/polish.donna.md` for deterministic checks before each handoff.
Follow its action requests and use the exact completion commands Donna returns.
Review changed artifacts against their governing specifications and the user's requirements using Depmesh guidance above.
Polish gives each check a focused repair action and restarts the sequence after repairs.
The current checks cover workflows, governance configuration, Rust formatting and linting, native tests, browser/build integration, and the release WASM build.
Extend the checks alongside approved implementation and report only coverage that exists.

Use `rg` for file/text discovery and `difft --display=inline --color=never` for reviewing edits when available.
Read specifications in full; search excerpts are only discovery hints.
Keep session state under ignored `.session/`.
Use the [session skill](.agents/skills/session/SKILL.md) when creating task scratch files or when the user requests a new session.
Do not install dependencies or create the application scaffold before the corresponding design approval.

## Reference project

The user named `/home/tiendil/repos/mine/feeds.fun` as inspiration for specifications and tooling.
Its instructions apply to that repository; its Docker, Taskwarrior, and other project-specific machinery are not requirements here.
This repository's development environment, current-state storage design, and HTTP/WebSocket acquisition are defined in `specs/architecture.md`.
