# dioxus-nox-dnd — Composable drag-and-drop

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Composable drag-and-drop for Dioxus 0.7 WASM/web. Layered architecture: Foundation → Primitives → Patterns → Conveniences. Pluggable collision detection. Supports sortable lists, multi-container (kanban), nested containers, and grouped (flat-data) lists.

## Module Structure
- `context.rs` — `DragContext`, `DragState`, `ActiveDrag`, `DragContextProvider`
- `types.rs` — `DragId`, `DragType`, `DragData`, `Position`, `Rect`, `DropLocation`, event types
- `collision/` — `CollisionStrategy` enum + 4 strategies (Pointer, ClosestCenter, Sortable, SortableWithMerge)
- `primitives/` — `Draggable`, `DropZone`, `DragOverlay` (atomic building blocks)
- `patterns/sortable/` — `SortableContext`, `SortableItem`, `SortableGroup`, `DropIndicator`
- `patterns/grouped.rs` — `GroupedItem` trait, `GroupedList`, reorder/merge/cleanup helpers

## Key Design Decisions
1. `SortableGroup` creates a shared `DragContextProvider` for cross-container drag (mirrors dnd-kit's single `DndContext`)
2. Hot-path position updates (~60fps) use `write_unchecked()` to avoid unnecessary re-renders
3. Collision is pluggable and cursor-aligned; direction-aware zone split uses drag `delta` sign, not pointer-vs-center

## Further Reading
Detailed context in `.context/` — read on demand:
- `architecture.md` — layered composition diagram, source layout, all 9 design decisions
- `collision.md` — CollisionStrategy enum, zone splitting math (15/55/30), DropLocation variants
- `patterns.md` — sortable/kanban/nested usage patterns, index convention, ReorderEvent/MoveEvent
- `gotchas.md` — behavioral edge cases, write_unchecked usage, CSS tier system, E2E test setup

## CI
```bash
cargo check -p dioxus-nox-dnd
cargo test -p dioxus-nox-dnd
cargo clippy -p dioxus-nox-dnd --target wasm32-unknown-unknown -- -D warnings
```
