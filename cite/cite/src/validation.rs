use cite_core::{CitationBehavior, CitationLevel};

/// Execute kwargs source validation and return the result
pub fn execute_kwargs_source_validation(
	_citation: &crate::Citation,
	_behavior: &CitationBehavior,
	_level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	// The kwargs have already been validated in prevalidation
	// Just return success
	Some(Ok(None))
}

/// Try to execute source expressions that we can handle during macro expansion
pub fn try_execute_source_expression(
	citation: &crate::Citation,
	behavior: &CitationBehavior,
	level_override: Option<CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	// Check if this uses kwargs syntax by looking for the unit expression
	if let syn::Expr::Tuple(tuple_expr) = &citation.source_expr {
		if tuple_expr.elems.is_empty() {
			// This is a unit expression, check if we have kwargs
			if citation.kwargs.is_some() {
				return execute_kwargs_source_validation(citation, behavior, level_override);
			}
		}
	}

	// Check if this uses keyword syntax by looking for the keyword_syntax marker
	if let syn::Expr::Path(path_expr) = &citation.source_expr {
		if path_expr.path.segments.len() == 1
			&& path_expr.path.segments[0].ident == "keyword_syntax"
		{
			// Use keyword syntax parsing - try all source types
			if let Some(args) = &citation.raw_args {
				// Try mock sources first
				if let Some(mock_source) =
					crate::mock::try_construct_mock_source_from_citation_args(args)
				{
					return execute_mock_source_validation(mock_source, behavior, level_override);
				}

				// Try HTTP sources
				if let Some(http_source) =
					crate::http::try_construct_http_source_from_citation_args(args)
				{
					return execute_http_source_validation(http_source, behavior, level_override);
				}

				// Try Git sources
				if let Some(git_source) =
					crate::git::try_construct_git_source_from_citation_args(args)
				{
					return execute_git_source_validation(git_source, behavior, level_override);
				}
			}
		}
	}

	// Check if this uses the new syntax where the source type is the first argument
	if let Some(args) = &citation.raw_args {
		if !args.is_empty() {
			// Try Git sources first (since git is the most common)
			if let Some(git_source) = crate::git::try_construct_git_source_from_citation_args(args)
			{
				return execute_git_source_validation(git_source, behavior, level_override);
			}

			// Try HTTP sources
			if let Some(http_source) =
				crate::http::try_construct_http_source_from_citation_args(args)
			{
				return execute_http_source_validation(http_source, behavior, level_override);
			}

			// Try mock sources
			if let Some(mock_source) =
				crate::mock::try_construct_mock_source_from_citation_args(args)
			{
				return execute_mock_source_validation(mock_source, behavior, level_override);
			}
		}
	}

	// Try to construct and execute MockSource using the traditional expression parsing
	if let Some(mock_source) =
		crate::mock::try_construct_mock_source_from_expr(&citation.source_expr)
	{
		return execute_mock_source_validation(mock_source, behavior, level_override);
	}

	// Add support for other source types here as needed
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
