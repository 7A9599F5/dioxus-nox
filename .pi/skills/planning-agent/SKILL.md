---
name: planning-agent
description: >
  Comprehensive planning agent that guides requirements gathering through assumption-challenging 
  and domain-expert researcher sub-agents. Produces thorough design specification documents. 
  Use when starting any significant feature or project to clarify scope, identify tradeoffs, 
  and create a validated specification before implementation.
license: MIT
metadata:
  version: "1.0.0"
  researchers:
    - security
    - ux-ui
    - performance
    - data-database
    - api
    - accessibility
    - testing
    - architecture
    - devops
    - compliance
---

# Planning Agent

A comprehensive planning agent that produces validated design specifications through assumption-challenging and domain-expert analysis.

## When to Use

Invoke this skill when:
- Starting a new feature or significant change
- Requirements are ambiguous or underspecified
- Multiple approaches are possible
- Cross-cutting concerns exist (security, performance, accessibility, etc.)
- You want a documented specification before implementation

## Core Principles

1. **Assumption Skepticism**: Surface and challenge every assumption
2. **Domain Expertise**: Deploy specialist researchers who ask informed questions
3. **Hybrid Thinking**: When options conflict, explore combinations
4. **Progressive Validation**: Confirm understanding at each phase

## Phase Model

### Phase 0: Intake
- Receive initial request
- Identify which researchers to dispatch
- List immediate clarifying questions

### Phase 1: Researcher Analysis
- Deploy relevant domain specialist researchers
- Each researcher produces structured findings
- Synthesize findings into prioritized concerns

### Phase 2: Assumption Mapping
- Enumerate implicit assumptions with researcher input
- Categorize by risk level (critical, high, medium, low)
- Cross-reference with researcher-identified concerns

### Phase 3: Guided Exploration
- Iterative Q&A using the questionnaire tool
- Questions informed by domain expertise
- Present options with tradeoffs and hybrid possibilities

### Phase 4: Option Synthesis
- Present coherent option sets
- Show how decisions interconnect across domains
- Identify locked-in vs. flexible choices

### Phase 5: Specification Drafting
- Produce structured design spec document
- Include rationale for each decision
- Document researcher insights

### Phase 6: Validation & Handoff
- User reviews and approves spec
- Confirm scope boundaries
- Clear transition to implementation

## Researcher Dispatch Guide

### Trigger Signals

| Signal | Dispatch Researcher |
|--------|---------------------|
| `auth`, `login`, `password`, `session`, `token`, `security` | security |
| `form`, `button`, `modal`, `navigation`, `dashboard`, `UI`, `UX` | ux-ui, accessibility |
| `scale`, `thousands`, `real-time`, `latency`, `slow`, `fast` | performance |
| `database`, `store`, `query`, `table`, `model`, `schema` | data-database |
| `API`, `endpoint`, `REST`, `GraphQL`, `webhook` | api |
| `module`, `package`, `dependency`, `architecture`, `structure` | architecture |
| `deploy`, `production`, `staging`, `CI/CD`, `infrastructure` | devops |
| `HIPAA`, `GDPR`, `SOC2`, `compliance`, `audit`, `PII` | compliance |
| **[ALWAYS]** | testing |

### Researcher Files

Load domain expertise from:
- [Security Researcher](researchers/security.md)
- [UX/UI Researcher](researchers/ux-ui.md)
- [Performance Researcher](researchers/performance.md)
- [Data/Database Researcher](researchers/data-database.md)
- [API Researcher](researchers/api.md)
- [Accessibility Researcher](researchers/accessibility.md)
- [Testing Researcher](researchers/testing.md) - **Always dispatch**
- [Architecture Researcher](researchers/architecture.md)
- [DevOps Researcher](researchers/devops.md)
- [Compliance Researcher](researchers/compliance.md)

## Assumption Statement Format

For every identified assumption:

```
ASSUMPTION [category] [risk]: <what I'm assuming>
BECAUSE: <why I might think this>
UNCERTAINTY: <confidence 1-5>
CHALLENGE: <what if I'm wrong?>
ALTERNATIVES: <other possibilities>
```

## Question Presentation Format

For each question to the user:

```markdown
**[Q#] Question Title**

*Source: [Researcher Name]*

**Context:** [Why this domain specialist raised this concern]

**Question:** [The actual question using domain vocabulary]

| Option | [Criterion 1] | [Criterion 2] | [Criterion 3] |
|--------|---------------|---------------|---------------|
| A) ... | ... | ... | ... |
| B) ... | ... | ... | ... |

**Hybrid:** [Can these be combined? How?]

**Impact:** [What downstream decisions does this affect?]
```

## Output: Design Specification

The final output is a structured specification document. See [DESIGN-SPEC.md](DESIGN-SPEC.md) for the complete format.

Key sections:
1. Problem Statement
2. Scope Definition (in/out of scope)
3. Researcher Insights (domain-specific findings)
4. Validated Decisions (with rationale)
5. Technical Approach
6. Open Questions (deferred decisions)
7. Risk Register
8. Success Criteria
9. Implementation Handoff

## Usage

Invoke via:
```
/skill:planning-agent
```

Or describe what you want to plan:
```
Help me plan a user authentication feature
```

## Anti-Patterns to Avoid

1. **Silent assumptions** - Always surface what you're assuming
2. **Generic questions** - Use domain vocabulary from researchers
3. **Binary thinking** - Always explore hybrid possibilities
4. **Skipping researchers** - Dispatch all relevant specialists
5. **Missing attribution** - Credit the researcher source for questions

## Example Flow

```
USER: I want to add user authentication to the app.

AGENT:
## Phase 0: Intake

Dispatching researchers:
- Security (auth triggers)
- UX/UI (user-facing)
- Testing (always)
- Data/Database (user data)
- API (auth endpoints)

## Phase 1: Researcher Findings

### Security Researcher Concerns:
- SEC-001: Authentication mechanism not specified
- SEC-002: Session management strategy unclear
- SEC-003: MFA requirements unknown

### UX/UI Researcher Concerns:
- UX-001: Login flow complexity unknown
- UX-002: Error handling for failed auth

### Testing Researcher Concerns:
- TEST-001: Auth flows require comprehensive coverage

## Phase 3: Guided Exploration

[Uses questionnaire tool with synthesized questions from all researchers, each attributed to the domain specialist who raised it]

## Phase 5: Specification

[Produces structured spec document with all validated decisions, rationale, and researcher insights]
```

---

For the complete design specification, see [DESIGN-SPEC.md](DESIGN-SPEC.md).
