# Implementation Plan: Subprocess-Based Planning Agent

**Created:** 2026-02-28
**Updated:** 2026-02-28
**Status:** Ready for Implementation
**Model:** glm-5 (Z.ai)

---

## Overview

A two-phase system for feature development:

1. **Planning Phase** - Researchers analyze, produce `SPEC.md` + `WORKFLOW.md`
2. **Implementation Phase** - Orchestrator executes `WORKFLOW.md` with progress tracking

**Key outputs:**
- `SPEC.md` - WHAT we're building (decisions, scope, rationale) - long-lived reference
- `WORKFLOW.md` - HOW to build it (ordered tasks, checkpoints) - implementation-time guide

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      PLANNING PHASE                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. User submits query                                               │
│          ↓                                                           │
│  2. Planning Orchestrator dispatches researcher sub-agents          │
│          ↓                                                           │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Each Researcher Returns:                                     │   │
│  │  • Concerns (with severity)                                   │   │
│  │  • Questions (for user)                                       │   │
│  │  • Tradeoff frameworks                                        │   │
│  │  • Workflow guidance (phase, tasks, checkpoints)              │   │
│  └──────────────────────────────────────────────────────────────┘   │
│          ↓                                                           │
│  3. Synthesizer agent combines findings → SPEC.md                   │
│          ↓                                                           │
│  4. Workflow Synthesizer agent → WORKFLOW.md                        │
│          ↓                                                           │
│  5. Interactive questionnaire (if needed)                           │
│          ↓                                                           │
│  6. Finalize SPEC.md + WORKFLOW.md                                   │
│          ↓                                                           │
│  7. "Ready to implement?" prompt                                     │
│     ├─→ Yes → Transition to implementation                          │
│     └─→ No → Pause for review                                       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    IMPLEMENTATION PHASE                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. User confirms: "proceed" or "/implement"                        │
│          ↓                                                           │
│  2. Implementation Orchestrator loads WORKFLOW.md                   │
│          ↓                                                           │
│  3. For each task (in order):                                       │
│     a. Show progress widget                                         │
│     b. Execute task (read, write, edit, bash)                       │
│     c. Update checkmark in WORKFLOW.md                              │
│     d. Check for checkpoint boundary                                │
│        ├─→ Checkpoint → Pause for verification                     │
│        └─→ No checkpoint → Continue                                │
│     e. Check for re-consult trigger                                 │
│        ├─→ Trigger → Pause, dispatch researcher, incorporate       │
│        └─→ No trigger → Continue                                   │
│          ↓                                                           │
│  4. All tasks complete → Final checkpoint                           │
│          ↓                                                           │
│  5. Archive WORKFLOW.md, SPEC.md remains as docs                    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Output Files

### SPEC.md (Long-lived)
- Problem statement
- Scope definition (in/out)
- Validated decisions with rationale
- Researcher insights
- Success criteria
- Risk register

### WORKFLOW.md (Implementation-time)
- Ordered task list (synthesized from researchers)
- Phase breakdown (foundation → core → integration → polish)
- Checkpoint questions (placed at appropriate phases)
- Re-consult triggers (when to bring researchers back)
- Testing milestones
- Progress tracking (checkmarks)

### File Structure
```
.pi/planning/
├── SPEC.md           # WHAT: Decisions, scope, rationale
├── WORKFLOW.md       # HOW: Tasks, order, checkpoints
└── archive/          # Completed workflows moved here
    └── 2026-02-[feature]-workflow.md
```

---

## Extensions

### Extension 1: Planning Agent

**Location:** `.pi/extensions/planning-agent/`

**Components:**

| File | Purpose |
|------|---------|
| `index.ts` | Main extension, registers `/plan` tool |
| `registry.ts` | Researcher dispatch logic (trigger signals) |
| `synthesis.ts` | Parse/merge researcher outputs → SPEC.md |
| `questionnaire.ts` | Interactive Q&A UI (`ctx.ui.custom()`) |
| `transition.ts` | Handoff to implementation phase |

**Commands:**
| Command | Description |
|---------|-------------|
| `/plan` | Full planning workflow |
| `/plan-quick` | Skip questionnaire, use defaults |
| `/research` | Dispatch researchers only, no spec |

**Tools:**
| Tool | Parameters | Description |
|------|------------|-------------|
| `plan` | `query`, `researchers?` | Run planning workflow |

**Events:**
```typescript
pi.on("tool_execution_start", ...) // Show researcher widgets
pi.on("tool_execution_end", ...)   // Update widgets
```

---

### Extension 2: Implementation Agent

**Location:** `.pi/extensions/implementation-agent/`

**Components:**

| File | Purpose |
|------|---------|
| `index.ts` | Main extension, registers `/implement` tool |
| `workflow-parser.ts` | Parse WORKFLOW.md, extract tasks/phases |
| `checkpoint.ts` | Checkpoint detection and handling |
| `progress.ts` | Progress tracking, widget display |
| `reconsult.ts` | Re-dispatch researchers on triggers |

**Commands:**
| Command | Description |
|---------|-------------|
| `/implement` | Start/resume implementation from WORKFLOW.md |
| `/implement-next` | Execute just the next task, then stop |
| `/implement-status` | Show current progress |
| `/checkpoint` | Manually trigger checkpoint review |
| `/reconsult [name]` | Re-dispatch a specific researcher |

**Tools:**
| Tool | Parameters | Description |
|------|------------|-------------|
| `implement` | `spec_path`, `workflow_path` | Execute WORKFLOW.md |

**Events:**
```typescript
pi.on("tool_execution_start", ...) // Show task widgets
pi.on("turn_end", ...)             // Check for checkpoints/triggers
```

**Core Logic:**

```typescript
// workflow-parser.ts
interface ParsedWorkflow {
  phases: Phase[];
  checkpoints: Checkpoint[];
  reconsultTriggers: ReconsultTrigger[];
  testingMilestones: TestingMilestone[];
}

interface Phase {
  name: string;
  tasks: Task[];
  researchers: string[];
}

interface Task {
  id: string;
  description: string;
  completed: boolean;
  file?: string;
}

interface Checkpoint {
  afterTask: string;
  questions: CheckpointQuestion[];
}

// progress.ts
function showProgressWidget(workflow: ParsedWorkflow, currentTask: string) {
  const lines = [];
  // Build widget showing phases, tasks, current progress
  ctx.ui.setWidget("implementation-progress", lines);
}

// checkpoint.ts
async function handleCheckpoint(checkpoint: Checkpoint, ctx: ExtensionContext) {
  // Show checkpoint questions
  // Get user verification
  // Optionally re-dispatch researcher
  // Continue or pause
}

// reconsult.ts
async function reconsultResearcher(researcher: string, context: string) {
  // Spawn researcher sub-agent
  // Incorporate findings
  // Update SPEC.md if needed
}
```

---

## Shared Library

**Location:** `.pi/lib/`

| File | Purpose |
|------|---------|
| `researcher-dispatch.ts` | Spawn researcher sub-agents (used by both extensions) |
| `widget-helpers.ts` | Shared widget formatting utilities |
| `yaml-parser.ts` | Parse researcher YAML outputs |

**Why shared:**
- Planning agent dispatches researchers initially
- Implementation agent re-dispatches on triggers
- Both need same spawning logic and output parsing

---

## Researcher Agents

**Location:** `.pi/agents/`

| File | Model | Tools | Purpose |
|------|-------|-------|---------|
| `security.md` | glm-5 | read, grep, find, ls | Auth, data protection, vulnerabilities |
| `ux-ui.md` | glm-5 | read, grep, find, ls | User flows, interaction patterns |
| `performance.md` | glm-5 | read, grep, find, ls | Latency, caching, scale |
| `data-database.md` | glm-5 | read, grep, find, ls | Schema, queries, consistency |
| `api.md` | glm-5 | read, grep, find, ls | Interface design, versioning |
| `accessibility.md` | glm-5 | read, grep, find, ls | WCAG, keyboard, screen readers |
| `testing.md` | glm-5 | read, grep, find, ls | Test strategy, coverage |
| `architecture.md` | glm-5 | read, grep, find, ls | Module boundaries, dependencies |
| `devops.md` | glm-5 | read, grep, find, ls | Deployment, CI/CD, monitoring |
| `compliance.md` | glm-5 | read, grep, find, ls | GDPR, HIPAA, regulations |
| `workflow-synthesizer.md` | glm-5 | read | Combine workflow guidance → WORKFLOW.md |

**Each researcher output format:**

```yaml
researcher: [name]
domain_summary: [1-2 sentence analysis]

concerns:
  - id: [ID]
    severity: critical|high|medium|low
    category: [category]
    description: [what]
    why_it_matters: [rationale]
    questions:
      - question: [specific question]
        options:
          - label: [Option A]
            tradeoffs: [tradeoffs]
          - label: [Option B]
            tradeoffs: [tradeoffs]
        hybrid_possible: true|false

workflow_guidance:
  phase: foundation|core|integration|polish
  order: [numeric priority]
  tasks:
    - "[task description]"
  checkpoint_questions:
    - "[question to verify]"
  reconsult_when:
    - "[trigger condition]"
  testing_milestones:
    - "[what tests when]"
```

---

## Widget Displays

### Planning Phase Widget

```
┌─────────────────────────────────────────┐
│ 📋 Planning Agent Progress               │
├─────────────────────────────────────────┤
│ ✓ security-researcher     (2 turns)     │
│ ✓ ux-ui-researcher        (1 turn)      │
│ ⏳ performance-researcher  running...    │
│ ○ testing-researcher      waiting       │
│ ○ workflow-synthesizer    waiting       │
└─────────────────────────────────────────┘
```

### Implementation Phase Widget

```
┌─────────────────────────────────────────┐
│ 🔧 Implementation: User Authentication   │
├─────────────────────────────────────────┤
│ Phase 1: Foundation                      │
│ ✓ Define auth types                      │
│ ✓ Set up password hashing                │
│ ⏳ Create session module   (current)     │
│ ○ Add CSRF protection                    │
│ ○ Write unit tests                       │
│                                          │
│ Progress: 2/5 (40%)                      │
│ Next checkpoint: After task 5            │
└─────────────────────────────────────────┘
```

### Checkpoint Widget

```
┌─────────────────────────────────────────┐
│ ⏸ Checkpoint 1.1                         │
├─────────────────────────────────────────┤
│ [Security] Is password hashing using     │
│ constant-time comparison?                │
│                                          │
│ > Yes, verified                          │
│ > Not yet, need to fix                   │
│ > Re-consult security researcher         │
└─────────────────────────────────────────┘
```

---

## Prompts

**Location:** `.pi/prompts/`

| File | Command | Description |
|------|---------|-------------|
| `plan.md` | `/plan` | Full planning workflow |
| `plan-quick.md` | `/plan-quick` | Quick planning, skip Q&A |
| `research.md` | `/research` | Researchers only, no spec |
| `implement.md` | `/implement` | Start implementation |
| `implement-next.md` | `/implement-next` | One task only |

---

## Templates

**Location:** `.pi/skills/planning-agent/`

### SPEC-TEMPLATE.md

```markdown
# Design Specification: [Feature Name]

**Created:** [Date]
**Status:** [ ] Draft [ ] Approved [ ] Complete

## 1. Problem Statement
## 2. Scope Definition
## 3. Researcher Insights
## 4. Validated Decisions
## 5. Technical Approach
## 6. Open Questions
## 7. Risk Register
## 8. Success Criteria
```

### WORKFLOW-TEMPLATE.md

```markdown
# Implementation Workflow: [Feature Name]

**Generated:** [Date]
**From Spec:** SPEC.md

## Overview
| Metric | Value |
|--------|-------|
| Total Phases | [N] |
| Total Tasks | [N] |
| Checkpoints | [N] |

---

## Phase 1: Foundation
*Source: [Researchers]*

### Tasks
- [ ] [Task 1]
- [ ] [Task 2]

### Checkpoint 1.1
- [ ] [Researcher]: [Question]?

---

## Phase 2: Core
...

---

## Re-consult Researchers
| Trigger | Researcher |
|---------|------------|
| [When] | [Who] |

## Testing Milestones
| Phase | Tests |
|-------|-------|
| After Phase 1 | [Tests] |

## Progress Log
| Date | Task | Notes |
|------|------|-------|
```

---

## File Structure

```
.pi/
├── agents/
│   ├── security.md
│   ├── ux-ui.md
│   ├── performance.md
│   ├── data-database.md
│   ├── api.md
│   ├── accessibility.md
│   ├── testing.md
│   ├── architecture.md
│   ├── devops.md
│   ├── compliance.md
│   └── workflow-synthesizer.md
├── extensions/
│   ├── planning-agent/
│   │   ├── index.ts
│   │   ├── registry.ts
│   │   ├── synthesis.ts
│   │   ├── questionnaire.ts
│   │   └── transition.ts
│   └── implementation-agent/
│       ├── index.ts
│       ├── workflow-parser.ts
│       ├── checkpoint.ts
│       ├── progress.ts
│       └── reconsult.ts
├── lib/
│   ├── researcher-dispatch.ts
│   ├── widget-helpers.ts
│   └── yaml-parser.ts
├── prompts/
│   ├── plan.md
│   ├── plan-quick.md
│   ├── research.md
│   ├── implement.md
│   └── implement-next.md
├── planning/
│   ├── SPEC.md
│   ├── WORKFLOW.md
│   └── archive/
└── skills/
    └── planning-agent/
        ├── DESIGN-SPEC.md
        ├── IMPLEMENTATION-PLAN.md
        ├── SPEC-TEMPLATE.md
        ├── WORKFLOW-TEMPLATE.md
        └── researchers/
```

---

## Implementation Order

| Step | What | Dependencies |
|------|------|--------------|
| **1** | **Researcher Agents** | |
| 1a | Create researcher `.md` files (with workflow_guidance format) | None |
| 1b | Create `workflow-synthesizer.md` agent | 1a |
| **2** | **Shared Library** | |
| 2a | Create `researcher-dispatch.ts` (spawn sub-agents) | 1a |
| 2b | Create `yaml-parser.ts` (parse outputs) | None |
| 2c | Create `widget-helpers.ts` | None |
| **3** | **Planning Extension** | |
| 3a | Create `registry.ts` (dispatch signals) | 1a |
| 3b | Create `synthesis.ts` (merge findings → SPEC.md) | 2b |
| 3c | Create `questionnaire.ts` (interactive UI) | None |
| 3d | Create `transition.ts` (handoff logic) | None |
| 3e | Create `index.ts` (plan tool, wire everything) | 3a-d, 2a |
| **4** | **Implementation Extension** | |
| 4a | Create `workflow-parser.ts` (parse WORKFLOW.md) | None |
| 4b | Create `progress.ts` (widgets, tracking) | 2c |
| 4c | Create `checkpoint.ts` (detection, handling) | 4a |
| 4d | Create `reconsult.ts` (re-dispatch researchers) | 2a |
| 4e | Create `index.ts` (implement tool, wire everything) | 4a-d |
| **5** | **Templates & Prompts** | |
| 5a | Create SPEC-TEMPLATE.md | None |
| 5b | Create WORKFLOW-TEMPLATE.md | None |
| 5c | Create prompt files (`/plan`, `/implement`, etc.) | None |
| **6** | **Integration & Testing** | |
| 6a | Test planning end-to-end | 3, 5 |
| 6b | Test implementation end-to-end | 4, 5 |
| 6c | Test transition (planning → implementation) | 6a, 6b |
| 6d | Test checkpoint flow | 6b |
| 6e | Test re-consult flow | 6b |
| **7** | **Documentation** | |
| 7a | Update DESIGN-SPEC.md with final architecture | All |
| 7b | Document commands and workflows | All |

---

## Key Decisions

| Decision | Choice |
|----------|--------|
| **Parallel vs Sequential researchers** | Parallel (spawn all at once) |
| **Researcher output format** | YAML structured |
| **Max concurrent researchers** | 4 |
| **Model** | `glm-5` (Z.ai) |
| **Spec + Workflow** | Separate files |
| **Workflow synthesis** | Dedicated agent |
| **Implementation tracking** | Dedicated extension |
| **Shared code** | `.pi/lib/` directory |

---

## Transition Flow Detail

```
PLANNING COMPLETE
        │
        ▼
┌─────────────────────────────────┐
│  "Planning complete.            │
│   SPEC.md and WORKFLOW.md       │
│   created in .pi/planning/      │
│                                 │
│   Ready to implement?"          │
│                                 │
│   [Yes] [Review first]          │
└─────────────────────────────────┘
        │
   ┌────┴────┐
   │         │
  Yes      Review
   │         │
   ▼         ▼
┌─────┐   ┌─────────────────────────────┐
│/impl│   │ User reviews/edits files     │
│     │   │ manually, then runs          │
│     │   │ /implement when ready        │
└─────┘   └─────────────────────────────┘
   │
   ▼
IMPLEMENTATION STARTS
```

---

## Reference Files

- Subagent Example: `~/.nvm/.../pi-coding-agent/examples/extensions/subagent/`
- Questionnaire Example: `~/.nvm/.../pi-coding-agent/examples/extensions/questionnaire.ts`
- Plan Mode Example: `~/.nvm/.../pi-coding-agent/examples/extensions/plan-mode/`
- Extension Docs: `~/.nvm/.../pi-coding-agent/docs/extensions.md`

---

## Handoff Checklist

Before beginning implementation, confirm:

- [ ] Plan reviewed and approved
- [ ] Current planning workflow completed or paused
- [ ] Ready to create/modify files in `.pi/` directory
- [ ] Z.ai token confirmed working for `glm-5`
- [ ] Understand two-phase flow: Planning → Implementation
- [ ] Understand: SPEC.md (WHAT) vs WORKFLOW.md (HOW)

**To begin implementation:** Say "proceed" or "begin implementation"
