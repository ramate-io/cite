use cite_core::CitationLevel;

pub fn level_output(output: String, level: CitationLevel) -> Result<Option<String>, String> {
	match level {
		// if level is silent, do nothing
		CitationLevel::Silent => Ok(None),
		CitationLevel::Warn => Ok(Some(output)),
		CitationLevel::Error => Err(output),
	}
}
