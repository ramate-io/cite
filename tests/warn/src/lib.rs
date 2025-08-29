#[cfg(test)]
pub mod tests {

	use cite::cite;

	// This should warn because the source does not have a reason, but we are under the warn level feature flag.
	#[cite(mock, same = "test content")]
	pub fn test_no_annotation() {
		println!("test");
	}
}
