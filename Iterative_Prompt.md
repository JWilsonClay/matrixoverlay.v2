# Iterative Debugging Prompt

**Role**: Senior Rust Systems Engineer & Architect

**Objective**: Fully install and operationalize Matrix Overlay v2 on Pop!_OS (X11).

**Context**:
- **Log**: `debugger.log` (Latest build/test output).
- **Status**: `CurrentProgramTrajectory.md` (Project goals and current state).
- **History**: `build_journal.md` (Change log).

**Instructions**:
1.  **Analyze**: Review `debugger.log` for compilation errors, test failures, OR runtime crashes (look for "Runtime Verification" section).
2.  **Diagnose**: Determine why the failure occurred (e.g., missing dependency, logic error, environment mismatch).
3.  **Fix**: Provide the exact code changes (using Search/Replace blocks or full file rewrites) to resolve the issue.
4.  **Document**:
    *   Update `CurrentProgramTrajectory.md` with the outcome (Pass/Fail) and findings.
    *   Update `build_journal.md` with a summary of the changes.
5.  **Next Step**: If the build and tests succeed, suggest the next logical action (e.g., "Run the binary", "Enable autostart").

**Current Focus**:
- Fix Runtime Crash: `XCB Wait Error: xcb protocol error`.
- Ensure `cargo test` passes (resolve any lingering benchmark issues).
- Verify binary runs for >5s without exiting.