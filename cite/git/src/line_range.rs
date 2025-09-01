use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::GitSourceError;

/// Line range specification for file content extraction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineRange {
	pub start: usize,
	pub end: usize,
}

impl LineRange {
	const LINE_RANGE_PATTERN: &str = r"^L(\d+)(-L(\d+))?$";

	pub fn new(start: usize, end: usize) -> Result<Self, GitSourceError> {
		if start > end || start == 0 {
			return Err(GitSourceError::InvalidPathPattern(format!(
				"Invalid line range: {}:{}",
				start, end
			)));
		}
		Ok(Self { start, end })
	}

	pub fn try_from_string(range_str: &str) -> Result<Self, GitSourceError> {
		// match to the pattern and get the capture groups
		let re = Regex::new(Self::LINE_RANGE_PATTERN).map_err(|_| {
			GitSourceError::InvalidPathPattern(format!(
				"Invalid line range: invalid regex pattern: {}",
				Self::LINE_RANGE_PATTERN
			))
		})?;
		let caps = re.captures(range_str).ok_or_else(|| {
			GitSourceError::InvalidPathPattern(format!(
				"Invalid line range: no captures:{}",
				range_str
			))
		})?;

		// get the first capture group (start line number)
		let start_capture_string = caps
			.get(1)
			.ok_or_else(|| {
				GitSourceError::InvalidPathPattern(format!(
					"Invalid line range: no start capture: {}",
					range_str
				))
			})?
			.as_str();
		let start = start_capture_string.parse::<usize>().map_err(|_| {
			GitSourceError::InvalidPathPattern(format!(
				"Invalid line range: cannot parse start: {}",
				range_str
			))
		})?;

		// Check if we have a range (two numbers) or just a single line
		// For "L5", caps.len() == 2 (Group 0: "L5", Group 1: "5")
		// For "L1-L10", caps.len() == 4 (Group 0: "L1-L10", Group 1: "1", Group 2: "-L10", Group 3: "10")
		if caps.len() <= 2 {
			// Single line: L5
			return Self::new(start, start);
		}

		// Range: L1-L10
		let end_capture_string = caps
			.get(3)
			.ok_or_else(|| {
				GitSourceError::InvalidPathPattern(format!(
					"Invalid line range: no end capture: {}",
					range_str
				))
			})?
			.as_str();
		let end = end_capture_string.parse::<usize>().map_err(|_| {
			GitSourceError::InvalidPathPattern(format!(
				"Invalid line range: cannot parse end: {}",
				range_str
			))
		})?;

		Self::new(start, end)
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::*;

	#[test]
	fn test_line_range_parsing() -> Result<(), anyhow::Error> {
		// Test range format
		let range = LineRange::try_from_string("L1-L10")?;
		assert_eq!(range.start, 1);
		assert_eq!(range.end, 10);

		// Test single line format
		let range = LineRange::try_from_string("L5")?;
		assert_eq!(range.start, 5);
		assert_eq!(range.end, 5);

		// Test single digit numbers
		let range = LineRange::try_from_string("L1-L9")?;
		assert_eq!(range.start, 1);
		assert_eq!(range.end, 9);

		// Test large numbers
		let range = LineRange::try_from_string("L100-L999")?;
		assert_eq!(range.start, 100);
		assert_eq!(range.end, 999);

		// Test same start and end
		let range = LineRange::try_from_string("L42-L42")?;
		assert_eq!(range.start, 42);
		assert_eq!(range.end, 42);

		Ok(())
	}

	#[test]
	fn test_line_range_invalid_formats() {
		// Test missing L prefix
		assert!(LineRange::try_from_string("1-10").is_err());
		assert!(LineRange::try_from_string("5").is_err());

		// Test malformed ranges
		assert!(LineRange::try_from_string("L1-").is_err());
		assert!(LineRange::try_from_string("L-10").is_err());
		assert!(LineRange::try_from_string("L1-L").is_err());
		assert!(LineRange::try_from_string("L-L10").is_err());

		// Test invalid characters
		assert!(LineRange::try_from_string("L1-L10a").is_err());
		assert!(LineRange::try_from_string("La-L10").is_err());
		assert!(LineRange::try_from_string("L1-L1a").is_err());

		// Test empty strings
		assert!(LineRange::try_from_string("").is_err());
		assert!(LineRange::try_from_string("L").is_err());

		// Test wrong separators
		assert!(LineRange::try_from_string("L1:L10").is_err());
		assert!(LineRange::try_from_string("L1_L10").is_err());
		assert!(LineRange::try_from_string("L1 L10").is_err());

		// Test multiple dashes
		assert!(LineRange::try_from_string("L1--L10").is_err());
		assert!(LineRange::try_from_string("L1-L-10").is_err());
	}

	#[test]
	fn test_line_range_edge_cases() {
		// Test zero line numbers (should be invalid)
		assert!(LineRange::try_from_string("L0").is_err());
		assert!(LineRange::try_from_string("L0-L5").is_err());
		assert!(LineRange::try_from_string("L5-L0").is_err());

		// Test reverse ranges (start > end)
		assert!(LineRange::try_from_string("L10-L5").is_err());

		// Test very large numbers
		assert!(LineRange::try_from_string("L999999-L1000000").is_ok());
	}

	#[test]
	fn test_line_range_validation() -> Result<(), anyhow::Error> {
		// Test that LineRange::new validates correctly
		assert!(LineRange::new(1, 10).is_ok());
		assert!(LineRange::new(5, 5).is_ok());

		// Test invalid ranges
		assert!(LineRange::new(10, 5).is_err()); // start > end
		assert!(LineRange::new(0, 5).is_err());  // start == 0
		assert!(LineRange::new(0, 0).is_err());  // start == 0

		Ok(())
	}
}
