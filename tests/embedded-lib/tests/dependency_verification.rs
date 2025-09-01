use cargo_metadata::{DependencyKind, MetadataCommand, Package, PackageId, Resolve};
use std::collections::HashSet;

/// Test that cite is listed as a dependency but doesn't add runtime dependencies in embedded lib
#[test]
fn test_cite_dependencies_embedded_lib() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	let test_package = metadata
		.packages
		.iter()
		.find(|p| p.name == "cite-embedded-lib-test")
		.expect("Failed to find cite-embedded-lib-test package");

	let runtime_deps: Vec<_> = test_package
		.dependencies
		.iter()
		.filter(|dep| matches!(dep.kind, DependencyKind::Normal))
		.collect();

	println!("Runtime dependencies for cite-embedded-lib-test:");
	for dep in &runtime_deps {
		println!("  - {} {}", dep.name, dep.req);
	}

	// Should have cite, embedded-hal, and nb as runtime dependencies
	assert!(
		runtime_deps.len() >= 2,
		"Expected at least 2 runtime dependencies (cite + embedded-hal), found {}",
		runtime_deps.len()
	);

	let dep_names: HashSet<_> = runtime_deps.iter().map(|d| d.name.as_str()).collect();
	assert!(dep_names.contains("cite"), "Missing cite dependency");
	assert!(dep_names.contains("embedded-hal"), "Missing embedded-hal dependency");
}

/// Test that cite procedural macro does not add heavy runtime dependencies in embedded lib
#[test]
fn test_cite_no_heavy_runtime_dependencies_embedded_lib() {
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
		.find(|p| p.name == "cite-embedded-lib-test")
		.expect("Failed to find cite-embedded-lib-test package")
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
fn test_cite_macro_expansion_embedded_lib() {
	// This test verifies that the cite macro expands correctly in an embedded library context
	// without causing compilation errors.
	// The functions/structs/traits in our lib.rs use cite macros - if they compile,
	// the macro is working.
}

/// Test that the final library has reasonable dependencies for embedded targets
#[test]
fn test_embedded_lib_dependency_footprint() {
	let metadata = MetadataCommand::new()
		.current_dir(".")
		.exec()
		.expect("Failed to get cargo metadata");

	let test_package_id = metadata
		.packages
		.iter()
		.find(|p| p.name == "cite-embedded-lib-test")
		.expect("Failed to find cite-embedded-lib-test package")
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
		runtime_deps.len() <= 50, // Threshold for embedded library context
		"Too many runtime dependencies ({}) for embedded library context. Expected <= 50",
		runtime_deps.len()
	);

	println!("✅ Reasonable dependency footprint for embedded library targets");
}

#[test]
fn test_embedded_hal_integration() {
	// This test ensures that the embedded library code integrates well with embedded-hal
	// and that cite macros don't interfere with HAL traits and implementations.
	use cite_embedded_lib_test::{
		config::SystemConfig, EmbeddedDriver, SensorReading, TemperatureSensor,
	};

	// Test HAL integration
	struct TestPin;

	impl embedded_hal::digital::ErrorType for TestPin {
		type Error = core::convert::Infallible;
	}

	impl embedded_hal::digital::OutputPin for TestPin {
		fn set_low(&mut self) -> Result<(), Self::Error> {
			Ok(())
		}
		fn set_high(&mut self) -> Result<(), Self::Error> {
			Ok(())
		}
	}

	let pin = TestPin;
	let mut driver = EmbeddedDriver::new(pin);
	let _ = driver.toggle();

	// Test sensor trait
	let mut sensor = TemperatureSensor::new();
	let _ = sensor.read_value();

	// Test configuration
	let _config = SystemConfig::default();
}
