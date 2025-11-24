# GitHub Projects Kanban - Visual Workflow

## 📊 Board Structure

```
╔════════════════════════════════════════════════════════════════════════════╗
║                        ULTIMO DEVELOPMENT BOARD                             ║
╚════════════════════════════════════════════════════════════════════════════╝

┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  📋 BACKLOG     │  │  🎯 READY       │  │ 🚧 IN PROGRESS  │  │  👀 REVIEW      │  │  ✅ DONE        │
│                 │  │                 │  │                 │  │                 │  │                 │
│ • New issues    │  │ • Prioritized   │  │ • Assigned      │  │ • PR opened     │  │ • Merged        │
│ • Not triaged   │  │ • All deps met  │  │ • Actively      │  │ • Needs review  │  │ • Deployed      │
│ • Need review   │  │ • Can start now │  │   working       │  │ • CI running    │  │ • Verified      │
│                 │  │                 │  │                 │  │                 │  │                 │
│ (Unlimited)     │  │ (10-15 items)   │  │ (Max 5 items)   │  │ (5-10 PRs)      │  │ (Auto-archive)  │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  └─────────────────┘
         │                    │                    │                    │
         │    Maintainer      │    Assignee        │    Open PR         │    Merge PR
         │    prioritizes     │    claims issue    │                    │
         └────────────────────┴────────────────────┴────────────────────┘
```

## 🔄 Issue Lifecycle

```
┌──────────┐
│  CREATE  │  New issue created (bug/feature/docs)
└─────┬────┘
      │
      ▼
┌──────────┐
│ TRIAGE   │  Maintainer reviews, adds labels
└─────┬────┘
      │
      ├─────▶ 📋 BACKLOG (needs more info / future work)
      │
      └─────▶ 🎯 READY (clear requirements, can start)
                 │
                 ▼
            ┌──────────┐
            │ ASSIGN   │  Team member picks up work
            └─────┬────┘
                  │
                  ▼
              🚧 IN PROGRESS (actively coding)
                  │
                  ▼
            ┌──────────┐
            │ OPEN PR  │  Create pull request
            └─────┬────┘
                  │
                  ▼
              👀 REVIEW (code review)
                  │
                  ├─────▶ Changes requested ──┐
                  │                           │
                  ▼                           │
            ┌──────────┐                      │
            │ APPROVED │                      │
            └─────┬────┘                      │
                  │                           │
                  ▼                           │
            ┌──────────┐                      │
            │  MERGE   │                      │
            └─────┬────┘                      │
                  │                           │
                  ▼                           │
              ✅ DONE                         │
                                             │
                  Address feedback ◀─────────┘
```

## 🏷️ Label Hierarchy

```
┌────────────────────────────────────────────────────────────────┐
│                         ISSUE LABELS                            │
└────────────────────────────────────────────────────────────────┘

PRIORITY                TYPE                    AREA
┌─────────────┐        ┌──────────────┐       ┌──────────────┐
│ 🔴 Critical │        │ 🚀 Feature   │       │ 🏗️  Core     │
│ 🟠 High     │   +    │ 🐛 Bug       │   +   │ 🌐 RPC       │
│ 🟡 Medium   │        │ 📚 Docs      │       │ 📖 OpenAPI   │
│ 🟢 Low      │        │ ⚡ Performance│      │ 🗄️  Database │
└─────────────┘        │ 🔧 Refactor  │       │ 🛠️  CLI      │
                       │ 🧪 Test      │       │ 📱 Examples  │
                       └──────────────┘       │ 📚 Docs      │
                                              └──────────────┘

Example: 🔴 priority: critical  +  🐛 type: bug  +  🏗️ area: core
         = Critical core framework bug
```

## 👥 Contributor Journey

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      NEW CONTRIBUTOR PATH                                │
└─────────────────────────────────────────────────────────────────────────┘

1. DISCOVER                    2. CLAIM                     3. DEVELOP
   ┌─────────┐                   ┌─────────┐                  ┌─────────┐
   │ Browse  │                   │ Comment │                  │  Code   │
   │ issues  │──────────────────▶│ on      │─────────────────▶│  and    │
   │ labeled │                   │ issue   │                  │  test   │
   │ "good   │                   │ to      │                  │         │
   │ first   │                   │ claim   │                  │         │
   │ issue"  │                   │         │                  │         │
   └─────────┘                   └─────────┘                  └─────────┘
                                                                    │
                                                                    │
4. REVIEW                      5. ITERATE                   6. MERGE
   ┌─────────┐                   ┌─────────┐                  ┌─────────┐
   │ Open PR │                   │ Address │                  │ Get     │
   │ with    │◀──────────────────│ review  │◀─────────────────│ merged  │
   │ tests & │                   │ feedback│                  │ to main │
   │ docs    │                   │         │                  │         │
   └─────────┘                   └─────────┘                  └─────────┘
         │                             │                            │
         └─────────────────────────────┘                            │
                                                                    ▼
                                                            ┌──────────────┐
                                                            │ Celebrate! 🎉│
                                                            │ You're a      │
                                                            │ contributor!  │
                                                            └──────────────┘
```

## 🤖 Automation Flow

```
┌────────────────────────────────────────────────────────────────────────┐
│                         AUTOMATION TRIGGERS                             │
└────────────────────────────────────────────────────────────────────────┘

EVENT                          ACTION                         RESULT
┌────────────────┐          ┌─────────────┐              ┌──────────────┐
│ Issue opened   │─────────▶│ Add to      │─────────────▶│ Appears in   │
│                │          │ project     │              │ 📋 Backlog   │
└────────────────┘          └─────────────┘              └──────────────┘

┌────────────────┐          ┌─────────────┐              ┌──────────────┐
│ Issue assigned │─────────▶│ Move to     │─────────────▶│ 🚧 In Progress│
│                │          │ In Progress │              │              │
└────────────────┘          └─────────────┘              └──────────────┘

┌────────────────┐          ┌─────────────┐              ┌──────────────┐
│ PR opened      │─────────▶│ Move to     │─────────────▶│ 👀 Review    │
│                │          │ Review      │              │              │
└────────────────┘          │ + Auto-label│              │ + Labels     │
                            └─────────────┘              └──────────────┘

┌────────────────┐          ┌─────────────┐              ┌──────────────┐
│ PR merged      │─────────▶│ Move to     │─────────────▶│ ✅ Done      │
│                │          │ Done        │              │              │
└────────────────┘          │ + Close     │              │ + Closed     │
                            │   issues    │              │   issues     │
                            └─────────────┘              └──────────────┘
```

## 📊 Project Views

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           PROJECT VIEWS                                  │
└─────────────────────────────────────────────────────────────────────────┘

VIEW 1: BOARD (Default)          VIEW 2: BY PRIORITY
┌──────────────────────┐          ┌──────────────────────┐
│ 📋→🎯→🚧→👀→✅       │          │ 🔴 Critical          │
│                      │          │   ├─ Issue #123      │
│ Visual kanban        │          │   └─ Issue #124      │
│ Drag & drop cards    │          │ 🟠 High              │
└──────────────────────┘          │   ├─ Issue #125      │
                                  │   └─ Issue #126      │
VIEW 3: BY AREA                   └──────────────────────┘
┌──────────────────────┐
│ 🏗️  Core            │          VIEW 4: MY WORK
│   ├─ Issue #127     │          ┌──────────────────────┐
│   └─ Issue #128     │          │ Assigned to me       │
│ 🌐 RPC              │          │   ├─ Issue #129      │
│   └─ Issue #129     │          │   └─ Issue #130      │
└──────────────────────┘          └──────────────────────┘
```

## 📈 Metrics Dashboard

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PROJECT METRICS                                  │
└─────────────────────────────────────────────────────────────────────────┘

VELOCITY                    CYCLE TIME                  THROUGHPUT
┌──────────────┐           ┌──────────────┐           ┌──────────────┐
│ Issues/Week  │           │ Ready→Done   │           │ Issues Done  │
│              │           │              │           │              │
│   ▄▄▄        │           │   ▄          │           │     ▄▄       │
│  ▐███▄       │           │  ▐█▌         │           │    ▐██▌      │
│ ▐█████▌      │           │ ▐███▌        │           │   ▐████▌     │
│▐███████▌     │           │▐█████▌       │           │  ▐██████▌    │
│ 8.5 avg      │           │ 4.2 days     │           │  34 total    │
└──────────────┘           └──────────────┘           └──────────────┘

COVERAGE                    WIP LIMIT                   REVIEW TIME
┌──────────────┐           ┌──────────────┐           ┌──────────────┐
│ Test Coverage│           │ In Progress  │           │ PR Review    │
│ ████████▒▒   │           │ ███▒▒▒▒▒▒▒▒▒ │           │ ██████▒▒▒▒   │
│ 63% → 80%    │           │ 3/5 (Good!)  │           │ 28h (Target  │
│              │           │              │           │      < 48h)   │
└──────────────┘           └──────────────┘           └──────────────┘
```

## 🎯 Priority Matrix

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     EISENHOWER MATRIX                                    │
└─────────────────────────────────────────────────────────────────────────┘

URGENT                              NOT URGENT
┌────────────────────────────┬────────────────────────────┐
│ IMPORTANT                  │ IMPORTANT                  │
│                            │                            │
│ 🔴 DO FIRST               │ 🟡 SCHEDULE                │
│                            │                            │
│ • Critical bugs            │ • WebSocket support        │
│ • Security issues          │ • SSE implementation       │
│ • Production down          │ • Session management       │
│                            │ • Testing utilities        │
│ (Work on immediately)      │ (Plan for next sprint)     │
├────────────────────────────┼────────────────────────────┤
│ NOT IMPORTANT              │ NOT IMPORTANT              │
│                            │                            │
│ 🟠 DELEGATE                │ 🟢 ELIMINATE               │
│                            │                            │
│ • Small bug fixes          │ • Nice-to-have features    │
│ • Documentation fixes      │ • Experimental ideas       │
│ • Good first issues        │ • Low-value tasks          │
│                            │                            │
│ (Assign to contributors)   │ (Backlog / Future)         │
└────────────────────────────┴────────────────────────────┘
```

## 🔍 Issue Discovery Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│              HOW TO FIND THE RIGHT ISSUE TO WORK ON                      │
└─────────────────────────────────────────────────────────────────────────┘

START
  │
  ├─ New contributor? ─────▶ Filter: "good first issue" ─────▶ Pick one
  │
  ├─ Want quick win? ───────▶ Filter: "size: S" or "size: XS" ─▶ Pick one
  │
  ├─ Core team member? ─────▶ Check 🎯 Ready column ───────────▶ Pick priority
  │
  ├─ Specific expertise? ───▶ Filter by area label ────────────▶ Pick one
  │
  └─ Browse all ────────────▶ Sort by priority ────────────────▶ Pick top
```

## 📞 Communication Channels

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    WHEN TO USE EACH CHANNEL                              │
└─────────────────────────────────────────────────────────────────────────┘

ISSUE                           PR                          DISCUSSION
┌──────────────┐              ┌──────────────┐            ┌──────────────┐
│ Report bugs  │              │ Code review  │            │ Questions    │
│ Request      │              │ Implementation│           │ Ideas        │
│ features     │              │ feedback     │            │ Help         │
│ Tasks        │              │              │            │ Announcements│
└──────────────┘              └──────────────┘            └──────────────┘
       │                            │                           │
       └────────────────────────────┴───────────────────────────┘
                                  │
                            All indexed
                            Searchable
                            Linked to board
```

## ✅ Success Indicators

```
┌─────────────────────────────────────────────────────────────────────────┐
│               HEALTHY PROJECT INDICATORS                                 │
└─────────────────────────────────────────────────────────────────────────┘

✅ GOOD                          ⚠️  NEEDS ATTENTION              ❌ POOR
┌────────────────────┐          ┌────────────────────┐          ┌────────────────────┐
│ • Backlog < 30     │          │ • Backlog 30-50    │          │ • Backlog > 50     │
│ • Ready: 10-15     │          │ • Ready: 5-10      │          │ • Ready: < 5       │
│ • In Progress: 3-5 │          │ • In Progress: 6-8 │          │ • In Progress: >10 │
│ • Review < 48h     │          │ • Review 48-72h    │          │ • Review > 72h     │
│ • Velocity: stable │          │ • Velocity: varies │          │ • Velocity: drops  │
│ • Coverage: >70%   │          │ • Coverage: 60-70% │          │ • Coverage: <60%   │
└────────────────────┘          └────────────────────┘          └────────────────────┘
      😊                              🤔                              😟
```

---

## 🎓 Quick Tips

- **Limit WIP:** Max 5 items in "In Progress" prevents context switching
- **Size PRs:** Smaller PRs = faster reviews = faster merges
- **Link issues:** Always link PRs to issues for traceability
- **Update status:** Move cards as work progresses
- **Review often:** Review PRs within 48h to maintain momentum
- **Archive done:** Clean up "Done" column weekly

---

*This visual guide complements the detailed documentation in `.github/` directory.*
