use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
	pub fn new(id: String) -> Self {
		Self(id)
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn as_string(&self) -> String {
		self.0.clone()
	}

	pub fn as_bytes(&self) -> &[u8] {
		self.0.as_bytes()
	}

	pub fn as_string_lossy(&self) -> String {
		self.0.clone()
	}

	pub fn as_string_lossy_mut(&mut self) -> String {
		self.0.clone()
	}
}
