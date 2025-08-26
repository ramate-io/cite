// Simple no-std binary-style functions for testing cite macros
use cite::cite;

#[cite(mock, same = "no-std binary function")]
pub fn binary_function() {
    // This function demonstrates cite usage in binary context
}

#[cite(mock, same = "no-std global function")]
pub fn global_function() {
    // This function exists in the binary context
}

fn main() {
    binary_function();
    global_function();
}
