use crate::sources;
use crate::Citation;
use cite_core::Source;

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

	// Generate link text by constructing the source and calling its methods
	let link_text = if let Some(kwargs) = &citation.kwargs {
		if let Some(src) = citation.get_src().ok() {
			match src.as_str() {
				"git" => {
					match sources::git::try_get_git_source_from_kwargs(kwargs) {
						Ok(git_source) => {
							let name = git_source.name();
							let link = git_source.link();
							Some(format!("[{}]({})", name, link))
						}
						Err(_) => None, // If construction fails, skip link generation
					}
				}
				"http" => {
					match sources::http::try_get_http_source_from_kwargs(kwargs) {
						Ok(http_source) => {
							let name = http_source.name();
							let link = http_source.link();
							Some(format!("[{}]({})", name, link))
						}
						Err(_) => None, // If construction fails, skip link generation
					}
				}
				"mock" => {
					match sources::mock::try_get_mock_source_from_kwargs(kwargs) {
						Ok(mock_source) => {
							let name = mock_source.name();
							let link = mock_source.link();
							Some(format!("[{}]({})", name, link))
						}
						Err(_) => None, // If construction fails, skip link generation
					}
				}
				_ => None, // Unknown source type
			}
		} else {
			None // No source found
		}
	} else {
		None // No kwargs available
	};

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
