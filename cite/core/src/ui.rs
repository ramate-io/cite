use crate::{Current, Diff, Referenced, Source};
use serde_json::{Map, Value};

/// Errors thrown by the [Source].
#[derive(Debug, thiserror::Error)]
pub enum SourceUiError {
	#[error("Source internal error: {0}")]
	Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait SourceUi<R, C, D>: Source<R, C, D> + Sized
where
	R: Referenced,
	C: Current<R, D>,
	D: Diff,
{
	fn from_kwarg_json() -> Result<Self, SourceUiError>;

	fn to_standard_json() -> Result<Map<String, Value>, SourceUiError>;

	fn to_above_doc_attr() -> Result<NotSureWhatsBestForMacro, SourceUiError>;
}
