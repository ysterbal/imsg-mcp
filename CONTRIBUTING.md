# Contributing to imsg-mcp

Thank you for your interest in contributing to `imsg-mcp`! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting a Pull Request](#submitting-a-pull-request)

## Code of Conduct

By participating in this project, you agree to abide by our code of conduct: be respectful, inclusive, and constructive in all interactions.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/yourusername/imsg-mcp.git
   cd imsg-mcp
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/originalusername/imsg-mcp.git
   ```

## Development Setup

### Prerequisites

- **Rust 1.75+**: Install from [rustup.rs](https://rustup.rs/)
- **macOS**: Required for iMessage database access and Contacts framework integration

### Building the Project

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Check code for errors and warnings
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy -- -D warnings
```

## Project Structure

```
imsg-mcp/
├── Cargo.toml          # Project dependencies and metadata
├── README.md           # Project documentation
├── CONTRIBUTING.md     # Contribution guidelines
├── LICENSE             # MIT License
└── src/
    ├── main.rs         # MCP server entry point and JSON-RPC handler
    ├── imessage.rs     # iMessage database reader
    ├── models.rs       # Data models (MessageDTO, etc.)
    ├── snowflake.rs    # Apple Snowflake timestamp decoder
    ├── address_book.rs # macOS contact lookup (stub)
    └── errors.rs       # Error types
```

## Making Changes

### Code Style

- Follow Rust formatting guidelines (`cargo fmt`)
- Run `cargo clippy -- -D warnings` to ensure code quality
- Use meaningful variable and function names
- Add comments for complex logic, especially around database queries and timestamp decoding

### Commit Messages

Use conventional commit messages:
- `feat: add new tool XYZ`
- `fix: resolve timestamp decoding issue`
- `docs: update README with installation instructions`
- `refactor: improve database query performance`
- `test: add unit tests for snowflake decoder`

## Testing

### Unit Tests

Add tests for new functionality in the relevant source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snowflake_decode() {
        // Test implementation
    }
}
```

Run tests with:
```bash
cargo test
```

### Manual Testing

To manually test the MCP server:

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Test the server using a JSON-RPC client or the MCP inspector:
   ```bash
   # Initialize
   echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05"},"id":1}' | ./target/release/imsg-mcp

   # List tools
   echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | ./target/release/imsg-mcp

   # Call inbox tool
   echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"inbox","arguments":{"limit":10}},"id":3}' | ./target/release/imsg-mcp
   ```

## Submitting a Pull Request

1. Ensure your code passes `cargo fmt` and `cargo clippy -- -D warnings`
2. Ensure all tests pass: `cargo test`
3. Create a new branch for your feature or bugfix
4. Commit your changes with descriptive commit messages
5. Push your branch to your fork
6. Open a Pull Request against the `main` branch of the upstream repository
7. Provide a clear description of the changes and any related issues

### PR Checklist

- [ ] Code follows the project's style guidelines
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Documentation has been updated if necessary
- [ ] Changes are tested manually (if applicable)

## Questions?

If you have questions or need help, please open an issue on the GitHub repository or reach out to the maintainers.

Thank you for contributing to `imsg-mcp`!
