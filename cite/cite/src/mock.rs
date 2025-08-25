//! Mock Source Implementation for Citation Macros
//!
//! This module provides parsing and construction of MockSource instances within
//! the `#[cite]` procedural macro. It handles the translation from macro syntax
//! to actual MockSource objects that can be validated at compile time.
//!
//! # Design Rationale
//!
//! Mock sources serve multiple purposes in the cite system:
//!
//! 1. **Testing**: Enable comprehensive testing of the citation system without
//!    external dependencies
//! 2. **Development**: Allow developers to prototype citation behavior before
//!    connecting to real data sources
//! 3. **Documentation**: Provide clear examples of how citations work
//!
//! # Syntax Design
//!
//! The mock syntax follows the keyword argument pattern:
//!
//! ```rust,ignore
//! #[cite(mock, same = "content")]           // Content unchanged
//! #[cite(mock, changed = ("old", "new"))]  // Content changed
//! ```
//!
//! This syntax was chosen because:
//! - **Clear Intent**: The `same` vs `changed` keywords make the test intent obvious
//! - **Type Safety**: Tuples for changed content prevent argument order confusion
//! - **Extensibility**: Easy to add new mock source types (e.g. `missing`, `error`)
//!
//! # Implementation Strategy
//!
//! The module uses a two-phase approach:
//!
//! 1. **Syntax Parsing**: Extract mock-specific arguments from the citation
//! 2. **Source Construction**: Create MockSource instances using cite-core helpers
//!
//! This separation allows the parsing logic to focus on syntax while delegating
//! the actual mock source creation to the core library.

use syn::Expr;
use cite_core::mock::MockSource;

mod macro_syntax;

/// Try to construct a MockSource from the citation expression using various syntax patterns
/// 
/// This function handles parsing of MockSource constructor expressions and creates
/// the appropriate MockSource during macro expansion.
pub fn try_construct_mock_source_from_expr(_expr: &Expr) -> Option<MockSource> {
    // This function is now deprecated in favor of try_construct_mock_source_from_citation_args
    // but kept for backwards compatibility with any remaining direct expression parsing
    None
}

/// Try to construct a MockSource from citation arguments using keyword syntax
/// 
/// Supports syntax like:
/// - `mock, same = "content"`  
/// - `mock, changed = ("old", "new")`
pub fn try_construct_mock_source_from_citation_args(args: &[Expr]) -> Option<MockSource> {
    macro_syntax::try_parse_from_citation_args(args)
}