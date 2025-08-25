// Test that MockSource::changed() with ERROR level fails compilation

use cite::cite;
use cite_core::mock::MockSource;

#[cite(MockSource::changed("old content", "new content"), level = "ERROR")]
fn function_that_should_fail_compilation() {
    println!("This should fail to compile due to citation validation");
}

fn main() {
    function_that_should_fail_compilation();
}
