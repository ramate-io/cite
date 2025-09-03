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

use cite_core::mock::MockSource;

/// Try to construct a MockSource from kwargs
///
/// Supports syntax like:
/// - `same = "content"` -> MockSource::same(content)
/// - `changed = ["old", "new"]` -> MockSource::changed(old, new)
pub fn try_get_mock_source_from_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<MockSource, String> {
	let same = kwargs.get("same").and_then(|v| v.as_str());
	let changed = kwargs.get("changed");

	if let Some(content) = same {
		Ok(MockSource::same(content.to_string()))
	} else if let Some(changed_val) = changed {
		// Parse the changed tuple from JSON array
		if let Some(changed_array) = changed_val.as_array() {
			if changed_array.len() == 2 {
				let old = changed_array[0].as_str().unwrap_or("").to_string();
				let new = changed_array[1].as_str().unwrap_or("").to_string();
				Ok(MockSource::changed(old, new))
			} else {
				Err("changed parameter must be a tuple of two strings".to_string())
			}
		} else {
			Err("changed parameter must be a tuple of two strings".to_string())
		}
	} else {
		Err("mock source requires either 'same' or 'changed' parameter".to_string())
	}
}
