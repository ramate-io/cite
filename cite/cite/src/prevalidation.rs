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

	// Validate source-specific parameters and check for invalid kwargs
	match src_str.as_str() {
		"git" => {
			// Define valid git source parameters
			let valid_params = [
				"src",
				"remote",
				"ref_rev",
				"cur_rev",
				"path",
				"name",
				"reason",
				"level",
				"annotation",
			];

			// Validate required git source parameters
			kwargs.get("remote").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'remote = \"...\"'",
				)
			})?;
			kwargs.get("ref_rev").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'ref_rev = \"...\"'",
				)
			})?;
			kwargs.get("cur_rev").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'cur_rev = \"...\"'",
				)
			})?;
			kwargs.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'path = \"...\"'",
				)
			})?;

			// Check for invalid kwargs
			let invalid_params: Vec<_> =
				kwargs.keys().filter(|key| !valid_params.contains(&key.as_str())).collect();

			if !invalid_params.is_empty() {
				let invalid_param = &invalid_params[0];
				return Err(syn::Error::new(
					find_param_span(args, invalid_param),
					format!("Unknown citation attribute: {}", invalid_param),
				));
			}
		}
		"http" => {
			// Define valid http source parameters
			let valid_params = [
				"src",
				"url",
				"pattern",
				"selector",
				"match_type",
				"fragment",
				"reason",
				"level",
				"annotation",
			];

			// Validate required http source parameters
			kwargs.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"http source requires 'url = \"...\"'",
				)
			})?;

			// Check for invalid kwargs
			let invalid_params: Vec<_> =
				kwargs.keys().filter(|key| !valid_params.contains(&key.as_str())).collect();

			if !invalid_params.is_empty() {
				let invalid_param = &invalid_params[0];
				return Err(syn::Error::new(
					find_param_span(args, invalid_param),
					format!("Unknown citation attribute: {}", invalid_param),
				));
			}
		}
		"mock" => {
			// Define valid mock source parameters
			let valid_params = ["src", "same", "changed", "reason", "level", "annotation"];

			// Validate mock source parameters
			let same = kwargs.get("same").and_then(|v| v.as_str());
			let changed = kwargs.get("changed");
			if same.is_none() && changed.is_none() {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					"mock source requires 'same = \"...\"' or 'changed = (\"old\", \"new\")'",
				));
			}

			// Check for invalid kwargs
			let invalid_params: Vec<_> =
				kwargs.keys().filter(|key| !valid_params.contains(&key.as_str())).collect();

			if !invalid_params.is_empty() {
				let invalid_param = &invalid_params[0];
				return Err(syn::Error::new(
					find_param_span(args, invalid_param),
					format!("Unknown citation attribute: {}", invalid_param),
				));
			}
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
