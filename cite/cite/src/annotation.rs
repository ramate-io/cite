use crate::{level::level_output, Citation};
use cite_core::{CitationAnnotation, CitationBehavior, CitationLevel};

/// Check if the citation meets annotation requirements based on feature flags
pub fn check_annotation_requirements(
	citation: &Citation,
	behavior: &CitationBehavior,
) -> Result<Option<String>, String> {
	let annotation = match &citation.annotation {
		Some(annotation) => Some(CitationAnnotation::from_str(annotation)?),
		None => None,
	};

	if behavior.requires_effective_annotation(annotation) && citation.reason.is_none() {
		// Citation level is optional, so we need to handle the case where it is not provided
		let citation_level = match &citation.level {
			Some(level) => Some(CitationLevel::from_str(level)?),
			None => None,
		};

		// Comput the effective level of the citation
		let effective_level = behavior.effective_level(citation_level);

		// Output the error message
		println!("effective_level: {:?}", effective_level);
		level_output(
			"Citation requires documentation but no annotation provided. \
			Add a 'reason = \"...\"' attribute or enable the annotationless feature"
				.to_string(),
			effective_level,
		)
	} else {
		Ok(None)
	}
}
