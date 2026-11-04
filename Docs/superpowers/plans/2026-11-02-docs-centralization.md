# Documentation Centralization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Place all project Markdown under `Docs/`, with the root `AGENTS.md` as the sole exception.

**Architecture:** Consolidate AI/governance documents under `Docs/ai/`, put the user-facing project overview in `Docs/README.md`, and organize planning docs under `Docs/planning/`. Move and merge all Markdown from the repository root and crate into `Docs/`; do not leave stubs in the crate. Keep `AGENTS.md` at root and update its links. The Rust source stays at `cachyos-boot-rescue/`.

**Tech Stack:** Markdown, relative links, existing Rust/Cargo repository unchanged.

**Spec:** `Docs/superpowers/specs/2026-11-02-docs-centralization-design.md`

## Global Constraints

- Keep all project Markdown under `/home/daniel/Projetos/CachyOS-Boot-Rescue/Docs`, except the required root `AGENTS.md`.
- Preserve all existing `AGENTS.md` rules and update references to the new canonical paths.
- Remove Markdown from `cachyos-boot-rescue/` after merging its content into `Docs/`.
- Set `readme = false` in `cachyos-boot-rescue/Cargo.toml` so Cargo metadata does not reference a moved README.
- Keep the Rust crate at `cachyos-boot-rescue/`; do not move code, CI, or repositories under `Sources/`.
- Preserve existing local edits and untracked work; move their Markdown content without loss.
- Canonical project status must keep Phase 2 **in progress** and must not declare it complete.
- Do not reorganize Markdown in the independent nested repositories under `Sources/`.
- Preserve Markdown under `.agents/skills/`; these files are installed tool resources and moving them would break their current loading paths.
- Do not claim tests/builds passed unless run during this task; documentation changes need link/consistency checks and Cargo metadata validation.

## Review Focus

- Old links may keep sending agents to duplicated or nonexistent `docs/ai/*` files; validate all local Markdown links and instructions.
- Phase 1/Phase 2 state conflicts may reappear in roadmaps; verify the canonical status and all entry points agree Phase 2 is in progress.
- Root `AGENTS.md` has a pre-existing user modification; preserve it in place and only correct outdated document paths.
- `.agents/skills/**.md` are tool resources, not project docs; leave them where their loaders expect them.
- The proposed documentation plan contains legacy `docs/...` paths; label it as historical/superseded so it cannot override the approved `Docs/` layout.
- The phase prompts may still reference lowercase `docs/` paths; check and update documentation-path references without changing source paths such as `src/`.

---

### Task 1: Create canonical Docs entry points and AI documentation

**Files:**
- Create: `Docs/README.md`
- Create: `Docs/ai/START_HERE.md`
- Create: `Docs/ai/CURRENT_STATUS.md`
- Create: `Docs/ai/PROJECT_CONTEXT.md`
- Create: `Docs/ai/DOCUMENTATION_MAP.md`
- Create: `Docs/ai/DEVELOPMENT_RULES.md`
- Create: `Docs/ai/TASK_WORKFLOW.md`
- Source content: root `START_HERE.md` and `CURRENT_STATUS.md`, plus all Markdown files under `cachyos-boot-rescue/`

**Interfaces:**
- Produces canonical paths for all subsequent tasks: `Docs/README.md` and the six non-AGENTS files under `Docs/ai/`.
- `Docs/ai/CURRENT_STATUS.md` is the only full project status document and states Phase 1 as previously documented and Phase 2 as in progress.
- Root `AGENTS.md` remains the only agent rules document and directs agents to canonical context under `Docs/ai/`.

- [x] Compare the root and crate documents; identify unique content and retain it in the canonical documents rather than copying whole duplicate sections blindly.
- [x] Write `Docs/README.md` as the project/documentation landing page with links to the phase plan, user/build guidance, and AI entry point.
- [x] Consolidate the quick start, project context, current status, map, development rules, and workflow in `Docs/ai/`; make the documented phase sequence consistent with the existing ten-phase plan and Phase 2 in progress.
- [x] Verify the canonical pages link only to files that exist or explicitly label future documents as planned.

### Task 2: Move root and crate Markdown into Docs

**Files:**
- Modify: `AGENTS.md`
- Move/merge: root `START_HERE.md` → `Docs/ai/START_HERE.md`
- Move/merge: root and crate `CURRENT_STATUS.md` → `Docs/ai/CURRENT_STATUS.md`
- Move/merge: crate `PROJECT_CONTEXT.md`, `DOCUMENTATION_MAP.md`, `DEVELOPMENT_RULES.md`, `TASK_WORKFLOW.md` → corresponding `Docs/ai/*.md`
- Move/merge: crate `README.md` → `Docs/README.md`
- Move/merge: crate `docs/ai/START_HERE.md` → `Docs/ai/START_HERE.md`
- Modify: `cachyos-boot-rescue/Cargo.toml`

**Interfaces:**
- All Markdown content is consolidated in `Docs/`; there are no Markdown stubs left in the crate or repository root other than `AGENTS.md`.
- Root `AGENTS.md` directs agents to `Docs/ai/START_HERE.md` and `Docs/ai/CURRENT_STATUS.md`.
- `Cargo.toml` no longer advertises a README path inside the crate.

- [x] Merge unique content from each old Markdown file into its canonical target; preserve phase evidence and the existing `Sources/` instruction in root `AGENTS.md`.
- [x] Remove each original Markdown file only after confirming its content is represented at its canonical location.
- [x] Update `AGENTS.md` and canonical workflow/map paths to `Docs/ai/*`.
- [x] Set `readme = false` in `Cargo.toml`; verify Cargo accepts the manifest metadata.

### Task 3: Align planning documents and links

**Files:**
- Move: `Docs/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md` → `Docs/planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`
- Move: `Docs/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md` → `Docs/planning/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md`
- Modify: `Docs/planning/CACHYOS_BOOT_RESCUE_10_PHASES_PROMPTS.md`
- Modify: `Docs/planning/CACHYOS_BOOT_RESCUE_AI_DOCUMENTATION_PLAN.md`
- Modify: `Docs/ai/DOCUMENTATION_MAP.md`
- Modify: `Docs/README.md`

- [x] Update references to canonical AI/status/documentation paths in the ten-phase plan; do not alter phase scope or the explicit Phase 2 in-progress rule.
- [x] Mark the existing AI documentation plan as a historical proposal superseded by the approved layout, retaining its useful content and preventing old lowercase `docs/...` paths from being mistaken as active locations.
- [x] Verify all phase/task links point to the canonical Docs tree and no roadmap claims Phase 2 is complete.

### Task 4: Validate the documentation reorganization

**Files:**
- Check: all Markdown files under the project root and crate, excluding independent nested repositories under `Sources/`

- [x] Run `git diff --check` and inspect `git status --short --untracked-files=all`; confirm unrelated pre-existing changes remain untouched.
- [x] Check relative Markdown links in the changed files, including links from repository root and crate paths into `Docs/`.
- [x] Search for active references to old canonical paths and resolve stale directory instructions; verify Phase 2 remains in progress.
- [x] Confirm no Rust source, lockfile, CI, or files under `Sources/` changed; validate `cargo metadata --no-deps` only if available without modifying dependencies.
- [x] Record the accepted baseline failure: `cargo test --all-targets` currently fails because `crate::logging` is unresolved in `src/app/mod.rs` and `src/cli/mod.rs`; do not change Rust code in this documentation task.
