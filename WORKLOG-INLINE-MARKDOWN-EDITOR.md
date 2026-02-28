# Inline Markdown Editor Worklog

Date: 2026-02-28
Branch at start: `feature/native-markdown-editor`

## Objective
Stabilize Obsidian-style inline markdown editing in `dioxus-nox-markdown` and verify behavior in the noxpad demo with automated regressions (Playwright), while preserving previous bug fixes and keeping Dioxus 0.7-native architecture.

## Work Completed

### 1. Core inline editor architecture and behavior
- Reworked token-aware inline editing flow in `crates/markdown/src/inline_editor.rs`.
- Added generation/race guards to prevent stale async cursor updates from overwriting newer input state.
- Added explicit pending caret restore paths (raw vs visible coordinate intent).
- Hardened input pipeline ordering so model updates, cursor math, and render restoration execute deterministically.
- Preserved IME composition handling and composition-safe restore timing.
- Reduced regressions where cursor disappeared, moved into dead zones, or got trapped in a non-editable segment.

### 2. Tokenization and marker reveal logic
- Added token and reveal support modules:
  - `crates/markdown/src/inline_tokens.rs`
  - `crates/markdown/src/reveal_engine.rs`
- Tightened scoped marker reveal behavior to reduce unrelated marker expansion on a line.
- Continued to enforce block marker behavior separately from inline delimiter behavior.

### 3. Caret/selection interop boundary
- Added/extended interop adapter in `crates/markdown/src/interop.rs` for contenteditable selection handling.
- Added JS bridge methods for:
  - detailed selection reads,
  - beforeinput metadata capture,
  - deterministic post-input caret restoration.
- Kept platform-specific DOM interaction isolated behind adapter methods.

### 4. Library and demo integration updates
- Updated markdown components/context/hooks/types and integration points:
  - `crates/markdown/src/components.rs`
  - `crates/markdown/src/context.rs`
  - `crates/markdown/src/hooks.rs`
  - `crates/markdown/src/types.rs`
  - `crates/markdown/src/lib.rs`
  - `crates/markdown/src/parser.rs`
  - `crates/markdown/src/tests.rs`
- Updated noxpad integration paths in suggest/demo crates and app wiring.

### 5. Playwright and test harness
- Added repo-local Playwright harness for noxpad in `crates/suggest/examples/noxpad/playwright/`.
- Added/expanded regression specs covering:
  - scoped strong-marker reveal,
  - list marker reveal behavior,
  - line clickability/editability,
  - keyboard navigation stability,
  - caret behavior around delimiter conceal/reveal transitions,
  - multi-token same-line typing regressions.
- Added Firefox + Chromium coverage in Playwright config.

### 6. Documentation/spec artifacts
- Added/updated planning/spec files:
  - `crates/markdown/IMPLEMENTATION-PLAN-STATUS.md`
  - `crates/markdown/OBSIDIAN-LIVE-PREVIEW-RULES.md`
  - `crates/markdown/antigravity-Seamless-Editor-Tech-Spec`

## What Was Tried During Debugging
- Multiple iterations of caret mapping strategy:
  - browser-postselection driven,
  - raw-offset normalization fallback,
  - deterministic pre/post input caret math using beforeinput metadata.
- Added stale-task cancellation and generation checks to avoid async races between oninput/onkeyup/onmouseup.
- Constrained key-event paths so printable edits are processed via input pipeline, and navigation remains cursor-only.
- Added scoped reveal logic so only active token envelope markers are shown where possible.
- Repeatedly validated no reintroduction of prior regressions (dead zones, phantom blank lines, cursor disappearance).

## Test Runs Performed
- `cargo test -p dioxus-nox-markdown` (previously run in this session and reported green after caret pipeline changes).
- `npm test` in `crates/suggest/examples/noxpad/playwright` (previously run in this session with Chromium + Firefox projects and reported green at that point).

## Current Known Status (Honest Assessment)
- A persistent user-observed caret jump issue remains reported in both Firefox and Chrome for the exact closing-`**` path:
  - expected: `**er**#`
  - observed: `**er*#*`
- This indicates there is still at least one unresolved edge path where local manual behavior diverges from the automated assertions.
- Additional runtime-specific instrumentation and exact repro-path capture are still needed to fully close this bug without regressions.

## Files Added in This Workstream (high impact)
- `WORKLOG-INLINE-MARKDOWN-EDITOR.md` (this file)
- `crates/markdown/src/inline_tokens.rs`
- `crates/markdown/src/reveal_engine.rs`
- `crates/markdown/src/interop.rs`
- `crates/markdown/src/viewport.rs`
- `crates/markdown/src/ime_proxy.rs`
- `crates/suggest/examples/noxpad/playwright/*`

## Notes
- This work intentionally favored native Dioxus 0.7 patterns and avoided introducing external editor frameworks.
- Breaking internal behavior changes were made in service of deterministic token-aware inline editing and regression containment.
