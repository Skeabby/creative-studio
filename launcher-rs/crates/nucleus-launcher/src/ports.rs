//! Free TCP port allocation for app servers.

use std::io;
use std::net::TcpListener;

/// Pick a free port. When `preferred` is `Some(p)` and `p != 0`, try to bind
/// exactly `127.0.0.1:p`; otherwise (and on conflict) bind port `0` and read
/// back the ephemeral port the OS assigned.
pub fn free_port(preferred: Option<u16>) -> io::Result<u16> {
    let preferred = preferred.filter(|port| *port != 0);
    let listener = match preferred {
        Some(port) => match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => listener,
            Err(_) => TcpListener::bind(("127.0.0.1", 0))?,
        },
        None => TcpListener::bind(("127.0.0.1", 0))?,
    };
    Ok(listener.local_addr()?.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_port() {
        let port = free_port(None).expect("ephemeral port");
        assert!(port > 0);
    }

    #[test]
    fn returns_preferred_when_free() {
        // Bind a listener to grab a port, release it, then request it again.
        let probe = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        assert_eq!(free_port(Some(port)).unwrap(), port);
    }

    #[test]
    fn zero_preferred_means_ephemeral() {
        assert!(free_port(Some(0)).unwrap() > 0);
    }

    #[test]
    fn falls_back_when_preferred_is_taken() {
        let held = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = held.local_addr().unwrap().port();
        let free = free_port(Some(port)).unwrap();
        assert_ne!(free, port);
    }
}
