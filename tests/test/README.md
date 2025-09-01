# Cite Test Suite

Comprehensive test suite for the cite system, covering both compile-time and runtime behavior.

## Test Structure

```
test/
├── src/
│   └── lib.rs              # Basic functionality and integration tests
├── tests/
│   ├── compile_behavior.rs # Compile-time behavior tests (trybuild)
│   ├── runtime_behavior_test.rs # Runtime behavior and environment tests
│   └── ui/                 # UI test cases for trybuild
│       ├── pass/           # Tests that should compile successfully
│       └── fail/           # Tests that should fail compilation
└── Cargo.toml
```

## Test Categories

### 1. Unit Tests (`src/lib.rs`)

Basic functionality tests that validate the core behavior:

```rust
#[test]
fn test_basic_functionality() {
    #[cite(mock, same = "test content")]
    fn test_fn() {}
    // Validates that basic citations work
}

#[test] 
fn test_citation_with_changed_content() {
    #[cite(mock, changed = ("original", "modified"))]
    fn test_fn() {}
    // Validates behavior with changed content
}
```

**Purpose**: Ensure fundamental citation functionality works correctly.

### 2. Compile-time Behavior Tests (`tests/compile_behavior.rs`)

Uses `trybuild` to test compile-time behavior:

```rust
#[test]
fn test_compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
```

**Purpose**: Validate that the macro generates correct compile-time errors and warnings.

### 3. UI Test Cases (`tests/ui/`)

Individual test files that demonstrate specific behaviors:

#### Pass Tests (`tests/ui/pass/`)

- `basic_citation.rs` - Basic syntax works on all item types
- `citation_with_attributes.rs` - Multiple attributes work correctly
- `behavior_demonstration.rs` - Different behavior levels compile
- `changed_content_silent.rs` - Silent mode allows changed content
- `mock_diff_display.rs` - Diff display and formatting
- `module_citation.rs` - Citations work on modules

#### Fail Tests (`tests/ui/fail/`)

- `changed_content_error.rs` - ERROR level fails on changed content
- `invalid_attribute.rs` - Unknown attributes are rejected
- `missing_source.rs` - Missing source expressions fail
- `wrong_target.rs` - Citations on invalid items fail

**Purpose**: Provide comprehensive coverage of syntax variations and edge cases.

### 4. Runtime Behavior Tests (`tests/runtime_behavior_test.rs`)

Tests for runtime utilities and environment integration:

```rust
#[test]
fn test_environment_variable_parsing() {
    // Test CITE_LEVEL, CITE_ANNOTATION, CITE_GLOBAL parsing
}

#[test]
fn test_mock_source_diffs() {
    // Test MockSource runtime behavior
}
```

**Purpose**: Validate environment variable handling and runtime utilities.

## Design Principles

### Comprehensive Coverage

The test suite aims to cover:
- **All syntax variations**: Every supported citation syntax
- **All item types**: Functions, structs, traits, `impl` blocks, modules
- **All behavior modes**: ERROR, WARN, SILENT levels
- **All error conditions**: Invalid syntax, missing sources, wrong targets

### Compile-time vs Runtime Testing

**Compile-time tests** validate:
- Macro expansion correctness
- Error message quality
- Compilation success/failure behavior

**Runtime tests** validate:
- Core trait implementations
- Environment variable parsing
- Helper function behavior

### Clear Test Intent

Each test has a clear purpose documented in comments:

```rust
// Test that mock keyword syntax with ERROR level fails compilation
#[cite(mock, changed = ("old", "new"), level = "ERROR")]
fn function_that_should_fail() {}
```

### Maintainable Test Cases

- **Small, focused tests**: Each test validates one specific behavior
- **Clear naming**: Test names indicate expected behavior
- **Comprehensive stderr files**: Expected compiler output is captured

## Test Execution

### Running All Tests

```bash
# Run complete test suite
cargo test -p cite-test

# Run with output
cargo test -p cite-test -- --nocapture
```

### Running Specific Test Categories

```bash
# Unit tests only
cargo test -p cite-test --lib

# Compile-time behavior tests
cargo test -p cite-test test_compile_pass
cargo test -p cite-test test_compile_fail

# Runtime behavior tests  
cargo test -p cite-test test_environment_variable_parsing
```

### Environment Variable Testing

```bash
# Test with different global settings
CITE_LEVEL=WARN cargo test -p cite-test
CITE_GLOBAL=LENIENT cargo test -p cite-test
```

### Updating Expected Output

When the macro output changes:

```bash
# Update stderr files to match current output
TRYBUILD=overwrite cargo test -p cite-test test_compile_fail
```

## Test Development Guidelines

### Adding New Tests

1. **Identify the behavior**: What specific functionality needs testing?
2. **Choose test type**: Compile-time (UI) vs runtime behavior?
3. **Create minimal example**: Focus on one specific aspect
4. **Document intent**: Clear comments explaining the test purpose

### UI Test Guidelines

For compile-time behavior tests:

```rust
// File: tests/ui/pass/new_feature.rs
// Test description of what should work

use cite::cite;

#[cite(new_syntax, param = "value")]
fn test_function() {
    println!("Test implementation");
}

fn main() {
    test_function();
}
```

For failure tests, include expected error in `.stderr` file.

### Runtime Test Guidelines

For runtime behavior tests:

```rust
#[test]
fn test_new_feature() -> Result<(), anyhow::Error> {
    // Arrange
    let source = mock_source_same("content");
    
    // Act
    let result = source.get()?;
    
    // Assert
    assert!(result.is_same());
    Ok(())
}
```

## Debugging Test Failures

### Compile-time Test Failures

1. **Check stderr files**: Are they up to date with current macro output?
2. **Run with `TRYBUILD=overwrite`**: Update expected output
3. **Verify syntax**: Ensure test cases use current citation syntax

### Runtime Test Failures

1. **Check environment variables**: Tests may be affected by global settings
2. **Verify imports**: Ensure correct crate and trait imports
3. **Check error handling**: Use `?` syntax instead of `unwrap()`

### Common Issues

- **Outdated stderr files**: Update with `TRYBUILD=overwrite`
- **Syntax changes**: Update test cases to use current keyword syntax
- **Import errors**: Ensure `cite::cite` and `cite_core::*` imports are correct
- **Environment interference**: Reset environment variables between tests

The test suite is designed to catch regressions and ensure the cite system works correctly across all supported use cases and syntax variations.
