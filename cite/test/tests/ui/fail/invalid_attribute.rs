// Test that invalid citation attributes fail to compile

use cite::cite;

#[cite(mock, same = "content", invalid_attr = "value", reason = "test reason")] // Should fail
fn test_function() {}

fn main() {}
