# Contributing to Ultimo

Thank you for your interest in contributing to Ultimo! This document provides guidelines and instructions for contributing.

## 📋 GitHub Projects Board

We use a GitHub Projects Kanban board to track all work. See [`.github/README.md`](.github/README.md) for:
- 🎯 [Complete setup guide](.github/GITHUB_PROJECTS_SETUP.md)
- 📊 [Project board structure](.github/PROJECT_BOARD.md)
- 📝 [35+ initial issues ready to work on](.github/INITIAL_ISSUES.md)
- ⚡ [Quick reference guide](.github/QUICK_REFERENCE.md)
- 🎨 [Visual workflow diagrams](.github/VISUAL_WORKFLOW.md)

**Quick links:**
- [View Project Board](https://github.com/ultimo-rs/ultimo/projects)
- [Browse Issues](https://github.com/ultimo-rs/ultimo/issues)
- [Good First Issues](https://github.com/ultimo-rs/ultimo/labels/good%20first%20issue)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### Prerequisites

- Rust 1.75.0 or higher
- Node.js 18+ (for examples and docs)
- Git

### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/ultimo-rs/ultimo.git
cd ultimo

# Install git hooks (recommended)
./scripts/install-hooks.sh

# Build the project
cargo build

# Run tests
cargo test

# Check coverage
cargo coverage
```

## Development Workflow

### 1. Find or Create an Issue

- Check existing issues in our [GitHub Project Board](https://github.com/ultimo-rs/ultimo/projects)
- For bugs: Use the bug report template
- For features: Use the feature request template
- Comment on the issue to indicate you're working on it

### 2. Create a Branch

```bash
# Create a feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/bug-description
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/improvements

### 3. Make Your Changes

#### Code Style

- Follow Rust idioms and conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add documentation comments for public APIs
- Write clear commit messages

#### Testing

- Add tests for new functionality
- Ensure existing tests pass
- Maintain or improve code coverage (minimum 60%)
- Test both success and error cases

```bash
# Run tests
cargo test

# Run tests with coverage
cargo coverage

# View coverage report
open target/coverage/html/index.html
```

#### Documentation

- Update relevant documentation
- Add examples for new features
- Update CHANGELOG.md
- Add inline code comments for complex logic

### 4. Commit Your Changes

Use conventional commit messages:

```bash
git commit -m "feat: add WebSocket support"
git commit -m "fix: resolve routing edge case"
git commit -m "docs: update RPC examples"
git commit -m "test: add middleware tests"
```

Commit types:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation only
- `style:` - Code style changes (formatting)
- `refactor:` - Code refactoring
- `test:` - Adding/updating tests
- `chore:` - Maintenance tasks

### 5. Push and Create Pull Request

```bash
# Push your branch
git push origin feature/your-feature-name

# Create PR on GitHub
```

Use the PR template and:
- Link related issues
- Describe your changes clearly
- Add screenshots if applicable
- Check all items in the checklist

## Pull Request Review Process

1. **Automated Checks**: CI will run tests, linting, and coverage
2. **Code Review**: Maintainers will review your code
3. **Feedback**: Address any requested changes
4. **Approval**: Once approved, your PR will be merged
5. **Recognition**: You'll be added to contributors!

## Project Structure

```
ultimo/
├── src/
│   ├── lib.rs          # Public API exports
│   ├── app.rs          # Main application struct
│   ├── context.rs      # Request/response context
│   ├── router.rs       # Routing engine
│   ├── middleware.rs   # Middleware system
│   ├── rpc.rs          # RPC system
│   ├── openapi.rs      # OpenAPI generation
│   ├── error.rs        # Error handling
│   └── ...
├── tests/              # Integration tests
└── examples/           # Example applications
```

## Areas to Contribute

### 🚀 High Priority (Coming Soon)
- WebSocket support
- Server-Sent Events (SSE)
- Session management
- Testing utilities
- Multi-language client generation

### 🐛 Bug Fixes
- Check [open bugs](https://github.com/ultimo-rs/ultimo/labels/type%3A%20bug)
- Reproduce the issue
- Write a failing test
- Fix the bug
- Verify test passes

### 📚 Documentation
- Improve existing docs
- Add more examples
- Write tutorials
- Fix typos and clarify explanations

### ⚡ Performance
- Profile code for bottlenecks
- Optimize hot paths
- Add benchmarks
- Reduce allocations

### 🧪 Testing
- Increase test coverage
- Add edge case tests
- Write integration tests
- Improve test helpers

## Style Guide

### Rust Code

```rust
// Use descriptive names
pub struct Context {
    req: Request,
    res: Response,
}

// Document public APIs
/// Creates a new Ultimo application instance.
///
/// # Examples
///
/// ```
/// let app = Ultimo::new();
/// ```
pub fn new() -> Self {
    // Implementation
}

// Prefer Result over panic
pub fn parse_json(&self) -> Result<Value> {
    serde_json::from_str(&self.body)
        .map_err(|e| Error::JsonParse(e.to_string()))
}

// Use type aliases for clarity
pub type Result<T> = std::result::Result<T, Error>;
```

### Error Handling

```rust
// Provide context in errors
return Err(Error::NotFound(format!(
    "Route not found: {} {}",
    method, path
)));

// Use custom error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Route not found: {0}")]
    NotFound(String),
    
    #[error("Validation failed: {0}")]
    Validation(String),
}
```

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_matching() {
        let router = Router::new();
        router.add_route("/users/:id", handler);
        
        let (handler, params) = router.match_route("/users/123");
        assert!(handler.is_some());
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use ultimo::prelude::*;

#[tokio::test]
async fn test_full_request_flow() {
    let app = Ultimo::new();
    app.get("/test", |c| async move {
        c.json(json!({"status": "ok"}))
    });
    
    let client = TestClient::new(app);
    let res = client.get("/test").await;
    
    assert_eq!(res.status(), 200);
    assert_eq!(res.json::<Value>().await?, json!({"status": "ok"}));
}
```

## Community

- 💬 [GitHub Discussions](https://github.com/ultimo-rs/ultimo/discussions)
- 🐛 [Issue Tracker](https://github.com/ultimo-rs/ultimo/issues)
- 📋 [Project Board](https://github.com/ultimo-rs/ultimo/projects)

## Recognition

Contributors are recognized in:
- README.md contributors section
- Release notes
- GitHub contributors page

## Questions?

- Check [documentation](https://docs.ultimo.dev)
- Search [existing issues](https://github.com/ultimo-rs/ultimo/issues)
- Ask in [discussions](https://github.com/ultimo-rs/ultimo/discussions)
- Open a new issue with the question label

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Ultimo! 🚀
