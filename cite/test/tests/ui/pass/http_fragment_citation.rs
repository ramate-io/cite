// Test that HTTP citation with fragment support compiles successfully

use cite::cite;

// Basic fragment citation - explicitly specify fragment
#[cite(http, url = "https://example.com/docs", fragment = "introduction")]
fn test_explicit_fragment() {
    println!("This function cites a specific document fragment");
}

// Auto-fragment detection from URL
#[cite(http, url = "https://example.com/docs#getting-started")]
fn test_auto_fragment_detection() {
    println!("This function auto-detects fragment from URL");
}

// Manual auto mode
#[cite(http, url = "https://example.com/docs#installation", match_type = "auto")]
fn test_manual_auto_mode() {
    println!("This function uses explicit auto mode");
}

// Fragment with behavior parameters
#[cite(http, url = "https://example.com/api#authentication", 
       level = "WARN", reason = "API documentation section")]
fn test_fragment_with_behavior() {
    println!("This function cites a fragment with warning level");
}

// CSS selector still works alongside fragments
#[cite(http, url = "https://example.com", selector = "h1")]
fn test_css_selector_without_fragment() {
    println!("This function uses CSS selector matching");
}

fn main() {
    test_explicit_fragment();
    test_auto_fragment_detection();
    test_manual_auto_mode();
    test_fragment_with_behavior();
    test_css_selector_without_fragment();
}
