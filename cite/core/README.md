# Cite Core

Core traits, types, and utilities for the cite system.

## Purpose

This crate provides the fundamental abstractions used by the cite system:

- **Traits**: Core interfaces for content sources and validation
- **Types**: Behavior configuration and result types  
- **Utilities**: Runtime helper functions for testing and development

## Design Philosophy

### Lightweight and Focused

The core crate is designed to be minimal and focused:
- Only essential traits and types
- No procedural macro dependencies  
- Standard library compatible
- Minimal external dependencies

### Runtime vs Compile-time Separation

While the main cite system operates at compile time, this crate provides:
- **Runtime utilities** for testing and development
- **Shared types** used by both compile-time and runtime code
- **Core abstractions** that can be implemented by any source type

## Key Components

### Source Trait

The fundamental abstraction for content sources:

```rust
pub trait Source {
    type Referenced: Referenced;
    type Current: Current; 
    type Diff: Diff;
    
    fn get(&self) -> Result<Comparison<Self::Referenced, Self::Current, Self::Diff>, SourceError>;
}
```

**Design Rationale:**
- Generic over content types to support different data formats
- Returns a `Comparison` that can be validated against behavior rules
- Error handling through `SourceError` for consistent error reporting

### MockSource Implementation

Primary implementation for testing and development:

```rust
// Runtime usage
let source = mock_source_same("content");
let source = mock_source_changed("old", "new");

// Compile-time usage (in macros)
#[cite(mock, same = "content")]
#[cite(mock, changed = ("old", "new"))]
```

**Design Rationale:**
- Dual API: runtime functions + compile-time macro syntax
- Clear intent with `same` vs `changed` terminology
- String-based for simplicity in testing scenarios

### Behavior Configuration

Types for controlling citation validation behavior:

```rust
pub enum CitationLevel {
    Error,   // Fail compilation on content mismatch
    Warn,    // Emit warning on content mismatch  
    Silent,  // No output on content mismatch
}

pub struct CitationBehavior {
    pub level: CitationLevel,
    pub annotation: CitationAnnotation,
    pub global: CitationGlobal,
}
```

**Design Rationale:**
- Environment variable integration (`CITE_LEVEL`, etc.)
- Local override capability in citation attributes
- Flexible configuration for different use cases

### Content Abstractions

Traits for different types of content:

```rust
pub trait Referenced {
    // Content that was originally referenced
}

pub trait Current {
    // Content as it exists now
}

pub trait Diff {
    // Representation of changes between referenced and current
}
```

**Design Rationale:**
- Separates "what was expected" from "what is now"
- Enables rich diff representations beyond simple string comparison
- Allows for different content types (text, binary, structured data)

## Usage Patterns

### Runtime Testing

```rust
use cite_core::{mock_source_same, mock_source_changed, Source};

#[test]
fn test_my_citation_logic() {
    let source = mock_source_changed("v1.0", "v2.0");
    let comparison = source.get().expect("Source should be valid");
    
    assert_eq!(comparison.referenced().0, "v1.0");
    assert_eq!(comparison.current().0, "v2.0");
    assert!(!comparison.is_same());
}
```

### Behavior Configuration

```rust
use cite_core::{CitationBehavior, CitationLevel};

let behavior = CitationBehavior::from_env();
let result = comparison.validate(&behavior, Some(CitationLevel::Warn));

match result {
    ValidationResult::Valid => println!("Citation is valid"),
    ValidationResult::Warning => println!("Citation has warnings"), 
    ValidationResult::Error => println!("Citation failed validation"),
}
```

### Custom Source Implementation

```rust
struct MyCustomSource {
    url: String,
}

impl Source for MyCustomSource {
    type Referenced = MyContent;
    type Current = MyContent;
    type Diff = MyDiff;
    
    fn get(&self) -> Result<Comparison<Self::Referenced, Self::Current, Self::Diff>, SourceError> {
        // Implementation for fetching and comparing content
    }
}
```

## Integration Points

### With Procedural Macro

The procedural macro in the `cite` crate:
1. Parses citation syntax into source construction calls
2. Creates source instances using the core traits
3. Calls `source.get()` during macro expansion  
4. Converts validation results into compile-time diagnostics

### With Environment Variables

Behavior types integrate with environment variables:
- `CITE_LEVEL=WARN` sets global citation level
- `CITE_ANNOTATION=FOOTNOTE` sets annotation style
- `CITE_GLOBAL=STRICT` sets global behavior mode

### With Testing Framework

The mock implementations enable comprehensive testing:
- Unit tests can validate citation logic without external dependencies
- Integration tests can verify behavior configuration
- Performance tests can measure validation overhead

## Future Extensions

The core traits are designed to support future source types:

- **HTTP Sources**: Validate web content and APIs
- **File Sources**: Validate local file content
- **Git Sources**: Validate repository content and history
- **Database Sources**: Validate database schema and data

The abstraction layer ensures new source types integrate seamlessly with the existing validation and behavior system.
