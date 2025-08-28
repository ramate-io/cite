use crate::Citation;
use cite_core::CitationAnnotation;

/// Check if the citation meets annotation requirements based on feature flags
pub fn check_annotation_requirements(citation: &Citation) -> Result<(), String> {
	// Determine annotation requirement based on feature flags
	#[cfg(feature = "annotation-footnote")]
	let requires_footnote = true;
	#[cfg(not(feature = "annotation-footnote"))]
	let requires_footnote = false;

	if requires_footnote && citation.reason.is_none() {
		return Err(
			"Citation requires documentation (annotation-footnote feature enabled) but no reason provided. \
			Add a 'reason = \"...\"' attribute or disable the annotation-footnote feature".to_string()
		);
	}

	Ok(())
}

/// Get the effective annotation requirement based on feature flags
pub fn get_effective_annotation() -> CitationAnnotation {
	#[cfg(feature = "annotation-footnote")]
	{
		CitationAnnotation::Footnote
	}
	#[cfg(not(feature = "annotation-footnote"))]
	{
		CitationAnnotation::Any
	}
}
