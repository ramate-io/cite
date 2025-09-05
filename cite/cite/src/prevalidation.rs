use crate::sources::{git, http, mock};
use cite_core::ui::SourceUi;
use cite_core::MockSource;
use cite_git::GitSource;
use cite_http::HttpMatch;
use syn::Result;

/// Find the span of a specific parameter in the args
fn find_param_span(args: &[syn::Expr], param_name: &str) -> proc_macro2::Span {
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let key = left_path.path.segments[0].ident.to_string();
					if key == param_name {
						return left_path.path.segments[0].ident.span();
					}
				}
			}
		}
	}
	proc_macro2::Span::call_site()
}

/// Check if a key is a top-level citation field (always valid)
fn is_citation_level_field(key: &str) -> bool {
	matches!(key, "src" | "reason" | "level" | "annotation")
}

/// Validate kwargs for git source and check for invalid attributes
fn validate_git_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
	args: &[syn::Expr],
) -> Result<()> {
	// First try to construct the source to validate required fields
	git::try_get_git_source_from_kwargs(kwargs)
		.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;

	// Then check for invalid attributes
	for key in kwargs.keys() {
		if !is_citation_level_field(key)
			&& !<GitSource as SourceUi<_, _, _>>::is_valid_attr_key(key)
		{
			return Err(syn::Error::new(
				find_param_span(args, key),
				format!("Unknown citation attribute: {}", key),
			));
		}
	}

	Ok(())
}

/// Validate kwargs for http source and check for invalid attributes
fn validate_http_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
	args: &[syn::Expr],
) -> Result<()> {
	// First try to construct the source to validate required fields
	http::try_get_http_source_from_kwargs(kwargs)
		.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;

	// Then check for invalid attributes
	for key in kwargs.keys() {
		if !is_citation_level_field(key)
			&& !<HttpMatch as SourceUi<_, _, _>>::is_valid_attr_key(key)
		{
			return Err(syn::Error::new(
				find_param_span(args, key),
				format!("Unknown citation attribute: {}", key),
			));
		}
	}

	Ok(())
}

/// Validate kwargs for mock source and check for invalid attributes
fn validate_mock_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
	args: &[syn::Expr],
) -> Result<()> {
	// First try to construct the source to validate required fields
	mock::try_get_mock_source_from_kwargs(kwargs)
		.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;

	// Then check for invalid attributes
	for key in kwargs.keys() {
		if !is_citation_level_field(key)
			&& !<MockSource as SourceUi<_, _, _>>::is_valid_attr_key(key)
		{
			return Err(syn::Error::new(
				find_param_span(args, key),
				format!("Unknown citation attribute: {}", key),
			));
		}
	}

	Ok(())
}

/// Validate kwargs and create citation
pub fn validate_with_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
	args: &[syn::Expr],
) -> Result<crate::Citation> {
	let src = kwargs.get("src").ok_or_else(|| {
		syn::Error::new(proc_macro2::Span::call_site(), "cite attribute requires a source type")
	})?;

	let src_str = match src {
		serde_json::Value::String(s) => s,
		_ => return Err(syn::Error::new(proc_macro2::Span::call_site(), "src must be a string")),
	};

	let reason = kwargs.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string());
	let level = kwargs.get("level").and_then(|v| v.as_str()).map(|s| s.to_string());
	let annotation = kwargs.get("annotation").and_then(|v| v.as_str()).map(|s| s.to_string());

	// Validate source-specific parameters using helper functions
	match src_str.as_str() {
		"git" => validate_git_kwargs(kwargs, args)?,
		"http" => validate_http_kwargs(kwargs, args)?,
		"mock" => validate_mock_kwargs(kwargs, args)?,
		_ => {
			return Err(syn::Error::new(
				proc_macro2::Span::call_site(),
				format!("unknown source type: {}", src_str),
			));
		}
	}

	// Create a simple source expression - just a unit tuple
	let source_expr = syn::parse_quote! { () };

	Ok(crate::Citation { source_expr, reason, level, annotation, kwargs: Some(kwargs.clone()) })
}
