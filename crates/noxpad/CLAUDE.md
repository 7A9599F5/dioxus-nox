# noxpad — 7-crate integration demo app

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Full integration demo exercising shell, markdown, suggest, cmdk, tag-input, dnd, and preview crates together. Single-file app (`src/main.rs`, ~1300 lines). Not a published crate — binary only.

## Dependencies
- `dioxus-nox-shell` — layout shell
- `dioxus-nox-markdown` — editor/preview
- `dioxus-nox-cmdk` — command palette
- `dioxus-nox-suggest` — inline suggestions
- `dioxus-nox-tag-input` — tag input
- `dioxus-nox-dnd` — drag-and-drop
- `dioxus-nox-preview` — debounced preview

## Running
```bash
dx serve -p noxpad
```

## E2E Tests (Playwright)
```bash
cd crates/noxpad/playwright
npm install
npm run install:browsers
npm test
```

## CI
```bash
cargo check -p noxpad --target wasm32-unknown-unknown
```
