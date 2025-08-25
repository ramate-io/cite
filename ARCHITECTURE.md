# Cite System Architecture

This document describes the design decisions and architectural patterns used in the cite system for Rust.

## Overview

The cite system provides compile-time citation validation through procedural macros. The key innovation is performing content validation during macro expansion, enabling zero runtime overhead while providing rich feedback to developers.

## High-Level Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User Code     │    │   Cite Macro    │    │   Cite Core     │
│                 │    │                 │    │                 │
│ #[cite(mock,    │───▶│ Parse Arguments │───▶│ Source Traits   │
│  same="api")]   │    │                 │    │                 │
│ fn my_func() {} │    │ Execute Source  │◀───│ MockSource      │
│                 │    │                 │    │                 │
│                 │◀───│ Generate Code   │    │ Behavior Config │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Design Principles

### 1. Zero Runtime Overhead

**Principle**: All citation validation happens at compile time.

**Implementation**:
- Procedural macros perform validation during `cargo build`
- Network requests and content fetching occur during compilation
- Generated code contains only minimal `const` assertions
- No runtime performance impact on the compiled application

**Trade-offs**:
- ✅ No runtime cost
- ✅ Early error detection
- ❌ Compilation time increases
- ❌ Network dependencies during build

### 2. Modular Architecture

**Principle**: Separate concerns into focused crates.

**Structure**:
```
cite/
├── cite/           # Procedural macro implementation (compile-time)
├── core/           # Core traits and types (runtime + compile-time)  
└── test/           # Comprehensive test suite
```

**Benefits**:
- **cite-core** can be used independently for runtime validation
- Clear separation between macro logic and core abstractions
- Easier testing and maintenance
- Potential for future `no_std` core if needed

### 3. Keyword Argument Syntax

**Principle**: Use clean, unambiguous syntax that avoids parser conflicts.

**Evolution**:
```rust
// Evolution 1: Direct construction (complex AST patterns)
#[cite(MockSource::same("content"))]

// Evolution 2: Helper macros (expansion order issues)  
#[cite(mock!(same!("content")))]

// Evolution 3: Function-like (parser ambiguities)
#[cite(mock(same("content")))]

// Evolution 4: Keyword arguments (current - clean solution)
#[cite(mock, same = "content")]
```

**Advantages**:
- **Unambiguous parsing**: No conflicts with Rust expression grammar
- **Extensible**: Easy to add new source types and parameters
- **Readable**: Natural flow from source type to parameters to behavior

### 4. Environment Integration

**Principle**: Enable global configuration while allowing local overrides.

**Implementation**:
```rust
// Global configuration via environment variables
CITE_LEVEL=WARN
CITE_ANNOTATION=FOOTNOTE  
CITE_GLOBAL=STRICT

// Local overrides in citation attributes
#[cite(mock, changed = ("old", "new"), level = "ERROR")]
```

**Use Cases**:
- **Development**: CITE_LEVEL=SILENT for rapid iteration
- **CI**: CITE_LEVEL=ERROR to catch all changes
- **Production**: CITE_LEVEL=WARN for monitoring

## Implementation Details

### Macro Expansion Flow

1. **Parse Arguments**
   ```rust
   // Input: mock, same = "content", level = "ERROR"
   // Parse into: source_type="mock", source_args={same="content"}, behavior_args={level="ERROR"}
   ```

2. **Source Construction**
   ```rust
   // Pattern match on source_type to select parser
   if args[0] == "mock" {
       mock::try_construct_mock_source_from_citation_args(&args[1..])
   }
   ```

3. **Content Validation**
   ```rust
   // Execute source.get() during macro expansion
   let comparison = mock_source.get()?;
   let result = comparison.validate(&behavior, level_override);
   ```

4. **Code Generation**
   ```rust
   // Convert validation result to compile-time diagnostic
   match result {
       Ok(None) => quote! { const _: () = (); },      // Success
       Ok(Some(msg)) => quote! { const _WARNING: &str = #msg; },  // Warning
       Err(msg) => syn::Error::new(span, msg).to_compile_error(),  // Error
   }
   ```

### Source Type System

The trait system enables extensible source types:

```rust
pub trait Source {
    type Referenced: Referenced;  // Original content
    type Current: Current;        // Current content  
    type Diff: Diff;             // Change representation
    
    fn get(&self) -> Result<Comparison<Self::Referenced, Self::Current, Self::Diff>, SourceError>;
}
```

**Design Benefits**:
- **Generic content types**: Support text, binary, structured data
- **Rich diff representations**: Beyond simple string comparison
- **Extensible**: New source types integrate seamlessly

### Error Propagation Strategy

Validation results flow through multiple layers:

```
Source::get() -> Result<Comparison, SourceError>
     ↓
Comparison::validate() -> CitationValidationResult  
     ↓
Macro validation -> Result<Option<String>, String>
     ↓  
Code generation -> TokenStream (const assertions or compile errors)
```

This enables:
- **Graceful degradation**: Warnings vs errors based on configuration
- **Rich error messages**: Include diff information and context
- **Proper error attribution**: Errors point to the citation location

## Syntax Design Deep Dive

### Parsing Strategy

The keyword argument parser uses a two-phase approach:

1. **Argument Categorization**
   ```rust
   // Separate source arguments from behavior arguments
   mock, same = "content", level = "ERROR"
   │     │                 │
   │     └─ source args     └─ behavior args
   └─ source type
   ```

2. **Source-Specific Parsing**
   ```rust
   // Each source type has its own parser
   mod mock::macro_syntax {
       fn try_parse_from_citation_args(args: &[Expr]) -> Option<MockSource>
   }
   ```

**Extensibility**: Adding new source types requires:
1. Create new parsing module (e.g., `http::macro_syntax`)
2. Add pattern match in main parser
3. Implement Source trait for the new type

### Error Recovery

The parser is designed for graceful failure:

```rust
// Try each source type in sequence
if let Some(source) = mock::try_construct_mock_source_from_citation_args(args) {
    return Some(source);
}
if let Some(source) = http::try_construct_http_source_from_citation_args(args) {
    return Some(source);  
}
// etc.
```

This enables:
- **Forward compatibility**: Unknown source types are ignored rather than causing errors
- **Source type discovery**: Clear error messages when no parser matches
- **Incremental adoption**: New source types can be added without breaking existing code

## Performance Considerations

### Compile-time Performance

**Trade-offs**:
- ✅ Zero runtime overhead
- ✅ Early validation feedback
- ❌ Increased compilation time
- ❌ Network dependencies during build

**Mitigation Strategies**:
- **Caching**: Future HTTP sources should cache responses
- **Parallel validation**: Independent citations can validate concurrently
- **Incremental compilation**: Unchanged citations skip re-validation

### Memory Usage

**Current Approach**:
- String-based content representation for simplicity
- Full content storage for rich diff generation
- Environment variable caching

**Future Optimizations**:
- Content hashing for large data
- Streaming validation for huge files
- Lazy diff computation

## Security Considerations

### Compile-time Network Access

**Risks**:
- Build process requires network access
- Potential for network-based build failures
- Trust relationship with external content sources

**Mitigations**:
- **Caching**: Reduce network dependencies
- **Fallback modes**: Allow builds to proceed with warnings
- **Content hashing**: Verify content integrity

### Content Validation

**Approach**:
- Explicit content comparison (not security validation)
- User-controlled validation rules
- No automatic content sanitization

**Scope**: Citation validation focuses on change detection, not security validation.

## Future Extensions

### Additional Source Types

**HTTP Sources**:
```rust
#[cite(http, url = "https://api.example.com/v1", 
       path = "$.version", expected = "1.2.3")]
```

**File Sources**:
```rust
#[cite(file, path = "docs/api.md", 
       hash = "sha256:abcd...")]
```

**Git Sources**:
```rust
#[cite(git, repo = "https://github.com/user/repo", 
       commit = "abc123", path = "README.md")]
```

### Enhanced Validation

**Structured Content**:
- JSON path validation
- XML schema validation
- Protocol buffer compatibility

**Semantic Validation**:
- API compatibility checking
- Schema evolution validation
- Breaking change detection

### Tooling Integration

**IDE Support**:
- Syntax highlighting for citation attributes
- Inline validation feedback
- Quick fixes for common issues

**CI Integration**:
- Citation coverage reporting
- Change impact analysis
- Automated citation updates

This architecture provides a solid foundation for compile-time citation validation while maintaining flexibility for future enhancements.
