// Test that local level overrides are ignored when global-strict feature is enabled

use cite::cite;

#[cite(mock, changed = ("old content", "new content"), level = "ERROR")]
fn function_with_local_error_level() {
	println!("This should fail compilation even with local ERROR level when global-strict feature is enabled");
}

fn main() {
	function_with_local_error_level();
}
