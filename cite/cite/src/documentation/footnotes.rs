use crate::Citation;

/// Generate citation footnote text
pub fn generate_citation_footnote(
	citation: &Citation,
	link_text: Option<String>,
	warning_text: String,
) -> String {
	let mut footnote = String::new();

	// Use provided link text or generate fallback
	let source_ref =
		if let Some(link) = link_text { link } else { "[Citation source]".to_string() };

	// Add annotation and level modifiers
	let mut modifiers = Vec::new();
	if let Some(level) = &citation.level {
		modifiers.push(format!("level={}", level.to_uppercase()));
	}
	if let Some(annotation) = &citation.annotation {
		modifiers.push(format!("annotation={}", annotation.to_uppercase()));
	}

	// Build the enumerated footnote
	footnote.push_str("\n1. ");
	footnote.push_str(&source_ref);
	if !modifiers.is_empty() {
		footnote.push_str(&format!(" [{}]", modifiers.join(", ")));
	}

	// Add reason if provided
	if let Some(reason) = &citation.reason {
		// Handle multiline reasons by splitting and prefixing each line with tab
		let formatted_reason =
			reason.lines().map(|line| format!("\t{}", line)).collect::<Vec<_>>().join("\n");

		footnote.push_str(&format!("\n\n{}", formatted_reason));
	}

	if !warning_text.is_empty() {
		// Handle multiline warning text by splitting and prefixing each line with tab
		let formatted_warning = warning_text
			.lines()
			.map(|line| format!("\t>{}", line))
			.collect::<Vec<_>>()
			.join("\n");

		// warning box for warning text
		footnote.push_str(&format!("\n\n\t**Warning!**\n\n{}", formatted_warning));
	}

	footnote
}
