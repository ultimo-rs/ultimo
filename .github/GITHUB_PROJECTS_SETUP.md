# GitHub Projects Kanban Setup Guide

This guide walks you through setting up a GitHub Projects Kanban board for the Ultimo framework.

## Quick Start

### 1. Create the Project Board

1. Go to https://github.com/ultimo-rs/ultimo
2. Click the **Projects** tab
3. Click **New project**
4. Choose **Board** template
5. Name it "Ultimo Development"
6. Set visibility to **Public** (or Private if preferred)
7. Click **Create project**

### 2. Configure Columns

GitHub Projects comes with default columns. Customize them:

**Default columns to rename/modify:**

- Rename "Todo" → "📋 Backlog"
- Rename "In Progress" → "🚧 In Progress"
- Rename "Done" → "✅ Done"

**Additional columns to add:**

1. Click **+ Add column**
2. Add "🎯 Ready" (between Backlog and In Progress)
3. Add "👀 Review" (between In Progress and Done)

**Final column order:**

1. 📋 Backlog
2. 🎯 Ready
3. 🚧 In Progress
4. 👀 Review
5. ✅ Done

### 3. Set Up Labels

Navigate to https://github.com/ultimo-rs/ultimo/labels and create:

#### Priority Labels

- `priority: critical` - Red (#d73a4a)
- `priority: high` - Orange (#d9730d)
- `priority: medium` - Yellow (#f9d0c4)
- `priority: low` - Green (#0e8a16)

#### Type Labels

- `type: feature` - Blue (#0075ca) - New functionality
- `type: bug` - Red (#d73a4a) - Bug fixes
- `type: docs` - Light blue (#0366d6) - Documentation
- `type: performance` - Purple (#5319e7) - Performance improvements
- `type: refactor` - Gray (#d4c5f9) - Code refactoring
- `type: test` - Green (#0e8a16) - Tests
- `type: chore` - Gray (#fef2c0) - Maintenance

#### Area Labels

- `area: core` - Dark blue (#1d76db) - Core framework
- `area: rpc` - Blue (#0e8a16) - RPC system
- `area: openapi` - Blue (#0075ca) - OpenAPI
- `area: database` - Purple (#7057ff) - Database integration
- `area: cli` - Orange (#d93f0b) - CLI tool
- `area: examples` - Green (#5319e7) - Examples
- `area: docs` - Light blue (#c5def5) - Documentation site

#### Status Labels

- `status: planning` - Yellow (#fbca04)
- `status: investigating` - Blue (#1d76db)
- `status: blocked` - Red (#b60205)
- `status: paused` - Gray (#d4c5f9)

#### Special Labels

- `good first issue` - Green (#7057ff) - Good for newcomers
- `help wanted` - Green (#008672) - Extra attention needed
- `size: XS` - Light gray (#e4e4e4) - < 10 lines
- `size: S` - Gray (#c2e0c6) - < 100 lines
- `size: M` - Yellow (#fbca04) - < 500 lines
- `size: L` - Orange (#d93f0b) - < 1000 lines
- `size: XL` - Red (#b60205) - > 1000 lines

### 4. Create Initial Issues

Use the templates in `.github/INITIAL_ISSUES.md` to create issues. For each issue:

1. Go to **Issues** → **New issue**
2. Copy the content from `INITIAL_ISSUES.md`
3. Fill in title, description, labels
4. Click **Create issue**
5. Add to project board (select "Ultimo Development" project)
6. Set initial status (usually "📋 Backlog")

**Priority order for creating issues:**

1. WebSocket Support (High Priority)
2. SSE Support (High Priority)
3. Session Management (High Priority)
4. Testing Utilities (High Priority)
5. Increase Test Coverage (High Priority)
6. Then create remaining issues as needed

### 5. Configure Project Automation

GitHub Actions are already set up in `.github/workflows/`:

- `project-automation.yml` - Auto-moves issues based on actions
- `label-pr.yml` - Auto-labels PRs based on changed files

**Enable workflows:**

1. Push these files to your repository
2. Go to **Actions** tab
3. Enable GitHub Actions if not already enabled
4. Workflows will run automatically

**What the automation does:**

- ✅ Moves issues to "In Progress" when assigned
- ✅ Moves PRs to "Review" when opened
- ✅ Moves PRs to "Done" when merged
- ✅ Auto-labels PRs based on changed files
- ✅ Adds size labels (XS/S/M/L/XL) to PRs

### 6. Set Up Project Views

Create custom views for different perspectives:

#### View 1: By Priority

1. In your project, click **+ New view**
2. Select **Table** layout
3. Name it "By Priority"
4. Add filters: `label:priority*`
5. Group by: Priority label
6. Sort by: Priority (High → Low)

#### View 2: By Area

1. Create another view
2. Name it "By Area"
3. Group by: Area label
4. Shows all work organized by component

#### View 3: My Work

1. Create view "My Work"
2. Filter by: `assignee:@me`
3. Shows only your assigned issues

#### View 4: Ready to Work

1. Create view "Ready to Work"
2. Filter by: Status = "Ready"
3. Filter by: No assignee
4. Shows available tasks to pick up

### 7. Configure Project Settings

1. Click **⋯** (three dots) in project header
2. Select **Settings**

**Recommended settings:**

- ✅ Enable **Public access** (if open source)
- ✅ Enable **Link issues and PRs**
- ✅ Enable **Automation**
- ✅ Set default view to **Board**

### 8. Pin Important Issues

Pin high-priority issues to the top:

1. Open an important issue
2. Click **Pin issue** on the right sidebar
3. Issue stays at top of board

**Suggested pins:**

- Roadmap issue (create one linking to all major features)
- Contributing guide issue
- Current sprint/milestone

## Daily Workflow

### For Maintainers

**Morning routine:**

1. Check **Review** column for PRs to review
2. Check **Backlog** for new issues to triage
3. Move prioritized issues to **Ready**

**During development:**

1. Pick issue from **Ready** column
2. Assign to yourself (auto-moves to **In Progress**)
3. Create branch: `git checkout -b feature/issue-name`
4. Work on the issue
5. Open PR (auto-moves to **Review**)
6. After merge, issue auto-moves to **Done**

### For Contributors

1. Browse **Ready** column for available tasks
2. Look for `good first issue` label
3. Comment on issue to claim it
4. Wait for assignment
5. Follow the same development flow

## Advanced Features

### Milestones

Create milestones for releases:

1. Go to **Issues** → **Milestones**
2. Click **New milestone**
3. Add milestone (e.g., "v0.2.0 - WebSockets")
4. Set due date
5. Assign issues to milestone

### Project Insights

Track progress with insights:

1. In project, click **Insights**
2. View:
   - Burn-down chart
   - Velocity
   - Cycle time
   - Throughput

### Custom Fields

Add custom fields to track more data:

1. In project settings, add fields:
   - **Estimated Size** (number) - Story points
   - **Actual Size** (number) - Actual effort
   - **Sprint** (text) - Sprint name
   - **Blocked By** (text) - Dependency tracking

### Saved Filters

Create quick filters:

1. Apply filters to a view
2. Click **Save filter**
3. Name it (e.g., "High Priority Bugs")
4. Access from filter dropdown

## Integration with CI/CD

### Auto-close issues from commits

In commit messages:

```bash
git commit -m "feat: add WebSocket support

Closes #123
Fixes #124"
```

### Link PRs to issues

In PR description:

```markdown
Closes #123
Resolves #124
Fixes #125
```

### Status checks

PRs automatically show:

- ✅ Tests passing
- ✅ Coverage maintained
- ✅ Linting passed
- ✅ Formatting correct

## Best Practices

### Issue Creation

- ✅ Use templates (bug report, feature request)
- ✅ Add clear acceptance criteria
- ✅ Estimate size (S/M/L/XL)
- ✅ Add relevant labels
- ✅ Link related issues

### PR Creation

- ✅ Use PR template
- ✅ Link to issue(s)
- ✅ Add screenshots if UI-related
- ✅ Keep PRs focused (one feature/fix)
- ✅ Write clear commit messages

### Code Review

- ✅ Review within 24-48 hours
- ✅ Provide constructive feedback
- ✅ Test locally if possible
- ✅ Check test coverage
- ✅ Approve or request changes

### Board Management

- ✅ Triage new issues daily
- ✅ Keep **Ready** column stocked
- ✅ Limit **In Progress** (WIP limit: 3-5)
- ✅ Clean up **Done** weekly
- ✅ Archive completed issues monthly

## Troubleshooting

### Issues not appearing in project

- Check if issue is added to project
- Verify project visibility settings
- Check filters on current view

### Automation not working

- Verify GitHub Actions are enabled
- Check workflow permissions
- Review workflow run logs in Actions tab

### Labels not applying automatically

- Check `.github/labeler.yml` configuration
- Verify workflow is running
- Manually apply labels if needed

## Resources

- 📚 [GitHub Projects Docs](https://docs.github.com/en/issues/planning-and-tracking-with-projects)
- 📖 [Project Board Setup](PROJECT_BOARD.md)
- 📝 [Initial Issues](INITIAL_ISSUES.md)
- 🤝 [Contributing Guide](../CONTRIBUTING.md)

## Quick Commands

```bash
# Clone the repo
git clone https://github.com/ultimo-rs/ultimo.git

# Create a new branch
git checkout -b feature/issue-123-websocket-support

# Commit with issue reference
git commit -m "feat: add WebSocket support

Implements WebSocket protocol for real-time communication.

Closes #123"

# Push and create PR
git push origin feature/issue-123-websocket-support
gh pr create --title "Add WebSocket support" --body "Closes #123"
```

## Need Help?

- 💬 Ask in [GitHub Discussions](https://github.com/ultimo-rs/ultimo/discussions)
- 🐛 Report issues with [Bug Report template](ISSUE_TEMPLATE/bug_report.md)
- 💡 Suggest features with [Feature Request template](ISSUE_TEMPLATE/feature_request.md)

---

**Ready to start?** Create your first issue and let's build Ultimo together! 🚀
