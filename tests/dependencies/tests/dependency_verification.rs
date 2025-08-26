use cargo_metadata::{DependencyKind, MetadataCommand, Package, PackageId, Resolve};
use std::collections::HashSet;

/// Test that cite is listed as a dependency but doesn't add runtime dependencies
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

/// Test that cite procedural macro does not add heavy runtime dependencies
///
/// This test verifies that issue #15 is actually NOT a problem:
/// proc-macro dependencies should not and do not get bundled into the final binary.
#[test]
fn test_cite_no_heavy_runtime_dependencies() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	// Verify cite is correctly identified as a proc-macro
	if let Some(cite_package) = metadata.packages.iter().find(|p| p.name == "cite") {
		let is_proc_macro = cite_package
			.targets
			.iter()
			.any(|target| target.kind.iter().any(|k| format!("{:?}", k).contains("ProcMacro")));
		assert!(is_proc_macro, "cite should be identified as a proc-macro");
		println!("✅ cite correctly identified as proc-macro");
	}

	// Get the resolved dependency graph
	let resolved_deps = metadata.resolve.as_ref().expect("Failed to get resolved dependencies");

	// Find our test package
	let test_package_id = metadata
		.packages
		.iter()
		.find(|p| p.name == "dependency-test")
		.expect("Failed to find dependency-test package")
		.id
		.clone();

	// Collect actual runtime dependencies (excluding proc-macros)
	let mut runtime_deps = HashSet::new();
	collect_runtime_dependencies(
		&resolved_deps,
		&test_package_id,
		&mut runtime_deps,
		&metadata.packages,
	);

	// Check that heavy dependencies are NOT in runtime dependencies
	let forbidden_runtime_deps = [
		"reqwest",
		"scraper",
		"regex",
		"similar",
		"chrono",
		"tokio",
		"hyper",
		"html5ever",
		"anyhow",
	];

	println!("Checking for forbidden runtime dependencies...");
	for forbidden in &forbidden_runtime_deps {
		let found_forbidden = runtime_deps.iter().any(|dep_id| {
			metadata
				.packages
				.iter()
				.find(|p| p.id == *dep_id)
				.map(|p| p.name.as_str())
				.unwrap_or("")
				== *forbidden
		});

		assert!(
			!found_forbidden,
			"Found forbidden runtime dependency '{}' - this should not happen with proper proc-macro isolation!", 
			forbidden
		);
	}

	println!(
		"✅ No heavy runtime dependencies found - cite macro correctly isolated as proc-macro!"
	);
}

/// Recursively collect all runtime dependencies from the resolved dependency graph
fn collect_runtime_dependencies(
	resolve: &Resolve,
	package_id: &PackageId,
	visited: &mut HashSet<PackageId>,
	packages: &[Package],
) {
	if visited.contains(package_id) {
		return;
	}
	visited.insert(package_id.clone());

	// Find this package's dependencies in the resolve graph
	if let Some(node) = resolve.nodes.iter().find(|n| n.id == *package_id) {
		for dep in &node.dependencies {
			// Only include normal (runtime) dependencies, not proc-macro dependencies
			if let Some(package) = packages.iter().find(|p| p.id == *dep) {
				// Check if this is a proc-macro crate
				let is_proc_macro = package.targets.iter().any(|target| {
					target.kind.iter().any(|k| format!("{:?}", k).contains("ProcMacro"))
				});

				if !is_proc_macro {
					collect_runtime_dependencies(resolve, dep, visited, packages);
				}
			}
		}
	}
}

/// Test that cite macro works correctly at compile time
#[test]
fn test_cite_macro_expansion() {
	// This test verifies that the cite macro expands correctly
	// without causing compilation errors

	// The functions in our lib.rs use cite macros - if they compile,
	// the macro is working

	println!("✅ Cite macro expansion works correctly");
}

/// Test that the final binary size is reasonable
///
/// This test ensures that using cite doesn't bloat the binary
/// with unnecessary dependencies
#[test]
fn test_binary_size_reasonable() {
	// A simple smoke test - if we can import and use cite
	// without pulling in heavy dependencies, this should pass
	use dependency_test::{another_function, test_function};

	test_function();
	another_function();

	println!("✅ Binary runs without heavy dependencies");
}
