use crate::Citation;
use cite_core::{CitationAnnotation, CitationBehavior};

/// Check if the citation meets annotation requirements based on feature flags
pub fn check_annotation_requirements(
	citation: &Citation,
	behavior: &CitationBehavior,
) -> Result<(), String> {
	let annotation = match &citation.annotation {
		Some(annotation) => Some(CitationAnnotation::from_str(annotation)?),
		None => None,
	};

	if behavior.requires_effective_annotation(annotation) && citation.reason.is_none() {
		return Err(format!(
			"Citation requires documentation (global {:?} annotation is default and local {:?} annotation is provided) but no reason provided. \
			Add a 'reason = \"...\"' attribute or enable the annotationless feature\
			Evaluated requires effective {:?} annotation and {:?} annotation is provided",
			behavior,
			annotation,
			behavior.requires_effective_annotation(annotation),
			citation.reason.is_none(),
		));
	}
	Ok(())
}
