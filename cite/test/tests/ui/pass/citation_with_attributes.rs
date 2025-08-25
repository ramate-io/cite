// Test that citation attributes compile successfully

use cite::cite;
use cite_core::mock::MockSource;

#[cite(MockSource::same("content"), reason = "This is why we need this reference")]
fn function_with_reason() {}

#[cite(MockSource::same("content"), level = "WARN")]
fn function_with_level() {}

#[cite(MockSource::same("content"), annotation = "ANY")]
fn function_with_annotation() {}

#[cite(MockSource::same("content"), reason = "Multi", level = "ERROR", annotation = "FOOTNOTE")]
fn function_with_all_attributes() {}

// Multiple citations should work
#[cite(MockSource::same("first"))]
#[cite(MockSource::same("second"))]
fn function_with_multiple_citations() {}

fn main() {
    function_with_reason();
    function_with_level();
    function_with_annotation();
    function_with_all_attributes();
    function_with_multiple_citations();
}
