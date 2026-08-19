//! S3-compatible object storage with AWS Signature V4 presigned requests.
//!
//! Credentials never leave the Cloud process. Clients receive short-lived,
//! method- and object-bound URLs plus the exact headers required by an upload.

use std::collections::BTreeMap;
use std::env;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;

const SERVICE: &str = "s3";
const DEFAULT_PRESIGN_TTL_SECONDS: u64 = 15 * 60;
const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error)]
pub enum ObjectStorageError {
    #[error("object storage is not configured")]
    Disabled,
    #[error("invalid object storage configuration: {0}")]
    Config(String),
    #[error("invalid object key: {0}")]
    InvalidKey(String),
    #[error("object storage signing failed: {0}")]
    Signing(String),
    #[error("object storage request failed: {0}")]
    Request(String),
}

#[derive(Debug, Clone)]
pub struct ObjectStorageConfig {
    endpoint: Url,
    region: String,
    bucket: String,
    access_key: String,
    secret_key: String,
    presign_ttl_seconds: u64,
    max_file_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ObjectStorage {
    config: ObjectStorageConfig,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHead {
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub domain: Option<String>,
}

impl ObjectStorageConfig {
    pub fn from_env() -> Result<Self, ObjectStorageError> {
        let endpoint = env_value("OBJECT_STORAGE_ENDPOINT");
        let region = env_value("OBJECT_STORAGE_REGION");
        let bucket = env_value("OBJECT_STORAGE_BUCKET");
        let access_key = env_value("OBJECT_STORAGE_ACCESS_KEY");
        let secret_key = env_value("OBJECT_STORAGE_SECRET_KEY");

        if endpoint.is_none()
            && region.is_none()
            && bucket.is_none()
            && access_key.is_none()
            && secret_key.is_none()
        {
            return Err(ObjectStorageError::Disabled);
        }

        let endpoint = endpoint.ok_or_else(|| missing("OBJECT_STORAGE_ENDPOINT"))?;
        let region = region.ok_or_else(|| missing("OBJECT_STORAGE_REGION"))?;
        let bucket = bucket.ok_or_else(|| missing("OBJECT_STORAGE_BUCKET"))?;
        let access_key = access_key.ok_or_else(|| missing("OBJECT_STORAGE_ACCESS_KEY"))?;
        let secret_key = secret_key.ok_or_else(|| missing("OBJECT_STORAGE_SECRET_KEY"))?;
        let endpoint = Url::parse(&endpoint)
            .map_err(|_| ObjectStorageError::Config("OBJECT_STORAGE_ENDPOINT must be an absolute URL".to_owned()))?;
        validate_endpoint(&endpoint)?;
        validate_bucket(&bucket)?;
        if region.len() > 128 || access_key.len() > 256 || secret_key.len() > 1024 {
            return Err(ObjectStorageError::Config(
                "object storage credentials/region exceed supported length".to_owned(),
            ));
        }

        let presign_ttl_seconds = env_value("OBJECT_STORAGE_PRESIGN_TTL_SECONDS")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PRESIGN_TTL_SECONDS)
            .clamp(60, 3600);
        let max_file_bytes = env_value("OBJECT_STORAGE_MAX_FILE_BYTES")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_FILE_BYTES)
            .clamp(1024, 16 * 1024 * 1024 * 1024);

        Ok(Self {
            endpoint,
            region,
            bucket,
            access_key,
            secret_key,
            presign_ttl_seconds,
            max_file_bytes,
        })
    }
}

impl ObjectStorage {
    pub fn from_env() -> Result<Self, ObjectStorageError> {
        Ok(Self {
            config: ObjectStorageConfig::from_env()?,
            client: reqwest::Client::new(),
        })
    }

    #[cfg(test)]
    fn from_config(config: ObjectStorageConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn max_file_bytes(&self) -> u64 {
        self.config.max_file_bytes
    }

    pub fn presign_upload(
        &self,
        object_key: &str,
        sha256: &str,
        domain: &str,
    ) -> Result<PresignedRequest, ObjectStorageError> {
        let mut headers = BTreeMap::new();
        headers.insert("x-amz-meta-lifetrace-domain".to_owned(), domain.to_owned());
        headers.insert("x-amz-meta-sha256".to_owned(), sha256.to_owned());
        self.presign_at("PUT", object_key, headers, Utc::now())
    }

    pub fn presign_download(
        &self,
        object_key: &str,
    ) -> Result<PresignedRequest, ObjectStorageError> {
        self.presign_at("GET", object_key, BTreeMap::new(), Utc::now())
    }

    pub async fn head_object(
        &self,
        object_key: &str,
    ) -> Result<Option<ObjectHead>, ObjectStorageError> {
        let signed = self.presign_at("HEAD", object_key, BTreeMap::new(), Utc::now())?;
        let response = self
            .client
            .head(&signed.url)
            .send()
            .await
            .map_err(|error| ObjectStorageError::Request(error.to_string()))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(ObjectStorageError::Request(format!(
                "HEAD returned {}",
                response.status()
            )));
        }
        let size_bytes = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| ObjectStorageError::Request("HEAD response has no valid Content-Length".to_owned()))?;
        let sha256 = header_text(response.headers(), "x-amz-meta-sha256");
        let domain = header_text(response.headers(), "x-amz-meta-lifetrace-domain");
        Ok(Some(ObjectHead {
            size_bytes,
            sha256,
            domain,
        }))
    }

    pub async fn delete_object(&self, object_key: &str) -> Result<(), ObjectStorageError> {
        let signed = self.presign_at("DELETE", object_key, BTreeMap::new(), Utc::now())?;
        let response = self
            .client
            .delete(&signed.url)
            .send()
            .await
            .map_err(|error| ObjectStorageError::Request(error.to_string()))?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(ObjectStorageError::Request(format!(
                "DELETE returned {}",
                response.status()
            )))
        }
    }

    fn presign_at(
        &self,
        method: &str,
        object_key: &str,
        headers: BTreeMap<String, String>,
        now: DateTime<Utc>,
    ) -> Result<PresignedRequest, ObjectStorageError> {
        validate_object_key(object_key)?;
        let canonical_uri = self.object_uri(object_key);
        let host = endpoint_host(&self.config.endpoint)?;
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let short_date = now.format("%Y%m%d").to_string();
        let scope = format!(
            "{short_date}/{}/{SERVICE}/aws4_request",
            self.config.region
        );

        let mut signed_headers_map = BTreeMap::new();
        signed_headers_map.insert("host".to_owned(), host);
        for (name, value) in &headers {
            signed_headers_map.insert(name.to_ascii_lowercase(), normalize_header(value));
        }
        let signed_headers = signed_headers_map
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(";");
        let canonical_headers = signed_headers_map
            .iter()
            .map(|(name, value)| format!("{name}:{value}\n"))
            .collect::<String>();

        let mut query = vec![
            ("X-Amz-Algorithm".to_owned(), "AWS4-HMAC-SHA256".to_owned()),
            (
                "X-Amz-Credential".to_owned(),
                format!("{}/{}", self.config.access_key, scope),
            ),
            ("X-Amz-Date".to_owned(), amz_date.clone()),
            (
                "X-Amz-Expires".to_owned(),
                self.config.presign_ttl_seconds.to_string(),
            ),
            ("X-Amz-SignedHeaders".to_owned(), signed_headers.clone()),
        ];
        let canonical_query = canonical_query(&mut query);
        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\nUNSIGNED-PAYLOAD"
        );
        let request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{request_hash}"
        );
        let signing_key = signing_key(&self.config.secret_key, &short_date, &self.config.region)?;
        let signature = hex::encode(hmac(&signing_key, string_to_sign.as_bytes())?);
        let origin = endpoint_origin(&self.config.endpoint)?;
        let url = format!(
            "{origin}{canonical_uri}?{canonical_query}&X-Amz-Signature={signature}"
        );
        let expires_at = now
            + chrono::Duration::seconds(self.config.presign_ttl_seconds as i64);

        Ok(PresignedRequest {
            method: method.to_owned(),
            url,
            headers,
            expires_at,
        })
    }

    fn object_uri(&self, object_key: &str) -> String {
        format!(
            "/{}/{}",
            aws_encode(&self.config.bucket),
            encode_object_key(object_key)
        )
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn missing(name: &str) -> ObjectStorageError {
    ObjectStorageError::Config(format!("{name} is required when object storage is enabled"))
}

fn validate_endpoint(endpoint: &Url) -> Result<(), ObjectStorageError> {
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !matches!(endpoint.path(), "" | "/")
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
    {
        return Err(ObjectStorageError::Config(
            "OBJECT_STORAGE_ENDPOINT must be an http(s) origin without path, credentials, query or fragment".to_owned(),
        ));
    }
    Ok(())
}

fn validate_bucket(bucket: &str) -> Result<(), ObjectStorageError> {
    if bucket.is_empty()
        || bucket.len() > 128
        || !bucket
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(ObjectStorageError::Config(
            "OBJECT_STORAGE_BUCKET contains unsupported characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_object_key(key: &str) -> Result<(), ObjectStorageError> {
    if key.is_empty()
        || key.len() > 1024
        || key.starts_with('/')
        || key.contains('\\')
        || key.split('/').any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(ObjectStorageError::InvalidKey(key.to_owned()));
    }
    Ok(())
}

fn endpoint_host(endpoint: &Url) -> Result<String, ObjectStorageError> {
    let host = endpoint
        .host_str()
        .ok_or_else(|| ObjectStorageError::Config("endpoint host missing".to_owned()))?;
    Ok(match endpoint.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

fn endpoint_origin(endpoint: &Url) -> Result<String, ObjectStorageError> {
    Ok(format!(
        "{}://{}",
        endpoint.scheme(),
        endpoint_host(endpoint)?
    ))
}

fn canonical_query(values: &mut [(String, String)]) -> String {
    let mut encoded = values
        .iter()
        .map(|(name, value)| (aws_encode(name), aws_encode(value)))
        .collect::<Vec<_>>();
    encoded.sort();
    encoded
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_object_key(key: &str) -> String {
    key.split('/').map(aws_encode).collect::<Vec<_>>().join("/")
}

fn aws_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn normalize_header(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hmac(key: &[u8], value: &[u8]) -> Result<Vec<u8>, ObjectStorageError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|error| ObjectStorageError::Signing(error.to_string()))?;
    mac.update(value);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn signing_key(
    secret: &str,
    date: &str,
    region: &str,
) -> Result<Vec<u8>, ObjectStorageError> {
    let first = format!("AWS4{secret}");
    let date_key = hmac(first.as_bytes(), date.as_bytes())?;
    let region_key = hmac(&date_key, region.as_bytes())?;
    let service_key = hmac(&region_key, SERVICE.as_bytes())?;
    hmac(&service_key, b"aws4_request")
}

fn header_text(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn storage() -> ObjectStorage {
        ObjectStorage::from_config(ObjectStorageConfig {
            endpoint: Url::parse("https://s3.example.test").unwrap(),
            region: "ap-southeast-1".to_owned(),
            bucket: "lifetrace-files".to_owned(),
            access_key: "AKIDEXAMPLE".to_owned(),
            secret_key: "secret".to_owned(),
            presign_ttl_seconds: 900,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        })
    }

    #[test]
    fn presigned_upload_is_bound_to_method_path_and_metadata_headers() {
        let now = Utc.with_ymd_and_hms(2026, 8, 19, 1, 2, 3).unwrap();
        let mut headers = BTreeMap::new();
        headers.insert("x-amz-meta-sha256".to_owned(), "a".repeat(64));
        let signed = storage()
            .presign_at("PUT", "users/u/notes/attachments/a b", headers, now)
            .unwrap();
        assert!(signed.url.starts_with(
            "https://s3.example.test/lifetrace-files/users/u/notes/attachments/a%20b?"
        ));
        assert!(signed.url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(signed.url.contains("X-Amz-Credential=AKIDEXAMPLE%2F20260819%2Fap-southeast-1%2Fs3%2Faws4_request"));
        assert!(signed.url.contains("X-Amz-SignedHeaders=host%3Bx-amz-meta-sha256"));
        assert_eq!(signed.method, "PUT");
        assert_eq!(signed.headers.get("x-amz-meta-sha256"), Some(&"a".repeat(64)));
    }

    #[test]
    fn object_key_rejects_path_escape() {
        assert!(validate_object_key("users/a/../secret").is_err());
        assert!(validate_object_key("/absolute").is_err());
        assert!(validate_object_key("users\\a").is_err());
        assert!(validate_object_key("users/a/file").is_ok());
    }

    #[test]
    fn aws_encoding_is_rfc3986_compatible() {
        assert_eq!(aws_encode("a b/+"), "a%20b%2F%2B");
    }
}
