# Moonrepo Integration

This project uses [Moonrepo](https://moonrepo.dev/) for monorepo management and task orchestration.

## Installation

```bash
curl -fsSL https://moonrepo.dev/install/moon.sh | bash
```

Add to your shell PATH:

```bash
export PATH="$HOME/.moon/bin:$PATH"
```

## Quick Start

### List all projects

```bash
moon query projects
```

### Build everything

```bash
moon run :build
```

### Run tests

```bash
moon run :test
```

## Common Commands

### Core Framework

```bash
# Build ultimo
moon run ultimo:build

# Run tests
moon run ultimo:test

# Check code
moon run ultimo:check

# Run clippy
moon run ultimo:clippy

# Format code
moon run ultimo:fmt-fix
```

### CLI Tool

```bash
# Build CLI
moon run ultimo-cli:build-release

# Run CLI
moon run ultimo-cli:run
```

### Documentation

```bash
# Install dependencies
moon run docs-site:install

# Start dev server
moon run docs-site:dev

# Build docs
moon run docs-site:build

# Preview built docs
moon run docs-site:preview
```

### Examples

```bash
# Build all examples
moon run examples/*:build

# Run specific example
moon run database-sqlx:run

# Test example
moon run rpc-modes:test
```

## Advanced Usage

### Run tasks across all projects

```bash
# Format all Rust code
moon run :fmt-fix

# Run all tests
moon run :test

# Clean all projects
moon run :clean
```

### Run affected tasks only

```bash
# Build only what changed
moon run :build --affected

# Test only affected projects
moon run :test --affected
```

### CI Mode

```bash
# Run in CI with optimal settings
moon ci
```

## Task Graph

View task dependencies:

```bash
moon query tasks --project ultimo --graph
```

## Caching

Moon automatically caches task outputs. To clear cache:

```bash
moon clean --cache
```

## Configuration

- `.moon/workspace.yml` - Workspace configuration and project locations
- `.moon/toolchain.yml` - Node.js and toolchain configuration
- `.moon/tasks.yml` - Global Rust tasks inherited by all packages
- `docs-site/moon.yml` - Documentation-specific Node.js tasks

**Note**: Project-level task configurations should be in `moon.yml` at the project root, not in `.moon/tasks.yml`.

## Learn More

- [Moonrepo Documentation](https://moonrepo.dev/docs)
- [Task Configuration](https://moonrepo.dev/docs/config/tasks)
- [Project Configuration](https://moonrepo.dev/docs/config/project)
