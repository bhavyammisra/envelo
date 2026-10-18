//! The locked folder.
//!
//! One random 32-byte key encrypts every locked note (AES-256-GCM). That key is stored twice in
//! `.locked/vault.json`, each copy encrypted with a key derived (Argon2id) from either the password
//! or the normalised security answer, so either one unlocks. Locked notes are `.locked/<random>.note`
//! files, so titles stay hidden too. While unlocked the key lives only in memory.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeroize::Zeroizing;

type Key = Zeroizing<[u8; 32]>;

/// The unlocked key, if any. Managed as Tauri state.
#[derive(Default)]
pub struct Vault(pub Mutex<Option<Key>>);

#[derive(Serialize, Deserialize, Clone)]
struct Wrapped {
    salt: String,
    nonce: String,
    key: String,
}

#[derive(Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    question: String,
    password: Wrapped,
    answer: Wrapped,
    #[serde(default)]
    failed: u32,
    #[serde(default)]
    wait_until: i64, // unix seconds; set after 5 wrong tries in a row
}

#[derive(Serialize)]
pub struct Status {
    exists: bool,
    unlocked: bool,
    question: Option<String>,
    wait: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LockedNote {
    #[serde(default)]
    pub id: String,
    pub body: String,
    pub created: String,
    pub modified: String,
}

const MAX_TRIES: u32 = 5;
const WAIT_SECS: i64 = 60;

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).expect("the OS random number generator");
    b
}

fn kdf(secret: &str, salt: &[u8]) -> Result<Key, String> {
    // 64 MiB, 3 passes: slow enough to make guessing expensive, about half a second here
    let params = Params::new(64 * 1024, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let mut out = Key::default();
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(secret.as_bytes(), salt, out.as_mut())
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new_from_slice(key).expect("a 32-byte key")
}

fn decode(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s).map_err(|_| "the locked folder file is damaged".to_string())
}

fn wrap(data_key: &[u8; 32], secret: &str) -> Result<Wrapped, String> {
    let salt = random::<16>();
    let nonce = random::<12>();
    let k = kdf(secret, &salt)?;
    let ct = cipher(&k).encrypt(Nonce::from_slice(&nonce), &data_key[..]).map_err(|e| e.to_string())?;
    Ok(Wrapped { salt: B64.encode(salt), nonce: B64.encode(nonce), key: B64.encode(ct) })
}

/// `Ok(None)` means the secret was wrong.
fn unwrap(w: &Wrapped, secret: &str) -> Result<Option<Key>, String> {
    let k = kdf(secret, &decode(&w.salt)?)?;
    let nonce = decode(&w.nonce)?;
    if nonce.len() != 12 {
        return Err("the locked folder file is damaged".into());
    }
    match cipher(&k).decrypt(Nonce::from_slice(&nonce), decode(&w.key)?.as_slice()) {
        Ok(pt) if pt.len() == 32 => {
            let mut key = Key::default();
            key.copy_from_slice(&pt);
            Ok(Some(key))
        }
        _ => Ok(None),
    }
}

/// "Bruno", " bruno " and "BRUNO" are the same answer.
pub fn normalize(answer: &str) -> String {
    answer.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn dir(root: &Path) -> PathBuf {
    root.join(".locked")
}

fn read_file(root: &Path) -> Result<Option<VaultFile>, String> {
    match fs::read_to_string(dir(root).join("vault.json")) {
        Ok(t) => serde_json::from_str(&t).map(Some).map_err(|_| "the locked folder file is damaged".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn write_file(root: &Path, v: &VaultFile) -> Result<(), String> {
    fs::create_dir_all(dir(root)).map_err(|e| e.to_string())?;
    let path = dir(root).join("vault.json");
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn status(root: &Path, vault: &Vault) -> Result<Status, String> {
    let file = read_file(root)?;
    Ok(Status {
        exists: file.is_some(),
        unlocked: vault.0.lock().unwrap().is_some(),
        question: file.as_ref().map(|f| f.question.clone()),
        wait: file.map(|f| (f.wait_until - now()).max(0)).unwrap_or(0),
    })
}

pub fn setup(root: &Path, vault: &Vault, password: &str, question: &str, answer: &str) -> Result<(), String> {
    if read_file(root)?.is_some() {
        return Err("The locked folder is already set up".into());
    }
    check_password(password)?;
    let (question, answer) = (question.trim(), normalize(answer));
    if question.is_empty() || answer.is_empty() {
        return Err("Choose a security question and give an answer".into());
    }
    let key = Key::new(random::<32>());
    write_file(root, &VaultFile {
        version: 1,
        question: question.to_string(),
        password: wrap(&key, password)?,
        answer: wrap(&key, &answer)?,
        failed: 0,
        wait_until: 0,
    })?;
    *vault.0.lock().unwrap() = Some(key);
    Ok(())
}

fn check_password(password: &str) -> Result<(), String> {
    if password.chars().count() < 6 {
        return Err("Use at least 6 characters for the password".into());
    }
    Ok(())
}

/// Tries the password (or the answer, with `by_answer`). Errors start with "wrong:" or "wait:<secs>".
pub fn unlock(root: &Path, vault: &Vault, secret: &str, by_answer: bool) -> Result<(), String> {
    let mut file = read_file(root)?.ok_or("The locked folder isn't set up yet")?;
    if file.wait_until > now() {
        return Err(format!("wait:{}", file.wait_until - now()));
    }
    let secret = if by_answer { normalize(secret) } else { secret.to_string() };
    let wrapped = if by_answer { &file.answer } else { &file.password };
    match unwrap(wrapped, &secret)? {
        Some(key) => {
            if file.failed != 0 || file.wait_until != 0 {
                file.failed = 0;
                file.wait_until = 0;
                write_file(root, &file)?;
            }
            *vault.0.lock().unwrap() = Some(key);
            Ok(())
        }
        None => {
            file.failed += 1;
            let left = MAX_TRIES.saturating_sub(file.failed);
            if left == 0 {
                file.failed = 0;
                file.wait_until = now() + WAIT_SECS;
            }
            write_file(root, &file)?;
            Err(if left == 0 { format!("wait:{WAIT_SECS}") } else { format!("wrong:{left}") })
        }
    }
}

pub fn lock(vault: &Vault) {
    *vault.0.lock().unwrap() = None;
}

fn key(vault: &Vault) -> Result<Key, String> {
    vault.0.lock().unwrap().clone().ok_or_else(|| "locked".to_string())
}

/// New password (after unlocking, e.g. via the security answer).
pub fn set_password(root: &Path, vault: &Vault, password: &str) -> Result<(), String> {
    check_password(password)?;
    let k = key(vault)?;
    let mut file = read_file(root)?.ok_or("The locked folder isn't set up yet")?;
    file.password = wrap(&k, password)?;
    write_file(root, &file)
}

pub fn set_question(root: &Path, vault: &Vault, question: &str, answer: &str) -> Result<(), String> {
    let (question, answer) = (question.trim(), normalize(answer));
    if question.is_empty() || answer.is_empty() {
        return Err("Choose a security question and give an answer".into());
    }
    let k = key(vault)?;
    let mut file = read_file(root)?.ok_or("The locked folder isn't set up yet")?;
    file.question = question.to_string();
    file.answer = wrap(&k, &answer)?;
    write_file(root, &file)
}

fn note_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid locked note".into());
    }
    Ok(dir(root).join(format!("{id}.note")))
}

#[derive(Serialize, Deserialize)]
struct Sealed {
    nonce: String,
    data: String,
}

fn seal(key: &[u8; 32], note: &LockedNote) -> Result<String, String> {
    let nonce = random::<12>();
    let plain = serde_json::to_vec(note).map_err(|e| e.to_string())?;
    // the id is bound in, so a sealed note can't be swapped into another file unnoticed
    let ct = cipher(key)
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: &plain, aad: note.id.as_bytes() })
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&Sealed { nonce: B64.encode(nonce), data: B64.encode(ct) }).map_err(|e| e.to_string())
}

fn open(key: &[u8; 32], id: &str, text: &str) -> Option<LockedNote> {
    let s: Sealed = serde_json::from_str(text).ok()?;
    let nonce = B64.decode(s.nonce).ok().filter(|n| n.len() == 12)?;
    let plain = cipher(key)
        .decrypt(Nonce::from_slice(&nonce), Payload { msg: &B64.decode(s.data).ok()?, aad: id.as_bytes() })
        .ok()?;
    let mut note: LockedNote = serde_json::from_slice(&plain).ok()?;
    note.id = id.to_string();
    Some(note)
}

pub fn list(root: &Path, vault: &Vault) -> Result<Vec<LockedNote>, String> {
    let k = key(vault)?;
    let Ok(entries) = fs::read_dir(dir(root)) else { return Ok(vec![]) };
    Ok(entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".note")?.to_string();
            open(&k, &id, &fs::read_to_string(e.path()).ok()?)
        })
        .collect())
}

pub fn save(root: &Path, vault: &Vault, id: Option<String>, body: String, created: String) -> Result<LockedNote, String> {
    let k = key(vault)?;
    let id = id.unwrap_or_else(|| hex::encode(random::<16>()));
    let path = note_path(root, &id)?;
    let note = LockedNote { id, body, created, modified: chrono::Local::now().to_rfc3339() };
    fs::create_dir_all(dir(root)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("note.tmp");
    fs::write(&tmp, seal(&k, &note)?).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(note)
}

/// Locked notes are deleted for good: putting them in the ordinary trash would un-encrypt them.
pub fn delete(root: &Path, vault: &Vault, id: &str) -> Result<(), String> {
    key(vault)?;
    fs::remove_file(note_path(root, id)?).map_err(|e| e.to_string())
}

pub fn read(root: &Path, vault: &Vault, id: &str) -> Result<LockedNote, String> {
    let k = key(vault)?;
    let text = fs::read_to_string(note_path(root, id)?).map_err(|e| e.to_string())?;
    open(&k, id, &text).ok_or_else(|| "this locked note can't be read".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("envelo-vault-test-{}", hex::encode(random::<6>())));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn password_or_answer_unlocks_and_notes_round_trip() {
        let root = temp();
        let v = Vault::default();
        setup(&root, &v, "correct horse", "First pet?", "  Bruno ").unwrap();
        let saved = save(&root, &v, None, "Secret plans\n\nhi".into(), "2026-10-19T00:00:00Z".into()).unwrap();
        let raw = fs::read_to_string(note_path(&root, &saved.id).unwrap()).unwrap();
        assert!(!raw.contains("Secret plans"), "note text must not be readable on disk");

        lock(&v);
        assert!(list(&root, &v).is_err());
        assert_eq!(unlock(&root, &v, "nope", false).unwrap_err(), "wrong:4");
        unlock(&root, &v, "BRUNO", true).unwrap();
        assert_eq!(list(&root, &v).unwrap(), vec![saved.clone()]);

        set_password(&root, &v, "new password").unwrap();
        lock(&v);
        assert!(unlock(&root, &v, "correct horse", false).is_err());
        unlock(&root, &v, "new password", false).unwrap();
        assert_eq!(read(&root, &v, &saved.id).unwrap().body, "Secret plans\n\nhi");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn five_wrong_tries_means_a_wait() {
        let root = temp();
        let v = Vault::default();
        setup(&root, &v, "123456", "Q?", "a").unwrap();
        lock(&v);
        for left in (1..5).rev() {
            assert_eq!(unlock(&root, &v, "x", false).unwrap_err(), format!("wrong:{left}"));
        }
        assert_eq!(unlock(&root, &v, "x", false).unwrap_err(), "wait:60");
        assert!(unlock(&root, &v, "123456", false).unwrap_err().starts_with("wait:"), "even the right password waits");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_weak_setup_and_bad_ids() {
        let root = temp();
        let v = Vault::default();
        assert!(setup(&root, &v, "123", "Q?", "a").is_err());
        assert!(setup(&root, &v, "123456", "Q?", "   ").is_err());
        assert!(note_path(&root, "../../x").is_err());
        assert_eq!(normalize("  Bruno   the  DOG "), "bruno the dog");
        fs::remove_dir_all(root).unwrap();
    }
}
