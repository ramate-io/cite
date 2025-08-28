use crate::Citation;

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
