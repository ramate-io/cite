/// Remove the <cite above> content from a doc comment while keeping the rest
pub fn remove_cite_above_from_doc_comment(attr: &mut syn::Attribute, doc_content: &str) {
	let start_tag = "<cite above>";
	let end_tag = "</cite above>";

	if let Some(start_pos) = doc_content.find(start_tag) {
		if let Some(end_pos) = doc_content.find(end_tag) {
			// Create new doc content without the <cite above> section
			let before_cite = doc_content[..start_pos].trim();
			let after_cite = doc_content[end_pos + end_tag.len()..].trim();
			
			let new_doc_content = if before_cite.is_empty() && after_cite.is_empty() {
				"".to_string()
			} else if before_cite.is_empty() {
				after_cite.to_string()
			} else if after_cite.is_empty() {
				before_cite.to_string()
			} else {
				format!("{}\n\n{}", before_cite, after_cite)
			};

			// Update the attribute with the new content
			if let syn::Meta::NameValue(meta_name_value) = &mut attr.meta {
				if let syn::Expr::Lit(expr_lit) = &mut meta_name_value.value {
					if let syn::Lit::Str(_) = &mut expr_lit.lit {
						// Create a new LitStr with the updated content
						expr_lit.lit = syn::Lit::Str(syn::LitStr::new(&new_doc_content, proc_macro2::Span::call_site()));
					}
				}
			}
		}
	}
}
