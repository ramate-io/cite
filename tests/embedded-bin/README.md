# Embedded Binary Test

This test crate verifies that the `cite` procedural macro works correctly in embedded Rust binaries, ensuring compatibility with the full embedded development stack including runtime, HAL, and Cortex-M ecosystem.

## Purpose

This test suite focuses on embedded-style code patterns and cite macro compatibility:

- ✅ **Embedded code patterns** - Tests `cite` macros with embedded-style abstractions
- ✅ **Embedded HAL compatibility** - Verifies cite works with embedded HAL traits
- ✅ **Compilation verification** - Ensures cite macros compile in embedded contexts
- ✅ **Dependency isolation** - Confirms no heavy dependencies leak into embedded builds

## Test Coverage

### 📋 **Dependency Verification Tests** (`tests/dependency_verification.rs`)

1. **`test_cite_dependencies_embedded_bin`** ✅ - Verifies all expected embedded dependencies present
2. **`test_cite_no_heavy_runtime_dependencies_embedded_bin`** ✅ - Confirms no heavy dependencies in embedded binary
3. **`test_embedded_bin_dependency_footprint`** ✅ - Ensures reasonable dependency count for embedded binaries
4. **`test_cite_macro_expansion_embedded_bin`** ✅ - Verifies macro expansion in embedded binary context
5. **`test_embedded_binary_functionality`** ✅ - Tests actual embedded binary functionality with cite macros

### 🔧 **Binary Structure**

- **`src/lib.rs`** - Embedded library code with drivers, state management, and HAL integration
- **`src/main.rs`** - Complete embedded binary with `#[entry]`, state machine, and task scheduling
- **`memory.x`** - Memory layout file for embedded linking (testing purposes)

### 🎯 **Embedded Patterns Tested**

- **Application Context** - State management with embedded-style patterns
- **Hardware Abstractions** - GPIO pin and LED driver abstractions
- **Embedded Utilities** - Delay functions and embedded helper utilities
- **Device Drivers** - Simple driver patterns with cite annotations
- **Error Handling** - Embedded-appropriate error states and recovery
- **Embedded-HAL Integration** - Compatibility with embedded HAL traits

## Key Features Tested

### ✅ **Embedded Compatibility**
- `embedded-hal` trait implementations with citations
- Embedded-style peripheral abstractions with citations
- Compilation verification for embedded patterns
- Lightweight embedded dependency footprint

### ✅ **Real-World Embedded Patterns**
- **State machines** with cited state transition functions
- **Driver abstractions** with cited hardware interfaces
- **Application contexts** with cited system management
- **Interrupt simulation** with cited handler functions

### ✅ **Citation Coverage**
- **Entry point** and main loop functions
- **Hardware drivers** and peripheral interfaces
- **State management** functions and methods
- **Utility functions** using cortex-m primitives
- **Error handling** and recovery functions

## Dependencies

- **`cite`** - The citation macro being tested
- **`embedded-hal`** - Standard embedded hardware abstraction layer

## Running the Tests

```bash
# Run all embedded binary tests
cargo test -p cite-embedded-bin-test

# Run just the library tests (avoids binary compilation issues)
cargo test -p cite-embedded-bin-test --lib

# Test embedded target compatibility
cargo test -p cite-embedded-bin-test test_embedded_target_compatibility
```

## Expected Results

All tests should pass, confirming:

1. ✅ **Zero heavy runtime dependencies** - Only embedded-appropriate dependencies in runtime graph
2. ✅ **Complete embedded compatibility** - All citations work in full embedded binary context
3. ✅ **Reasonable footprint** - Dependency count suitable for embedded binary development
4. ✅ **Runtime integration** - cite macros work seamlessly with cortex-m-rt and embedded runtime

This test suite ensures that `cite` is fully compatible with complete embedded Rust binary development, including all aspects of the embedded development stack.
