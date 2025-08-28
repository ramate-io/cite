// Test that citations fail when CITE_ANNOTATION=FOOTNOTE is set but no reason provided

use cite::cite;

#[cite(mock, same = "content")]
fn function_without_reason() {
    println!("This should fail when CITE_ANNOTATION=FOOTNOTE");
}

fn main() {
    function_without_reason();
}
