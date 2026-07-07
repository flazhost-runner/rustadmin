//! Storage abstraction — driver-aware object storage (`local` filesystem / `oss` / `s3`).
//!
//! Port of NodeAdmin `src/config/storageClient.ts`. Switching `STORAGE_DRIVER` between
//! `local` and `oss`/`s3` needs ONLY a `.env` change — no code or view edits:
//!
//! * **local** — files are written under [`local_storage_dir`] (resolves `STORAGE_BASE_PATH`)
//!   and served by a Rocket `FileServer` mounted at [`LOCAL_URL_PREFIX`] (`/storage`). The
//!   public URL is **decoupled** from the filesystem path, so an absolute `STORAGE_BASE_PATH`
//!   (e.g. `/app/storage` in Docker) still yields a valid URL (`/storage/<key>`, never
//!   `//app/storage/...`). The mount is registered only when the driver is `local`.
//! * **oss / s3** — objects live in the bucket; [`object_url`] returns an **absolute**
//!   AWS-SigV4 presigned `GET` URL. Uploads/lists/deletes are performed over HTTPS with
//!   the same SigV4 query-presigning (no SDK dependency — mirrors the reference's inline
//!   `crypto` implementation). OSS is addressed through its S3-compatible endpoint.
//!
//! In every case the **object key** (e.g. `editor/<uuid>.png`) is what callers persist —
//! the render URL is derived from the key at read time.

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use openssl::hash::{hash, MessageDigest};
use openssl::pkey::PKey;
use openssl::sign::Signer;

use crate::config::StorageConfig;
use crate::errors::{AppError, AppResult};

/// Stable public URL prefix for locally-served objects. Decoupled from the filesystem base
/// path — the `FileServer` mount in `build_rocket` binds this prefix to [`local_storage_dir`].
pub const LOCAL_URL_PREFIX: &str = "/storage";

/// Presigned-URL lifetime for cloud objects (seconds).
const PRESIGN_TTL: u32 = 3600;

/// `true` when the active driver serves files from the local filesystem.
pub fn is_local() -> bool {
    StorageConfig::from_env().driver == "local"
}

/// Absolute local storage directory (resolves `STORAGE_BASE_PATH`). Absolute paths are used
/// as-is; relative paths resolve against the app root so the app runs from any CWD.
pub fn local_storage_dir() -> PathBuf {
    let bp = StorageConfig::from_env().base_path;
    let p = PathBuf::from(&bp);
    if p.is_absolute() {
        p
    } else {
        crate::config::asset(&bp)
    }
}

/// Driver-aware public URL for an object `key`.
///
/// * local → stable `/storage/<key>` (served by the `FileServer` mount).
/// * oss/s3 → absolute SigV4 presigned `GET` URL.
pub fn object_url(key: &str) -> String {
    let c = StorageConfig::from_env();
    if c.driver == "local" {
        collapse_slashes(&format!("{LOCAL_URL_PREFIX}/{key}"))
    } else {
        presign(&c, "GET", &object_uri(&c, key), &[], PRESIGN_TTL)
    }
}

/// Store `bytes` at `key`.
pub async fn put(key: &str, bytes: &[u8]) -> AppResult<()> {
    let c = StorageConfig::from_env();
    if c.driver == "local" {
        let dest = local_storage_dir().join(key);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("storage init: {e}")))?;
        }
        return fs::write(&dest, bytes).map_err(|e| AppError::internal(format!("write file: {e}")));
    }
    ensure_cloud_configured(&c)?;
    let url = presign(&c, "PUT", &object_uri(&c, key), &[], PRESIGN_TTL);
    let res = http()
        .put(&url)
        .body(bytes.to_vec())
        .send()
        .await
        .map_err(|e| AppError::internal(format!("storage put: {e}")))?;
    check_status(res, "put").await
}

/// List object keys under `prefix` (returns full keys, e.g. `editor/x.png`).
pub async fn list(prefix: &str) -> AppResult<Vec<String>> {
    let c = StorageConfig::from_env();
    if c.driver == "local" {
        let dir = local_storage_dir().join(prefix);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let entries =
            fs::read_dir(&dir).map_err(|e| AppError::internal(format!("read dir: {e}")))?;
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('.') {
                    continue;
                }
                out.push(collapse_slashes(&format!("{prefix}/{name}")));
            }
        }
        return Ok(out);
    }
    ensure_cloud_configured(&c)?;
    // ListObjectsV2 on the bucket (path-style when an endpoint is configured).
    let uri = if c.endpoint.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", c.bucket)
    };
    let extra = [("list-type", "2"), ("prefix", prefix), ("max-keys", "1000")];
    let url = presign(&c, "GET", &uri, &extra, PRESIGN_TTL);
    let body = http()
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("storage list: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::internal(format!("storage list body: {e}")))?;
    Ok(extract_keys(&body)
        .into_iter()
        .filter(|k| k != prefix && !k.ends_with('/'))
        .collect())
}

/// Delete the object at `key`.
pub async fn delete(key: &str) -> AppResult<()> {
    let c = StorageConfig::from_env();
    if c.driver == "local" {
        let dest = local_storage_dir().join(key);
        if dest.exists() {
            fs::remove_file(&dest).map_err(|e| AppError::internal(format!("delete file: {e}")))?;
        }
        return Ok(());
    }
    ensure_cloud_configured(&c)?;
    let url = presign(&c, "DELETE", &object_uri(&c, key), &[], PRESIGN_TTL);
    let res = http()
        .delete(&url)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("storage delete: {e}")))?;
    check_status(res, "delete").await
}

// --- cloud helpers -----------------------------------------------------------

fn http() -> reqwest::Client {
    reqwest::Client::new()
}

fn ensure_cloud_configured(c: &StorageConfig) -> AppResult<()> {
    if c.access_key_id.is_empty() || c.secret_access_key.is_empty() {
        return Err(AppError::internal(
            "Storage belum dikonfigurasi (STORAGE_ACCESS_KEY_ID/STORAGE_SECRET_ACCESS_KEY kosong)",
        ));
    }
    Ok(())
}

async fn check_status(res: reqwest::Response, op: &str) -> AppResult<()> {
    if res.status().is_success() {
        Ok(())
    } else {
        Err(AppError::internal(format!(
            "storage {op}: HTTP {}",
            res.status()
        )))
    }
}

/// Canonical object URI (path-style `/bucket/key` when an endpoint is set, else `/key`).
fn object_uri(c: &StorageConfig, key: &str) -> String {
    if c.endpoint.is_empty() {
        format!("/{key}")
    } else {
        format!("/{}/{key}", c.bucket)
    }
}

/// Extract `<Key>...</Key>` values from an S3 ListObjectsV2 XML response (dependency-free).
fn extract_keys(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<Key>") {
        rest = &rest[start + 5..];
        if let Some(end) = rest.find("</Key>") {
            out.push(rest[..end].to_string());
            rest = &rest[end + 6..];
        } else {
            break;
        }
    }
    out
}

// --- AWS Signature V4 query presigning (no SDK; mirrors NodeAdmin `s3PresignedUrl`) -------

/// Build an AWS-SigV4 presigned URL for `method` on `canonical_uri`, folding `extra_query`
/// into the signed query string. Works for AWS S3 and S3-compatible endpoints (MinIO, R2,
/// Alibaba OSS S3-compat, …): path-style is used whenever `STORAGE_ENDPOINT` is set.
fn presign(
    c: &StorageConfig,
    method: &str,
    canonical_uri: &str,
    extra_query: &[(&str, &str)],
    ttl: u32,
) -> String {
    presign_at(c, method, canonical_uri, extra_query, ttl, Utc::now())
}

fn presign_at(
    c: &StorageConfig,
    method: &str,
    canonical_uri: &str,
    extra_query: &[(&str, &str)],
    ttl: u32,
    now: chrono::DateTime<Utc>,
) -> String {
    let region = if c.region.is_empty() {
        "us-east-1"
    } else {
        &c.region
    };
    let date = now.format("%Y%m%d").to_string();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();

    let path_style = !c.endpoint.is_empty();
    let host = if path_style {
        c.endpoint
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    } else {
        format!("{}.s3.{}.amazonaws.com", c.bucket, region)
    };

    let cred_scope = format!("{date}/{region}/s3/aws4_request");
    let mut qp: Vec<(String, String)> = vec![
        ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
        (
            "X-Amz-Credential".into(),
            format!("{}/{}", c.access_key_id, cred_scope),
        ),
        ("X-Amz-Date".into(), datetime.clone()),
        ("X-Amz-Expires".into(), ttl.to_string()),
        ("X-Amz-SignedHeaders".into(), "host".into()),
    ];
    for (k, v) in extra_query {
        qp.push(((*k).to_string(), (*v).to_string()));
    }
    qp.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_qs = qp
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&");

    let encoded_uri = canonical_uri
        .split('/')
        .map(enc)
        .collect::<Vec<_>>()
        .join("/");

    let canonical_request =
        format!("{method}\n{encoded_uri}\n{canonical_qs}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD");
    let req_hash = sha256_hex(canonical_request.as_bytes());
    let string_to_sign = format!("AWS4-HMAC-SHA256\n{datetime}\n{cred_scope}\n{req_hash}");

    let k_date = hmac_sha256(
        format!("AWS4{}", c.secret_access_key).as_bytes(),
        date.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = to_hex(&hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let protocol = if c.ssl { "https" } else { "http" };
    format!("{protocol}://{host}{canonical_uri}?{canonical_qs}&X-Amz-Signature={signature}")
}

/// Percent-encode per RFC 3986 (SigV4): everything except unreserved `A-Za-z0-9-_.~`.
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let pkey = PKey::hmac(key).expect("hmac key");
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).expect("hmac signer");
    signer.update(data).expect("hmac update");
    signer.sign_to_vec().expect("hmac finalize")
}

fn sha256_hex(data: &[u8]) -> String {
    to_hex(&hash(MessageDigest::sha256(), data).expect("sha256"))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn collapse_slashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_slash = false;
    for ch in s.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloud_cfg() -> StorageConfig {
        StorageConfig {
            driver: "s3".into(),
            base_path: "storage".into(),
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            endpoint: String::new(),
            bucket: "examplebucket".into(),
            region: "us-east-1".into(),
            ssl: true,
        }
    }

    #[test]
    fn sha256_known_answer() {
        // FIPS-180 test vector for "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hmac_known_answer() {
        // RFC 4231 test case 2.
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            to_hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn enc_encodes_reserved_but_not_unreserved() {
        assert_eq!(enc("AZaz09-_.~"), "AZaz09-_.~");
        assert_eq!(enc("a/b c"), "a%2Fb%20c");
    }

    #[test]
    fn presign_is_deterministic_and_well_formed() {
        let c = cloud_cfg();
        let now = chrono::TimeZone::with_ymd_and_hms(&Utc, 2013, 5, 24, 0, 0, 0).unwrap();
        let url = presign_at(&c, "GET", "/test.txt", &[], 86400, now);
        // Virtual-hosted style, absolute, and carries a signature.
        assert!(url.starts_with("https://examplebucket.s3.us-east-1.amazonaws.com/test.txt?"));
        assert!(url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(url.contains("X-Amz-Signature="));
        assert!(url.contains(
            "X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request"
        ));
        // Deterministic for a fixed clock.
        assert_eq!(url, presign_at(&c, "GET", "/test.txt", &[], 86400, now));
    }

    #[test]
    fn object_url_local_is_decoupled_from_base_path() {
        // Local driver → stable /storage/<key> prefix regardless of base path.
        std::env::set_var("STORAGE_DRIVER", "local");
        assert_eq!(object_url("editor/x.png"), "/storage/editor/x.png");
        std::env::remove_var("STORAGE_DRIVER");
    }

    #[test]
    fn extract_keys_parses_listv2_xml() {
        let xml = "<ListBucketResult><Contents><Key>editor/a.png</Key></Contents>\
                   <Contents><Key>editor/b.png</Key></Contents></ListBucketResult>";
        assert_eq!(extract_keys(xml), vec!["editor/a.png", "editor/b.png"]);
    }
}
