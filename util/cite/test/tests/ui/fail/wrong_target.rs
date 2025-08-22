// Test that citations on invalid targets fail to compile

use cite_util::{cite, mock::MockSource};

#[cite(MockSource::same("content"))]  // Should fail - can't cite a module
mod test_module {}

fn main() {}
