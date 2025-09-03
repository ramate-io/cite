use crate::Citation;

/// Generate source reference with hyperlink where applicable
pub fn generate_source_reference(citation: &Citation) -> Result<String, String> {
	// Try to extract source information from the citation

	match citation.get_src()?.as_str() {
		"git" => {
			try_construct_git_source_kwargs(citation.kwargs.as_ref())?;

			return Ok(generate_git_source_reference(citation));
		}
		"http" => {
			try_construct_http_source_kwargs(citation.kwargs.as_ref())?;

			return Ok(generate_http_source_reference(citation));
		}
		"mock" => {
			try_construct_mock_source_kwargs(citation.kwargs.as_ref())?;

			return Ok(generate_mock_source_reference(citation));
		}
		_ => {
			return Err("Unknown source".to_string());
		}
	}
}

/// Generate git source reference with hyperlink from macro arguments (fallback)
fn generate_git_source_reference_from_args_fallback(args: &[syn::Expr]) -> String {
	let mut remote = None;
	let mut path = None;
	let mut referenced_revision = None;
	let mut name = None;

	// Extract git source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let param_name = &left_path.path.segments[0].ident.to_string();

					match param_name.as_str() {
						"remote" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								remote = Some(lit_str.value());
							}
						}
						"path" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								path = Some(lit_str.value());
							}
						}
						"referenced_revision" | "ref_rev" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								referenced_revision = Some(lit_str.value());
							}
						}
						"name" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								name = Some(lit_str.value());
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let (Some(remote_url), Some(file_path), Some(rev)) = (remote, path, referenced_revision) {
		// Use the provided name as link text if available, otherwise use default format
		let link_text = if let Some(custom_name) = name {
			custom_name
		} else {
			let short_rev = if rev.len() > 8 { &rev[..8] } else { &rev };
			format!("Git: {} @ {}", file_path, short_rev)
		};

		// Try to create a hyperlink for GitHub URLs
		if remote_url.contains("github.com") {
			// Extract owner/repo from GitHub URL
			if let Some(_repo_part) = remote_url.split("github.com/").nth(1) {
				return format!(
					"[{}]({}/blob/{}/{}#L1)",
					link_text,
					remote_url.trim_end_matches(".git"),
					rev,
					file_path
				);
			}
		}

		// Fallback for non-GitHub URLs
		format!("{} ({})", link_text, remote_url)
	} else {
		"Git source (incomplete parameters)".to_string()
	}
}

/// Generate git source reference with hyperlink using GitSource
fn generate_git_source_reference(git_source: &cite_git::GitSource) -> String {
	use cite_core::Source;

	let name = git_source.name();
	let url = git_source.link();

	// Format as [name](url)
	format!("**[{}]({})**", name, url)
}

/// Generate HTTP source reference with hyperlink from source object
fn generate_http_source_reference_from_source(http_source: &cite_http::HttpMatch) -> String {
	use cite_core::Source;

	let url = http_source.source_url.as_str();
	let link_text = http_source.link();

	format!("**[{}]({})**", link_text, url)
}

/// Generate HTTP source reference with hyperlink from macro arguments
fn generate_http_source_reference(args: &[syn::Expr]) -> String {
	let mut url = None;
	let mut pattern = None;
	let mut selector = None;

	// Extract HTTP source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let name = &left_path.path.segments[0].ident.to_string();

					match name.as_str() {
						"url" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								url = Some(lit_str.value());
							}
						}
						"pattern" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								pattern = Some(lit_str.value());
							}
						}
						"selector" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								selector = Some(lit_str.value());
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let Some(url_str) = url {
		let extraction_method = if pattern.is_some() {
			"regex pattern"
		} else if selector.is_some() {
			"CSS selector"
		} else {
			"full content"
		};

		return format!("[HTTP: {}]({})", extraction_method, url_str);
	} else {
		"HTTP source (incomplete parameters)".to_string()
	}
}

/// Generate mock source reference from source object
fn generate_mock_source_reference_from_source(mock_source: &cite_core::mock::MockSource) -> String {
	use cite_core::Source;

	let link_text = mock_source.link();
	format!("[{}](https://github.com/ramate-io/cite#mock-sources)", link_text)
}

/// Generate mock source reference from macro arguments
fn generate_mock_source_reference(args: &[syn::Expr]) -> String {
	let mut same = None;
	let mut changed = None;

	// Extract mock source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let name = &left_path.path.segments[0].ident.to_string();

					match name.as_str() {
						"same" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								same = Some(lit_str.value());
							}
						}
						"changed" => {
							// Handle tuple syntax for changed
							if let syn::Expr::Tuple(tuple_expr) = &*assign_expr.right {
								if tuple_expr.elems.len() == 2 {
									if let (Some(old), Some(new)) =
										(tuple_expr.elems.first(), tuple_expr.elems.get(1))
									{
										if let (
											syn::Expr::Lit(syn::ExprLit {
												lit: syn::Lit::Str(old_lit),
												..
											}),
											syn::Expr::Lit(syn::ExprLit {
												lit: syn::Lit::Str(new_lit),
												..
											}),
										) = (old, new)
										{
											changed = Some((old_lit.value(), new_lit.value()));
										}
									}
								}
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let Some(content) = same {
		let preview = if content.len() > 50 { format!("{}...", &content[..50]) } else { content };
		format!("[Mock: same = \"{}\"](https://github.com/ramate-io/cite#mock-sources)", preview)
	} else if let Some((old, new)) = changed {
		let old_preview = if old.len() > 30 { format!("{}...", &old[..30]) } else { old };
		let new_preview = if new.len() > 30 { format!("{}...", &new[..30]) } else { new };
		format!(
			"[Mock: changed = (\"{}\", \"{}\")](https://github.com/ramate-io/cite#mock-sources)",
			old_preview, new_preview
		)
	} else {
		"[Mock source (incomplete parameters)](https://github.com/ramate-io/cite#mock-sources)"
			.to_string()
	}
}

/// Construct a source from citation arguments and return its link text
pub fn construct_source_from_citation(citation: &Citation) -> Option<String> {
	if let Some(args) = &citation.raw_args {
		if !args.is_empty() {
			// Try Git sources first (since git is the most common)
			if let Some(git_source) = crate::git::try_construct_git_source_from_citation_args(args)
			{
				return Some(generate_git_source_reference(&git_source));
			}

			// Try HTTP sources
			if let Some(http_source) =
				crate::http::try_construct_http_source_from_citation_args(args)
			{
				return Some(generate_http_source_reference_from_source(&http_source));
			}

			// Try mock sources
			if let Some(mock_source) =
				crate::mock::try_construct_mock_source_from_citation_args(args)
			{
				return Some(generate_mock_source_reference_from_source(&mock_source));
			}
		}
	}

	// Try to construct and execute MockSource using the traditional expression parsing
	if let Some(mock_source) =
		crate::mock::try_construct_mock_source_from_expr(&citation.source_expr)
	{
		return Some(generate_mock_source_reference_from_source(&mock_source));
	}

	None
}
