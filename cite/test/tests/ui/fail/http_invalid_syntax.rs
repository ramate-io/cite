// Test that invalid HTTP citation syntax fails to compile

use cite::cite;

// Missing URL parameter
#[cite(http, selector = "h1")]
fn test_missing_url() {
    println!("This should fail - missing URL");
}

fn main() {
    test_missing_url();
}
