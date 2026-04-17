use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::error::LsqError;

// ── Credentials struct ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
    pub host: String,
    /// Full name of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Email of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    /// Role of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_role: Option<String>,
}

// ── Encrypted envelope ────────────────────────────────────────────────────

/// On-disk format for encrypted credentials (v2).
///
/// Layout:
/// ```json
/// { "v": 2, "n": "<base64 nonce>", "ct": "<base64 ciphertext+tag>" }
/// ```
///
/// Cipher: AES-256-GCM
/// Key:    32 bytes loaded from `~/.lsq-mcp/.key` (generated on first configure)
/// Nonce:  12 bytes, randomly generated per save, stored with the ciphertext
/// Tag:    16 bytes appended by GCM, included in `ct`
///
/// The credentials file and the key file must both be present to decrypt.
/// This provides defence-in-depth: an attacker who exfiltrates only the
/// credentials file (e.g. via a cloud-sync leak) cannot read it.
#[derive(Serialize, Deserialize)]
struct Envelope {
    /// Format version — always 2 for encrypted files.
    v: u8,
    /// Base64-encoded 12-byte AES-GCM nonce (fresh random value per save).
    n: String,
    /// Base64-encoded AES-256-GCM ciphertext with appended 16-byte auth tag.
    ct: String,
}

// ── Key management ────────────────────────────────────────────────────────

/// Load the 32-byte symmetric key from `~/.lsq-mcp/.key`, or generate and
/// persist a fresh one if the file does not exist.
///
/// The key file is created with mode 0o600 (owner read/write only).
/// It is intentionally stored separately from the credentials file so that
/// an attacker must obtain both to decrypt credentials.
fn get_or_create_key() -> Result<[u8; 32], LsqError> {
    let path = config::keyfile_path()?;

    if path.exists() {
        let bytes = fs::read(&path)?;
        if bytes.len() != 32 {
            return Err(LsqError::Auth(
                "Credential key file is corrupt (expected 32 bytes). \
                 Delete ~/.lsq-mcp/.key and run 'lsq-mcp configure' to regenerate."
                    .into(),
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }

    // Generate a cryptographically random 256-bit key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    f.write_all(&key)?;

    Ok(key)
}

// ── Encryption / decryption ───────────────────────────────────────────────

fn encrypt_credentials(creds: &Credentials) -> Result<String, LsqError> {
    let key_bytes = get_or_create_key()?;
    let plaintext = serde_json::to_string(creds)?;

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| LsqError::Auth("Failed to initialise cipher".into()))?;

    // Fresh random 96-bit nonce for every save — reusing a nonce with the same
    // key under GCM would be catastrophic, so we generate one per write.
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ct = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| LsqError::Auth("Credential encryption failed".into()))?;

    let envelope = Envelope {
        v: 2,
        n: B64.encode(nonce.as_slice()),
        ct: B64.encode(&ct),
    };
    Ok(serde_json::to_string_pretty(&envelope)?)
}

fn decrypt_credentials(contents: &str) -> Result<Credentials, LsqError> {
    let envelope: Envelope = serde_json::from_str(contents)
        .map_err(|e| LsqError::Auth(format!("Credentials file is malformed: {}", e)))?;

    let key_bytes = get_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| LsqError::Auth("Failed to initialise cipher".into()))?;

    let nonce_bytes = B64
        .decode(&envelope.n)
        .map_err(|_| LsqError::Auth("Credentials file is corrupt (bad nonce encoding)".into()))?;
    let ct_bytes = B64
        .decode(&envelope.ct)
        .map_err(|_| LsqError::Auth("Credentials file is corrupt (bad ciphertext encoding)".into()))?;

    if nonce_bytes.len() != 12 {
        return Err(LsqError::Auth("Credentials file is corrupt (nonce wrong length)".into()));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ct_bytes.as_slice())
        .map_err(|_| LsqError::Auth(
            "Failed to decrypt credentials — the key file may be corrupt or from a different \
             configure session. Run 'lsq-mcp configure' to reconfigure.".into(),
        ))?;

    let creds: Credentials = serde_json::from_slice(&plaintext)
        .map_err(|e| LsqError::Auth(format!("Decrypted credentials are malformed: {}", e)))?;
    Ok(creds)
}

// ── Public credential API ─────────────────────────────────────────────────

/// Load credentials from disk.
///
/// Supports two on-disk formats:
/// - **v2** (encrypted): JSON envelope `{ "v": 2, "n": "...", "ct": "..." }`.
/// - **v1** (plaintext): legacy JSON with `access_key`, `secret_key`, etc.
///   v1 files are transparently migrated to v2 on first load.
pub fn load_credentials() -> Result<Option<Credentials>, LsqError> {
    let path = config::credentials_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    // Detect format by probing the top-level "v" field.
    let probe: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| LsqError::Auth(format!("Credentials file is malformed: {}", e)))?;

    if probe.get("v").and_then(|v| v.as_u64()) == Some(2) {
        return Ok(Some(decrypt_credentials(&contents)?));
    }

    // v1 plaintext — parse and silently upgrade to encrypted v2.
    let creds: Credentials = serde_json::from_value(probe)
        .map_err(|e| LsqError::Auth(format!("Credentials file is malformed: {}", e)))?;
    save_credentials(&creds)?; // in-place migration
    Ok(Some(creds))
}

/// Encrypt and write credentials to disk.
///
/// The credentials are encrypted with AES-256-GCM using a per-machine key
/// stored at `~/.lsq-mcp/.key`. A fresh random nonce is generated for each
/// write so successive saves produce different ciphertexts.
pub fn save_credentials(creds: &Credentials) -> Result<(), LsqError> {
    let path = config::credentials_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }

    let contents = encrypt_credentials(creds)?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

/// Delete the credentials file. The key file is left in place so that a
/// subsequent `configure` reuses the same key. To fully wipe all lsq-mcp
/// state, delete the entire `~/.lsq-mcp/` directory.
pub fn delete_credentials() -> Result<(), LsqError> {
    let path = config::credentials_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Return a display-safe version of an API key.
/// Shows the first 4 characters followed by `****`.
pub fn mask_key(key: &str) -> String {
    if key.len() <= 4 {
        return "****".to_string();
    }
    format!("{}****", &key[..4])
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_key_shows_first_four() {
        assert_eq!(mask_key("abcdefgh"), "abcd****");
    }

    #[test]
    fn mask_key_short_input() {
        assert_eq!(mask_key("ab"), "****");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let _guard = crate::ENV_MUTEX.lock().unwrap();
        // SAFETY: ENV_MUTEX serialises all env-var-touching tests across modules.
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-auth"); }
        let creds = Credentials {
            access_key: "test_access".into(),
            secret_key: "test_secret".into(),
            host: "api.leadsquared.com".into(),
            user_name: Some("Test User".into()),
            user_email: Some("test@example.com".into()),
            user_role: Some("Admin".into()),
        };
        save_credentials(&creds).unwrap();
        let loaded = load_credentials().unwrap().unwrap();
        assert_eq!(loaded.access_key, "test_access");
        assert_eq!(loaded.secret_key, "test_secret");
        assert_eq!(loaded.host, "api.leadsquared.com");
        delete_credentials().unwrap();
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let _guard = crate::ENV_MUTEX.lock().unwrap();
        // SAFETY: ENV_MUTEX serialises all env-var-touching tests across modules.
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-nofile"); }
        let result = load_credentials().unwrap();
        assert!(result.is_none());
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }

    #[test]
    fn v1_plaintext_is_migrated_to_v2_on_load() {
        let _guard = crate::ENV_MUTEX.lock().unwrap();
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-migrate"); }

        // Write a v1 plaintext credentials file directly, bypassing save_credentials
        let path = config::credentials_path().unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let v1_json = r#"{"access_key":"ak","secret_key":"sk","host":"api.leadsquared.com"}"#;
        std::fs::write(&path, v1_json).unwrap();

        // load_credentials should detect v1, return the creds, and upgrade the file
        let loaded = load_credentials().unwrap().unwrap();
        assert_eq!(loaded.access_key, "ak");

        // The file on disk should now be v2 (encrypted envelope)
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let probe: serde_json::Value = serde_json::from_str(&on_disk).unwrap();
        assert_eq!(probe.get("v").and_then(|v| v.as_u64()), Some(2));

        // Cleanup
        delete_credentials().unwrap();
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }
}
