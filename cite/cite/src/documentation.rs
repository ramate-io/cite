use crate::Citation;

/// Add citation footnote to doc comments
pub fn add_citation_footnote_to_item(
	attrs: &mut Vec<syn::Attribute>,
	citation: &Citation,
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

	// Generate link text from kwargs - simplified for now
	let link_text = None;

	// Add the specific citation footnote
	complete_footnote.push_str(&footnotes::generate_citation_footnote(
		citation,
		link_text,
		warning_text,
	));

	// Create a new doc comment attribute
	let doc_attr = syn::parse_quote! {
		#[doc = #complete_footnote]
	};

	// Add it to the attributes
	attrs.push(doc_attr);
}

pub mod footnotes;
pub mod formatting;
