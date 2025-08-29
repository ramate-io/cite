// Test that local level overrides are ignored when using default strict behavior

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), annotation = "ANY")]
fn function_with_local_warn_level() {
	println!("This should fail compilation even with local annotation = ANY level when using default strict behavior because it overrides annotation = ANY thus requiring a reason");
}

fn main() {
	function_with_local_warn_level();
}
