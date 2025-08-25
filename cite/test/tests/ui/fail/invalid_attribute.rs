// Test that invalid citation attributes fail to compile

use cite::{cite, mock, same};

#[cite(mock(same("content")), invalid_attr = "value")]  // Should fail
fn test_function() {}

fn main() {}
