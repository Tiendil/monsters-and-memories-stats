# Polish Workflow

```toml donna
kind = "donna.lib.workflow"
start_operation_id = "validate_workflows"
```

Run the available deterministic repository checks in order.
Each failure has a focused repair action; every repair restarts the check sequence so success describes the final files.
The current checks cover workflow syntax and Depmesh configuration loading.
Rust formatting, linting, runtime checks, tests, and release builds belong here when the approved implementation introduces them.

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
goto_on_success = "finish"
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

## Finish

```toml donna
id = "finish"
kind = "donna.lib.finish"
```

The current deterministic checks passed.
This result does not establish specification consistency or application correctness.
Report the checks performed, then follow the caller's remaining review instructions.
