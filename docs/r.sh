#!/bin/bash
# =============================================================================
# Task Executor - 自动执行 docs/task.md 任务的脚本
# Version: 2.0
# Source: docs/task.md
# =============================================================================

set -e

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TASK_FILE="$PROJECT_ROOT/docs/task.md"
PROGRESS_FILE="$PROJECT_ROOT/progress.txt"
TEMP_LOG="$PROJECT_ROOT/iter.log"

# -----------------------------------------------------------------------------
# Validation
# -----------------------------------------------------------------------------
validate() {
    # Check iterations argument
    if [ -z "$1" ]; then
        echo "Usage: $0 <iterations>"
        echo "Example: $0 10"
        exit 1
    fi

    # Validate iterations is a positive integer
    if ! [[ "$1" =~ ^[1-9][0-9]*$ ]]; then
        echo "Error: iterations must be a positive integer"
        exit 1
    fi

    # Check claude CLI exists
    if ! command -v claude >/dev/null 2>&1; then
        echo "Error: claude CLI not found. Please install it first."
        exit 1
    fi

    # Check task.md exists and is valid JSON
    if [ ! -f "$TASK_FILE" ]; then
        echo "Error: $TASK_FILE not found"
        exit 1
    fi

    if ! python3 -m json.tool "$TASK_FILE" >/dev/null 2>&1; then
        echo "Error: $TASK_FILE is not valid JSON"
        exit 1
    fi

    # Ensure progress.txt exists
    touch "$PROGRESS_FILE" 2>/dev/null || {
        echo "Error: Cannot create $PROGRESS_FILE"
        exit 1
    }

    # Check for remaining tasks
    local remaining=$(grep -c '"passes": null' "$TASK_FILE" 2>/dev/null || echo "0")
    if [ "$remaining" -eq 0 ]; then
        echo "✅ All tasks already completed. No work remaining."
        echo "<promise>COMPLETE</promise>"
        exit 0
    fi

    echo "📋 Found $remaining tasks with passes=null"
}

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------
cleanup() {
    local exit_code=$?
    rm -f "$TEMP_LOG" 2>/dev/null || true
    if [ $exit_code -ne 0 ]; then
        echo "⚠️  Script exited with code $exit_code"
    fi
    exit $exit_code
}

trap cleanup EXIT INT TERM

# -----------------------------------------------------------------------------
# Prompt Definition
# -----------------------------------------------------------------------------
export PROMPT="@docs/task.md @progress.txt

# Role & Goal
You are a Senior Full-Stack Engineer (Tech Lead). Your mission is to precisely execute tasks from docs/task.md with high engineering stability and autonomous priority assessment.

# Execution SOP
1. 【Task Selection】Read docs/task.md (JSON format), skip tasks with 'passes': true.
   - Evaluate remaining 'passes': null tasks
   - Consider dependencies, critical path, and blocking factors
   - Select ONE task with highest priority/value
   - Log your selection rationale

2. 【Implementation】Write Rust backend code or tests based on task nature.
   - Use Mock data for E2E tests
   - Follow existing code patterns in the project

3. 【Dependency Management】Add dependencies only via terminal:
   - Rust: cargo add <crate>
   - Node: npm install <package>

4. 【Verification & Circuit Breaker】
   - Run: cargo clippy && cargo test (or npm run typecheck && npm test)
   - Self-fix if failures occur
   - ⚠️ Circuit breaker: After 3 consecutive failures on same task, SKIP it
   - Record failure reason in progress.txt

5. 【Safe State Update】
   - Only update docs/task.md when ALL tests pass (exit code 0)
   - Change 'passes': null → true for completed task
   - Ensure JSON syntax remains valid

6. 【Documentation】
   - Append to progress.txt: [timestamp] - [type] - [rationale] - Task: <description>
   - Create Git commit with proper message format

# Constraints (MUST NOT violate)
- 🚫 Process ONE task per iteration only
- 🚫 Do NOT modify package.json scripts or Cargo.toml core config
- 🚫 Do NOT break existing code - regression tests must pass
- 🚫 Do NOT use unwrap() or expect() in billing logic
- 🚫 Do NOT sync query database in hot path (proxy_handler)

# Exit Condition
When no 'passes': null tasks remain in docs/task.md, output:
<promise>COMPLETE</promise>"

# -----------------------------------------------------------------------------
# Main Execution
# -----------------------------------------------------------------------------
main() {
    local iterations=$1
    local start_time=$SECONDS
    local iter_start
    local iter_dur

    validate "$iterations"

    echo "========================================"
    echo "🚀 Starting Task Executor"
    echo "   Task file: $TASK_FILE"
    echo "   Max iterations: $iterations"
    echo "========================================"

    for ((i=1; i<=iterations; i++)); do
        iter_start=$SECONDS

        echo ""
        echo "========================================"
        echo "📍 Iteration $i/$iterations"
        echo "========================================"

        # Check remaining tasks before each iteration
        local remaining=$(grep -c '"passes": null' "$TASK_FILE" 2>/dev/null || echo "0")
        echo "📊 Remaining tasks: $remaining"

        if [ "$remaining" -eq 0 ]; then
            echo ""
            echo "✅ No more tasks to process"
            echo "<promise>COMPLETE</promise>" > "$TEMP_LOG"
            break
        fi

        # Execute claude with the prompt
        echo "⏳ Executing task..."
        if script -q -e -c 'claude --dangerously-skip-permissions -p "$PROMPT"' "$TEMP_LOG"; then
            : # Success, continue
        else
            echo "⚠️  Claude exited with non-zero code"
        fi

        # Calculate iteration duration
        iter_dur=$((SECONDS - iter_start))
        printf "⏱️  Iteration %d completed in %d min %d sec\n" $i $((iter_dur/60)) $((iter_dur%60))

        # Check for completion signal
        if grep -q "<promise>COMPLETE</promise>" "$TEMP_LOG" 2>/dev/null; then
            local total_dur=$((SECONDS - start_time))
            echo ""
            echo "========================================"
            echo "✅ ALL TASKS COMPLETE"
            echo "   Iterations used: $i"
            printf "   Total time: %d min %d sec\n" $((total_dur/60)) $((total_dur%60))
            echo "========================================"

            # Optional notification
            if command -v tt >/dev/null 2>&1; then
                tt notify "Task execution complete after $i iterations" 2>/dev/null || true
            fi

            exit 0
        fi
    done

    # Reached max iterations
    local total_dur=$((SECONDS - start_time))
    echo ""
    echo "========================================"
    echo "🏁 Reached max iterations ($iterations)"
    printf "   Total time: %d min %d sec\n" $((total_dur/60)) $((total_dur%60))

    local final_remaining=$(grep -c '"passes": null' "$TASK_FILE" 2>/dev/null || echo "?")
    echo "   Remaining tasks: $final_remaining"
    echo "========================================"
}

# Run main function
main "$@"
