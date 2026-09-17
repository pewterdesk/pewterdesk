//! OS-keychain bridge — the only place private key material is allowed to
//! live. See CLAUDE.md's security-sensitive-code section, and run
//! `/security-review` against any diff touching this file.
//!
//! Backed by the `keyring` crate: macOS Keychain, Windows Credential Manager,
//! Linux Secret Service. PewterDesk itself writes nothing to disk — the OS
//! owns the encryption and the unlock gate.

use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Matches `identifier` in tauri.conf.json, so macOS groups our entries under
/// the app instead of scattering them across the login keychain.
const SERVICE: &str = "xyz.pewterdesk.app";

/// Account labels are ours to pick (a wallet address, or "hyperliquid:api").
/// Bounded and charset-restricted so a buggy or compromised frontend can't
/// address arbitrary keychain entries or smuggle control characters into the
/// OS store.
const MAX_ACCOUNT_LEN: usize = 128;

/// A secret on its way *into* the keychain.
///
/// Deliberately not `Serialize`, with a redacted `Debug`, so it can't be
/// logged or handed back to the frontend by accident. Zeroized on drop.
///
/// Caveat: this clears only *our* copy. Tauri's IPC layer already parsed the
/// JSON payload into buffers we don't own, so treat it as defense in depth,
/// not a guarantee.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum KeychainError {
    /// The account label failed validation — a caller bug, not a user problem.
    InvalidAccount(String),
    /// Nothing stored under that account.
    NotFound,
    /// The OS store refused or failed. Message is safe to surface.
    Backend(String),
}

/// Map a `keyring` error without letting credential material into the message.
///
/// `BadEncoding` carries the raw stored bytes and `Ambiguous` carries whole
/// `Credential`s, so neither is ever formatted. The remaining variants are
/// OS/attribute errors with no secret in them.
fn map_err(err: KeyringError) -> KeychainError {
    match err {
        KeyringError::NoEntry => KeychainError::NotFound,
        KeyringError::BadEncoding(_) => {
            KeychainError::Backend("stored credential is not valid UTF-8".into())
        }
        KeyringError::Ambiguous(_) => {
            KeychainError::Backend("multiple credentials matched this account".into())
        }
        other => KeychainError::Backend(other.to_string()),
    }
}

fn entry(account: &str) -> Result<Entry, KeychainError> {
    if account.is_empty() {
        return Err(KeychainError::InvalidAccount(
            "account must not be empty".into(),
        ));
    }
    if account.len() > MAX_ACCOUNT_LEN {
        return Err(KeychainError::InvalidAccount(format!(
            "account must be at most {MAX_ACCOUNT_LEN} characters"
        )));
    }
    if !account
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    {
        return Err(KeychainError::InvalidAccount(
            "account may only contain [A-Za-z0-9-_.:]".into(),
        ));
    }
    Entry::new(SERVICE, account).map_err(map_err)
}

/// Store (or overwrite) a secret.
///
/// `(async)` is load-bearing: keychain writes block, and macOS may raise an
/// authorization prompt. On the main thread that freezes the window.
#[tauri::command(async)]
pub fn store_secret(account: String, secret: Secret) -> Result<(), KeychainError> {
    entry(&account)?.set_password(&secret.0).map_err(map_err)
}

/// Read a secret back — the *only* path by which key material reaches JS, and
/// it should stay that way. Callers go through `withSecret` on the TS side so
/// the value is never parked in component state.
///
/// The returned `String` is serialized into the IPC response, so it can't be
/// zeroized; it lives until the JS string is collected. That is the standing
/// cost of signing with viem in TS. Moving signing into Rust is what removes
/// it — see the PRD's note on this.
#[tauri::command(async)]
pub fn get_secret(account: String) -> Result<String, KeychainError> {
    entry(&account)?.get_password().map_err(map_err)
}

/// Whether a secret exists, without handing it to the frontend. Lets the UI
/// render "key configured / not configured" without touching key material.
///
/// `keyring` has no portable existence check, so this reads and discards. On
/// macOS that means it can still trigger an auth prompt — don't call it on
/// every render.
#[tauri::command(async)]
pub fn has_secret(account: String) -> Result<bool, KeychainError> {
    match entry(&account)?.get_password() {
        Ok(mut secret) => {
            secret.zeroize();
            Ok(true)
        }
        Err(KeyringError::NoEntry) => Ok(false),
        Err(e) => Err(map_err(e)),
    }
}

/// Delete a secret. Deleting something absent is not an error — "ensure no key
/// is stored for this account" should be idempotent.
#[tauri::command(async)]
pub fn delete_secret(account: String) -> Result<(), KeychainError> {
    match entry(&account)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(map_err(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    /// keyring ships a platform-independent mock store. The default credential
    /// builder is global, so install it exactly once per test binary.
    fn init() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        });
    }

    #[test]
    fn rejects_empty_account() {
        init();
        assert!(matches!(entry(""), Err(KeychainError::InvalidAccount(_))));
    }

    #[test]
    fn rejects_overlong_account() {
        init();
        let long = "a".repeat(MAX_ACCOUNT_LEN + 1);
        assert!(matches!(
            entry(&long),
            Err(KeychainError::InvalidAccount(_))
        ));
    }

    #[test]
    fn rejects_out_of_charset_account() {
        init();
        for bad in [
            "has space",
            "new\nline",
            "emoji-🔑",
            "quote\"d",
            "null\0byte",
        ] {
            assert!(
                matches!(entry(bad), Err(KeychainError::InvalidAccount(_))),
                "expected {bad:?} to be rejected"
            );
        }
    }

    /// The happy path can only be tested against a *real* OS store. keyring's
    /// mock builder hands out a fresh, empty credential per `Entry`, and every
    /// command here builds its own entry, so a secret stored through one is
    /// invisible to the next. The mock is for error injection, not persistence.
    ///
    /// Opt in with `cargo test -- --ignored`, which is also what keeps this
    /// honest: `--ignored` runs *only* ignored tests, so `init()` never fires
    /// and the real store is in play. It writes to your actual login keychain
    /// (macOS may prompt) and cleans up after itself.
    #[test]
    #[ignore = "touches the real OS keychain; run with: cargo test -- --ignored"]
    fn round_trips_a_secret_against_the_real_store() {
        let account = "test:round-trip";
        store_secret(account.into(), Secret("0xdeadbeef".into())).unwrap();
        assert_eq!(get_secret(account.into()).unwrap(), "0xdeadbeef");
        assert!(has_secret(account.into()).unwrap());
        delete_secret(account.into()).unwrap();
        assert!(!has_secret(account.into()).unwrap());
    }

    #[test]
    fn missing_account_reports_not_found() {
        init();
        assert!(matches!(
            get_secret("test:definitely-absent".into()),
            Err(KeychainError::NotFound)
        ));
        assert!(!has_secret("test:definitely-absent".into()).unwrap());
    }

    #[test]
    fn delete_is_idempotent() {
        init();
        let account = "test:idempotent-delete";
        delete_secret(account.into()).unwrap();
        delete_secret(account.into()).unwrap();
    }

    /// The wrong-input case that matters: `BadEncoding` carries the raw stored
    /// bytes, so the mapped error must not contain them in any form that
    /// reaches the frontend.
    #[test]
    fn bad_encoding_error_never_carries_the_bytes() {
        let raw = b"\xff\xfe-sentinel-material-".to_vec();
        let mapped = map_err(KeyringError::BadEncoding(raw));
        let json = serde_json::to_string(&mapped).unwrap();
        assert!(
            !json.contains("sentinel"),
            "leaked credential bytes: {json}"
        );
        assert!(matches!(mapped, KeychainError::Backend(_)));
    }

    #[test]
    fn secret_debug_is_redacted() {
        let s = Secret("0xdeadbeef".into());
        assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
    }
}
