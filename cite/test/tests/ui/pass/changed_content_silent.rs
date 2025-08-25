// Test that MockSource::changed() with SILENT level compiles

use cite_util::{cite, mock::MockSource};

#[cite(MockSource::changed("old content", "new content"), level = "SILENT")]
fn function_with_silent_diff() {
    println!("This should compile despite diff because level is SILENT");
}

fn main() {
    function_with_silent_diff();
}
