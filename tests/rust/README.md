# CSD Rust Tests

This directory contains integration tests for the CSD Rust codebase. Tests have been moved from inline `#[cfg(test)]` modules to this separate directory structure to improve code readability and reduce token consumption when using AI tools.

## Directory Structure

```
tests/rust/
├── README.md           # This file
├── mod.rs             # Main test module with common utilities
├── cli/               # CLI-related tests
│   ├── mod.rs
│   └── test_args.rs   # Command line argument parsing tests
├── core/              # Core functionality tests
│   └── mod.rs         # TODO: Move matrix, scanner tests here
├── plugins/           # Plugin system tests
│   └── mod.rs         # TODO: Move communication, manager tests here
└── utils/             # Utility tests
    └── mod.rs         # TODO: Move config tests here
```

## Running Tests

### All Rust Tests

```bash
# From project root
cargo test

# Run only integration tests
cargo test --test '*'

# Run with verbose output
cargo test --test '*' -- --nocapture
```

### Specific Test Modules

```bash
# Run only CLI tests
cargo test --test cli

# Run only args tests
cargo test cli::test_args

# Run a specific test
cargo test cli::test_args::basic_parsing_tests::test_scan_command_basic
```

## Test Organization

Tests are organized to mirror the `src/` directory structure:

- **`cli/`** - Tests for command-line interface components
- **`core/`** - Tests for core business logic (matrix, scanner, etc.)
- **`plugins/`** - Tests for plugin system components
- **`utils/`** - Tests for utility functions and configurations

## Benefits of External Tests

1. **Reduced Token Consumption**: Source files are now much shorter, making them more suitable for AI analysis
2. **Better Organization**: Tests are grouped logically in their own directory structure
3. **Improved Readability**: Source code focuses purely on implementation
4. **Easier Test Discovery**: All tests are in one predictable location
5. **Better IDE Support**: Most IDEs can better navigate and run external test files

## Migration Progress

- ✅ `src/cli/args.rs` - Tests moved to `tests/rust/cli/test_args.rs`
- ⏳ `src/core/matrix.rs` - TODO: Move tests to `tests/rust/core/test_matrix.rs`
- ⏳ `src/core/scanner.rs` - TODO: Move tests to `tests/rust/core/test_scanner.rs`
- ⏳ `src/utils/config.rs` - TODO: Move tests to `tests/rust/utils/test_config.rs`

## Test Utilities

Common test utilities are available in `tests/rust/mod.rs` under the `common` module:
