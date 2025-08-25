// Test that basic citation syntax compiles successfully

use cite::cite;
use cite_core::mock::MockSource;

#[cite(MockSource::same("test content"))]
fn test_function() {
    println!("Hello, world!");
}

#[cite(MockSource::same("struct content"))]
struct TestStruct {
    field: i32,
}

#[cite(MockSource::same("trait content"))]
trait TestTrait {
    fn test_method(&self);
}

#[cite(MockSource::same("impl content"))]
impl TestStruct {
    fn new(field: i32) -> Self {
        Self { field }
    }
}

fn main() {
    test_function();
    let _test_struct = TestStruct::new(42);
}
