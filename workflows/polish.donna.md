# Polish Workflow

```toml donna
kind = "donna.lib.workflow"
start_operation_id = "validate_workflows"
```

Run the available deterministic repository checks in order.
Each failure has a focused repair action; every repair restarts the check sequence so success describes the final files.
Checks cover Donna and GitHub workflow syntax, Depmesh configuration, Compose and shell syntax, Rust formatting and linting, native behavior tests, browser/build integration, and the release WASM build.

## Validate Donna workflows

```toml donna
id = "validate_workflows"
kind = "donna.lib.run_script"
save_stdout_to = "workflows_stdout"
save_stderr_to = "workflows_stderr"
goto_on_success = "validate_depmesh"
goto_on_failure = "fix_workflows"
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
donna -p llm validate --all
```

## Fix Donna workflows

```toml donna
id = "fix_workflows"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("workflows_stdout") }}
{{ donna.lib.task_variable("workflows_stderr") }}
```

Repair the reported workflow syntax or transition problem within the authorized scope.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Validate Depmesh configuration

```toml donna
id = "validate_depmesh"
kind = "donna.lib.run_script"
save_stdout_to = "depmesh_stdout"
save_stderr_to = "depmesh_stderr"
goto_on_success = "check_environment"
goto_on_failure = "fix_depmesh"
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
depmesh -p llm relations
```

## Fix Depmesh configuration

```toml donna
id = "fix_depmesh"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("depmesh_stdout") }}
{{ donna.lib.task_variable("depmesh_stderr") }}
```

Repair the reported Depmesh configuration problem within the authorized scope.
Keep only `governs` and `governed_by`.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Check development environment

```toml donna
id = "check_environment"
kind = "donna.lib.run_script"
save_stdout_to = "environment_stdout"
save_stderr_to = "environment_stderr"
goto_on_success = "check_actions"
goto_on_failure = "fix_environment"
timeout = 120
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/check-environment.sh
```

## Repair development environment

```toml donna
id = "fix_environment"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("environment_stdout") }}
{{ donna.lib.task_variable("environment_stderr") }}
```

Repair the reported Compose configuration or shell syntax error.
Dependency installation belongs to the explicit setup command, not a check.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Check GitHub workflows

```toml donna
id = "check_actions"
kind = "donna.lib.run_script"
save_stdout_to = "actions_stdout"
save_stderr_to = "actions_stderr"
goto_on_success = "check_format"
goto_on_failure = "fix_actions"
timeout = 120
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/check-actions.sh
```

## Repair GitHub workflows

```toml donna
id = "fix_actions"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("actions_stdout") }}
{{ donna.lib.task_variable("actions_stderr") }}
```

Repair the reported GitHub Actions syntax, expression, or action-input error.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Check Rust formatting

```toml donna
id = "check_format"
kind = "donna.lib.run_script"
save_stdout_to = "check_format_stdout"
save_stderr_to = "check_format_stderr"
goto_on_success = "check_lints"
goto_on_failure = "fix_check_format"
timeout = 120
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/check-format.sh
```

## Repair: check rust formatting

```toml donna
id = "fix_check_format"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("check_format_stdout") }}
{{ donna.lib.task_variable("check_format_stderr") }}
```

Repair the reported Rust formatting.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Check Rust lints

```toml donna
id = "check_lints"
kind = "donna.lib.run_script"
save_stdout_to = "check_lints_stdout"
save_stderr_to = "check_lints_stderr"
goto_on_success = "test_native"
goto_on_failure = "fix_check_lints"
timeout = 600
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/check-lints.sh
```

## Repair: check rust lints

```toml donna
id = "fix_check_lints"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("check_lints_stdout") }}
{{ donna.lib.task_variable("check_lints_stderr") }}
```

Repair the reported native or WASM lint failures.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Test native behavior

```toml donna
id = "test_native"
kind = "donna.lib.run_script"
save_stdout_to = "test_native_stdout"
save_stderr_to = "test_native_stderr"
goto_on_success = "test_browser"
goto_on_failure = "fix_test_native"
timeout = 600
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/test.sh
```

## Repair: test native behavior

```toml donna
id = "fix_test_native"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("test_native_stdout") }}
{{ donna.lib.task_variable("test_native_stderr") }}
```

Repair the reported model or CLI behavior using local test inputs.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Test browser and build integration

```toml donna
id = "test_browser"
kind = "donna.lib.run_script"
save_stdout_to = "test_browser_stdout"
save_stderr_to = "test_browser_stderr"
goto_on_success = "build_dashboard"
goto_on_failure = "fix_test_browser"
timeout = 1200
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/test-browser.sh
```

## Repair: test browser and build integration

```toml donna
id = "fix_test_browser"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("test_browser_stdout") }}
{{ donna.lib.task_variable("test_browser_stderr") }}
```

Repair the reported browser, embedded-history, download, or rebuild failure. Use only local test inputs; tool installation is a separate setup step.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Build release dashboard

```toml donna
id = "build_dashboard"
kind = "donna.lib.run_script"
save_stdout_to = "build_dashboard_stdout"
save_stderr_to = "build_dashboard_stderr"
goto_on_success = "finish"
goto_on_failure = "fix_build_dashboard"
timeout = 600
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/build-dashboard.sh
```

## Repair: build release dashboard

```toml donna
id = "fix_build_dashboard"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("build_dashboard_stdout") }}
{{ donna.lib.task_variable("build_dashboard_stderr") }}
```

Repair the reported release WASM build failure.
After the repair, {{ donna.lib.goto("validate_workflows") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Finish

```toml donna
id = "finish"
kind = "donna.lib.finish"
```

The current deterministic checks passed.
This result does not establish specification consistency or application correctness.
Report the checks performed, then follow the caller's remaining review instructions.
