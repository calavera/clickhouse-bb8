# Contributing to ClickHouse-BB8

Thank you for your interest in contributing to ClickHouse-BB8! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions with other contributors and maintainers.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Create a new branch for your changes
4. Make your changes
5. Run tests: `cargo test`
6. Run formatting: `cargo fmt`
7. Run linting: `cargo clippy`
8. Commit your changes with clear commit messages
9. Push to your fork
10. Open a pull request

## Development Setup

### Prerequisites

- Rust 1.89 or later
- Cargo

### Running Tests

```bash
cargo test
```

### Running Checks

```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Build documentation
cargo doc --no-deps --open
```

## Commit Messages

Please write clear and concise commit messages. Use the following format:

```
<type>: <subject>

<body>

<footer>
```

Where type is one of:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, semicolons, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `ci`: CI/CD changes
- `chore`: Other changes that don't modify source or test files

## Pull Request Process

1. Update the CHANGELOG.md with your changes
2. Ensure all tests pass
3. Ensure the code is properly formatted and linted
4. Keep pull requests focused on a single concern
5. Include a clear description of what you changed and why

## Reporting Issues

- Use the GitHub issue tracker
- Include a clear description of the issue
- Include steps to reproduce if applicable
- Include your Rust version and environment details

## Release Process

Releases are automated using release-plz. When a PR is merged to main:
1. release-plz automatically creates a release PR
2. Merging the release PR triggers the publish workflow
3. The crate is published to crates.io

## Questions?

Feel free to open an issue or discussion if you have questions about contributing.
