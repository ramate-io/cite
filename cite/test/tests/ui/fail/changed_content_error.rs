// Test that mock keyword syntax with ERROR level fails compilation

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), level = "ERROR")]
fn function_that_should_fail_compilation() {
    println!("This should fail to compile due to citation validation");
}

fn main() {
    function_that_should_fail_compilation();
}
