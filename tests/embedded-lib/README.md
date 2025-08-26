# Embedded Library Test

This test crate verifies that the `cite` procedural macro works correctly with embedded Rust libraries, ensuring compatibility with the embedded HAL ecosystem and typical embedded development patterns.

## Purpose

This test suite focuses on embedded library scenarios:

- ✅ **Embedded HAL integration** - Tests compatibility with embedded HAL traits and patterns
- ✅ **No-std library patterns** - Verifies cite works with typical embedded library structures
- ✅ **Driver and sensor patterns** - Tests citations on embedded drivers, sensors, and hardware abstractions
- ✅ **Embedded-specific dependencies** - Ensures cite doesn't conflict with embedded ecosystem crates

## Test Coverage

### 📋 **Dependency Verification Tests** (`tests/dependency_verification.rs`)

1. **`test_cite_dependencies_embedded_lib`** ✅ - Verifies `cite`, `embedded-hal`, and `nb` as runtime dependencies
2. **`test_cite_no_heavy_runtime_dependencies_embedded_lib`** ✅ - Confirms no heavy dependencies in embedded library context
3. **`test_embedded_lib_dependency_footprint`** ✅ - Ensures reasonable dependency count for embedded libraries
4. **`test_cite_macro_expansion_embedded_lib`** ✅ - Verifies macro expansion in embedded library context
5. **`test_embedded_hal_integration`** ✅ - Tests actual embedded HAL trait integration with cite macros

### 🔧 **Library Structure** (`src/lib.rs`)

The test library demonstrates embedded patterns with cite macros:

- **Hardware Drivers** - GPIO pin drivers with embedded HAL traits
- **Sensor Abstractions** - Temperature sensor with custom trait implementations  
- **Error Types** - Embedded-specific error enums
- **Configuration Modules** - `const` configuration structures
- **HAL Integration** - Proper use of embedded HAL `OutputPin` trait

## Key Features Tested

### ✅ **Embedded HAL Compatibility**
- embedded HAL traits work seamlessly with cite macros
- GPIO pin abstractions with cite annotations
- Sensor traits and implementations with citations

### ✅ **Embedded Library Patterns**
- **Driver structures** with embedded HAL trait bounds
- **Sensor abstractions** with custom error types
- **Configuration modules** with `const` functions
- **Utility functions** for embedded contexts

### ✅ **Citation Coverage**
- **Driver constructors** and methods
- **Trait implementations** for sensors
- **Error enums** specific to embedded contexts
- **Module-level** configurations
- **Generic functions** with trait bounds

## Dependencies

- **`embedded-hal`** - Standard embedded hardware abstraction layer
- **`nb`** - Non-blocking APIs for embedded systems
- **`cite`** - The citation macro being tested

## Running the Tests

```bash
# Run all embedded library tests
cargo test -p cite-embedded-lib-test

# Check that library compiles for embedded targets
cargo check -p cite-embedded-lib-test --target thumbv7em-none-eabihf
```

## Expected Results

All tests should pass, confirming:

1. ✅ **Zero heavy runtime dependencies** - Only embedded-appropriate dependencies in runtime graph
2. ✅ **Embedded HAL compatibility** - All citations work with embedded HAL patterns
3. ✅ **Reasonable footprint** - Dependency count suitable for embedded library development
4. ✅ **Macro isolation** - Heavy compile-time dependencies do not leak to embedded runtime

This test suite ensures that `cite` is fully compatible with embedded Rust library development and the embedded HAL ecosystem.
