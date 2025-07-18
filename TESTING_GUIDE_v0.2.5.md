# Silva v0.2.5 Job Management Testing Guide

## Overview
This guide provides detailed testing instructions for the new job management features in Silva v0.2.5. Each test case includes specific steps, expected results, and verification methods.

## Prerequisites
- Silva v0.2.5_job-mng branch compiled and ready to run
- Docker installed and running
- Sample projects with multiple job configurations
- Terminal access for monitoring logs

## High Priority Tests

### 1. Test Multiple Job Configuration Support
**Objective**: Verify the system can detect and load multiple job configuration files.

**Setup**:
1. Create a test project directory with multiple job configs:
   ```
   test-project/
   ├── @job.toml      # Default job config
   ├── @job_1.toml    # First numbered config
   ├── @job_2.toml    # Second numbered config
   └── input.txt      # Sample input file
   ```

**Test Steps**:
1. Start Silva: `./target/release/silva`
2. Navigate to Projects tab (Tab key)
3. Select or create the test project
4. Press 'N' to create a new job

**Expected Results**:
- [ ] Configuration selection screen appears
- [ ] All three configs are listed: "@job.toml", "@job_1.toml", "@job_2.toml"
- [ ] Each config shows its name clearly
- [ ] Arrow keys navigate between configs
- [ ] Enter key selects a config

**Verification**:
- Check that the job created uses the selected config
- Verify job name includes config identifier

### 2. Test Job Configuration Selection UI
**Objective**: Verify the new configuration selection interface works correctly.

**Test Steps**:
1. From Projects page, press 'N' for new job
2. When config selection appears:
   - Use ↑/↓ arrows to navigate
   - Press Enter to select
   - Press Esc to cancel

**Expected Results**:
- [ ] UI displays all available configs in a list
- [ ] Current selection is highlighted
- [ ] Config names are clearly visible
- [ ] Esc returns to project page without creating job
- [ ] Enter creates job with selected config

**Edge Cases to Test**:
- [ ] Project with only @job.toml (should skip selection)
- [ ] Project with no job configs (should show error)
- [ ] Project with gaps (@job_1.toml, @job_3.toml)

### 3. Test Job Queue System
**Objective**: Verify jobs execute sequentially when multiple are queued.

**Setup**:
1. Create 3 jobs from the same project using different configs
2. Each job should have a simple command like `sleep 10 && echo "Job X done"`

**Test Steps**:
1. Create Job 1 and start it (should go to Running)
2. Create Job 2 while Job 1 is running
3. Create Job 3 while Jobs 1&2 are pending/running
4. Navigate to Jobs tab to monitor

**Expected Results**:
- [ ] Job 1: Created → Running
- [ ] Job 2: Created → Queued (while Job 1 runs)
- [ ] Job 3: Created → Queued (while Jobs 1&2 are pending)
- [ ] Job 2 automatically starts when Job 1 completes
- [ ] Job 3 automatically starts when Job 2 completes
- [ ] Final states: All jobs show "Completed"

**Verification**:
- Monitor job states in real-time
- Check timestamps to confirm sequential execution
- Verify no jobs run simultaneously

### 4. Test Job Lifecycle States
**Objective**: Verify all job state transitions work correctly.

**Test Scenarios**:

**A. Normal Completion**:
1. Create a job with command: `echo "Success"`
2. Run the job
3. Expected transitions:
   - [ ] Created (initial state)
   - [ ] Running (when execution starts)
   - [ ] Completed (when finished successfully)

**B. Job Failure**:
1. Create a job with command: `exit 1`
2. Run the job
3. Expected transitions:
   - [ ] Created → Running → Failed

**C. Job Cancellation**:
1. Create a job with command: `sleep 60`
2. Run the job
3. Press 'C' to cancel while running
4. Expected transitions:
   - [ ] Created → Running → Cancelled

**D. Queued Job**:
1. Start a long-running job
2. Create and run another job
3. Expected transitions for Job 2:
   - [ ] Created → Queued → Running → Completed

### 5. Test Job Persistence
**Objective**: Verify jobs persist across application restarts.

**Test Steps**:
1. Create multiple jobs in different states:
   - Job A: Run to completion
   - Job B: Start and cancel
   - Job C: Create but don't run
   - Job D: Start a long-running job
2. Note the job IDs and states
3. Quit Silva (Ctrl+Q)
4. Restart Silva
5. Navigate to Jobs tab

**Expected Results**:
- [ ] All jobs are present after restart
- [ ] Job A shows "Completed" state
- [ ] Job B shows "Cancelled" state
- [ ] Job C shows "Created" state
- [ ] Job D shows appropriate state (Running/Completed)
- [ ] Job details (config, timestamps) are preserved

## Medium Priority Tests

### 6. Test Enhanced Job List UI
**Objective**: Verify the new job list columns display correctly.

**Test Steps**:
1. Create jobs with different configs and states
2. Navigate to Jobs tab
3. Observe the job list table

**Expected Results**:
- [ ] Columns displayed: ID, Name, Config, Status, Created
- [ ] Config column shows: "@job.toml", "@job_1.toml", etc.
- [ ] Status column shows: Created, Running, Completed, etc.
- [ ] Created column shows timestamp
- [ ] Table resizes properly with terminal width
- [ ] Long text is truncated with "..."

### 7. Test Job Dependencies Tab
**Objective**: Verify the new Dependencies tab in job details.

**Setup**:
Create a job with dependencies in the config:
```toml
[dependencies]
packages = ["numpy", "pandas"]
files = ["data.csv", "config.json"]
```

**Test Steps**:
1. Navigate to Jobs tab
2. Select the job with dependencies
3. Press 'D' for Dependencies tab

**Expected Results**:
- [ ] Dependencies tab shows in the tab bar
- [ ] Package dependencies listed clearly
- [ ] File dependencies listed clearly
- [ ] UI handles missing dependencies gracefully
- [ ] Tab navigation works (can switch to other tabs)

### 8. Test Job Status Tab
**Objective**: Verify real-time status monitoring during job execution.

**Test Steps**:
1. Create a job with periodic output:
   ```bash
   for i in {1..10}; do echo "Step $i"; sleep 2; done
   ```
2. Run the job
3. Press 'S' for Status tab while running

**Expected Results**:
- [ ] Status tab shows current job state
- [ ] Start time is displayed
- [ ] Running time updates in real-time
- [ ] Resource usage shown (if available)
- [ ] Tab refreshes automatically
- [ ] End time shown when completed

### 9. Test Job Cancellation
**Objective**: Verify job cancellation works properly.

**Test Scenarios**:

**A. Cancel Running Job**:
1. Start a long-running job (`sleep 60`)
2. Press 'C' to cancel
3. Confirm cancellation
- [ ] Job transitions to "Cancelled" state
- [ ] Docker container is stopped
- [ ] Resources are cleaned up

**B. Cancel Queued Job**:
1. Start a long-running job
2. Queue another job
3. Cancel the queued job
- [ ] Queued job transitions to "Cancelled"
- [ ] First job continues running
- [ ] Queue processes next job (if any)

**C. Cancel from List View**:
1. Select a running job from the list
2. Press 'C' to cancel
- [ ] Confirmation dialog appears
- [ ] Job cancels without entering detail view

### 10. Test Concurrent Job Execution
**Objective**: Verify multiple jobs can run simultaneously on local Docker.

**Setup**:
1. Ensure Docker has sufficient resources
2. Create jobs with different Docker images if possible

**Test Steps**:
1. Create Job 1 with `sleep 30`
2. Create Job 2 with `sleep 30` 
3. Run both jobs quickly
4. Monitor Jobs list

**Expected Results**:
- [ ] Both jobs show "Running" state simultaneously
- [ ] Different Docker containers are created
- [ ] Both complete independently
- [ ] System remains responsive

## Lower Priority Tests

### 11. Test Job Filtering/Sorting
**Objective**: Verify job list filtering and sorting capabilities.

**Test Steps**:
1. Create 10+ jobs with various states and configs
2. In Jobs list, test filtering:
   - Press 'f' for filter menu (if implemented)
   - Filter by status
   - Filter by config
   - Filter by date range

**Expected Results**:
- [ ] Filter options are accessible
- [ ] Filtered results update immediately
- [ ] Multiple filters can be combined
- [ ] Clear filter option available
- [ ] Sort by date/status/name works

### 12. Test Error Handling
**Objective**: Verify graceful error handling.

**Test Scenarios**:

**A. Invalid Job Config**:
1. Create a job with malformed @job.toml
- [ ] Error message displayed clearly
- [ ] App doesn't crash
- [ ] Can return to project page

**B. Docker Not Running**:
1. Stop Docker daemon
2. Try to run a job
- [ ] Clear error about Docker
- [ ] Job marked as Failed
- [ ] Suggestion to start Docker

**C. Missing Files**:
1. Create job referencing non-existent file
2. Run the job
- [ ] Error logged in job output
- [ ] Job fails gracefully

### 13. Test Backwards Compatibility
**Objective**: Ensure existing single @job.toml projects work.

**Test Steps**:
1. Use a project with only @job.toml (no numbered configs)
2. Create a new job
3. Run the job

**Expected Results**:
- [ ] No config selection screen (direct job creation)
- [ ] Job works as in previous versions
- [ ] All existing features functional
- [ ] No regression in basic workflow

### 14. Performance Test
**Objective**: Verify UI remains responsive with many jobs.

**Test Steps**:
1. Create 50+ jobs programmatically
2. Navigate through Jobs list
3. Open job details
4. Scroll through list

**Expected Results**:
- [ ] List renders without lag
- [ ] Scrolling is smooth
- [ ] Job details open quickly
- [ ] No memory leaks observed
- [ ] Terminal remains responsive

### 15. Test Job Auto-Save
**Objective**: Verify job state saves automatically.

**Test Steps**:
1. Start a long-running job
2. While running, note the job state
3. Kill Silva process (not graceful exit)
4. Restart Silva
5. Check job state

**Expected Results**:
- [ ] Job state is preserved
- [ ] Last known status is accurate
- [ ] Partial output (if any) is saved
- [ ] Can resume or clean up job

## Testing Tips

1. **Logging**: Run Silva with `RUST_LOG=debug` for detailed logs
2. **Docker Monitoring**: Use `docker ps` to verify container management
3. **File System**: Check `~/.local/share/silva/` for persistence files
4. **Multiple Terminals**: Use one for Silva, one for monitoring

## Bug Reporting Template

When reporting issues, include:
```
1. Test Case: [Which test failed]
2. Steps to Reproduce: [Exact steps]
3. Expected Result: [What should happen]
4. Actual Result: [What actually happened]
5. Silva Version: v0.2.5_job-mng
6. OS: [Linux/Mac/Windows]
7. Docker Version: [docker --version]
8. Logs: [Relevant error messages]
```

## Summary Checklist

After completing all tests, verify:
- [ ] All high priority tests pass
- [ ] No regressions from v0.2.4
- [ ] UI is intuitive and responsive
- [ ] Error messages are helpful
- [ ] Documentation matches behavior