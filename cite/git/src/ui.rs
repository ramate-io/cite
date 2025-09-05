use crate::GitSource;
use cite_core::ui::SourceUi;

impl SourceUi<ReferencedGitContent, CurrentGitContent, GitDiff> for GitSource {
	fn from_kwarg_json() -> Result<Self, SourceUiError> {
		Ok(Self::from_kwarg_json(kwargs))
	}
}
