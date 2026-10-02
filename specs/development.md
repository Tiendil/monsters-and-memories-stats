# Development workflow

## Goal of the document

This document describes the agent development process and its review checkpoints.

## Scope

This specification covers the agent-assisted development process for this repository.
Application behavior and application architecture are outside this document's scope.

## Working agreement

`AGENTS.md` MUST direct agents to applicable specifications and the user's approval checkpoints.
Agents MUST keep implementation within the approved step and MUST request review and a user commit before the next step.
The user MUST review and commit each completed step before the next begins.
They MUST NOT make Git commits or publish changes on the user's behalf without authorization.
The repository MUST remain usable for its current stage at every handoff.
Each step MUST leave the capabilities it introduces working and verified; scaffolding MUST NOT be presented as a finished application.
Specifications MUST state the current contracts directly, without revision history, draft labels, or task-specific approval status.
Specifications MUST NOT imply that the functionality they require has already been implemented.

Task-specific implementation plans, approval records, and progress notes MUST be kept under ignored `.session/`, separate from project specifications.
Plans MUST follow the applicable specifications and approval checkpoints.
If deployment setup needs an additional user action, the agent MUST first prepare the concrete workflow/configuration for review and explain the exact remaining action.

## Governance

Depmesh MUST expose only `governs` and `governed_by`.
Both directions MUST agree for existing governed artifacts.
All specifications MUST link to `specs/meta/general.md` through these relations.
Tooling and agent instructions MUST link to this document.
Approved implementation steps MUST extend governance to all artifacts they introduce.
Governance configuration SHOULD use file/path rules to keep dependency discovery understandable without additional infrastructure.
A different discovery mechanism MAY be proposed when those rules cannot express a required relationship.

## Project commands

The Rust implementation MUST expose reusable commands under `bin/` for verification, collection, and dashboard builds.
Tool invocations and their options MUST be owned by these commands or the underlying tool configuration.
Local use, Donna checks, and GitHub Actions MUST reuse the applicable commands so their behavior does not diverge.
GitHub Actions MUST own event handling and job orchestration, while Donna MUST own the agent's check sequence and repair actions.
CI verification commands MUST report failures without requiring agent interaction.
Shared application commands MUST execute in the project's Docker development environment, including when invoked by Donna or CI.
Repository inspection, editing, Donna, and Depmesh MAY run on the host.
Setup MUST provide the required development tools before checks; checks MUST fail clearly when those tools are missing.
Containers MUST preserve reusable build caches and produce host-user-writable artifacts.

## Browser inspection

For frontend changes affecting rendered behavior, agents MUST inspect the supported running preview through Playwright MCP before handoff.
Inspection MUST cover the changed interactions and relevant desktop or narrow-screen layouts, including console and network diagnostics.
Agents MUST retain screenshots and other browser evidence under ignored `.session/playwright/` and distinguish observed behavior from assumptions.
An unavailable MCP connection MUST be reported instead of claiming browser inspection succeeded.
Automated browser regression tests remain required independently of interactive inspection.
Agents MUST stop temporary services they start before handoff unless the user asks to keep them running.

## Donna

Donna MUST discover project workflows under `workflows/` and keep runtime state under ignored `.session/donna/`.
The polish workflow MUST run deterministic checks for artifacts that exist at the current delivery stage.
Polish MUST run locally without project Git operations or invoking hosted workflows, deployments, or repository-management APIs.
Normal builds MAY resolve and download package dependencies.
Test execution MUST use only local inputs and services.
Each check MUST have a focused failure handler that exposes its diagnostics.
Every repair MUST restart the check sequence so successful completion applies to the final artifact state.
Checks MUST NOT report skipped or nonexistent application checks as passed.
Donna's own event journal is sufficient; external journaling infrastructure is not required.

## Verification

The documentation foundation MUST pass Donna workflow validation and representative Depmesh queries in both directions.
Agents MUST review artifacts against their governing specifications and the user's requirements directly, using Depmesh to discover relevant relationships.
Successful discovery commands alone MUST NOT count as a consistency judgment.
After a consistency repair, polish MUST pass and the affected relationships MUST be reviewed again.
Agents MUST review the following before handing off a step:

- changed and newly added files.
- document links.
- approval records for the implementation scope.
- requirement coverage.

Rust implementation MUST incorporate the following into polish:

- formatting.
- linting.
- focused behavior tests.
- a release WebAssembly build.

Tests SHOULD cover failure-prone contracts such as source parsing, history preservation, and metric calculations rather than mirror incidental implementation details.
This keeps regression coverage useful when implementation details change.
Implementation-specific tests MAY be used when needed to reproduce a concrete defect.
Application tests MUST follow [tests.md](tests.md) and be introduced with the behavior they verify.
Tests run by local checks and CI MUST NOT make internet requests; source investigation and deployment verification MUST remain separate from test execution.
The README MUST identify the current project state and provide navigation to the specifications.
Once tests exist, it MUST document test commands and any required dependency setup.
As working commands become available, it MUST document:

- local use.
- collection.
- deployment.
- notification setup.

## Investigation notes

Dated source findings MUST be kept in analysis documents outside `specs/`, with inspection dates and source references.
These documents MUST distinguish observations from unresolved questions and link to the relevant specifications.
Behavioral requirements and approval gates MUST remain in their governing specifications.

## Reference

The general specification rules and session skill are copied unchanged from the user-specified Feeds Fun repository.
The Donna polish structure follows its focused failure handlers and restarting checks after repairs.
The separation between GitHub Actions orchestration and project commands follows its code-checks workflow and `bin/` scripts.
The Docker preview and Playwright MCP workflow draws on the user-specified Clio repository's project launchers and browser inspection setup.
