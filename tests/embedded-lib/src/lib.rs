#![no_std]

use cite::cite;
use embedded_hal::digital::OutputPin;

#[cite(mock, same = "embedded HAL driver", reason = "test reason")]
pub struct EmbeddedDriver<PIN> {
	pin: PIN,
	state: bool,
}

impl<PIN> EmbeddedDriver<PIN>
where
	PIN: OutputPin,
{
	#[cite(mock, same = "driver constructor", reason = "test reason")]
	pub fn new(pin: PIN) -> Self {
		Self { pin, state: false }
	}

	#[cite(mock, same = "toggle", level = "SILENT", reason = "test reason")]
	pub fn toggle(&mut self) -> Result<(), PIN::Error> {
		self.state = !self.state;
		if self.state {
			self.pin.set_high()
		} else {
			self.pin.set_low()
		}
	}

	#[cite(mock, same = "state getter", reason = "test reason")]
	pub fn is_on(&self) -> bool {
		self.state
	}
}

#[cite(mock, same = "embedded sensor trait", reason = "test reason")]
pub trait SensorReading {
	type Error;

	fn read_value(&mut self) -> Result<u16, Self::Error>;
}

#[cite(mock, same = "temperature sensor", reason = "test reason")]
pub struct TemperatureSensor {
	last_reading: u16,
}

impl TemperatureSensor {
	#[cite(mock, same = "sensor constructor", reason = "test reason")]
	pub fn new() -> Self {
		Self { last_reading: 0 }
	}
}

impl Default for TemperatureSensor {
	fn default() -> Self {
		Self::new()
	}
}

#[cite(mock, same = "sensor error type", reason = "test reason")]
#[derive(Debug)]
pub enum SensorError {
	CommunicationError,
	CalibrationError,
	TimeoutError,
}

impl SensorReading for TemperatureSensor {
	type Error = SensorError;

	#[cite(mock, same = "read value", reason = "test reason")]
	fn read_value(&mut self) -> Result<u16, Self::Error> {
		// Simulate sensor reading
		self.last_reading = self.last_reading.wrapping_add(1);
		Ok(self.last_reading)
	}
}

#[cite(mock, same = "embedded utility function", reason = "test reason")]
pub fn delay_ms(_ms: u32) {
	// Placeholder for embedded delay function
	// In real embedded code, this would interface with a timer
}

#[cite(mock, same = "embedded configuration", reason = "test reason")]
pub mod config {
	use cite::cite;

	#[cite(mock, same = "system configuration", reason = "test reason")]
	pub struct SystemConfig {
		pub clock_speed: u32,
		pub sensor_interval: u32,
	}

	impl SystemConfig {
		#[cite(mock, same = "default config", reason = "test reason")]
		pub const fn default() -> Self {
			Self { clock_speed: 16_000_000, sensor_interval: 1000 }
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct MockPin {
		state: bool,
	}

	impl embedded_hal::digital::ErrorType for MockPin {
		type Error = core::convert::Infallible;
	}

	impl OutputPin for MockPin {
		fn set_low(&mut self) -> Result<(), Self::Error> {
			self.state = false;
			Ok(())
		}

		fn set_high(&mut self) -> Result<(), Self::Error> {
			self.state = true;
			Ok(())
		}
	}

	impl MockPin {
		fn new() -> Self {
			Self { state: false }
		}
	}

	#[test]
	fn test_embedded_lib_functions_work() {
		let pin = MockPin::new();
		let mut driver = EmbeddedDriver::new(pin);

		assert!(!driver.is_on());
		driver.toggle().unwrap();
		assert!(driver.is_on());

		let mut sensor = TemperatureSensor::new();
		let _reading = sensor.read_value().unwrap();

		delay_ms(100);

		let _config = config::SystemConfig::default();
	}
}
