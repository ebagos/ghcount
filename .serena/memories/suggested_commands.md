# Essential Development Commands

## Core Development
- `cargo check` - Quick syntax and type checking
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Build and run the application
- `cargo run -- [args]` - Run with command line arguments

## Code Quality
- `cargo fmt` - Format code using rustfmt
- `cargo clippy` - Run the Rust linter
- `cargo clippy -- -D warnings` - Run clippy treating warnings as errors

## Testing
- `cargo test` - Run all tests
- `cargo test [test_name]` - Run specific test
- `cargo test --doc` - Run documentation tests

## Dependencies
- `cargo add [crate]` - Add a new dependency
- `cargo update` - Update dependencies
- `cargo tree` - Show dependency tree

## Cleanup
- `cargo clean` - Remove build artifacts

## Git Commands (macOS/Darwin)
- `git status` - Check repository status
- `git add .` - Stage all changes
- `git commit -m "message"` - Commit changes
- `git push` - Push to remote
- `git pull` - Pull from remote

## System Tools (macOS)
- `ls` - List files
- `find . -name "pattern"` - Find files
- `grep -r "pattern" .` - Search in files