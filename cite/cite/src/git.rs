//! Git Source Implementation for Citation Macros
//!
//! This module provides parsing and construction of GitSource sources within
//! the `#[cite]` procedural macro. It handles the translation from macro syntax
//! to actual GitSource objects that can validate git content at compile time.
//!
//! # Design Rationale
//!
//! Git sources enable citations to validate git repository content:
//!
//! 1. **Cross-Repository Validation**: Ensure content in other repositories hasn't changed
//! 2. **Version Tracking**: Track changes in specific commits or branches
//! 3. **Dependency Validation**: Verify that referenced code hasn't been modified
//!
//! # Syntax Design
//!
//! The Git syntax follows the keyword argument pattern:
//!
//! ```rust,ignore
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md")]
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10")]
//! #[cite(git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs")]
//! ```
//!
//! This syntax was chosen because:
//! - **Clear Intent**: Revision and path are explicitly separated
//! - **Type Safety**: Different path types (file, line range, glob pattern)
//! - **Extensibility**: Easy to add new path types and git options
//!
//! # Implementation Strategy
//!
//! The module uses a multi-phase approach:
//!
//! 1. **Syntax Parsing**: Extract Git-specific arguments from the citation
//! 2. **Validation**: Validate revision format and path syntax at compile time
//! 3. **Source Construction**: Create GitSource instances using cite-git
//!
//! This separation allows the parsing logic to focus on syntax while delegating
//! the actual Git source creation to the git library.

use syn::Expr;
use cite_git::GitSource;

pub mod macro_syntax;

/// Try to construct a GitSource from citation arguments using keyword syntax
/// 
/// Supports syntax like:
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "README.md"`
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/lib.rs#L1-L10"`
/// - `git, revision = "74aa653664cd90adcc5f836f1777f265c109045b", path = "src/**/*.rs"`
pub fn try_construct_git_source_from_citation_args(args: &[Expr]) -> Option<GitSource> {
    macro_syntax::try_parse_from_citation_args(args)
}
