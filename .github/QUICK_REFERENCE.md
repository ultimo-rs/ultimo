# 🎯 Quick Reference - GitHub Projects Kanban

## 📋 Project Board Columns

```
📋 Backlog → 🎯 Ready → 🚧 In Progress → 👀 Review → ✅ Done
```

## 🏷️ Label Quick Reference

### Priority

- 🔴 `priority: critical` - Urgent, blocking
- 🟠 `priority: high` - Important
- 🟡 `priority: medium` - Standard
- 🟢 `priority: low` - Nice to have

### Type

- 🚀 `type: feature` - New feature
- 🐛 `type: bug` - Bug fix
- 📚 `type: docs` - Documentation
- ⚡ `type: performance` - Performance
- 🔧 `type: refactor` - Refactor
- 🧪 `type: test` - Tests

### Area

- 🏗️ `area: core` - Core framework
- 🌐 `area: rpc` - RPC system
- 📖 `area: openapi` - OpenAPI
- 🗄️ `area: database` - Database
- 🛠️ `area: cli` - CLI tool
- 📱 `area: examples` - Examples
- 📚 `area: docs` - Docs site

## 🚀 Quick Start Checklist

- [ ] Create GitHub Project board
- [ ] Set up 5 columns (Backlog, Ready, In Progress, Review, Done)
- [ ] Create labels (priority, type, area)
- [ ] Enable GitHub Actions workflows
- [ ] Create initial issues from `INITIAL_ISSUES.md`
- [ ] Pin important issues
- [ ] Set up project views (By Priority, By Area, My Work)

## 💻 Common Git Commands

```bash
# Start working on an issue
git checkout -b feature/issue-123-description

# Commit with issue reference
git commit -m "feat: description

Closes #123"

# Push and create PR
git push origin feature/issue-123-description
gh pr create --title "Title" --body "Closes #123"

# Update your branch
git fetch origin
git rebase origin/main
```

## 📝 Issue Creation Template

```markdown
**Title:** [TYPE] Clear description

**Labels:**

- priority: [critical/high/medium/low]
- type: [feature/bug/docs/etc]
- area: [core/rpc/cli/etc]

**Description:**
What needs to be done?

**Acceptance Criteria:**

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Tests added
- [ ] Docs updated

**Estimated Size:** [XS/S/M/L/XL]
```

## 🔄 Workflow States

| State           | Who         | Action                       |
| --------------- | ----------- | ---------------------------- |
| **Backlog**     | Anyone      | Created, not yet prioritized |
| **Ready**       | Maintainers | Prioritized, ready to start  |
| **In Progress** | Assignee    | Actively working             |
| **Review**      | Reviewers   | PR open, needs review        |
| **Done**        | Auto        | Merged and closed            |

## 📊 Project Views

### 1. Board View (Default)

See all issues in Kanban columns

### 2. By Priority

Group by priority labels

### 3. By Area

Group by area labels

### 4. My Work

Filter: `assignee:@me`

### 5. Ready to Work

Filter: Status = Ready, No assignee

## 🎯 Current Priorities (Top 5)

1. **WebSocket Support** - `priority: high` `area: core`
2. **SSE Support** - `priority: high` `area: core`
3. **Session Management** - `priority: high` `area: core`
4. **Testing Utilities** - `priority: high` `type: test`
5. **Increase Test Coverage to 80%** - `priority: high` `type: test`

## 🤝 Contributor Workflow

```
1. Find issue in 🎯 Ready column
2. Comment "I'd like to work on this"
3. Get assigned (auto-moves to 🚧 In Progress)
4. Create feature branch
5. Make changes + add tests
6. Open PR (auto-moves to 👀 Review)
7. Address review feedback
8. Get merged (auto-moves to ✅ Done)
```

## 🔍 Finding Issues to Work On

**Good first issues:**

```
label:"good first issue"
```

**Available tasks:**

```
is:open is:issue no:assignee label:"status: ready"
```

**High priority:**

```
is:open is:issue label:"priority: high"
```

**By area:**

```
is:open is:issue label:"area: core"
```

## 📈 Metrics to Track

- **Velocity:** Issues closed per week
- **Cycle Time:** Days from Ready → Done
- **WIP Limit:** Max 5 items in "In Progress"
- **Test Coverage:** Target 80%
- **Response Time:** Review PRs within 48h

## 🛠️ Automation Features

✅ Auto-label PRs by changed files  
✅ Auto-size PRs (XS/S/M/L/XL)  
✅ Move to "In Progress" when assigned  
✅ Move to "Review" when PR opened  
✅ Move to "Done" when PR merged  
✅ Close issues from commit messages

## 📚 Documentation Links

- [Full Setup Guide](GITHUB_PROJECTS_SETUP.md)
- [Project Board Details](PROJECT_BOARD.md)
- [Initial Issues List](INITIAL_ISSUES.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Issue Templates](ISSUE_TEMPLATE/)

## 💡 Tips

- **Keep PRs small:** Easier to review, faster to merge
- **Write tests:** All new features need tests
- **Update docs:** Keep documentation in sync
- **Link issues:** Use "Closes #123" in PR description
- **Ask questions:** Use GitHub Discussions

## 🚨 Emergency Contacts

**Critical bugs:** Label with `priority: critical`  
**Security issues:** Email security@ultimo.dev  
**Questions:** GitHub Discussions  
**Live chat:** Discord (coming soon)

## ⚡ Quick Actions

| Want to...         | Do this...                   |
| ------------------ | ---------------------------- |
| Report a bug       | Use bug report template      |
| Suggest a feature  | Use feature request template |
| Start contributing | Pick from "good first issue" |
| Review code        | Check "Review" column        |
| Track progress     | View project insights        |

---

**Updated:** 2025-11-24  
**Version:** 1.0  
**Maintainer:** @ultimo-rs
