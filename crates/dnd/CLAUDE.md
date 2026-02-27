# dioxus-nox-dnd

Composable drag-and-drop component library for Dioxus 0.7 targeting WASM/web.
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Architecture: Layered Composition

```
Layer 3: CONVENIENCES
  └── SortableGroup         (shared DragContextProvider for multi-list)

Layer 2: PATTERNS
  └── SortableContext       (standalone: own provider; in group: DropZone; nested: dual registration)
  └── SortableItem          (wraps children with drag behavior + displacement)

Layer 1: PRIMITIVES
  └── Draggable, DropZone, DragOverlay (atomic building blocks)

Foundation: DragContext + CollisionStrategy enum + Event types
```

## Source Layout

```
src/
├── lib.rs               # Module aliases (sortable, grouped, styles), selective root re-exports, prelude
├── types.rs             # DragId, DragType, DragData, Position, Rect, DropLocation, all event types
├── context.rs           # DragContext, DragState, ActiveDrag, DragContextProvider
├── collision/
│   ├── mod.rs           # CollisionStrategy enum with detect() dispatch
│   ├── pointer.rs       # Simple pointer-in-rect
│   ├── closest.rs       # Closest center distance
│   └── sortable.rs      # AtIndex/IntoItem with displacement awareness
├── primitives/
│   ├── draggable.rs     # Draggable (pointer capture, drag start)
│   ├── dropzone.rs      # DropZone (registration, rect updates)
│   └── overlay.rs       # DragOverlay (cursor-following overlay)
├── patterns/
│   ├── sortable/
│   │   ├── context.rs   # SortableContext
│   │   ├── item.rs      # SortableItem + compute_displacement()
│   │   ├── group.rs     # SortableGroup (multi-container)
│   │   └── indicator.rs # DropIndicator
│   └── grouped.rs       # GroupedItem trait, GroupedList, grouped_reorder/merge/cleanup helpers
├── utils.rs             # CSS styles, attribute merging, find_contiguous_block
├── styles.css           # Default component CSS
└── grouped.css          # Grouped list theme CSS (CSS variable tokens)
```

## Key Design Decisions

1. **Cross-container drag requires shared state.** `SortableGroup` creates a shared `DragContextProvider` (mirrors dnd-kit's single `DndContext`).
2. **SortableContext has 3 modes:** Standalone (own provider), inside SortableGroup top-level (DropZone), or nested (dual registration via `register_nested_container`). Inner container ID = `{id}-container`.
3. **Events use index-based positioning.** `ReorderEvent`: `from_index` + `to_index`. `MoveEvent`: `from_index`, `to_index`, `from_container`, `to_container`. Both have `.apply()` convenience methods.
4. **Collision detection is pluggable and cursor-aligned.** `CollisionStrategy` enum: `Pointer`, `ClosestCenter`, `Sortable`, `SortableWithMerge`. Direction-aware zone splitting uses drag `delta` sign (not pointer-vs-center comparison). `OVERSHOOT_PX = 40.0` expands container rects for fallback when pointer drifts out.
5. **Zone split details.** Positive delta → DOWN (15/55/30), negative → UP (30/55/15), near-zero → symmetric (25/50/25). `MIN_ZONE_PX = 15.0` clamps zone halves. Group dragged item suppresses all IntoItem targeting (`dragged_is_group` computed once at top of `detect()`).
6. **Nested containers replace group-aware collision.** `DropZoneState.inner_container_id: Option<DragId>` — when set, detector skips item zone and delegates to inner container. Eliminates ~120 lines of group-specific collision branching.
7. **Grouped lists use flat data with nested rendering.** `patterns/grouped.rs` implements grouping on a flat `Vec<T>` via `GroupedItem` trait. Group cleanup (removing headers when members < min) is automatic.
8. **Drop preview system.** `DropIndicator` renders a thin line or ghost preview card. `gap_height` from `ctx.get_zone_height(&dragged_id)` fills the gap via `min-height`. `position: absolute` allows growth beyond gap.
9. **Primitive container customization via `render_container`.** All three primitives accept `render_container: Option<Callback<*RenderProps, Element>>`. **Critical:** `DropZoneRenderProps.onmounted` must be wired or rect measurement breaks.

## Signal Reactivity Patterns

- Hot-path position updates (~60/sec) use `write_unchecked()` to avoid unnecessary re-renders
- Target changes use `*current_target.write() = target` only when the value changes
- `peek()` for one-shot lookups in event handlers (no reactive subscription)
- Traversal signals: `traversal_item` (infrequent) and `traversal_fraction` (60fps, single subscriber). Traversal item gets `transition: transform 0s`; non-traversal items keep CSS transitions.
- **Borrow gotcha:** `ctx.active_signal().read()` fails — bind first: `let sig = ctx.active_signal(); let val = sig.read();`

## Common Usage Patterns

### Single sortable list
```rust
SortableContext {
    id: DragId::new("my-list"),
    items: item_ids,
    on_reorder: move |e: ReorderEvent| { e.apply_single(&items, |t: &Task| t.id()); },
}
```

### Multi-list (Kanban)
```rust
SortableGroup {
    on_reorder: move |e| { e.apply(&containers, |t| t.id()); },
    on_move: move |e| { e.apply(&containers, |t| t.id()); },
    SortableContext { id: "todo", items: todo_ids, /* children */ }
    SortableContext { id: "done", items: done_ids, /* children */ }
    DragOverlay { /* overlay content */ }
}
```

### Nested container conventions
- Inner container ID = `{id}-container`
- `MoveEvent.to_container` ending in `-container` means moving INTO a group
- `cleanup_orphaned_groups()` dissolves groups with < 2 members after moves

## Feature Flags

- `web` / `desktop` / `mobile` — platform selection
- `styles` — gates `THEME_STYLES` and `GROUPED_THEME_STYLES`
- Headless/Tailwind: `default-features = false, features = ["web"]` — omits theme CSS; `FUNCTIONAL_STYLES` and `FEEDBACK_STYLES` always available

## Behavioral Gotchas

- **Activation threshold:** Collision skips until pointer moves ≥3px (`ACTIVATION_DISTANCE_SQ`). Prevents false targets from synthetic `pointermove` after `setPointerCapture()`.
- **No-target placeholder:** `is_dragging && drop_location.is_none()` → SortableItem shows 30% opacity dashed placeholder (`.no-target` class). `SortableItemState.is_placeholder` exposes this.
- **Merge member order is [target, source].** Drop target retains position, becomes first member.
- **Nested container boundary sensitivity.** Bottom zone (85% height) on last item can cross container boundary. Use 70–75% for reliable within-container targeting.
- **Displacement shifts bounding boxes mid-drag.** For multi-step drag tests, capture bounds before drag start.
- **Displacement returns full style strings.** `"transform: translateY(Xpx)"` — use `style: "{displacement_transform}"` in RSX.
- **Collision code flow:** Item matching (2D `effective_contains`) runs BEFORE container matching.
- **Stale `dx serve` builds.** After branch switches, kill and restart — hot-reload can serve stale WASM.
- **Unique ARIA instruction IDs.** Each `DragContextProvider` gets a unique `instructions_id` via `AtomicU32` counter (`"dxdnd-drag-instructions-{N}"`).

## DropLocation Variants

| Variant | Meaning | Used by |
|---|---|---|
| `AtIndex { container_id, index }` | Insert at position | Sortable collision, keyboard |
| `IntoItem { container_id, item_id }` | Merge/group with item | SortableWithMerge (55% zone + gap extension) |
| `IntoContainer { container_id }` | Drop into container | Pointer/ClosestCenter |

**Index convention:** `AtIndex.index` uses "filtered-list" convention (dragged item removed). Final position after drag completes. Filtered→full: `if source_idx <= filtered_idx { filtered + 1 } else { filtered }`.

## CSS Tier System

- `FUNCTIONAL_STYLES` — positioning, pointer-events (always available)
- `FEEDBACK_STYLES` — DnD visual indicators (always available)
- `THEME_STYLES` — colors, spacing, layout (gated by `styles` feature)
- `GROUPED_FUNCTIONAL_STYLES`, `GROUPED_FEEDBACK_STYLES`, `GROUPED_THEME_STYLES` — same tiers for grouped patterns; theme uses CSS variable tokens (`--dxdnd-grouped-*`)
- Container layout (`display: flex; flex-direction: column; gap`) is in theme CSS only

## E2E Tests

Live in `tests/e2e/` (gitignored). Use Playwright + running dev server.

```bash
dx serve --example workout_tracker --port 8080
cd ~/.claude/skills/playwright-skill && node run.js /home/glitch/code/dioxus-nox/crates/dnd/tests/e2e/stories-single-moves.js
```

Suites: `stories-single-moves.js`, `stories-merge-supersets.js`, `stories-group-ops.js`.

**Timing sensitivity:** Adjacent merges (idx 0 → idx 1) need 20 steps/30ms delay/500ms hold. Non-adjacent merges work with default timing.

### UX Filmstrip Testing

```bash
cd ~/.claude/skills/playwright-skill && node run.js /home/glitch/code/dioxus-nox/crates/dnd/tests/e2e/ux-smoke.js
```

Files: `ux-smoke.js`, `ux-analysis.js`, `ux-partial-drags.js`, `ux-groups.js`, `ux-grab-position.js`.
Output: `tests/e2e/filmstrips/` — `{scenario}.png` + `{scenario}-metrics.json`.

## CI Commands

```bash
cargo test -p dioxus-nox-dnd
cargo clippy -p dioxus-nox-dnd -- -D warnings
cargo clippy -p dioxus-nox-dnd --target wasm32-unknown-unknown -- -D warnings
```
