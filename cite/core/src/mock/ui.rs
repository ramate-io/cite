use crate::ui::SourceUi;
use crate::MockSource;

impl SourceUi<ReferencedString, CurrentString, StringDiff> for MockSource {
	fn from_kwarg_json() -> Result<Self, SourceUiError> {
		Ok(Self::from_kwarg_json(kwargs))
	}
}
