//! Admin API bearer token helpers shared by the daemon and `wpadm`.
//!
//! Token values are never logged or stored in state snapshots. Only an
//! 8-hex-character fingerprint of the SHA-256 digest is exposed so operators can
//! confirm that the engine and a caller hold the same token without copying
//! secrets around.

use std::fmt;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Environment variable read by `wpadm` when no explicit token source is given.
pub const TOKEN_ENV_VAR: &str = "WPARSE_ADMIN_TOKEN";

/// Hex length of the public token fingerprint.
pub const FINGERPRINT_HEX_LEN: usize = 8;

/// SHA-256 digest of a token value.
pub fn digest(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().into()
}

/// Public, non-reversible token identifier (`sha256(token)` hex, 8 chars).
///
/// High-entropy tokens are required for this to stay non-reversible, which is
/// why [`crate::token`] consumers must generate tokens with a CSPRNG.
pub fn fingerprint(token: &str) -> String {
    hex_prefix(&digest(token), FINGERPRINT_HEX_LEN)
}

fn hex_prefix(bytes: &[u8], hex_len: usize) -> String {
    let mut out = String::with_capacity(hex_len);
    for byte in bytes {
        if out.len() >= hex_len {
            break;
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out.truncate(hex_len);
    out
}

/// Token value held by the admin API, kept as a digest.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenAuth {
    digest: [u8; 32],
    fingerprint: String,
}

impl TokenAuth {
    pub fn from_token(token: &str) -> Self {
        Self {
            digest: digest(token),
            fingerprint: fingerprint(token),
        }
    }

    /// Public identifier, safe to log.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Constant-time verification against a presented bearer value.
    pub fn verify(&self, presented: &str) -> bool {
        let presented = digest(presented);
        self.digest.as_slice().ct_eq(presented.as_slice()).into()
    }
}

impl fmt::Debug for TokenAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never render the digest or the token itself.
        f.debug_struct("TokenAuth")
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenContentError {
    Empty,
    MultipleTokens(usize),
}

impl fmt::Display for TokenContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenContentError::Empty => write!(f, "token file is empty"),
            TokenContentError::MultipleTokens(count) => write!(
                f,
                "token file contains {count} non-empty lines; exactly one token is required"
            ),
        }
    }
}

impl std::error::Error for TokenContentError {}

/// Parse token file content that must hold exactly one non-empty line.
///
/// Blank lines are ignored (trailing newlines are normal), but more than one
/// non-empty line is rejected: silently accepting it would produce a token that
/// can never match a `Authorization: Bearer` header.
pub fn parse_single_line(body: &str) -> Result<&str, TokenContentError> {
    let mut lines = body.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err(TokenContentError::Empty);
    };
    let extra = lines.count();
    if extra > 0 {
        return Err(TokenContentError::MultipleTokens(extra + 1));
    }
    Ok(first)
}

/// Generate a 32-byte CSPRNG token encoded as 64 hex characters.
pub fn generate_token() -> String {
    use rand::RngExt;

    let bytes: [u8; 32] = rand::rng().random();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_and_eight_hex_chars() {
        let fp = fingerprint("test-token");
        assert_eq!(fp.len(), FINGERPRINT_HEX_LEN);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(fp, fingerprint("test-token"));
        assert_ne!(fp, fingerprint("test-token2"));
    }

    #[test]
    fn verify_accepts_only_the_exact_token() {
        let auth = TokenAuth::from_token("secret");
        assert!(auth.verify("secret"));
        assert!(!auth.verify(""));
        assert!(!auth.verify("secre"));
        assert!(!auth.verify("secret "));
        assert!(!auth.verify("SECRET"));
    }

    #[test]
    fn parse_single_line_accepts_trailing_blank_lines() {
        assert_eq!(parse_single_line("abc\n").expect("token"), "abc");
        assert_eq!(parse_single_line("  abc  \r\n\r\n").expect("token"), "abc");
    }

    #[test]
    fn parse_single_line_rejects_empty_content() {
        assert_eq!(parse_single_line(""), Err(TokenContentError::Empty));
        assert_eq!(parse_single_line("\n \r\n"), Err(TokenContentError::Empty));
    }

    #[test]
    fn parse_single_line_rejects_multiple_tokens() {
        let err = parse_single_line("one\ntwo\n").unwrap_err();
        assert_eq!(err, TokenContentError::MultipleTokens(2));
        assert!(err.to_string().contains("exactly one token"));
    }

    #[test]
    fn generated_tokens_are_unique_and_hex() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn token_auth_debug_does_not_leak_the_token() {
        let auth = TokenAuth::from_token("super-secret-value");
        let rendered = format!("{auth:?}");
        assert!(!rendered.contains("super-secret-value"), "{rendered}");
        assert!(rendered.contains(auth.fingerprint()), "{rendered}");
    }
}
