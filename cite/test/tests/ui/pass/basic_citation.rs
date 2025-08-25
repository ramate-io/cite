// Test that basic citation syntax compiles successfully

use cite::cite;

#[cite(mock, same = "test content")]
fn test_function() {
    println!("Hello, world!");
}

#[cite(mock, same = "struct content")]
struct TestStruct {
    field: i32,
}

#[cite(mock, same = "trait content")]
trait TestTrait {
    fn test_method(&self);
}

#[cite(mock, same = "impl content")]
impl TestStruct {
    fn new(field: i32) -> Self {
        Self { field }
    }
}

fn main() {
    test_function();
    let _test_struct = TestStruct::new(42);
}
