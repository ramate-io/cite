use cite::{cite, mock, changed};

#[cite(mock(changed("old", "new")), level = "ERROR")]
fn test_fn() {
    println!("test");
}

fn main() {
    test_fn();
}
