// Test that citations without a source fail to compile

use cite_util::cite;

#[cite()]  // This should fail - no source provided
fn test_function() {}

fn main() {}
