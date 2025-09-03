use crate::sources;
use cite_core::{CitationBehavior, CitationLevel};

/// Execute kwargs source validation and return the result
pub fn execute_kwargs_source_validation(
	citation: &crate::Citation,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	let kwargs = citation.kwargs.as_ref()?;

	match citation.get_src().ok()?.as_str() {
		"git" => {
			// Construct GitSource from kwargs
			let remote = kwargs.get("remote").and_then(|v| v.as_str())?;
			let ref_rev = kwargs.get("ref_rev").and_then(|v| v.as_str())?;
			let cur_rev = kwargs.get("cur_rev").and_then(|v| v.as_str())?;
			let path = kwargs.get("path").and_then(|v| v.as_str())?;
			let name = kwargs.get("name").and_then(|v| v.as_str());

			let git_source = cite_git::GitSource::try_new(
				remote,
				path,
				ref_rev,
				cur_rev,
				name.map(|s| s.to_string()),
			)
			.ok()?;

			return execute_git_source_validation(git_source, behavior, level_override);
		}
		"http" => {
			// Construct HttpMatch from kwargs using the utility function
			match sources::http::try_get_http_source_from_kwargs(kwargs) {
				Ok(http_source) => {
					return execute_http_source_validation(http_source, behavior, level_override)
				}
				Err(e) => return Some(Err(e)),
			}
		}
		"mock" => {
			// Construct MockSource from kwargs using the utility function
			match sources::mock::try_get_mock_source_from_kwargs(kwargs) {
				Ok(mock_source) => {
					return execute_mock_source_validation(mock_source, behavior, level_override)
				}
				Err(e) => return Some(Err(e)),
			}
		}
		_ => {
			// Unknown source type
			return None;
		}
	}
}

/// Try to execute source expressions that we can handle during macro expansion
pub fn try_execute_source_expression(
	citation: &crate::Citation,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	// All citations now use kwargs syntax
	if citation.kwargs.is_some() {
		return execute_kwargs_source_validation(citation, behavior, level_override);
	}

	// Fallback for any remaining direct expression parsing
	None
}

/// Execute mock source validation and return the result
fn execute_mock_source_validation(
	mock_source: cite_core::mock::MockSource,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::Source;

	// Execute the real API!
	match mock_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = format!(
					"Citation content has changed!\n         Referenced: {}\n         Current: {}",
					comparison.referenced().0,
					comparison.current().0
				);

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("Citation source error: {:?}", e))),
	}
}

/// Execute HTTP source validation and return the result
fn execute_http_source_validation(
	http_source: cite_http::HttpMatch,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::Source;

	// HTTP sources now handle caching internally
	match http_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = if let Some(unified_diff) = comparison.diff().unified_diff() {
					format!(
						"HTTP citation content has changed!\n         URL: {}\n{}",
						comparison.current().source_url.as_str(),
						unified_diff
					)
				} else {
					format!(
                        "HTTP citation content has changed!\n         URL: {}\n         Current: {}\n         Referenced: {}",
                        comparison.current().source_url.as_str(),
                        comparison.current().content,
                        comparison.referenced().content
                    )
				};

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("HTTP citation source error: {:?}", e))),
	}
}

/// Execute Git source validation and return the result
fn execute_git_source_validation(
	git_source: cite_git::GitSource,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::Source;

	// Git sources handle git operations internally
	match git_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = if let Some(unified_diff) = comparison.diff().unified_diff() {
					format!(
						"Git citation content has changed!\n         Remote: {}\n         Path: {}\n         Revision: {}\n{}",
						comparison.current().remote,
						comparison.current().path_pattern.path,
						comparison.current().revision,
						unified_diff
					)
				} else {
					format!(
						"Git citation content has changed!\n         Remote: {}\n         Path: {}\n         Revision: {}",
						comparison.current().remote,
						comparison.current().path_pattern.path,
						comparison.current().revision
					)
				};

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("Git citation source error: {:?}", e))),
	}
}
