# Implementation Status: Inline UX + Tree/Tab DnD

This tracks execution of the plan to stabilize inline editing, add sidebar/tree+tab DnD, and prepare for collaborative architecture integration.

## Completed

- Inline editor path is library-owned again in `dioxus-nox-markdown`:
  - `markdown::Editor` in `LivePreviewVariant::Inline` now routes to `InlineEditor`.
  - Demo-local inline editor logic in noxpad has been removed.
- Interop isolation is in place:
  - DOM eval/caret behavior is centralized behind `crates/markdown/src/interop.rs`.
- Parser/render source-map contract normalized:
  - Preview click mapping now uses `data-source-start` consistently.
- Security default restored:
  - HTML is escaped by default.
  - Explicit trusted mode is available through `Root { html_render_policy: HtmlRenderPolicy::Trusted, .. }`.
- Regex extension scanning removed:
  - Parser custom token pass now uses native scanning.
- Noxpad app now uses markdown inline mode directly and includes:
  - Folder reorder DnD.
  - Folder note reorder + cross-folder move DnD.
  - Tab strip reorder DnD.
  - Pure state operations and tests for reorder/move/close-tab behavior.

## Verified

- `cargo check` (workspace) passes.
- `cargo test -p dioxus-nox-markdown` passes.
- `cargo test -p noxpad` passes.

## Deferred / Next Phase

- Full collaborative CRDT architecture integration (Automerge-first, headless sync core).
- Cross-target E2E parity gates for Web/Desktop/iOS/Android.
- Dedicated adapter behavior tests per target runtime.
- Final dependency reevaluation for replacing `crop::Rope` with `String` hot path based on benchmark evidence.
