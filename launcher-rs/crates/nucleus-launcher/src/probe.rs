//! Health probing for spawned app servers.

use std::time::{Duration, Instant};

use ureq::AgentBuilder;

/// Default readiness timeout (SPEC: 20 s).
pub const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a single probe request may take.
const REQUEST_TIMEOUT: Duration = Duration::from_millis(1500);

/// Poll `http://127.0.0.1:{port}{path}` until it answers 2xx.
///
/// Wait: a 1.5 s request timeout, a 250 ms loop interval, hard stop at
/// `timeout`. Returns the last error on timeout so the UI can say something
/// honest instead of a bare "did not answer". `token` is sent as a bearer so the
/// readiness probe passes the app's request guard; pass `""` when the app has no
/// token.
pub fn wait_ready_with_timeout(
    port: u16,
    path: &str,
    timeout: Duration,
    token: &str,
) -> Result<(), String> {
    let path = path.trim_start_matches('/');
    let url = format!("http://127.0.0.1:{port}/{path}");
    let agent = AgentBuilder::new()
        .timeout_connect(REQUEST_TIMEOUT)
        .timeout_read(REQUEST_TIMEOUT)
        .timeout_write(REQUEST_TIMEOUT)
        .build();

    let deadline = Instant::now() + timeout;
    let mut last_error = "server never answered".to_string();
    while Instant::now() < deadline {
        let request = if token.is_empty() {
            agent.get(&url)
        } else {
            agent
                .get(&url)
                .set("Authorization", &format!("Bearer {token}"))
        };
        match request.call() {
            Ok(response) if (200..300).contains(&response.status()) => return Ok(()),
            Ok(response) => last_error = format!("server replied HTTP {}", response.status()),
            Err(err) => last_error = format!("{err}"),
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "app did not answer on port {port} ({} within {}s: {last_error}",
        if path.is_empty() { "/" } else { path },
        timeout.as_secs()
    ))
}
