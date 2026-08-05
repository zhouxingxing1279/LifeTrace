use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue};
use ipnet::IpNet;
use subtle::ConstantTimeEq;

use crate::config::Config;

#[derive(Debug, Clone, Copy, Default)]
pub struct PeerAddr(pub Option<SocketAddr>);

impl<S> FromRequestParts<S> for PeerAddr
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|connect_info| connect_info.0),
        ))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub origin: Option<String>,
}

impl RequestContext {
    pub fn from_headers(headers: &HeaderMap, peer: Option<SocketAddr>, config: &Config) -> Self {
        Self {
            ip: client_ip(peer, headers, &config.auth_trusted_proxy_cidrs),
            user_agent: headers
                .get("user-agent")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            origin: headers
                .get("origin")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        }
    }
}

pub fn client_ip(
    peer: Option<SocketAddr>,
    headers: &HeaderMap,
    trusted_cidrs: &[String],
) -> Option<IpAddr> {
    let peer_ip = peer.map(|value| value.ip())?;
    let trusted = trusted_cidrs
        .iter()
        .filter_map(|value| value.parse::<IpNet>().ok())
        .any(|network| network.contains(&peer_ip));
    if !trusted {
        return Some(peer_ip);
    }
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.trim().parse().ok())
        .or(Some(peer_ip))
}

pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
}

pub fn build_session_cookie(config: &Config, value: &str, max_age: u64) -> HeaderValue {
    let secure = if config.auth_cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; HttpOnly{}; SameSite={}; Max-Age={}",
        config.auth_cookie_name, value, secure, config.auth_cookie_same_site, max_age,
    ))
    .expect("validated cookie configuration")
}

pub fn clear_session_cookie(config: &Config) -> HeaderValue {
    let secure = if config.auth_cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{}=; Path=/; HttpOnly{}; SameSite={}; Max-Age=0",
        config.auth_cookie_name, secure, config.auth_cookie_same_site,
    ))
    .expect("validated cookie configuration")
}

pub fn csrf_matches(expected_hash: &[u8], actual_hash: &[u8]) -> bool {
    expected_hash.len() == actual_hash.len() && bool::from(expected_hash.ct_eq(actual_hash))
}

pub fn origin_allowed(origin: Option<&str>, config: &Config) -> bool {
    let Some(origin) = origin else {
        return false;
    };
    config.public_web_base_url.as_deref().is_some_and(|base| {
        let normalized = base.trim_end_matches('/');
        origin == normalized
    }) || config
        .cors_allowed_origins
        .iter()
        .any(|allowed| origin == allowed.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_for_is_ignored_without_trusted_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        let peer: SocketAddr = "198.51.100.3:1234".parse().unwrap();
        assert_eq!(client_ip(Some(peer), &headers, &[]), Some(peer.ip()));
    }

    #[test]
    fn cookie_has_epic04_security_attributes() {
        let config = Config {
            auth_cookie_secure: true,
            ..Config::default()
        };
        let cookie = build_session_cookie(&config, "secret", 60)
            .to_str()
            .unwrap()
            .to_owned();
        assert!(cookie.starts_with("__Host-lifetrace_session="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(!cookie.contains("Domain="));
    }
}
