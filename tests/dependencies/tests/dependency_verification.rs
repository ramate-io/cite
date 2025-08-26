use cargo_metadata::{DependencyKind, MetadataCommand};
use std::process::Command;

/// Test that cite has minimal dependencies when used
#[test]
fn test_cite_dependencies() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	// Find our test package
	let test_package = metadata
		.packages
		.iter()
		.find(|p| p.name == "dependency-test")
		.expect("Failed to find dependency-test package");

	// Get only runtime dependencies (not dev dependencies)
	let runtime_deps: Vec<_> = test_package
		.dependencies
		.iter()
		.filter(|dep| matches!(dep.kind, DependencyKind::Normal))
		.collect();

	println!("Runtime dependencies for dependency-test:");
	for dep in &runtime_deps {
		println!("  - {} {}", dep.name, dep.req);
	}

	// Should only have cite as runtime dependency
	assert_eq!(
		runtime_deps.len(),
		1,
		"Expected exactly 1 runtime dependency, found {}",
		runtime_deps.len()
	);
	assert_eq!(runtime_deps[0].name, "cite");
}

/// Test that demonstrates the current dependency issue
///
/// This test currently FAILS because the cite procedural macro
/// pulls in heavy dependencies at runtime. This is the core issue
/// that needs to be resolved as described in issue #15.
///
/// TODO: Once issue #15 is resolved, this test should pass.
#[test]
#[should_panic(expected = "Found forbidden runtime dependency")]
fn test_cite_currently_has_heavy_dependencies() {
	let output = Command::new("cargo")
		.args(["tree", "--format", "{p}"])
		.current_dir(".")
		.output()
		.expect("Failed to run cargo tree");

	let tree_output = String::from_utf8(output.stdout).expect("Invalid UTF-8 in cargo tree output");
	println!("dependency-test dependency tree:\n{}", tree_output);

	// Heavy dependencies that currently DO appear but SHOULD NOT
	// This demonstrates the problem described in issue #15
	let forbidden_runtime_deps =
		["reqwest", "scraper", "regex", "similar", "chrono", "tokio", "hyper", "html5ever"];

	for forbidden in &forbidden_runtime_deps {
		if tree_output.contains(forbidden) {
			panic!(
                "Found forbidden runtime dependency '{}' - this demonstrates issue #15: cite should not add heavy dependencies at runtime", 
                forbidden
            );
		}
	}

	println!("✅ No heavy runtime dependencies found (this should not print if test is working correctly)");
}

/// Test that will verify the fix for issue #15
///
/// This test should be enabled once issue #15 is resolved.
/// It verifies that cite macro doesn't add heavy runtime dependencies.
#[test]
#[ignore = "Enable once issue #15 is resolved"]
fn test_cite_has_no_heavy_dependencies_future() {
	let output = Command::new("cargo")
		.args(["tree", "--format", "{p}"])
		.current_dir(".")
		.output()
		.expect("Failed to run cargo tree");

	let tree_output = String::from_utf8(output.stdout).expect("Invalid UTF-8 in cargo tree output");
	println!("dependency-test dependency tree:\n{}", tree_output);

	// Heavy dependencies that should NOT appear at runtime
	let forbidden_runtime_deps =
		["reqwest", "scraper", "regex", "similar", "chrono", "tokio", "hyper", "html5ever"];

	for forbidden in &forbidden_runtime_deps {
		assert!(
            !tree_output.contains(forbidden),
            "Found forbidden runtime dependency '{}' - cite should not add heavy dependencies at runtime", 
            forbidden
        );
	}

	println!("✅ No heavy runtime dependencies found - issue #15 has been resolved!");
}

/// Test that the cite macro works at compile time
#[test]
fn test_cite_compilation() {
	let output = Command::new("cargo")
		.args(["check"])
		.current_dir(".")
		.output()
		.expect("Failed to run cargo check");

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let stdout = String::from_utf8_lossy(&output.stdout);
		panic!("Failed to compile with cite macros:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
	}

	println!("✅ Successfully compiled with cite macros");
}

/// Test that the code with cite macros runs correctly
#[test]
fn test_cite_runtime() {
	let output = Command::new("cargo")
		.args(["test", "--lib"])
		.current_dir(".")
		.output()
		.expect("Failed to run cargo test");

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let stdout = String::from_utf8_lossy(&output.stdout);
		panic!("Failed to run tests with cite:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
	}

	println!("✅ Tests with cite macros pass");
}
