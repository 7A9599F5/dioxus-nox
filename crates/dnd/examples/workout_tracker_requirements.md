# Workout Tracker Requirements

## Overview

A workout planning interface where users can organize exercises into a single workout routine. Exercises can be grouped into **supersets** (pairs/groups of exercises performed back-to-back without rest).

---

## Domain Concepts

### Exercise
A single exercise in the workout (e.g., "Bench Press 3×10").
- Has: id, name, sets, reps
- May belong to a superset (superset_id) or be standalone (superset_id = None)

### Superset
A group of 2+ exercises performed consecutively without rest between them.
- Visually represented by a **SupersetHeader** followed by member exercises
- Members share the same superset_id
- Minimum 2 members required; auto-dissolves if < 2 members remain

### SupersetHeader
A draggable header element that:
- Visually labels the superset group
- Acts as a drag handle to move the entire superset as a unit

---

## Data Model

```
Initial State:
┌─────────────────────────┐
│ Bench Press      3 × 10 │  ← standalone exercise
├─────────────────────────┤
│ Incline Press    3 × 10 │  ← standalone exercise
├─────────────────────────┤
│ Chest Fly        3 × 12 │  ← standalone exercise
├─────────────────────────┤
│ Tricep Dips      3 × 15 │  ← standalone exercise
├─────────────────────────┤
│ Overhead Ext.    3 × 12 │  ← standalone exercise
└─────────────────────────┘

After creating a superset (Bench + Incline):
┌─────────────────────────┐
│ ⋮⋮ SUPERSET             │  ← SupersetHeader (draggable)
├─────────────────────────┤
│ │ Bench Press    3 × 10 │  ← exercise in superset
├─────────────────────────┤
│ │ Incline Press  3 × 10 │  ← exercise in superset
├─────────────────────────┤
│ Chest Fly        3 × 12 │  ← standalone exercise
├─────────────────────────┤
│ Tricep Dips      3 × 15 │  ← standalone exercise
├─────────────────────────┤
│ Overhead Ext.    3 × 12 │  ← standalone exercise
└─────────────────────────┘
```

---

## User Stories

### US-1: Reorder Standalone Exercises

**As a** user
**I want to** drag exercises to reorder them
**So that** I can organize my workout in the order I want to perform them

#### Acceptance Criteria

**Scenario 1.1: Basic reorder (drag to gap between items)**
```
Given: Exercises A, B, C in order
When:  User drags A to the gap between B and C
Then:  Order becomes B, A, C
```

**Scenario 1.2: Drag to top of list**
```
Given: Exercises A, B, C in order
When:  User drags C to the gap above A
Then:  Order becomes C, A, B
```

**Scenario 1.3: Drag to bottom of list**
```
Given: Exercises A, B, C in order
When:  User drags A to the gap below C
Then:  Order becomes B, C, A
```

#### Interaction Details
- **Drop zones**: Top/bottom 30% of each item triggers Before/After placement
- **Visual feedback**: Drop indicator line appears at insertion point
- **Drag overlay**: Shows "Moving..." overlay following cursor

---

### US-2: Create Superset by Merging

**As a** user
**I want to** drag one exercise onto the center of another
**So that** I can group them into a superset

#### Acceptance Criteria

**Scenario 2.1: Merge two standalone exercises**
```
Given: Standalone exercises A and B
When:  User drags A onto the CENTER of B (middle 40% zone)
Then:
  - A new SupersetHeader is created above B
  - Both A and B become members of the new superset
  - A is positioned immediately after B (after the target)
  - Visual: Both exercises show superset styling (left border, background)
```

**Scenario 2.2: Add exercise to existing superset**
```
Given: Superset containing [X, Y] and standalone exercise Z
When:  User drags Z onto the CENTER of X (a superset member)
Then:
  - Z joins the existing superset
  - Z is positioned immediately after X
  - No new header is created
  - Visual: Z shows superset styling
```

**Scenario 2.3: Merge from different superset**
```
Given: Superset1 containing [A, B] and Superset2 containing [X, Y]
When:  User drags A onto the CENTER of X
Then:
  - A leaves Superset1 and joins Superset2
  - A is positioned immediately after X
  - If Superset1 now has < 2 members, it auto-dissolves (see US-5)
```

#### Interaction Details
- **Merge zone**: Center 40% of item height triggers IntoItem (merge)
- **Visual feedback**: Target item shows merge highlight (outline + background)
- **Collision detection**: Uses 30/40/30 split (Before/IntoItem/After zones)

---

### US-3: Reorder Within Superset

**As a** user
**I want to** drag exercises within a superset to reorder them
**So that** I can control the order of exercises in my superset

#### Acceptance Criteria

**Scenario 3.1: Reorder within same superset**
```
Given: Superset containing [A, B, C] in order
When:  User drags A to the gap between B and C
Then:
  - Order becomes [B, A, C] within the superset
  - All items remain in the superset
  - SupersetHeader position unchanged
```

**Scenario 3.2: Drag to different position in superset block**
```
Given: Superset containing [A, B, C]
When:  User drags C to just after the SupersetHeader
Then:
  - Order becomes [C, A, B] within the superset
  - Item remains in superset
```

---

### US-4: Move Entire Superset

**As a** user
**I want to** drag the superset header to move the entire group
**So that** I can reposition a superset without breaking it apart

#### Acceptance Criteria

**Scenario 4.1: Move superset up in list**
```
Given:
  - Standalone exercise A
  - Superset [Header, B, C]
  - Standalone exercise D
When:  User drags the SupersetHeader above A
Then:
  - Entire superset moves: [Header, B, C], A, D
  - Superset membership preserved
  - All members move together as a unit
```

**Scenario 4.2: Move superset down in list**
```
Given:
  - Superset [Header, A, B]
  - Standalone exercise C
  - Standalone exercise D
When:  User drags the SupersetHeader below D
Then:
  - Order becomes: C, D, [Header, A, B]
  - Superset remains intact
```

**Scenario 4.3: Move superset between other supersets**
```
Given:
  - Superset1 [Header1, A, B]
  - Superset2 [Header2, X, Y]
When:  User drags Header1 below Header2
Then:
  - Order: [Header2, X, Y], [Header1, A, B]
  - Both supersets remain intact and separate
```

#### Interaction Details
- **Dragging header**: Moves header + all members with matching superset_id
- **Group collection**: All items belonging to superset are collected and moved as unit
- **Insertion**: Entire group inserted at computed drop position

---

### US-5: Remove Exercise from Superset

**As a** user
**I want to** drag an exercise out of a superset
**So that** I can make it standalone again

#### Acceptance Criteria

**Scenario 5.1: Drag exercise outside superset block**
```
Given: Superset [Header, A, B, C] followed by standalone D
When:  User drags B to the gap after D (outside superset block)
Then:
  - B becomes standalone (superset_id = None)
  - Superset now contains [Header, A, C]
  - B positioned after D
```

**Scenario 5.2: Auto-dissolve when < 2 members remain**
```
Given: Superset [Header, A, B]
When:  User drags A outside the superset block
Then:
  - A becomes standalone
  - Superset now has only 1 member (B)
  - Superset auto-dissolves:
    - Header is removed
    - B becomes standalone (superset_id = None)
```

**Scenario 5.3: Drag to position before superset**
```
Given: Standalone X, then Superset [Header, A, B]
When:  User drags A to gap between X and Header
Then:
  - A is now standalone, positioned between X and Header
  - Superset auto-dissolves (only B remains)
  - Final order: X, A, B (all standalone)
```

#### Interaction Details
- **Context detection**: `get_superset_at_index()` determines if drop position is within a superset block
- **Auto-dissolve trigger**: After every reorder, `cleanup_orphaned_supersets()` runs
- **Cleanup logic**: Supersets with < 2 members have headers removed and members become standalone

---

### US-6: Join Existing Superset via Reorder

**As a** user
**I want to** drag a standalone exercise into a superset block
**So that** I can add it to the superset without using merge

#### Acceptance Criteria

**Scenario 6.1: Drag standalone into superset block**
```
Given: Superset [Header, A, B] followed by standalone C
When:  User drags C to gap between A and B (within superset block)
Then:
  - C joins the superset (superset_id set)
  - Order within superset: [A, C, B]
  - C shows superset styling
```

**Scenario 6.2: Drag standalone to end of superset**
```
Given: Superset [Header, A, B] followed by standalone C, D
When:  User drags D to gap after B but before C
Then:
  - D joins the superset
  - Superset now contains [A, B, D]
  - C remains standalone
```

#### Interaction Details
- **Block detection**: Drop position checked against superset boundaries
- **Boundary definition**: Superset block = Header + consecutive members with same superset_id
- **Auto-join**: Exercise dropped within block automatically gets superset_id set

---

## Visual States

### Exercise Card States
| State | Visual |
|-------|--------|
| Standalone | White background, full border-radius |
| In Superset | Left purple border, gradient background, no top/bottom radius |
| Dragging | 50% opacity at original position |
| Merge Target | Purple outline, purple tinted background |

### SupersetHeader States
| State | Visual |
|-------|--------|
| Normal | Purple gradient, uppercase "SUPERSET" label, drag handle |
| Dragging | 50% opacity at original position |

### Drop Indicators
| Zone | Visual | Action |
|------|--------|--------|
| Top 30% | Line above item | Insert before |
| Middle 40% | Item highlight | Merge into |
| Bottom 30% | Line below item | Insert after |

---

## Edge Cases

### EC-1: Drag exercise onto itself
```
When:  User drags A and drops on A's center
Then:  No change (merge with self is no-op)
```

### EC-2: Drag superset header onto member
```
When:  User drags Header and drops on member A within same superset
Then:  Standard reorder within list (no special handling needed)
```

### EC-3: Rapid successive merges
```
Given: Standalone A, B, C
When:  User quickly merges A→B, then C→(A,B superset)
Then:  All three end up in same superset
```

### EC-4: Last exercise in list
```
Given: Exercises A, B with B at bottom
When:  User drags A below B
Then:  Order becomes B, A (drop after last item works)
```

### EC-5: Single item cannot form superset
```
Given: Superset [Header, A, B, C]
When:  User drags A and B out of superset
Then:
  - Superset auto-dissolves (only C remains)
  - C becomes standalone
  - Header is removed
```

---

## Collision Detection Zones

```
┌─────────────────────────────┐
│         TOP 30%             │  → DropLocation::Before
│         (reorder)           │
├─────────────────────────────┤
│                             │
│        MIDDLE 40%           │  → DropLocation::IntoItem
│         (merge)             │
│                             │
├─────────────────────────────┤
│        BOTTOM 30%           │  → DropLocation::After
│         (reorder)           │
└─────────────────────────────┘
```

When `enable_merge: false`, zones are 50/50 (Before/After only).

---

## Event Flow

### Reorder Event (same container, Before/After zones)
```
1. User drops item
2. SortableGroup fires on_reorder with ReorderEvent
3. Handler checks if dragging header → group move logic
4. Handler checks if dragging exercise → single item logic
   a. Remove item from current position
   b. Compute target index
   c. Check if target is within superset block
   d. Update superset_id accordingly
   e. Insert at target
5. cleanup_orphaned_supersets() runs
6. UI re-renders
```

### Merge Event (IntoItem zone)
```
1. User drops item on center of target
2. SortableGroup fires on_merge with MergeEvent
3. Handler determines superset_id:
   a. If target in superset → use existing superset_id
   b. If target standalone → create new superset + header
4. e.apply() called:
   a. Removes dragged item from current position
   b. Calls set_parent callback to set superset_id
   c. Inserts dragged item after target
5. cleanup_orphaned_supersets() runs
6. UI re-renders
```

---

## Test Scenarios Summary

| # | Scenario | Trigger | Expected Result |
|---|----------|---------|-----------------|
| 1 | Reorder standalone exercises | Drop in Before/After zone | Items reorder, no superset changes |
| 2 | Create new superset | Drop standalone on standalone center | New header + both joined |
| 3 | Add to existing superset | Drop on superset member center | Item joins superset |
| 4 | Move entire superset | Drag header | Header + all members move together |
| 5 | Remove from superset | Drop outside superset block | Item becomes standalone |
| 6 | Auto-dissolve superset | Remove member leaving < 2 | Header removed, remaining becomes standalone |
| 7 | Join superset via reorder | Drop into superset block gap | Item joins superset |
| 8 | Reorder within superset | Drop in gap within block | Items reorder, superset preserved |
