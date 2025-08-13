# Task Completion Workflow

## Before Committing Changes
Always run these commands to ensure code quality:

1. **Format the code**
   ```bash
   cargo fmt
   ```

2. **Check for compilation errors**
   ```bash
   cargo check
   ```

3. **Run linter (clippy)**
   ```bash
   cargo clippy -- -D warnings
   ```

4. **Run tests (when available)**
   ```bash
   cargo test
   ```

5. **Build the project**
   ```bash
   cargo build
   ```

## Recommended Order
1. Make your changes
2. Format with `cargo fmt`
3. Check compilation with `cargo check`
4. Fix any clippy warnings with `cargo clippy`
5. Run tests with `cargo test`
6. Final build with `cargo build`
7. Test run with `cargo run`

## CI/CD Considerations
- Ensure all commands pass before committing
- Consider adding these as pre-commit hooks
- The project should build cleanly on fresh checkout

## Documentation Updates
- Update README.md if adding new features
- Update inline documentation for public APIs
- Run `cargo test --doc` to verify documentation examples

## Performance Considerations
- Use `cargo build --release` for performance testing
- Profile with tools like `perf` on Linux or Instruments on macOS
- Consider `cargo bench` for benchmarking (requires benchmark setup)