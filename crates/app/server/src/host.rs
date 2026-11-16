//! Host ヘッダーの許可リスト（DNS リバインディング対策）。ローカルの API は設計ファイルを書けるので、
//! ブラウザがほかのサイトの名前で 127.0.0.1 に送ってきた要求を 403 で拒む。
//!
//! 許可するのは `localhost`・`127.0.0.1`・`[::1]`（ポートは問わない）と、明示した名前（`--host` の IP・`--allow-host`）。
//! Host ヘッダーの無い要求（ブラウザ以外のクライアント）は通す。

use crate::error::ApiError;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::net::IpAddr;
use std::sync::Arc;

/// 常に許可する名前。
const LOCAL_HOSTS: &[&str] = &["localhost", "127.0.0.1", "[::1]"];

/// 許可する Host の一覧。
#[derive(Debug, Clone, Default)]
pub struct HostPolicy {
    extra: Vec<String>,
}

impl HostPolicy {
    /// ローカルホストだけ。
    pub fn local_only() -> Self {
        Self::default()
    }

    /// 名前か IP を足す（`100.64.0.1`・`fd00::1`・`my-pc.tailnet.ts.net`）。
    pub fn allow(mut self, host: &str) -> Self {
        let h = normalize(host.trim());
        if !h.is_empty() && !self.extra.contains(&h) {
            self.extra.push(h);
        }
        self
    }

    /// 待ち受けのアドレスと `--allow-host` から作る（`0.0.0.0`・`::` は名前にならないので足さない）。
    pub fn for_bind(host: IpAddr, allow: &[String]) -> Self {
        let mut p = Self::local_only();
        if !host.is_unspecified() {
            p = p.allow(&host.to_string());
        }
        allow.iter().fold(p, |p, h| p.allow(h))
    }

    /// Host ヘッダーの値（`name:port` か `[v6]:port`）を許すか。
    pub fn allows(&self, host_header: &str) -> bool {
        let name = normalize(strip_port(host_header.trim()));
        LOCAL_HOSTS.contains(&name.as_str()) || self.extra.contains(&name)
    }
}

/// 小文字にし、IPv6 の生の表記は `[...]` で囲む。
fn normalize(host: &str) -> String {
    let lower = host.to_ascii_lowercase();
    if !lower.starts_with('[') && lower.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("[{lower}]")
    } else {
        lower
    }
}

/// `example.com:8787` → `example.com`、`[::1]:8787` → `[::1]`。
fn strip_port(host: &str) -> &str {
    if host.starts_with('[') {
        return host.find(']').map_or(host, |end| &host[..=end]);
    }
    match host.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name,
        _ => host,
    }
}

/// 許可されない Host の要求を 403 で拒む。
pub async fn check(policy: Arc<HostPolicy>, req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| req.uri().authority().map(|a| a.to_string()));
    match host {
        Some(h) if !policy.allows(&h) => ApiError::new(
            StatusCode::FORBIDDEN,
            "host_not_allowed",
            format!("Requests from Host `{h}` are not accepted (allow it with `fune gui --allow-host <name>`)"),
        )
        .into_response(),
        _ => next.run(req).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_names_with_any_port() {
        let p = HostPolicy::local_only();
        for h in [
            "localhost",
            "localhost:8787",
            "127.0.0.1:5173",
            "[::1]:8787",
            "[::1]",
            "LOCALHOST",
        ] {
            assert!(p.allows(h), "{h}");
        }
        for h in [
            "evil.example",
            "evil.example:8787",
            "127.0.0.1.evil.example",
            "100.64.0.1:8787",
        ] {
            assert!(!p.allows(h), "{h}");
        }
    }

    #[test]
    fn bind_address_and_extra_names() {
        let p = HostPolicy::for_bind(
            "0.0.0.0".parse().unwrap(),
            &["100.64.0.1".into(), "fd00::1".into()],
        );
        assert!(p.allows("100.64.0.1:8787"));
        assert!(p.allows("[fd00::1]:8787"));
        assert!(!p.allows("0.0.0.0:8787"));
        let p = HostPolicy::for_bind("192.168.1.5".parse().unwrap(), &[]);
        assert!(p.allows("192.168.1.5:8787"));
    }
}
