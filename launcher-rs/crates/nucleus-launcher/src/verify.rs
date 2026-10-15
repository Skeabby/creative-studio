//! Streaming file hashing — the I/O half of `nucleus_launcher_core::verify`.

use std::io::{self, Read};

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of a reader's remaining bytes, consumed in 64 KiB
/// chunks. Takes `&mut impl Read` so callers can hash and then rewind **the same
/// open handle** — never reopening the path (closes the TOCTOU window).
pub fn sha256_reader(reader: &mut impl Read) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_match_the_core_implementation() {
        let bytes = b"abc";
        assert_eq!(
            sha256_reader(&mut &bytes[..]).unwrap(),
            nucleus_launcher_core::verify::sha256_hex(bytes)
        );
    }
}
