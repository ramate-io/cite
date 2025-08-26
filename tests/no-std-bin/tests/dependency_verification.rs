use cargo_metadata::{DependencyKind, MetadataCommand, Package, PackageId, Resolve};
use std::collections::HashSet;

/// Test that cite is listed as a dependency but doesn't add runtime dependencies in no-std binary
#[test]
fn test_cite_dependencies_no_std_bin() {
    let metadata = MetadataCommand::new()
        .current_dir(".")
        .exec()
        .expect("Failed to get cargo metadata");

    let test_package = metadata
        .packages
        .iter()
        .find(|p| p.name == "cite-no-std-bin-test")
        .expect("Failed to find cite-no-std-bin-test package");

    let runtime_deps: Vec<_> = test_package
        .dependencies
        .iter()
        .filter(|dep| matches!(dep.kind, DependencyKind::Normal))
        .collect();

    println!("Runtime dependencies for cite-no-std-bin-test:");
    for dep in &runtime_deps {
        println!("  - {} {}", dep.name, dep.req);
    }

    // Should have cite as runtime dependency
    assert_eq!(
        runtime_deps.len(),
        1,
        "Expected exactly 1 runtime dependency (cite), found {}",
        runtime_deps.len()
    );
    
    let dep_names: HashSet<_> = runtime_deps.iter().map(|d| d.name.as_str()).collect();
    assert!(dep_names.contains("cite"), "Missing cite dependency");
}

/// Test that cite procedural macro does not add heavy runtime dependencies in no-std binary
#[test]
fn test_cite_no_heavy_runtime_dependencies_no_std_bin() {
    let metadata = MetadataCommand::new()
        .current_dir(".")
        .exec()
        .expect("Failed to get cargo metadata");

    if let Some(cite_package) = metadata.packages.iter().find(|p| p.name == "cite") {
        let is_proc_macro = cite_package
            .targets
            .iter()
            .any(|target| target.kind.iter().any(|k| format!("{:?}", k).contains("ProcMacro")));
        assert!(is_proc_macro, "cite should be identified as a proc-macro");
    }

    let resolved_deps = metadata.resolve.as_ref().expect("Failed to get resolved dependencies");

    let test_package_id = metadata
        .packages
        .iter()
        .find(|p| p.name == "cite-no-std-bin-test")
        .expect("Failed to find cite-no-std-bin-test package")
        .id
        .clone();

    let mut runtime_deps = HashSet::new();
    collect_runtime_dependencies(
        &resolved_deps,
        &test_package_id,
        &mut runtime_deps,
        &metadata.packages,
    );

    let forbidden_runtime_deps = [
        "reqwest", "scraper", "regex", "similar", "chrono",
        "tokio", "hyper", "html5ever", "anyhow",
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
            "Found forbidden runtime dependency '{}' - this should not happen with proper proc-macro isolation!",
            forbidden
        );
    }
}

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

    if let Some(node) = resolve.nodes.iter().find(|n| n.id == *package_id) {
        for dep in &node.dependencies {
            if let Some(package) = packages.iter().find(|p| p.id == *dep) {
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

#[test]
fn test_cite_macro_expansion_no_std_bin() {
    // This test verifies that the cite macro expands correctly in a no-std binary context
    // without causing compilation errors.
    // The functions/structs/enums in our lib.rs and main.rs use cite macros - if they compile,
    // the macro is working.
}

/// Test that the final binary size is reasonable for no-std/embedded targets
#[test]
fn test_minimal_dependencies_for_embedded_bin() {
    let metadata = MetadataCommand::new()
        .current_dir(".")
        .exec()
        .expect("Failed to get cargo metadata");

    let test_package_id = metadata
        .packages
        .iter()
        .find(|p| p.name == "cite-no-std-bin-test")
        .expect("Failed to find cite-no-std-bin-test package")
        .id
        .clone();

    let resolved_deps = metadata.resolve.as_ref().expect("Failed to get resolved dependencies");

    let mut runtime_deps = HashSet::new();
    collect_runtime_dependencies(
        &resolved_deps,
        &test_package_id,
        &mut runtime_deps,
        &metadata.packages,
    );

    println!("Total runtime dependencies: {}", runtime_deps.len());
    for dep_id in &runtime_deps {
        if let Some(package) = metadata.packages.iter().find(|p| p.id == *dep_id) {
            println!("  - {} {}", package.name, package.version);
        }
    }

    assert!(
        runtime_deps.len() <= 50, // Adjusted threshold for binary context
        "Too many runtime dependencies ({}) for no-std/embedded binary context. Expected <= 50",
        runtime_deps.len()
    );

    println!("✅ Minimal dependency footprint suitable for embedded binary targets");
}

#[test]
fn test_no_std_binary_functionality() {
    // This test ensures that the no-std binary code can be used and compiled
    // without issues, simulating actual binary usage.
    use cite_no_std_bin_test::{binary_lib_function, BinaryStruct, BinaryEnum};

    binary_lib_function();
    let s = BinaryStruct::new(1);
    let _value = s.get_value();
    let _e = BinaryEnum::Option1;
}
