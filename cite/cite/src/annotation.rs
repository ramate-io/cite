use crate::Citation;

/// Check if the citation meets annotation requirements based on feature flags
pub fn check_annotation_requirements(citation: &Citation) -> Result<(), String> {
	#[cfg(feature = "annotationless")]
	let requires_footnote = false;
	#[cfg(not(feature = "annotationless"))]
	let requires_footnote = true;

	if requires_footnote && citation.reason.is_none() {
		return Err(
			"Citation requires documentation (footnote annotation is default) but no reason provided. \
			Add a 'reason = \"...\"' attribute or enable the annotationless feature".to_string()
		);
	}
	Ok(())
}
