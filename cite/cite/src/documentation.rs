use crate::Citation;

/// Add citation footnote to doc comments
pub fn add_citation_footnote_to_item(
	attrs: &mut Vec<syn::Attribute>,
	citation: &Citation,
	link_text: Option<String>,
	warning_text: String,
) {
	// Check if global formatting watermark already exists
	let has_watermark = formatting::has_citation_watermark(attrs);

	// Generate the complete footnote
	let mut complete_footnote = String::new();

	// Add global formatting only if watermark doesn't exist
	if !has_watermark {
		complete_footnote.push_str(&formatting::generate_global_citation_formatting());
	}

	// Add the specific citation footnote
	complete_footnote.push_str(&footnotes::generate_citation_footnote(
		citation,
		link_text,
		warning_text,
	));

	// Remove any existing doc comments that might have contained <cite above> blocks
	attrs.retain(|attr| !attr.path().is_ident("doc"));

	// Create a new doc comment attribute
	let doc_attr = syn::parse_quote! {
		#[doc = #complete_footnote]
	};

	// Add it to the attributes
	attrs.push(doc_attr);
}

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
						expr_lit.lit = syn::Lit::Str(syn::LitStr::new(
							&new_doc_content,
							proc_macro2::Span::call_site(),
						));
					}
				}
			}
		}
	}
}

pub mod footnotes;
pub mod formatting;
pub mod sources;
