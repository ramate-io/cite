// Simplified embedded-style main demonstrating cite usage
use cite::cite;

#[cite(mock, same = "embedded main function", reason = "test reason")]
fn main() {
	embedded_startup();
	embedded_main_loop();
}

#[cite(mock, same = "embedded startup", reason = "test reason")]
fn embedded_startup() {
	// Demonstrates cite in embedded startup context
}

#[cite(mock, same = "embedded main loop", reason = "test reason")]
fn embedded_main_loop() {
	// Demonstrates cite in embedded main loop context
}

#[cite(mock, same = "embedded task", level = "WARN", reason = "test reason")]
#[allow(dead_code)]
fn embedded_task() {
	// Demonstrates cite in embedded task context
}

#[cite(mock, same = "embedded interrupt handler", reason = "test reason")]
#[allow(dead_code)]
fn embedded_interrupt_handler() {
	// Demonstrates cite in interrupt context
}
