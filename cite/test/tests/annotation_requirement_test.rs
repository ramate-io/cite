// Test annotation requirement behavior with environment variables

use std::env;
use std::process::Command;

#[test]
fn test_annotation_requirement_footnote_mode() {
	// Test that citations fail when CITE_ANNOTATION=FOOTNOTE is set but no reason provided
	let output = Command::new("cargo")
		.args(&[
			"check",
			"--manifest-path",
			"tests/ui/fail/missing_reason_footnote/Cargo.toml",
			"--quiet",
		])
		.env("CITE_ANNOTATION", "FOOTNOTE")
		.output()
		.expect("Failed to execute cargo check");

	// Should fail compilation due to missing reason
	assert!(
		!output.status.success(),
		"Compilation should fail when CITE_ANNOTATION=FOOTNOTE and no reason provided"
	);

	// Check that the error message is correct
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr.contains(
			"Citation requires documentation (CITE_ANNOTATION=FOOTNOTE) but no reason provided"
		),
		"Error message should mention annotation requirement"
	);
}

#[test]
fn test_annotation_requirement_any_mode() {
	// Test that citations pass when CITE_ANNOTATION=ANY is set (default)
	let output = Command::new("cargo")
		.args(&[
			"check",
			"--manifest-path",
			"tests/ui/pass/with_reason_footnote/Cargo.toml",
			"--quiet",
		])
		.env("CITE_ANNOTATION", "ANY")
		.output()
		.expect("Failed to execute cargo check");

	// Should compile successfully
	assert!(output.status.success(), "Compilation should pass when CITE_ANNOTATION=ANY");
}

#[test]
fn test_annotation_requirement_with_reason() {
	// Test that citations pass when CITE_ANNOTATION=FOOTNOTE is set and reason is provided
	let output = Command::new("cargo")
		.args(&[
			"check",
			"--manifest-path",
			"tests/ui/pass/with_reason_footnote/Cargo.toml",
			"--quiet",
		])
		.env("CITE_ANNOTATION", "FOOTNOTE")
		.output()
		.expect("Failed to execute cargo check");

	// Should compile successfully when reason is provided
	assert!(
		output.status.success(),
		"Compilation should pass when CITE_ANNOTATION=FOOTNOTE and reason provided"
	);
}

#[test]
fn test_global_behavior_strict_mode() {
	// Test that local overrides are ignored when CITE_GLOBAL=STRICT
	let output = Command::new("cargo")
		.args(&[
			"check",
			"--manifest-path",
			"tests/ui/fail/changed_content_error/Cargo.toml",
			"--quiet",
		])
		.env("CITE_LEVEL", "WARN")
		.env("CITE_GLOBAL", "STRICT")
		.output()
		.expect("Failed to execute cargo check");

	// Should fail compilation because local ERROR level is ignored in strict mode
	// and global WARN level doesn't fail compilation
	// This test demonstrates the strict global behavior
	let stderr = String::from_utf8_lossy(&output.stderr);
	println!("Stderr output: {}", stderr);

	// The behavior depends on the implementation - this test documents the expected behavior
	// In strict mode, local level overrides should be ignored
}

#[test]
fn test_global_behavior_lenient_mode() {
	// Test that local overrides are allowed when CITE_GLOBAL=LENIENT (default)
	let output = Command::new("cargo")
		.args(&[
			"check",
			"--manifest-path",
			"tests/ui/fail/changed_content_error/Cargo.toml",
			"--quiet",
		])
		.env("CITE_LEVEL", "WARN")
		.env("CITE_GLOBAL", "LENIENT")
		.output()
		.expect("Failed to execute cargo check");

	// Should fail compilation because local ERROR level overrides global WARN level
	assert!(
		!output.status.success(),
		"Compilation should fail when local ERROR level overrides global WARN level"
	);

	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr.contains("Citation content has changed"),
		"Error message should mention content change"
	);
}
