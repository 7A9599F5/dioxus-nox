# Architecture & Code Review: dioxus-nox

## Context

**dioxus-nox** is a headless component library for Dioxus 0.7, following the Radix Primitives design pattern. It provides 11 crates of composable, unstyled UI primitives (command palette, drag-and-drop, markdown editor, suggestions, tag input, shell layout, virtual scroll, preview, gestures, extensions) plus a demo app (noxpad). The README states it is "100% AI-written for a personal learning project."

**Codebase size:** ~42,000 lines of Rust across 95 source files, ~8,300 lines of tests across 6 crates, 39 example files. Most mature crates: cmdk (11.3K lines), dnd (15.2K lines including collision detection), markdown (10.2K lines).

---

## Findings

---

### [PARTIALLY IMPLEMENTED] [PRIORITY: High]
**Area:** Feature Assessment — Incomplete/Stub Crates
**Problem:** Two workspace-declared crates are completely empty:
- ~~`crates/extensions/src/lib.rs` (13 lines) — only doc comments describing a "Planned API" with references to non-existent SPEC.md and BUILD_PROMPT.md~~ **IMPLEMENTED**: extensions crate now provides a runtime plugin system with `Extension` trait, `PluginCommand` struct, `ExtensionContext` (signal-based registry with O(1) lookup), `use_extensions` hook returning `ExtensionHandle`, and `filter_commands` search. 21 unit tests covering types, trait implementation, lifecycle tracking, and command filtering.
- ~~`crates/gestures/src/lib.rs` (11 lines) — same pattern, only planned API docs~~ **IMPLEMENTED**: gestures crate now provides a layered API (pure math, hooks, compound components) for swipe-to-reveal and long-press gestures, integrated into noxpad sidebar.
- `crates/tabs/` — empty directory with only a `.gitignore`, no `Cargo.toml`, not in workspace

**Suggestion:** ~~These stub crates MUST be implemented with their planned APIs. The `extensions` crate should implement its runtime plugin system (`Extension` trait, `PluginCommand`, `ExtensionHandle`).~~ Done. The `tabs/` directory needs a `Cargo.toml` and implementation of a headless tab management crate. Do NOT remove these crates — they represent planned functionality.

**Expected Impact:** Complete component library coverage; all workspace crates deliver real functionality.

---

### [IMPLEMENTED] ~~[PRIORITY: High]~~
**Area:** Security — `dangerous_inner_html` with User Content
**Problem:** The markdown crate uses `dangerous_inner_html` in three locations (`crates/markdown/src/parser.rs:330,423,442` and `crates/markdown/src/viewport.rs:163`). While code highlighting output from syntect is generated server-side and is relatively safe, the raw HTML rendering path at `parser.rs:330` is gated only by `HtmlRenderPolicy::Trusted`:

```rust
CustomEvent::Standard(Event::Html(h)) | CustomEvent::Standard(Event::InlineHtml(h)) => {
    let html = h.to_string();
    if config.html_render_policy == HtmlRenderPolicy::Trusted {
        rsx! { span { dangerous_inner_html: "{html}" } }
    } else {
        rsx! { span { "{html}" } }
    }
}
```

The `HtmlRenderPolicy` enum and its usage lack documentation about XSS implications. The noxpad demo doesn't appear to set this policy explicitly, but any consumer enabling `Trusted` mode with user-generated markdown is exposed to XSS.

**Suggestion:**
1. Add prominent doc comments to `HtmlRenderPolicy::Trusted` warning about XSS.
2. Consider adding a `Sanitized` variant that uses a lightweight HTML sanitizer (e.g., `ammonia` crate).
3. Document the security model in the markdown crate's top-level docs.

**Expected Impact:** Prevents accidental XSS in consumer applications; clearer security contract.

---

### [IMPLEMENTED] ~~[PRIORITY: High]~~
**Area:** Maintainability — Monolithic Demo File
**Problem:** `crates/noxpad/src/main.rs` was 1,368 lines in a single file containing:
- ~210 lines of inline CSS as a `const` string
- Data model definitions
- 8+ component functions
- Utility functions for folder/note management
- All state management

This makes the demo — which serves as the primary showcase for 6+ library crates — difficult to understand, modify, or extend.

**Suggestion:** Split into modules: `css.rs` (styles), `models.rs` (Note, FolderNode, seed data), `components/` directory (sidebar, editor, command palette, preview, tabs), `utils.rs` (reorder helpers, text replacement). Keep `main.rs` to ~50 lines.

**Expected Impact:** Demo becomes genuinely useful as a reference implementation; easier to maintain and evolve.

---

### [IMPLEMENTED] ~~[PRIORITY: High]~~
**Area:** Developer Experience — No CI/CD
**Problem:** No GitHub Actions, no CI configuration of any kind. The project has 492+ tests across 6 crates but no automated way to verify they pass on PRs. No clippy, no rustfmt enforcement, no WASM build validation.

**Suggestion:** Add a `.github/workflows/ci.yml` with:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (native target for unit tests)
- `cargo build --target wasm32-unknown-unknown` for WASM compilation check
- Matrix across stable + nightly Rust

**Expected Impact:** Catches regressions automatically; enforces code quality standards; builds confidence for contributors.

---

### [IMPLEMENTED] ~~[PRIORITY: Medium]~~
**Area:** Architecture — Memory Leaks via `Box::leak`
**Problem:** Two instances of `Box::leak`:
1. `crates/markdown/src/highlight.rs:47` — Leaks prefix strings into a `PREFIX_CACHE`. Documented as intentional since "only 1-2 prefixes are used per app."
2. ~~`crates/noxpad/src/main.rs:470` — Leaks the entire syntax theme CSS string on every app mount: `Box::leak(generate_theme_css(...).into_boxed_str())`~~ **IMPLEMENTED**: Replaced `Box::leak` with `use_memo` — the CSS string is now owned by the memo, computed once and cached. No memory leak on remount.

The highlight.rs usage is acceptable (bounded, cached). ~~The noxpad usage leaks memory on each `App` component remount — though in practice this is likely once per session, it's still a code smell and poor pattern for a demo that teaches.~~ Added explicit bounded-leak documentation comment in highlight.rs.

**Suggestion:** ~~In noxpad, use `use_signal` or `use_resource` with a `'static` memo pattern instead of `Box::leak`.~~ Done. ~~In highlight.rs, add a comment noting the leak is bounded by the prefix cache.~~ Done.

**Expected Impact:** Eliminates memory leak in demo; better pattern for consumers to copy.

---

### [IMPLEMENTED] ~~[PRIORITY: Medium]~~
**Area:** Architecture — Version Inconsistency Across Crates
**Problem:** Crate versions are wildly inconsistent:
- cmdk: 0.13.0
- dnd: 0.2.0
- shell: 0.2.0
- All others: 0.1.0

With a workspace-level edition of 2024 and pinned `dioxus = "=0.7.3"`, the version numbers suggest the cmdk crate has had 13 releases while others are effectively unreleased. This is confusing for consumers trying to understand compatibility.

**Suggestion:** Either adopt a unified workspace version (all crates share the same version, bumped together) or document the versioning strategy. For a learning project, a unified version is simpler.

**IMPLEMENTED**: Added `version = "0.13.0"` to `[workspace.package]` in the root `Cargo.toml`. All 17 crate `Cargo.toml` files (11 library crates, 1 demo app, 5 example apps) now use `version.workspace = true` to inherit the unified version. Version bumps are now a single-line change in the workspace root.

**Expected Impact:** Clearer compatibility story; simpler dependency management.

---

### [PRIORITY: Medium]
**Area:** Maintainability — DnD Collision Detection Complexity
**Problem:** `crates/dnd/src/collision/sortable.rs` is 4,825 lines — the single largest file in the project. It contains the sortable collision algorithm with extensive test coverage (most of the file is tests), but the production logic portion is still very dense with multiple nested match arms and complex geometric calculations.

**Suggestion:** Extract the test module into a separate `tests/` directory. Consider splitting the algorithm into sub-modules: `boundary_detection.rs`, `traversal.rs`, `projection.rs`. The tests themselves are thorough and well-structured, which is a strength.

**Expected Impact:** Easier navigation and maintenance; clearer separation of algorithm phases.

---

### [IMPLEMENTED] ~~[PRIORITY: Medium]~~
**Area:** Architecture — `DragContext` File Size and Responsibility
**Problem:** `crates/dnd/src/context.rs` was 3,412 lines. It handled drag state management, pointer event processing, auto-scroll, animation frames, hysteresis, ARIA announcements, and the provider component — all in one file. The file had well-documented constants and clear intent, but the sheer size made navigation difficult.

**Suggestion:** Extract WASM-specific pointer handling into `pointer.rs`, auto-scroll logic into `auto_scroll.rs`, and keep `context.rs` focused on state management and the provider component.

**IMPLEMENTED**: Split `context.rs` into a `context/` directory module with 5 focused files:
- `mod.rs` (~700 lines) — DragContext struct, constructors, registration, queries, ARIA announcements, state structs (DropZoneState, ActiveDrag, DragState), free functions, and all tests
- `pointer.rs` (~500 lines) — Pointer drag lifecycle (start/update/end/cancel), traversal computation, hysteresis constants, timing helpers
- `keyboard.rs` (~400 lines) — Keyboard drag navigation, container switching, merge toggling, nested entry/exit, helper free functions
- `auto_scroll.rs` (~60 lines) — Scroll velocity constants and computation (WASM + stub)
- `provider.rs` (~350 lines) — DragContextProvider component, props, auto-scroll RAF loop, keyboard event handler

DragContext fields changed from private to `pub(super)` for submodule access. No public API changes — all re-exports from `lib.rs` remain valid. All 272 tests pass.

**Expected Impact:** Each file has a single clear responsibility; easier to modify pointer handling without risking state management bugs.

---

### [PRIORITY: Medium]
**Area:** Performance — cmdk CommandContext Signal Count
**Problem:** `CommandContext` (crates/cmdk/src/context.rs) contains 30+ signals/memos. Every palette instance allocates this entire state surface. While Dioxus signals are lightweight, the derived memos (`scored_items`, `filtered_count`, `visible_items`, `visible_item_ids`, `visible_item_set`, `visible_group_ids`, `active_mode`, `mode_query`, `active_page`) create a complex dependency graph that re-evaluates on many state changes.

The comments reference optimization tickets (P-050, P-051, P-052) indicating awareness of this, and the optimizations applied (Rc wrapping, HashMap index, merged memo) are sensible.

**Suggestion:** Consider lazy initialization for rarely-used features (action panel, page navigation, modes) — initialize those signals only when the feature is first used. This would reduce baseline memory for simple palette use cases.

**Expected Impact:** Lower memory footprint and fewer memo re-evaluations for the common "simple search palette" case.

---

### [PRIORITY: Medium]
**Area:** Developer Experience — No Prelude or Unified Import
**Problem:** There is no top-level `dioxus-nox` crate that re-exports all sub-crates. Consumers must individually depend on and import each crate (`dioxus-nox-cmdk`, `dioxus-nox-dnd`, `dioxus-nox-markdown`, etc.). The noxpad demo shows this results in 9 import lines just for library types.

**Suggestion:** Create a `dioxus-nox` umbrella crate with feature-gated re-exports of each sub-crate. This is the standard pattern for workspace libraries (similar to `tokio`, `axum`).

**Expected Impact:** Single dependency line for consumers; simpler getting-started experience.

---

### [IMPLEMENTED] ~~[PRIORITY: Medium]~~
**Area:** Maintainability — Inline CSS Constants
**Problem:** Multiple crates embed CSS as large `const &str` blocks:
- `crates/noxpad/src/main.rs` — 210+ lines of CSS in a const
- `crates/dnd/src/utils.rs` — exports `FUNCTIONAL_STYLES`, `FEEDBACK_STYLES`, `THEME_STYLES`, `GROUPED_*` variants

While inline CSS is a legitimate pattern for headless components (zero-dependency styling), the noxpad demo mixes component library CSS (`FUNCTIONAL_STYLES`) with application CSS in one massive const. There's no separation between structural CSS (needed for functionality) and decorative CSS (app-specific theming).

**Suggestion:** For noxpad, extract app CSS to a `.css` file loaded via `include_str!` or Dioxus asset system. Keep the library's `FUNCTIONAL_STYLES` pattern — it's appropriate for headless components.

**IMPLEMENTED**: Extracted noxpad app CSS from inline raw string literal in `css.rs` to `noxpad.css`, loaded via `include_str!()` — matching the dnd crate's existing pattern. Library CSS (`FUNCTIONAL_STYLES`, `FEEDBACK_STYLES`) remains unchanged.

**Expected Impact:** Clearer separation of library vs. application concerns in the demo.

---

### [PRIORITY: Low]
**Area:** Code Quality — `crate-type = ["cdylib", "rlib"]` on Library Crate
**Problem:** `crates/dnd/Cargo.toml` specifies `crate-type = ["cdylib", "rlib"]`. The `cdylib` target is only needed for the final WASM binary, not for library crates consumed by other Rust code. This causes `cargo build` to produce an unnecessary `.wasm` file for the library itself and can cause issues with `cargo test`.

**Suggestion:** Remove `cdylib` from `crate-type`. Only the final binary crate (noxpad or consumer apps) needs `cdylib`. Library crates should only be `rlib`.

**Expected Impact:** Cleaner build output; faster compilation; avoids potential test runner issues.

---

### [PRIORITY: Low]
**Area:** Code Quality — Test Coverage Gaps
**Problem:** Testing is strong in cmdk (4,355 lines), markdown (2,615 lines), tag-input (889 lines), and dnd collision detection. However, several crates have no tests at all:
- `dnd` primitives, context, patterns (the core drag-and-drop runtime — only collision has tests)
- `shell` (223 lines of tests, relatively thin)
- `virtualize` (inline tests, adequate for the simple viewport math)
- `preview` (166 lines, basic)

The dnd crate's collision algorithm is heavily tested (good), but the actual component behavior (DragContextProvider, SortableItem, Draggable, DropZone) has zero tests. Given the complexity of the context.rs (3,412 lines), this is a gap.

**Suggestion:** Add unit tests for dnd context state transitions (drag start, move, end), drop zone registration/deregistration, and collision strategy selection. The existing `wasm-bindgen-test` dev dependency is unused — consider using it for integration tests.

**Expected Impact:** Confidence in the most complex runtime behavior; regression protection for the drag-and-drop system.

---

### [PRIORITY: Low]
**Area:** Developer Experience — Documentation
**Problem:** Individual crate `lib.rs` docs are good (quick start examples, feature tables, module overviews). However:
- README is minimal (~15 lines, just a crate table)
- No CONTRIBUTING guide
- No architecture documentation explaining how crates compose
- No changelog
- Doc examples use `rust,ignore` — they won't be tested by `cargo test --doc`

**Suggestion:** Expand README with architecture diagram, "How crates compose" section, and "Getting Started" guide. Add `CHANGELOG.md`. Convert `rust,ignore` examples to testable `no_run` where possible.

**Expected Impact:** Lower barrier to understanding; doc-tested examples prevent drift.

---

### [PRIORITY: Low]
**Area:** Architecture — Edition 2024
**Problem:** The workspace specifies `edition = "2024"` which is the latest Rust edition. This requires recent Rust toolchains and may limit adoption. Dioxus 0.7 itself targets edition 2021.

**Suggestion:** Unless 2024-specific features (like `gen` blocks or the new `use` semantics) are actively used, consider pinning to edition 2021 for broader compatibility. If 2024 features are desired, document the minimum Rust version.

**Expected Impact:** Broader toolchain compatibility; explicit MSRV policy.

---

## What Works Well

1. **Headless component design** — The data-attribute-based styling API (`data-shell-*`, `data-md-*`, `data-suggest-*`) is clean and follows Radix Primitives conventions well. Components emit semantic attributes without imposing visual styles.

2. **Separation of pure logic and UI** — The virtualize crate is a great example: pure `VirtualViewport` math with an optional `hooks` feature. The collision detection in dnd similarly separates geometry math from DOM interaction.

3. **Compound component pattern** — Each crate exposes a `mod suggest { ... }` / `mod markdown { ... }` / `mod preview { ... }` namespace for component composition. This is ergonomic and well-executed.

4. **Test quality in core crates** — cmdk and markdown have thorough test suites that test edge cases, scoring algorithms, state transitions, and parsing behavior. The dnd collision tests are exceptionally detailed.

5. **ARIA accessibility** — The cmdk crate implements proper ARIA roles, keyboard navigation, screen reader announcements, focus trapping, and inert background management. This is unusually thorough for a component library.

6. **Feature flag design** — Clean separation of `web`/`desktop`/`mobile` features, optional `syntax-highlighting`, optional `router` and `virtualize` integrations.

7. **NoxPad as integration proof** — Despite being monolithic, noxpad successfully demonstrates 6+ crates composing together in a realistic application (folder tree with DnD, markdown editing with slash commands, command palette with preview, tab management).

---

## Highest Leverage Change

**Implement CI/CD with `cargo fmt`, `clippy`, `cargo test`, and WASM build verification.**

This is the single change with the most outsized impact because:

1. **It protects existing value** — The project has 492+ well-written tests that currently run only manually. Automated CI ensures they continue to pass as the code evolves.

2. **It enables confident iteration** — Every other improvement in this review (splitting files, removing stubs, adding tests) becomes safer with CI catching regressions.

3. **It's low effort, high reward** — A basic GitHub Actions workflow is ~30 lines of YAML. The project already has the test infrastructure; it just needs automation.

4. **It signals project maturity** — Even for a learning project, CI demonstrates engineering discipline and makes the project a better learning resource for others.

Without CI, any refactoring risks silent breakage. With CI, the project can evolve confidently.

---

## Verification

To validate this review's findings:
- `cargo test --workspace` — runs all 492+ tests
- `cargo clippy --workspace -- -D warnings` — catches lint issues
- `cargo build --workspace --target wasm32-unknown-unknown` — verifies WASM compilation
- Inspect `crates/extensions/src/lib.rs` and `crates/gestures/src/lib.rs` to confirm they're stubs
- Check `crates/dnd/Cargo.toml` for `cdylib` crate-type
- Grep for `Box::leak` and `dangerous_inner_html` to confirm security findings
