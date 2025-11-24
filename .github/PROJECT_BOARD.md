# Ultimo GitHub Projects Setup

This document describes our GitHub Projects Kanban board configuration for managing the Ultimo framework development.

## Board Structure

### Columns

1. **📋 Backlog** - Features and tasks that are planned but not yet started
2. **🎯 Ready** - Tasks that are ready to be worked on (dependencies resolved, requirements clear)
3. **🚧 In Progress** - Currently being worked on
4. **👀 Review** - Code complete, awaiting review or testing
5. **✅ Done** - Completed and merged

## Label System

### Priority

- 🔴 `priority: critical` - Blocking issues, security vulnerabilities
- 🟠 `priority: high` - Important features, significant bugs
- 🟡 `priority: medium` - Standard features and improvements
- 🟢 `priority: low` - Nice-to-have features, minor improvements

### Type

- 🚀 `type: feature` - New functionality
- 🐛 `type: bug` - Bug fixes
- 📚 `type: docs` - Documentation updates
- ⚡ `type: performance` - Performance improvements
- 🔧 `type: refactor` - Code refactoring
- 🧪 `type: test` - Test additions/improvements
- 🎨 `type: ui` - UI/UX improvements (docs site, examples)

### Area

- 🏗️ `area: core` - Core framework functionality
- 🌐 `area: rpc` - RPC system
- 📖 `area: openapi` - OpenAPI generation
- 🗄️ `area: database` - Database integration
- 🛠️ `area: cli` - CLI tool
- 📱 `area: examples` - Example applications
- 📚 `area: docs` - Documentation site

### Status

- 💡 `status: planning` - In planning phase
- 🔍 `status: investigating` - Investigating technical approach
- 🚫 `status: blocked` - Blocked by dependencies
- ⏸️ `status: paused` - Temporarily paused

## Current Roadmap Items

Based on the README and project structure, here are the key items for the board:

### High Priority (Coming Soon)

- [ ] WebSocket support
- [ ] Server-Sent Events (SSE)
- [ ] Session management
- [ ] Testing utilities
- [ ] Multi-language client generation

### Documentation

- [ ] Add more examples for middleware patterns
- [ ] Create video tutorials
- [ ] Add troubleshooting guide
- [ ] Document best practices for production deployment

### Performance & Quality

- [ ] Increase test coverage to 80%
- [ ] Benchmark against more frameworks
- [ ] Add performance regression tests
- [ ] Optimize TypeScript client generation

### Developer Experience

- [ ] CLI: Project scaffolding (`ultimo new`)
- [ ] CLI: Hot reload dev server (`ultimo dev`)
- [ ] CLI: Production build tools (`ultimo build`)
- [ ] Better error messages
- [ ] Add debug logging utilities

### Community

- [ ] Create contribution guidelines
- [ ] Set up issue templates
- [ ] Create PR template
- [ ] Add code of conduct
- [ ] Create Discord/community channel

## Workflow

### For New Issues

1. Issues start in **📋 Backlog**
2. Once prioritized and requirements clear → move to **🎯 Ready**
3. When starting work → move to **🚧 In Progress** and assign yourself
4. When PR is open → move to **👀 Review**
5. When merged → move to **✅ Done**

### For Pull Requests

1. Link to related issue(s)
2. Automatically moves to **👀 Review** when opened
3. Moves to **✅ Done** when merged
4. Auto-closes linked issues

## Automation

We can set up GitHub Actions to automatically:

- Move issues to "In Progress" when assigned
- Move to "Review" when PR is opened
- Move to "Done" when PR is merged
- Label PRs based on changed files
- Add size labels (S/M/L/XL) based on diff

## Team Roles

- **Maintainers**: Can move items between any columns, merge PRs
- **Contributors**: Can work on items in Ready, move to In Progress/Review
- **Triagers**: Can add labels, move items to Ready

## Metrics to Track

- 📊 **Velocity**: Issues completed per week/sprint
- ⏱️ **Cycle Time**: Time from Ready → Done
- 🎯 **Throughput**: Number of issues completed
- 📈 **Coverage**: Test coverage percentage
- 🐛 **Bug Rate**: Bugs reported vs fixed

## Quick Start

### Setting Up the Board

1. Go to your repository on GitHub
2. Click **Projects** tab → **New project**
3. Choose **Board** template
4. Create columns as described above
5. Add automation rules
6. Start adding issues!

### Creating Issues from This Document

You can create issues directly from the roadmap items listed above. Each issue should include:

- Clear description
- Acceptance criteria
- Related documentation
- Estimated complexity
- Dependencies (if any)
