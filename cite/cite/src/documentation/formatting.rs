/// Check if the global citation formatting watermark already exists in the attributes
pub fn has_citation_watermark(attrs: &[syn::Attribute]) -> bool {
	for attr in attrs {
		if let syn::Meta::NameValue(name_value) = &attr.meta {
			if name_value.path.is_ident("doc") {
				if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) =
					&name_value.value
				{
					let doc_content = lit_str.value();
					// Check for the citation watermark
					if doc_content.contains(
						"Cited with <a href=\"https://github.com/ramate-io/cite\">cite</a>",
					) {
						return true;
					}
				}
			}
		}
	}
	false
}

/// Generate the global citation formatting (badge and behavior hint)
pub fn generate_global_citation_formatting() -> String {
	let mut global_formatting = String::new();

	// Add citation badge linking to the cite repo
	global_formatting.push_str("## References\n\n");
	global_formatting.push_str(
		"\n\n<div style=\"background-color:#E6E6FA; border-left:4px solid #9370DB; padding:8px; font-weight:bold;\">\
	Cited with <a href=\"https://github.com/ramate-io/cite\">cite</a>.\
	</div>\n\n"
	);

	// Add behavior hint boxes
	let behavior = cite_core::CitationBehavior::from_features();

	// Global behavior
	match behavior.global {
		cite_core::CitationGlobal::Strict => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	This code uses strict citation validation.\
	</div>\n\n",
			);
		}
		cite_core::CitationGlobal::Lenient => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	This code uses lenient citation validation.\
	</div>\n\n",
			);
		}
	}

	// Annotation behavior
	match behavior.annotation {
		cite_core::CitationAnnotation::Footnote => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	Annotations are required for citations.\
	</div>\n\n",
			);
		}
		cite_core::CitationAnnotation::Any => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	Annotations are optional for citations.\
	</div>\n\n",
			);
		}
	}

	// Level behavior
	match behavior.level {
		cite_core::CitationLevel::Error => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	Citation validation errors will fail compilation.\
	</div>\n\n",
			);
		}
		cite_core::CitationLevel::Warn => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	Citation validation issues will generate warnings.\
	</div>\n\n",
			);
		}
		cite_core::CitationLevel::Silent => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFEBEE; border-left:4px solid #F44336; padding:8px;\">\
	Citation validation issues will be silently ignored.\
	</div>\n\n",
			);
		}
	}

	global_formatting
}
