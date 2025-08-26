// Simplified embedded-style main demonstrating cite usage
use cite::cite;

#[cite(mock, same = "embedded main function")]
fn main() {
	embedded_startup();
	embedded_main_loop();
}

#[cite(mock, same = "embedded startup")]
fn embedded_startup() {
	// Demonstrates cite in embedded startup context
}

#[cite(mock, same = "embedded main loop")]
fn embedded_main_loop() {
	// Demonstrates cite in embedded main loop context
}

#[cite(mock, changed = ("old embedded task", "new embedded task"), level = "WARN")]
#[allow(dead_code)]
fn embedded_task() {
	// Demonstrates cite in embedded task context
}

#[cite(mock, same = "embedded interrupt handler")]
#[allow(dead_code)]
fn embedded_interrupt_handler() {
	// Demonstrates cite in interrupt context
}
