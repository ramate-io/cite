// Test that citations on truly invalid targets fail to compile

use cite::cite;

// This should fail - can't cite a const item
#[cite(mock, same = "content", reason = "test reason")]
const INVALID_TARGET: i32 = 42;

fn main() {}
