//! Minimal S3-compatible SigV4 presigning used by EPIC-12.
//!
//! The cloud service owns metadata and authorization. Object bytes travel
//! directly between trusted clients and S3-compatible storage through short-
//! lived signed URLs, so large files never enter the sync JSON payload.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use url::Url;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct ObjectStorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub expires_seconds: u32,
}

#[derive(Debug, Clone)]
pub struct PresignedRequest {
    pub url: String,
    pub required_headers: BTreeMap<String, String>,
    pub expires_seconds: u32,
}

impl ObjectStorageConfig {
    pub fn from_env() -> Result<Self, String> {
        let endpoint = required_env("FILE_OBJECT_STORAGE_ENDPOINT")?;
        let bucket = required_env("FILE_OBJECT_STORAGE_BUCKET")?;
        let access_key_id = required_env("FILE_OBJECT_STORAGE_ACCESS_KEY_ID")?;
        let secret_access_key = required_env("FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY")?;
        let region = std::env::var("FILE_OBJECT_STORAGE_REGION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "us-east-1".to_owned());
        let expires_seconds = std::env::var("FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(900)
            .clamp(60, 3600);
        let parsed = Url::parse(endpoint.trim_end_matches('/'))
            .map_err(|error| format!("FILE_OBJECT_STORAGE_ENDPOINT 无效: {error}"))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || (parsed.path() != "" && parsed.path() != "/")
        {
            return Err("FILE_OBJECT_STORAGE_ENDPOINT 必须是无路径的 HTTP(S) origin".to_owned());
        }
        validate_segment(&bucket, "bucket")?;
        validate_segment(&region, "region")?;
        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_owned(),
            bucket,
            region,
            access_key_id,
            secret_access_key,
            expires_seconds,
        })
    }

    pub fn presign_put(
        &self,
        object_key: &str,
        sha256_hex: &str,
        now: DateTime<Utc>,
    ) -> Result<PresignedRequest, String> {
        let digest = hex::decode(sha256_hex).map_err(|_| "SHA-256 必须是十六进制".to_owned())?;
        if digest.len() != 32 {
            return Err("SHA-256 长度无效".to_owned());
        }
        let checksum = STANDARD.encode(digest);
        let mut headers = BTreeMap::new();
        headers.insert("x-amz-checksum-sha256".to_owned(), checksum);
        self.presign("PUT", object_key, headers, now)
    }

    pub fn presign_get(
        &self,
        object_key: &str,
        now: DateTime<Utc>,
    ) -> Result<PresignedRequest, String> {
        self.presign("GET", object_key, BTreeMap::new(), now)
    }

    fn presign(
        &self,
        method: &str,
        object_key: &str,
        required_headers: BTreeMap<String, String>,
        now: DateTime<Utc>,
    ) -> Result<PresignedRequest, String> {
        validate_object_key(object_key)?;
        let endpoint = Url::parse(&self.endpoint).map_err(|error| error.to_string())?;
        let mut host = endpoint
            .host_str()
            .ok_or_else(|| "对象存储 endpoint 缺少 host".to_owned())?
            .to_owned();
        if let Some(port) = endpoint.port() {
            host.push(':');
            host.push_str(&port.to_string());
        }

        let canonical_uri = format!(
            "/{}/{}",
            aws_encode(&self.bucket, true),
            object_key
                .split('/')
                .map(|segment| aws_encode(segment, true))
                .collect::<Vec<_>>()
                .join("/")
        );
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();
        let scope = format!("{date}/{}/s3/aws4_request", self.region);

        let mut signed_header_names = vec!["host".to_owned()];
        signed_header_names.extend(required_headers.keys().cloned());
        signed_header_names.sort();
        let signed_headers = signed_header_names.join(";");

        let mut query = BTreeMap::new();
        query.insert("X-Amz-Algorithm".to_owned(), "AWS4-HMAC-SHA256".to_owned());
        query.insert(
            "X-Amz-Credential".to_owned(),
            format!("{}/{}", self.access_key_id, scope),
        );
        query.insert("X-Amz-Date".to_owned(), amz_date.clone());
        query.insert("X-Amz-Expires".to_owned(), self.expires_seconds.to_string());
        query.insert("X-Amz-SignedHeaders".to_owned(), signed_headers.clone());
        let canonical_query = canonical_query(&query);

        let mut canonical_headers = format!("host:{}\n", host.to_ascii_lowercase());
        for name in required_headers.keys() {
            let value = required_headers
                .get(name)
                .expect("required header exists")
                .trim();
            canonical_headers.push_str(&format!("{}:{}\n", name.to_ascii_lowercase(), value));
        }
        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\nUNSIGNED-PAYLOAD"
        );
        let canonical_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{canonical_hash}");
        let signature = hex::encode(
            signing_key(&self.secret_access_key, &date, &self.region)
                .and_then(|key| hmac(&key, string_to_sign.as_bytes()))?,
        );
        let url = format!(
            "{}{}?{}&X-Amz-Signature={signature}",
            self.endpoint, canonical_uri, canonical_query
        );

        // `host` is always part of SigV4 canonical headers, but browsers and
        // WebViews are not allowed to set it manually. Return only headers the
        // client really must provide, such as the upload checksum.
        Ok(PresignedRequest {
            url,
            required_headers,
            expires_seconds: self.expires_seconds,
        })
    }
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("缺少 {name}"))
}

fn validate_segment(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 255
        || value.contains('/')
        || value.chars().any(char::is_control)
    {
        Err(format!("对象存储 {label} 无效"))
    } else {
        Ok(())
    }
}

fn validate_object_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.len() > 1024
        || key.starts_with('/')
        || key
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || key.chars().any(char::is_control)
    {
        Err("对象存储 key 无效".to_owned())
    } else {
        Ok(())
    }
}

fn canonical_query(query: &BTreeMap<String, String>) -> String {
    query
        .iter()
        .map(|(key, value)| format!("{}={}", aws_encode(key, true), aws_encode(value, true)))
        .collect::<Vec<_>>()
        .join("&")
}

fn aws_encode(value: &str, encode_slash: bool) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~');
        if unreserved || (!encode_slash && *byte == b'/') {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn signing_key(secret: &str, date: &str, region: &str) -> Result<Vec<u8>, String> {
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let k_region = hmac(&k_date, region.as_bytes())?;
    let k_service = hmac(&k_region, b"s3")?;
    hmac(&k_service, b"aws4_request")
}

fn hmac(key: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|error| error.to_string())?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn config() -> ObjectStorageConfig {
        ObjectStorageConfig {
            endpoint: "https://storage.example.com".to_owned(),
            bucket: "lifetrace-files".to_owned(),
            region: "us-east-1".to_owned(),
            access_key_id: "AKIDEXAMPLE".to_owned(),
            secret_access_key: "test-secret".to_owned(),
            expires_seconds: 900,
        }
    }

    #[test]
    fn put_presign_binds_checksum_and_never_contains_secret() {
        let request = config()
            .presign_put(
                "notes_attachments/123/ab/file.bin",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                Utc.with_ymd_and_hms(2026, 8, 19, 2, 0, 0).unwrap(),
            )
            .unwrap();
        assert!(request.url.contains("X-Amz-Signature="));
        assert!(request.url.contains("X-Amz-Expires=900"));
        assert!(request
            .required_headers
            .contains_key("x-amz-checksum-sha256"));
        assert!(!request.required_headers.contains_key("host"));
        assert!(!request.url.contains("test-secret"));
    }

    #[test]
    fn get_presign_is_path_style_and_short_lived() {
        let request = config()
            .presign_get(
                "photos/123/ab/file.bin",
                Utc.with_ymd_and_hms(2026, 8, 19, 2, 0, 0).unwrap(),
            )
            .unwrap();
        assert!(request
            .url
            .starts_with("https://storage.example.com/lifetrace-files/photos/123/ab/file.bin?"));
        assert_eq!(request.expires_seconds, 900);
        assert!(request.required_headers.is_empty());
    }

    #[test]
    fn traversal_object_key_is_rejected() {
        assert!(config()
            .presign_get(
                "notes_attachments/../secret",
                Utc.with_ymd_and_hms(2026, 8, 19, 2, 0, 0).unwrap(),
            )
            .is_err());
    }
}
