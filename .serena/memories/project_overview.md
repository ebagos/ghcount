# Project Overview

## Purpose
**ghcount** is a Rust CLI application project. Currently it's a basic "Hello, world!" application but appears to be intended for GitHub-related counting functionality based on the name.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **Build System**: Cargo
- **Version**: 0.1.0
- **Development Environment**: mise for tool management

## Project Structure
```
ghcount/
├── Cargo.toml          # Project configuration
├── Cargo.lock          # Dependency lockfile
├── mise.toml           # Development environment tools
├── .gitignore          # Git ignore rules
└── src/
    └── main.rs         # Main application entry point
```

## Current State
- Basic Rust project with a simple "Hello, world!" main function
- No external dependencies currently defined
- Ready for development expansion
- Uses Rust 2024 edition (latest)

## Development Environment
- Rust: 1.89.0
- Cargo: 1.89.0
- mise tool manager with uv configured
- Standard Rust toolchain (rustfmt, clippy available)