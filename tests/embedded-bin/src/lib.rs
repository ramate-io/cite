#![no_std]

use cite::cite;
use embedded_hal::digital::OutputPin;

#[cite(mock, same = "embedded binary lib driver")]
pub struct BinaryDriver<PIN> {
	output_pin: PIN,
	enabled: bool,
}

impl<PIN> BinaryDriver<PIN>
where
	PIN: OutputPin,
{
	#[cite(mock, same = "binary driver constructor")]
	pub fn new(pin: PIN) -> Self {
		Self { output_pin: pin, enabled: false }
	}

	#[cite(mock, changed = ("old enable", "new enable"), level = "SILENT")]
	pub fn enable(&mut self) -> Result<(), PIN::Error> {
		self.enabled = true;
		self.output_pin.set_high()
	}

	#[cite(mock, same = "disable method")]
	pub fn disable(&mut self) -> Result<(), PIN::Error> {
		self.enabled = false;
		self.output_pin.set_low()
	}

	#[cite(mock, same = "status check")]
	pub fn is_enabled(&self) -> bool {
		self.enabled
	}
}

#[cite(mock, same = "embedded system state")]
pub enum SystemState {
	Initializing,
	Running,
	Sleeping,
	Error(u8),
}

#[cite(mock, same = "embedded application context")]
pub struct AppContext {
	state: SystemState,
	tick_count: u32,
}

impl AppContext {
	#[cite(mock, same = "app context constructor")]
	pub const fn new() -> Self {
		Self { state: SystemState::Initializing, tick_count: 0 }
	}

	#[cite(mock, changed = ("old tick", "new tick"), level = "WARN")]
	pub fn tick(&mut self) {
		self.tick_count = self.tick_count.wrapping_add(1);

		match self.state {
			SystemState::Initializing if self.tick_count > 100 => {
				self.state = SystemState::Running;
			}
			SystemState::Running if self.tick_count % 1000 == 0 => {
				self.state = SystemState::Sleeping;
			}
			SystemState::Sleeping if self.tick_count % 100 == 0 => {
				self.state = SystemState::Running;
			}
			_ => {}
		}
	}

	#[cite(mock, same = "state getter")]
	pub fn get_state(&self) -> &SystemState {
		&self.state
	}

	#[cite(mock, same = "uptime getter")]
	pub fn uptime(&self) -> u32 {
		self.tick_count
	}
}

#[cite(mock, same = "embedded utility function")]
pub fn system_delay_cycles(cycles: u32) {
	// Placeholder for embedded delay function
	// Demonstrates cite usage in embedded utility context
	let _ = cycles;
}

#[cite(mock, same = "embedded GPIO abstraction")]
pub mod embedded_gpio {
	use cite::cite;

	#[cite(mock, same = "embedded pin abstraction")]
	pub struct EmbeddedPin {
		id: u8,
	}

	impl EmbeddedPin {
		#[cite(mock, same = "pin constructor")]
		pub fn new(id: u8) -> Self {
			Self { id }
		}

		#[cite(mock, same = "pin set high")]
		pub fn set_high(&mut self) {
			// Placeholder - demonstrates cite in embedded context
		}

		#[cite(mock, same = "pin set low")]
		pub fn set_low(&mut self) {
			// Placeholder - demonstrates cite in embedded context
		}
	}

	#[cite(mock, same = "embedded LED driver")]
	pub struct EmbeddedLed {
		pin: EmbeddedPin,
		is_on: bool,
	}

	impl EmbeddedLed {
		#[cite(mock, same = "LED constructor")]
		pub fn new(pin: EmbeddedPin) -> Self {
			Self { pin, is_on: false }
		}

		#[cite(mock, changed = ("old toggle", "new toggle"), level = "SILENT")]
		pub fn toggle(&mut self) {
			if self.is_on {
				self.pin.set_low();
				self.is_on = false;
			} else {
				self.pin.set_high();
				self.is_on = true;
			}
		}

		#[cite(mock, same = "LED state getter")]
		pub fn is_on(&self) -> bool {
			self.is_on
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct MockPin {
		high: bool,
	}

	impl embedded_hal::digital::ErrorType for MockPin {
		type Error = core::convert::Infallible;
	}

	impl OutputPin for MockPin {
		fn set_low(&mut self) -> Result<(), Self::Error> {
			self.high = false;
			Ok(())
		}

		fn set_high(&mut self) -> Result<(), Self::Error> {
			self.high = true;
			Ok(())
		}
	}

	impl MockPin {
		fn new() -> Self {
			Self { high: false }
		}
	}

	#[test]
	fn test_embedded_binary_lib_functions_work() {
		let pin = MockPin::new();
		let mut driver = BinaryDriver::new(pin);

		assert!(!driver.is_enabled());
		driver.enable().unwrap();
		assert!(driver.is_enabled());
		driver.disable().unwrap();
		assert!(!driver.is_enabled());

		let mut ctx = AppContext::new();
		ctx.tick();
		let _state = ctx.get_state();
		let _uptime = ctx.uptime();

		system_delay_cycles(1000);
	}
}
