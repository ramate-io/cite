use crate::HttpMatch;
use cite_core::ui::SourceUi;
use serde_json::{Map, Value};

impl SourceUi<ReferencedHttp, CurrentHttp, HttpDiff> for HttpMatch {
	fn from_kwarg_json() -> Result<Self, SourceUiError> {
		Ok(Self::from_kwarg_json(kwargs))
	}
}
