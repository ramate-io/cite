// Test that HTTP citation syntax compiles successfully

use cite::cite;

#[cite(http, url = "https://example.com", selector = "h1", reason = "test reason")]
fn test_function_with_http_citation() {
	println!("This function has an HTTP citation");
}

#[cite(
	http,
	url = "https://example.com",
	pattern = r#""version":\s*"([^"]+)""#,
	reason = "test reason"
)]
fn test_function_with_api_citation() {
	println!("This function cites an API endpoint");
}

#[cite(http, url = "https://example.com", match_type = "full", reason = "test reason")]
fn test_function_with_full_document() {
	println!("This function cites a full document");
}

#[cite(http, url = "https://example.com", selector = "h1", level = "WARN", reason = "test reason")]
fn test_function_with_level() {
	println!("This function has HTTP citation with warning level");
}

#[cite(http, url = "https://example.com", selector = "h1", reason = "API documentation reference")]
fn test_function_with_reason() {
	println!("This function has HTTP citation with reason");
}

#[cite(
	http,
	url = "https://jsonplaceholder.typicode.com/todos/1",
	match_type = "full",
	level = "WARN",
	reason = "API documentation reference"
)]
fn test_function_with_http_bin() {
	println!("This function cites an https://jsonplaceholder.typicode.com/todos/1 endpoint");
}

fn main() {
	test_function_with_http_citation();
	test_function_with_api_citation();
	test_function_with_full_document();
	test_function_with_level();
	test_function_with_reason();
	test_function_with_http_bin();
}
