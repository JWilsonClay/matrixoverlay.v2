# Project Architecture

## System Overview
This project follows the **Sovereign Substrate** pattern.

## Directory Structure
*Source of Truth: FOLDER_OWNERSHIP.md*

[DIRECTORY_LIST_PLACEHOLDER]

---

## Communication Patterns
- **Direct Imports**: Cross-substrate communication is achieved via explicit, direct imports.
- **Path Isolation**: All file I/O must use absolute paths anchored in the workspace root.

## Hardening Standards
- **Atomic Persistence**: Database writes and configuration updates are performed surgically to prevent state corruption.
- **Refactor Protocol**: All structural changes must follow the 5-phase Refactor Protocol (Bridge, Migrate, Surgery, Verify, Clean).

## Global API Map (Discovered Interfaces)
<!-- Discovered interfaces will be automatically synced below -->

---
