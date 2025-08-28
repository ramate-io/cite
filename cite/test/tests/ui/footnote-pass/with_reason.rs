// Test that citations pass when CITE_ANNOTATION=FOOTNOTE is set and a reason is provided

use cite::cite;

#[cite(mock, same = "content", reason = "This function implements the documented API")]
fn function_with_reason() {
    println!("This should pass when CITE_ANNOTATION=FOOTNOTE");
}

fn main() {
    function_with_reason();
}
