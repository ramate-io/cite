use crate::GitSourceError;

// Takes a closure that returns a Result<(), GitSourceError> and retries it with an exponential backoff.
pub fn with_retry<F, T>(mut f: F) -> Result<T, GitSourceError>
where
	F: FnMut() -> Result<T, GitSourceError>,
{
	const MAX_RETRIES: usize = 5;
	const BASE_DELAY_MS: u64 = 100;

	for attempt in 0..MAX_RETRIES {
		match f() {
			Ok(t) => return Ok(t),
			Err(e) => {
				// Check if this is a lock-related error that we should retry
				if attempt < MAX_RETRIES - 1 {
					// Wait with exponential backoff
					let delay_ms = BASE_DELAY_MS * (1 << attempt);
					std::thread::sleep(std::time::Duration::from_millis(delay_ms));
					continue;
				}
				return Err(e);
			}
		}
	}

	Err(GitSourceError::InvalidRemote("Failed after maximum retries".to_string()))
}
