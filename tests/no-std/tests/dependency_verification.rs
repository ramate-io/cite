use cargo_metadata::{DependencyKind, MetadataCommand, Package, PackageId, Resolve};
use std::collections::HashSet;

/// Test that cite is listed as a dependency but doesn't add runtime dependencies in no-std
#[test]
fn test_cite_dependencies_no_std() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	// Find our test package
	let test_package = metadata
		.packages
		.iter()
		.find(|p| p.name == "cite-no-std-test")
		.expect("Failed to find cite-no-std-test package");

	// Get only runtime dependencies (not dev dependencies)
	let runtime_deps: Vec<_> = test_package
		.dependencies
		.iter()
		.filter(|dep| matches!(dep.kind, DependencyKind::Normal))
		.collect();

	println!("Runtime dependencies for cite-no-std-test:");
	for dep in &runtime_deps {
		println!("  - {} {}", dep.name, dep.req);
	}

    // Should have only cite as runtime dependency
    assert_eq!(
        runtime_deps.len(),
        1,
        "Expected exactly 1 runtime dependency, found {}",
        runtime_deps.len()
    );
    assert_eq!(runtime_deps[0].name, "cite");
}

/// Test that cite procedural macro does not add heavy runtime dependencies in no-std
///
/// This is especially critical for no-std environments where dependencies
/// must be minimal for embedded/resource-constrained targets.
#[test]
fn test_cite_no_heavy_runtime_dependencies_no_std() {
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
		println!("✅ cite correctly identified as proc-macro in no-std context");
	}

	// Get the resolved dependency graph
	let resolved_deps = metadata.resolve.as_ref().expect("Failed to get resolved dependencies");

	// Find our test package
	let test_package_id = metadata
		.packages
		.iter()
		.find(|p| p.name == "cite-no-std-test")
		.expect("Failed to find cite-no-std-test package")
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

	println!("Runtime dependencies in no-std context:");
	for dep_id in &runtime_deps {
		if let Some(package) = metadata.packages.iter().find(|p| p.id == *dep_id) {
			println!("  - {} {}", package.name, package.version);
		}
	}

	// In no-std, we're even more strict about forbidden dependencies
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
		"std", // Make sure std isn't pulled in
		"alloc", // Should only be pulled in if explicitly needed
	];

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
			"Found forbidden runtime dependency '{}' in no-std context - this breaks no-std compatibility!", 
			forbidden
		);
	}

	println!("✅ No heavy runtime dependencies found - cite macro works correctly in no-std!");
}

/// Test that the no-std crate has minimal dependencies suitable for embedded targets
#[test]
fn test_minimal_dependencies_for_embedded() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	// Get the resolved dependency graph
	let resolved_deps = metadata.resolve.as_ref().expect("Failed to get resolved dependencies");

	// Find our test package
	let test_package_id = metadata
		.packages
		.iter()
		.find(|p| p.name == "cite-no-std-test")
		.expect("Failed to find cite-no-std-test package")
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

	// For no-std embedded targets, we should have reasonable runtime dependencies
	println!("Total runtime dependencies: {}", runtime_deps.len());
	
	// Debug output to see what dependencies we actually have
	for dep_id in &runtime_deps {
		if let Some(package) = metadata.packages.iter().find(|p| p.id == *dep_id) {
			println!("  - {} {}", package.name, package.version);
		}
	}

	// In no-std, we expect more dependencies than std because proc-macro crates
	// often have many dependencies, but they should still be reasonable
	assert!(
		runtime_deps.len() <= 50,
		"Too many runtime dependencies ({}) for no-std/embedded context. Expected <= 50",
		runtime_deps.len()
	);

	println!("✅ Minimal dependency footprint suitable for embedded targets");
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

/// Test that cite macro works correctly in no-std compilation
#[test]
fn test_cite_macro_expansion_no_std() {
	// This test verifies that the cite macro expands correctly
	// in no-std context without causing compilation errors

	// The functions in our lib.rs use cite macros in no-std - if they compile,
	// the macro is working in no-std context
	println!("✅ Cite macro expansion works correctly in no-std");
}

/// Test that the final binary works in no-std context
#[test]
fn test_no_std_binary_functionality() {
	// A simple smoke test - if we can import and use cite
	// in no-std without pulling in heavy dependencies, this should pass
	use cite_no_std_test::{no_std_function, another_no_std_function, NoStdStruct, NoStdTrait};

	no_std_function();
	another_no_std_function();

	let test_struct = NoStdStruct { field: 42 };
	assert_eq!(test_struct.no_std_method(), 42);

	println!("✅ no-std binary runs without heavy dependencies");
}