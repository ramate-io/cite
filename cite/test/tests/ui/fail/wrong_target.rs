// Test that citations on truly invalid targets fail to compile

use cite_util::{cite, mock::MockSource};

// This should fail - can't cite a const item
#[cite(MockSource::same("content"))]
const INVALID_TARGET: i32 = 42;

fn main() {}
