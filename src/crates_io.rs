use std::time::{Duration, Instant};

pub struct ApiClient {
    last_request_time: Option<Instant>,
    agent: ureq::Agent,
}

impl Default for ApiClient {
    fn default() -> Self {
        ApiClient {
            last_request_time: None,
            agent: ureq::agent()
                .set(
                    "User-Agent",
                    "cargo supply-chain (https://github.com/rust-secure-code/cargo-supply-chain)",
                )
                .set("Accept", "application/json")
                .build(),
        }
    }
}

impl ApiClient {
    pub fn get(&mut self, url: &str) -> ureq::Request {
        self.wait_to_honor_rate_limit();
        self.agent.get(url)
    }

    /// Waits until at least 1 second has elapsed since last request,
    /// as per https://crates.io/data-access
    fn wait_to_honor_rate_limit(&mut self) {
        if let Some(prev_req_time) = self.last_request_time {
            let next_req_time = prev_req_time + Duration::from_secs(1);
            if let Some(time_to_wait) = next_req_time.checked_duration_since(Instant::now()) {
                std::thread::sleep(time_to_wait);
            }
        }
        self.last_request_time = Some(Instant::now());
    }
}
