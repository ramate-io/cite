// Test that demonstrates behavior-driven validation (without environment variables)

use cite::cite;
use cite_core::mock::MockSource;

// Test different local level overrides
#[cite(MockSource::changed("old", "new"), level = "SILENT")]
fn function_with_silent_level() {
    // This would not report even if content changed
}

#[cite(MockSource::changed("old", "new"), level = "WARN")]
fn function_with_warn_level() {
    // This would report as warning if content changed
}

#[cite(MockSource::same("content"), level = "ERROR")]
fn function_with_error_level() {
    // This would fail compilation if content changed, but this content matches
}

// Test with multiple attributes
#[cite(MockSource::changed("v1.0", "v2.0"), reason = "Version upgrade", level = "WARN", annotation = "ANY")]
fn function_with_all_attributes() {
    println!("Full behavior demonstration");
}

fn main() {
    function_with_silent_level();
    function_with_warn_level();
    function_with_error_level();
    function_with_all_attributes();
}
