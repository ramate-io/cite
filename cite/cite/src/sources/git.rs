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

use cite_core::ui::SourceUi;
use cite_git::GitSource;
use serde_json::Value;
use std::collections::HashMap;

/// Try to construct a GitSource from kwargs using the SourceUi trait
///
/// Supports syntax like:
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "README.md"` -> GitSource for file
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "src/lib.rs#L1-L10"` -> GitSource with line range
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "src/**/*.rs"` -> GitSource with glob pattern
/// - `remote = "https://github.com/ramate-io/cite", ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694", cur_rev = "main", path = "README.md#L5"` -> GitSource with single line
pub fn try_get_git_source_from_kwargs(
	kwargs: &HashMap<String, Value>,
) -> Result<GitSource, String> {
	GitSource::from_kwarg_json(kwargs).map_err(|e| format!("Failed to create Git source: {}", e))
}
