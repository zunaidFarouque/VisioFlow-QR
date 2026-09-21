# AI Agent Guidelines & Safety Rules

## Critical Safety Rule: Releases
> [!CAUTION]
> **CRITICAL SAFETY RULE: RELEASES**
> NEVER execute `scripts/release.ps1` without the user's explicit, direct permission in the current prompt (e.g. "Please release version X.Y.Z"). Agents may build, test, and edit files, but must never autonomously trigger an official GitHub release or publish to the public Scoop bucket.

## Development & Testing Workflow
- Always follow Test-Driven Development (TDD).
- Verify compilation and unit/integration tests before committing changes.
- Automated release workflow is handled via `.\scripts\release.ps1 -Version "X.Y.Z" -Force`.
