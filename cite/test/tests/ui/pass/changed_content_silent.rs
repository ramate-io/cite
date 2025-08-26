// Test that mock(changed()) with SILENT level compiles

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), level = "SILENT")]
fn function_with_silent_diff() {
    println!("This should compile despite diff because level is SILENT");
}

fn main() {
    function_with_silent_diff();
}
