# Cite Workspace

This directory contains the core implementation of the cite system for Rust.

## Structure

```
cite/
├── cite/           # Main procedural macro crate (user-facing)
├── core/           # Core traits and types (lightweight, runtime utilities)
└── test/           # Comprehensive test suite
```

## Crates

### `cite` - Procedural Macro Implementation

The main user-facing crate that provides the `#[cite]` attribute macro.

**Key Features:**
- Keyword argument syntax parsing
- Compile-time content validation
- Multiple source type support
- Environment variable integration

**Design Highlights:**
- Zero runtime overhead - all validation at compile time
- Extensible architecture for new source types
- Clean separation of parsing and validation logic

### `cite-core` - Core Traits and Types

Lightweight crate containing the fundamental traits and types used by the citation system.

**Key Components:**
- `Source` trait for content validation
- `MockSource` implementation for testing
- Behavior configuration types (`CitationLevel`, `CitationBehavior`, etc.)
- Runtime utility functions

**Design Highlights:**
- Standard library compatible (no `no_std` constraints)
- Focused on core abstractions
- Minimal dependencies

### `cite-test` - Test Suite

Comprehensive test suite that validates both compile-time and runtime behavior.

**Test Categories:**
- Unit tests for runtime behavior
- UI tests for compile-time behavior (using `trybuild`)
- Integration tests for environment variable handling
- Performance and behavior validation

**Design Highlights:**
- Extensive compile-time behavior testing
- Clear separation of passing vs failing test cases
- Comprehensive coverage of syntax variations

## Design Decisions

### Architecture Choices

1. **Separate Crates**: Split functionality to enable lightweight core usage
2. **Keyword Syntax**: Chosen after evaluating function-like and struct-like alternatives
3. **Compile-time Validation**: All validation during macro expansion for zero runtime cost
4. **Environment Integration**: Global configuration via environment variables

### Syntax Evolution

The citation syntax evolved through several iterations:

1. **Direct Source Construction** (`MockSource::same("content")`) - Complex AST patterns
2. **Helper Macros** (`mock!(same!("content"))`) - Macro expansion order issues  
3. **Function-like** (`mock(same("content"))`) - Parser ambiguities
4. **Keyword Arguments** (`mock, same = "content"`) - ✅ Current clean solution

### Implementation Strategy

- **Modular Parsing**: Each source type has its own parsing module
- **Graceful Fallback**: Parsers return `None` to allow trying other source types
- **Error Propagation**: Validation results become compile-time diagnostics
- **Future Extensibility**: Framework supports adding new source types

## Getting Started

For development work on the cite system:

```bash
# Run all tests
cargo test

# Run just the macro tests
cargo test -p cite-test

# Run with compile-time behavior tests
cargo test -p cite-test test_compile_pass
cargo test -p cite-test test_compile_fail

# Test with environment variables
CITE_LEVEL=WARN cargo test -p cite-test
```

## Contributing

When adding new source types:

1. Add parsing logic in a new module under `cite/src/`
2. Implement core traits in `cite-core/src/`
3. Add comprehensive tests in `cite-test/tests/`
4. Update documentation in both crates

See the `mock` source implementation as a reference for the expected patterns.
