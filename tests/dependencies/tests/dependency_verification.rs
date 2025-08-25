use std::process::Command;
use cargo_metadata::{MetadataCommand, DependencyKind};

/// Test that cite-util-core has minimal dependencies when used without std
#[test]
fn test_no_std_dependencies() {
    let metadata = MetadataCommand::new()
        .current_dir(".")
        .exec()
        .expect("Failed to get cargo metadata");
    
    // Find our test package
    let test_package = metadata.packages
        .iter()
        .find(|p| p.name == "dependency-test")
        .expect("Failed to find dependency-test package");
    
    // Get only runtime dependencies (not dev dependencies)
    let runtime_deps: Vec<_> = test_package.dependencies
        .iter()
        .filter(|dep| matches!(dep.kind, DependencyKind::Normal))
        .collect();
    
    println!("Runtime dependencies for dependency-test:");
    for dep in &runtime_deps {
        println!("  - {} {}", dep.name, dep.req);
    }
    
    // Should only have cite-util-core as runtime dependency
    assert_eq!(runtime_deps.len(), 1, "Expected exactly 1 runtime dependency, found {}", runtime_deps.len());
    assert_eq!(runtime_deps[0].name, "cite-util-core");
}

/// Test that cite-util-core itself has minimal dependencies in no_std mode
#[test] 
fn test_core_crate_minimal_deps() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "cite-util-core", "--no-default-features", "--format", "{p}"])
        .current_dir("../../")
        .output()
        .expect("Failed to run cargo tree");
    
    let tree_output = String::from_utf8(output.stdout).expect("Invalid UTF-8 in cargo tree output");
    println!("cite-util-core dependency tree (no_std):\n{}", tree_output);
    
    // Heavy dependencies that should NOT appear in no_std mode
    let forbidden_deps = [
        "reqwest",
        "scraper", 
        "regex",
        "similar",
        "chrono",
        "tokio",
        "hyper",
        "html5ever",
    ];
    
    for forbidden in &forbidden_deps {
        assert!(
            !tree_output.contains(forbidden),
            "Found forbidden dependency '{}' in no_std build of cite-util-core", 
            forbidden
        );
    }
    
    // Verify only lightweight dependencies are present
    // The exact list may vary, but should be minimal
    let lines: Vec<&str> = tree_output.lines().collect();
    println!("Total dependency lines: {}", lines.len());
    
    // In no_std mode, should have very few dependencies
    assert!(
        lines.len() < 10, 
        "Too many dependencies ({}) for no_std mode. Expected < 10",
        lines.len()
    );
}

/// Test that the test crate compiles and runs in no_std mode
#[test]
fn test_no_std_compilation() {
    let output = Command::new("cargo")
        .args(["check", "--target", "thumbv7em-none-eabihf", "--no-default-features"])
        .current_dir(".")
        .output()
        .expect("Failed to run cargo check");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Failed to compile for embedded target:\nSTDOUT:\n{}\nSTDERR:\n{}", 
            stdout, stderr
        );
    }
    
    println!("✅ Successfully compiled for embedded target (thumbv7em-none-eabihf)");
}

/// Test that normal std mode works with more features
#[test]
fn test_std_mode_works() {
    let output = Command::new("cargo")
        .args(["test", "--lib"])
        .current_dir(".")
        .output()
        .expect("Failed to run cargo test");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Failed to run tests in std mode:\nSTDOUT:\n{}\nSTDERR:\n{}", 
            stdout, stderr
        );
    }
    
    println!("✅ Tests pass in std mode");
}
