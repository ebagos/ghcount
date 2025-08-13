# Code Style and Conventions

## Rust Edition
- Using Rust 2024 edition (latest)

## Formatting
- Use `cargo fmt` for consistent formatting
- Follows standard Rust formatting guidelines
- 4-space indentation
- Line length typically 100 characters (rustfmt default)

## Naming Conventions
- **Functions and variables**: snake_case
- **Types and traits**: PascalCase
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case
- **Crates**: kebab-case (as seen in Cargo.toml)

## Code Organization
- Main entry point in `src/main.rs`
- Modules should be in separate files as project grows
- Use `mod.rs` files for module hierarchies

## Documentation
- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Include examples in documentation when appropriate
- Run `cargo test --doc` to test documentation examples

## Error Handling
- Prefer `Result<T, E>` over panicking
- Use `?` operator for error propagation
- Consider using `anyhow` or `thiserror` for error handling as project grows

## Dependencies
- Keep dependencies minimal and well-maintained
- Prefer standard library solutions when possible
- Add dependencies via `cargo add` command

## General Guidelines
- Write tests for public APIs
- Use descriptive variable and function names
- Avoid deep nesting - prefer early returns
- Use pattern matching effectively
- Leverage Rust's type system for safety