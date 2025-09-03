use syn::Result;

/// Validate kwargs and create citation
pub fn validate_with_kwargs(
	kwargs: &std::collections::HashMap<String, serde_json::Value>,
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

	// Validate source-specific parameters
	match src_str.as_str() {
		"git" => {
			// Validate git source parameters
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
		}
		"http" => {
			// Validate http source parameters
			kwargs.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"http source requires 'url = \"...\"'",
				)
			})?;
		}
		"mock" => {
			// Validate mock source parameters
			let same = kwargs.get("same").and_then(|v| v.as_str());
			let changed = kwargs.get("changed");
			if same.is_none() && changed.is_none() {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					"mock source requires 'same = \"...\"' or 'changed = (\"old\", \"new\")'",
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
