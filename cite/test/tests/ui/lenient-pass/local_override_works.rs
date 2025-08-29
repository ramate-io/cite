// This should pass when lenient feature is enabled and local overrides are respected

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), level = "WARN")]
fn function_with_local_warn_level() {
	println!("This should pass when lenient feature is enabled and local WARN level is respected");
}

fn main() {
	function_with_local_warn_level();
}
