# Build Polish Workflow

```toml donna
kind = "donna.lib.workflow"
start_operation_id = "test_build_metadata"
```

Validate changes to build logic after regular code polish.
Checks cover collector build metadata with reused caches, browser/build integration with repeated history/token rebuilds and preview checks, and the release WASM build.
Each failure has a focused repair action; every repair restarts this workflow's sequence.
All checks run locally without project Git operations or hosted workflows.
Normal build dependency resolution and downloads are allowed; tests use local metric inputs and never contact the original statistics service.
Third-party runtime assets may load from the internet.

## Test collector build metadata

```toml donna
id = "test_build_metadata"
kind = "donna.lib.run_script"
save_stdout_to = "build_metadata_stdout"
save_stderr_to = "build_metadata_stderr"
goto_on_success = "test_browser"
goto_on_failure = "fix_build_metadata"
timeout = 600
```

```bash donna script
#!/usr/bin/env bash
set -euo pipefail
./bin/test-build-metadata.sh
```

## Repair collector build metadata

```toml donna
id = "fix_build_metadata"
kind = "donna.lib.request_action"
```

```text
{{ donna.lib.task_variable("build_metadata_stdout") }}
{{ donna.lib.task_variable("build_metadata_stderr") }}
```

Repair the reported collector build-metadata or cache-invalidation failure using local source fixtures.
After the repair, {{ donna.lib.goto("test_build_metadata") }}.
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

Repair the reported browser, embedded-history, download, or rebuild failure. Use local metric inputs and never contact the original statistics service; third-party runtime assets may load from the internet. Tool installation is a separate setup step.
After the repair, {{ donna.lib.goto("test_build_metadata") }}.
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
After the repair, {{ donna.lib.goto("test_build_metadata") }}.
If an external action is required, leave this request pending and report the concrete blocker.

## Finish

```toml donna
id = "finish"
kind = "donna.lib.finish"
```

The build-validation checks passed.
This result does not establish specification consistency or application correctness.
Report the checks performed, then follow the caller's remaining review instructions.
