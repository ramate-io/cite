use crate::sources::{git, http, mock};
use syn::Result;

/// Validate kwargs and create citation
pub fn validate_with_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
	_args: &[syn::Expr],
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

	// Validate source-specific parameters using SourceUi trait
	match src_str.as_str() {
		"git" => {
			// Use SourceUi trait to validate and construct GitSource
			git::try_get_git_source_from_kwargs(kwargs)
				.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;
		}
		"http" => {
			// Use SourceUi trait to validate and construct HttpMatch
			http::try_get_http_source_from_kwargs(kwargs)
				.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;
		}
		"mock" => {
			// Use SourceUi trait to validate and construct MockSource
			mock::try_get_mock_source_from_kwargs(kwargs)
				.map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))?;
		}
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
