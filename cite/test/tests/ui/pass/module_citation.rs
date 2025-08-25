// Test that modules can be cited

use cite::cite;
use cite_core::mock::MockSource;

#[cite(MockSource::same("module content"))]
mod test_module {
    pub fn internal_function() {
        println!("Inside cited module");
    }
}

#[cite(MockSource::changed("old module API", "new module API"), reason = "Module API evolution")]
mod evolving_module {
    pub struct ModuleStruct;
}

fn main() {
    test_module::internal_function();
}
