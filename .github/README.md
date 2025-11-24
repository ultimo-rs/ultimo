# 📋 GitHub Projects Kanban - Complete Setup

Welcome! This directory contains everything you need to set up and manage the Ultimo project using GitHub Projects Kanban board.

## 📁 What's Included

### Documentation
- **[GITHUB_PROJECTS_SETUP.md](GITHUB_PROJECTS_SETUP.md)** - Complete setup guide (START HERE!)
- **[PROJECT_BOARD.md](PROJECT_BOARD.md)** - Board structure and workflow details
- **[INITIAL_ISSUES.md](INITIAL_ISSUES.md)** - 35+ issues ready to create
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Quick reference for daily use

### Templates
- **[ISSUE_TEMPLATE/bug_report.md](ISSUE_TEMPLATE/bug_report.md)** - Bug report template
- **[ISSUE_TEMPLATE/feature_request.md](ISSUE_TEMPLATE/feature_request.md)** - Feature request template
- **[PULL_REQUEST_TEMPLATE.md](PULL_REQUEST_TEMPLATE.md)** - PR template

### Automation
- **[workflows/project-automation.yml](workflows/project-automation.yml)** - Auto-moves issues/PRs
- **[workflows/label-pr.yml](workflows/label-pr.yml)** - Auto-labels PRs
- **[labeler.yml](labeler.yml)** - Label configuration

## 🚀 Getting Started (5 Minutes)

### Step 1: Create the Board
1. Go to your GitHub repo → **Projects** tab
2. Click **New project** → Choose **Board** template
3. Name it "Ultimo Development"
4. Create 5 columns: 📋 Backlog, 🎯 Ready, 🚧 In Progress, 👀 Review, ✅ Done

### Step 2: Set Up Labels
1. Go to **Issues** → **Labels**
2. Create priority labels: critical, high, medium, low
3. Create type labels: feature, bug, docs, test, performance
4. Create area labels: core, rpc, cli, database, docs

### Step 3: Create Initial Issues
1. Open **[INITIAL_ISSUES.md](INITIAL_ISSUES.md)**
2. Start with the top 5 high-priority issues
3. Copy each issue template to GitHub
4. Add to project board in "📋 Backlog" column

### Step 4: Enable Automation
1. Commit the `.github/workflows/` files to your repo
2. GitHub Actions will automatically run
3. Issues/PRs will auto-move between columns

### Step 5: Start Working!
1. Move high-priority issues to "🎯 Ready"
2. Assign issues to team members
3. Issues auto-move as work progresses

## 📊 Project Board Structure

```
┌─────────────┐   ┌─────────┐   ┌─────────────┐   ┌──────────┐   ┌──────┐
│ 📋 Backlog  │──▶│ 🎯 Ready│──▶│ 🚧 In Prog. │──▶│ 👀 Review│──▶│ ✅ Done│
│             │   │         │   │             │   │          │   │      │
│ All new     │   │ Priori- │   │ Actively    │   │ PR open  │   │ Merged│
│ issues      │   │ tized   │   │ working     │   │ needs    │   │ closed│
│             │   │         │   │             │   │ review   │   │      │
└─────────────┘   └─────────┘   └─────────────┘   └──────────┘   └──────┘
```

## 🎯 Top 5 Priorities

Based on the roadmap and project needs:

1. **WebSocket Support** 🔴 High Priority
   - Real-time bidirectional communication
   - ~XL size (~2-3 weeks)

2. **Server-Sent Events (SSE)** 🔴 High Priority
   - Server-to-client streaming
   - ~L size (~1-2 weeks)

3. **Session Management** 🔴 High Priority
   - Cookie-based sessions with multiple backends
   - ~XL size (~2-3 weeks)

4. **Testing Utilities** 🔴 High Priority
   - TestClient and assertion helpers
   - ~L size (~1-2 weeks)

5. **Increase Test Coverage to 80%** 🔴 High Priority
   - Current: 63.58% → Target: 80%
   - ~L size (~1-2 weeks)

## 📋 Issue Statistics

| Category | Count | Priority |
|----------|-------|----------|
| High Priority Features | 5 | 🔴 Critical path |
| Documentation | 4 | 🟡 Important |
| Performance & Quality | 4 | 🟠 High |
| CLI Improvements | 5 | 🟠 High |
| Community | 5 | 🟡 Medium |
| Bug Fixes | 3 | 🟡 Medium |
| Quick Wins | 3 | 🟢 Good first issues |
| **Total** | **35+** | - |

## 🏷️ Label System

### Priority (4 labels)
- 🔴 `priority: critical` - Blocking, security
- 🟠 `priority: high` - Important features
- 🟡 `priority: medium` - Standard work
- 🟢 `priority: low` - Nice to have

### Type (7 labels)
- 🚀 `type: feature` - New functionality
- 🐛 `type: bug` - Bug fixes
- 📚 `type: docs` - Documentation
- ⚡ `type: performance` - Performance
- 🔧 `type: refactor` - Refactoring
- 🧪 `type: test` - Tests
- 🎨 `type: ui` - UI/UX

### Area (7 labels)
- 🏗️ `area: core` - Core framework
- 🌐 `area: rpc` - RPC system
- 📖 `area: openapi` - OpenAPI
- 🗄️ `area: database` - Database
- 🛠️ `area: cli` - CLI tool
- 📱 `area: examples` - Examples
- 📚 `area: docs` - Docs site

### Size (5 labels)
Auto-applied to PRs:
- `size: XS` - < 10 lines
- `size: S` - < 100 lines
- `size: M` - < 500 lines
- `size: L` - < 1000 lines
- `size: XL` - > 1000 lines

## 🤖 Automation Features

✅ **Auto-move issues:** Assigned → In Progress  
✅ **Auto-move PRs:** Opened → Review  
✅ **Auto-move merged:** PR Merged → Done  
✅ **Auto-label PRs:** Based on changed files  
✅ **Auto-size PRs:** XS/S/M/L/XL labels  
✅ **Auto-close issues:** Via commit messages  

## 👥 Team Roles

| Role | Permissions | Responsibilities |
|------|-------------|------------------|
| **Maintainers** | Admin | Triage, prioritize, merge PRs |
| **Contributors** | Write | Work on issues, create PRs |
| **Community** | Read | Report bugs, suggest features |

## 📈 Success Metrics

Track these metrics to measure project health:

- **Velocity:** 5-10 issues per week (target)
- **Cycle Time:** 3-7 days from Ready → Done
- **WIP Limit:** Max 5 items in Progress
- **Test Coverage:** 80% (current: 63.58%)
- **Review Time:** < 48 hours for PR review
- **Bug Rate:** < 10% of total issues

## 🔄 Weekly Workflow

### Monday
- Review Backlog
- Prioritize new issues → Ready
- Plan sprint/week

### Daily
- Check Review column
- Review open PRs
- Update In Progress items

### Friday
- Move Done items to archive
- Review week's progress
- Update metrics

## 📚 Documentation Guide

| Document | When to Use |
|----------|-------------|
| [GITHUB_PROJECTS_SETUP.md](GITHUB_PROJECTS_SETUP.md) | First-time setup |
| [PROJECT_BOARD.md](PROJECT_BOARD.md) | Understanding workflow |
| [INITIAL_ISSUES.md](INITIAL_ISSUES.md) | Creating issues |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Daily use |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Contributing guide |

## 🎓 Learning Resources

### GitHub Docs
- [Projects Documentation](https://docs.github.com/en/issues/planning-and-tracking-with-projects)
- [GitHub Actions](https://docs.github.com/en/actions)
- [Issue Templates](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests)

### Ultimo Specific
- [Main README](../README.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Documentation Site](https://docs.ultimo.dev)

## 🛠️ Troubleshooting

### Common Issues

**Q: Issues not showing in project?**  
A: Add issue to project manually via issue sidebar

**Q: Automation not working?**  
A: Check GitHub Actions are enabled in repo settings

**Q: Labels not auto-applying?**  
A: Verify `.github/labeler.yml` is committed

**Q: How to bulk-create issues?**  
A: Use GitHub CLI: `gh issue create --title "..." --body "..."`

## 🚦 Getting Help

- 📖 Read the [full setup guide](GITHUB_PROJECTS_SETUP.md)
- 💬 Ask in [GitHub Discussions](https://github.com/ultimo-rs/ultimo/discussions)
- 🐛 Report issues with project setup
- 📧 Contact maintainers

## ✅ Setup Checklist

Use this checklist to track your setup progress:

- [ ] Read GITHUB_PROJECTS_SETUP.md
- [ ] Create project board with 5 columns
- [ ] Set up all labels (priority, type, area, size)
- [ ] Create first 5 high-priority issues
- [ ] Enable GitHub Actions workflows
- [ ] Configure project views (By Priority, By Area, My Work)
- [ ] Pin important issues
- [ ] Set up milestones (optional)
- [ ] Invite team members
- [ ] Announce project board to team

## 🎉 Next Steps

Once setup is complete:

1. **Create issues** from INITIAL_ISSUES.md (start with top 5)
2. **Prioritize** issues by moving to Ready column
3. **Assign** issues to team members
4. **Track progress** daily/weekly
5. **Review metrics** to optimize workflow

## 📞 Contact

- **Project:** [ultimo-rs/ultimo](https://github.com/ultimo-rs/ultimo)
- **Discussions:** [GitHub Discussions](https://github.com/ultimo-rs/ultimo/discussions)
- **Issues:** [Issue Tracker](https://github.com/ultimo-rs/ultimo/issues)
- **Documentation:** [docs.ultimo.dev](https://docs.ultimo.dev)

---

**Ready to start?** 🚀

1. Open [GITHUB_PROJECTS_SETUP.md](GITHUB_PROJECTS_SETUP.md)
2. Follow the step-by-step guide
3. Create your first issue
4. Start building!

**Questions?** Check [QUICK_REFERENCE.md](QUICK_REFERENCE.md) for common tasks.

---

*Last updated: 2025-11-24*  
*Version: 1.0*
