// Test that invalid citation attributes fail to compile

use cite_util::{cite, mock::MockSource};

#[cite(MockSource::same("content"), invalid_attr = "value")]  // Should fail
fn test_function() {}

fn main() {}
