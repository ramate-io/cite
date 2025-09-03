use syn::Result;

/// Parse doc comment into key-value map and remove the cite above content
pub fn parse_above_into_kwargs(
	item: &mut syn::Item,
) -> Result<std::collections::HashMap<String, serde_json::Value>> {
	// Look for doc comments in the item's attributes
	let attrs = match item {
		syn::Item::Fn(item_fn) => &mut item_fn.attrs,
		syn::Item::Struct(item_struct) => &mut item_struct.attrs,
		syn::Item::Enum(item_enum) => &mut item_enum.attrs,
		syn::Item::Trait(item_trait) => &mut item_trait.attrs,
		syn::Item::Impl(item_impl) => &mut item_impl.attrs,
		syn::Item::Mod(item_mod) => &mut item_mod.attrs,
		_ => return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"cite above can only be used on functions, structs, enums, traits, impl blocks, or modules",
		)),
	};

	// Find the doc attribute that contains <cite above> and extract content
	for attr in attrs {
		if attr.path().is_ident("doc") {
			if let syn::Meta::NameValue(meta_name_value) = &attr.meta {
				if let syn::Expr::Lit(expr_lit) = &meta_name_value.value {
					if let syn::Lit::Str(lit_str) = &expr_lit.lit {
						let doc_content = lit_str.value();
						if doc_content.contains("<cite above>")
							&& doc_content.contains("</cite above>")
						{
							// Extract the content between the tags
							if let Some(cite_content) = extract_cite_content(&doc_content) {
								// Remove the <cite above> content from the doc comment
								crate::documentation::remove_cite_above_from_doc_comment(
									attr,
									&doc_content,
								);

								// Parse the JSON content
								return parse_json_content_to_kwargs(&cite_content);
							}
						}
					}
				}
			}
		}
	}

	Err(syn::Error::new(
		proc_macro2::Span::call_site(),
		"no <cite above> block found in doc comments",
	))
}

/// Extract content between <cite above> and </cite above> tags and remove the tags and content
fn extract_cite_content(doc_content: &str) -> Option<String> {
	let start_tag = "<cite above>";
	let end_tag = "</cite above>";

	if let Some(start_pos) = doc_content.find(start_tag) {
		if let Some(end_pos) = doc_content.find(end_tag) {
			let start = start_pos + start_tag.len();
			if start < end_pos {
				return Some(doc_content[start..end_pos].trim().to_string());
			}
		}
	}
	None
}

/// Parse JSON content into key-value map
fn parse_json_content_to_kwargs(
	cite_content: &str,
) -> Result<std::collections::HashMap<String, serde_json::Value>> {
	// Parse the JSON content
	let json_value: serde_json::Value = serde_json::from_str(cite_content).map_err(|e| {
		syn::Error::new(
			proc_macro2::Span::call_site(),
			format!("invalid JSON in <cite above> block: {}", e),
		)
	})?;

	// Convert JSON object to HashMap<String, serde_json::Value>
	let mut kwargs = std::collections::HashMap::new();

	if let serde_json::Value::Object(obj) = json_value {
		for (key, value) in obj {
			kwargs.insert(key, value);
		}
	} else {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"<cite above> JSON must be an object",
		));
	}

	// Ensure src field is present
	if !kwargs.contains_key("src") {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"<cite above> JSON must contain a 'src' field",
		));
	}

	Ok(kwargs)
}
