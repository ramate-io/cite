use crate::{Current, Diff, Referenced, Source};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Errors thrown by the [SourceUi] trait implementations.
#[derive(Debug, thiserror::Error)]
pub enum SourceUiError {
	#[error("Source internal error: {0}")]
	Internal(#[source] Box<dyn std::error::Error + Send + Sync>),

	#[error("Invalid JSON format: {0}")]
	InvalidJson(String),

	#[error("Missing required parameter: {0}")]
	MissingParameter(String),

	#[error("Invalid parameter value: {0}")]
	InvalidParameter(String),

	#[error("Serialization error: {0}")]
	Serialization(String),
}

/// Represents the content for `#[cite(above)]` macro attributes
/// This is designed to be embedded in doc comments and parsed by the macro system
#[derive(Debug, Clone, PartialEq)]
pub struct AboveDocAttr {
	/// The JSON content to be embedded in the doc attribute
	pub json_content: String,
	/// The source type identifier (e.g., "git", "http", "mock")
	pub source_type: String,
}

impl AboveDocAttr {
	/// Create a new AboveDocAttr from JSON content
	pub fn new(json_content: String, source_type: String) -> Self {
		Self { json_content, source_type }
	}

	/// Format as a doc attribute string
	pub fn to_doc_attr_string(&self) -> String {
		format!(
			"<div style=\"display: none;\"><cite above content [{}] end_content/></div>",
			self.json_content
		)
	}
}

/// Trait for UI-friendly source operations
/// This trait provides methods for creating sources from keyword arguments
/// and converting them to various formats useful for macro generation
pub trait SourceUi<R, C, D>: Source<R, C, D> + Sized
where
	R: Referenced,
	C: Current<R, D>,
	D: Diff,
{
	/// Create a source from keyword arguments provided as JSON
	/// This method should support both direct serialization and user-friendly variants
	fn from_kwarg_json(kwargs: &HashMap<String, Value>) -> Result<Self, SourceUiError>;

	/// Convert the source to standard JSON format
	/// This should produce a JSON representation that can be used for serialization
	fn to_standard_json(&self) -> Result<Map<String, Value>, SourceUiError>;

	/// Convert the source to a format suitable for `#[cite(above)]` macro attributes
	/// This should produce content that can be embedded in doc comments
	fn to_above_doc_attr(&self) -> Result<AboveDocAttr, SourceUiError>;
}
