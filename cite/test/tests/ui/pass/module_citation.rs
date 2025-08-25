// Test that modules can be cited

use cite::{cite, mock, same, changed};

#[cite(mock(same("module content")))]
mod test_module {
    pub fn internal_function() {
        println!("Inside cited module");
    }
}

#[cite(mock(changed("module content", "updated module content")), reason = "Module API evolution")]
mod evolving_module {
    pub struct ModuleStruct;
}

fn main() {
    test_module::internal_function();
}
