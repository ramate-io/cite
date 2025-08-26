# Dependency Verification Test

This test crate verifies that the `cite` procedural macro does not add heavy runtime dependencies to consuming applications.

## Issue Context

This test addresses [issue #15](https://github.com/ramate-io/cite/issues/15) - ensuring that the `cite` macro has zero runtime overhead and doesn't pull in heavy dependencies like `reqwest`, `scraper`, `regex`, etc. into the final binary.

## Current Status

The test currently **identifies the problem** described in issue #15. The `cite` procedural macro is currently pulling in heavy dependencies at runtime, which is the exact issue that needs to be resolved.

## Test Structure

### `src/lib.rs`
- Simple library that uses the `cite` macro
- Demonstrates various citation patterns
- Serves as a realistic test case for dependency analysis

### `tests/dependency_verification.rs`
Contains several tests:

1. **`test_cite_dependencies`** ✅ - Verifies that only `cite` is listed as a direct dependency
2. **`test_cite_currently_has_heavy_dependencies`** ✅ - Documents the current problem (uses `#[should_panic]`)
3. **`test_cite_has_no_heavy_dependencies_future`** 🚧 - Will verify the fix (currently `#[ignore]`)
4. **`test_cite_compilation`** ✅ - Verifies the macro works at compile time
5. **`test_cite_runtime`** ✅ - Verifies the generated code runs correctly

## Running the Tests

```bash
# Run all tests
cargo test

# Run with ignored tests (will fail until issue #15 is resolved)
cargo test -- --ignored
```

## Expected Behavior

### Current (Issue #15 exists):
- `test_cite_currently_has_heavy_dependencies` passes (panics as expected)
- `test_cite_has_no_heavy_dependencies_future` is ignored

### After Issue #15 is resolved:
- `test_cite_currently_has_heavy_dependencies` should be removed or updated
- `test_cite_has_no_heavy_dependencies_future` should have `#[ignore]` removed and pass

## What Needs to be Fixed

The fundamental issue is that Rust procedural macros include ALL their dependencies in the final binary, not just at compile time. The `cite` crate needs to be restructured to avoid this, possibly by:

1. Moving heavy dependencies to build-time only
2. Using feature flags to make heavy dependencies optional
3. Restructuring the macro to use lighter-weight alternatives
4. Using a different architecture that separates compile-time and runtime concerns

## Integration

This test is part of the workspace and runs with `cargo test` from the repository root.
