use cite::{cite, mock, same, changed};

#[cite(mock(same("test content")))]
fn test_same() {
    println!("Testing same content");
}

#[cite(mock(changed("old", "new")))]
fn test_changed() {
    println!("Testing changed content");
}

fn main() {
    test_same();
    test_changed();
}
