// This should pass when lenient feature is enabled and local overrides are respected

use cite::cite;

#[cite(mock, same = "content", annotation = "ANY")]
fn function_with_local_warn_level() {
	println!("This should pass when lenient feature is enabled and local annotation = ANY level is respected");
}

fn main() {
	function_with_local_warn_level();
}
