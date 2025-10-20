# Contributing to Flux

Thank you for your interest in contributing to Flux! This document provides guidelines and instructions for contributing.

## Development Setup

1. **Install Rust**: Make sure you have Rust installed. Visit [rustup.rs](https://rustup.rs/) for installation instructions.

2. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/flux.git
   cd flux
   ```

3. **Build the project**:
   ```bash
   cargo build
   ```

4. **Run tests**:
   ```bash
   cargo test
   ```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to check for common issues
- Follow Rust naming conventions
- Write descriptive commit messages

## Testing

- Add tests for new functionality
- Ensure all tests pass before submitting PR
- Include both unit and integration tests where appropriate

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Commit Message Guidelines

We follow conventional commits:
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `test:` for test additions/changes
- `refactor:` for code refactoring
- `chore:` for maintenance tasks

Example: `feat: add support for brotli compression`

## Questions?

Feel free to open an issue for any questions or discussions!