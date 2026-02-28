# Obsidian Live Preview Rules Matrix (Executable Contract)

## Baseline

- Observation baseline: Obsidian Desktop `1.12.4` (release tag `v1.12.4`).
- Scope: `LivePreviewVariant::Inline` behavior in `dioxus-nox-markdown`.
- This document is test-backed: each rule maps to unit and/or Playwright assertions.

## Core Rule

- Keep rendered markdown visible by default.
- Reveal raw syntax tokens only when caret is in the token context.

## Inline Syntax Rules

| Syntax | Default | Reveal Trigger | Expected Reveal |
|---|---|---|---|
| `*em*`, `_em_` | Render emphasized text | Caret inside token envelope | Show delimiters for that token only |
| `**strong**`, `__strong__` | Render strong text | Caret inside token envelope | Show delimiters for that token only |
| `~~strike~~` | Render strikethrough | Caret inside token envelope | Show delimiters for that token only |
| `` `code` `` | Render inline code | Caret inside token envelope | Show backticks for that token only |
| `[text](url)` | Render link text | Caret inside token envelope | Show only the active link syntax |
| `![](url)` | Render image node/text | Caret inside token envelope | Show only the active image syntax |
| `[[wikilink]]` | Render wikilink | Caret inside token envelope | Show only active wikilink syntax |
| `#tag` extension | Render tag | Caret inside token envelope | Show tag marker in active token context |

## Block Marker Rules

| Syntax | Default | Reveal Trigger | Expected Reveal |
|---|---|---|---|
| Heading `#`, `##`, ... | Render heading text | Caret immediately before/on/after marker window | Show heading marker |
| Blockquote `>` | Render quote text | Caret immediately before/on/after marker window | Show `>` marker |
| List `-`, `*`, `+`, `1.` | Render semantic bullet/number | Caret immediately before/on/after marker window | Show raw marker plus semantic bullet/number |
| Task list `- [ ]`, `- [x]` | Render semantic list item | Caret immediately before/on/after marker window | Show raw task marker in window |
| Fenced code ``` | Render code block | Deferred in this cutover | Keep existing raw block editor behavior |

## Regression Lock Rules

1. Entire block remains clickable (no dead zones).
2. Keyboard movement must not drop the caret into a non-visible state.
3. Exactly one active editor surface per block.
4. No blank line insertion on click activation.
5. Active editor width remains block width (no narrow wrap regression).

## Must-Pass Scenarios

1. `Borrowing lets you ref**er**ence data`:
  - Caret in plain text: no visible `**`.
  - Caret in `er` token context: only that token's `**` is visible.
2. `- Each value has a single *owner*`:
  - Caret in content: semantic bullet only, raw `-` hidden.
  - Caret near marker window: raw `-` revealed.
