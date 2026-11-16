//! `fune gui`: HTTP サーバー（fune-server）を起動し、ブラウザで開く。

use crate::output::Out;
use fune_service::Workspace;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;

/// サーバーを起動して、Ctrl-C まで待つ。
pub fn run(
    ws: &Workspace,
    host: &str,
    port: Option<u16>,
    allow_host: &[String],
    no_open: bool,
    out: &Out,
) -> anyhow::Result<u8> {
    let host: IpAddr = host
        .parse()
        .map_err(|_| anyhow::anyhow!("--host is not a valid IP address: {host}"))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let ws = Arc::new(ws.clone());
    let hosts = fune_server::HostPolicy::for_bind(host, allow_host);
    runtime.block_on(async move {
        let listener = bind(host, port, out).await?;
        let addr = listener.local_addr()?;
        let url = browser_url(host, addr);
        out.ok(format!("Started the GUI: {url}"));
        println!(
            "{}",
            out.dim(format!(
                "Project: {}  (Ctrl-C to quit)",
                ws.root().display()
            ))
        );
        if !no_open && let Err(e) = open::that_detached(&url) {
            out.warn(format!(
                "Could not open the browser ({e}). Open {url} yourself"
            ));
        }
        if !allow_host.is_empty() {
            println!(
                "{}",
                out.dim(format!(
                    "Allowed Host: localhost and {}",
                    allow_host.join(", ")
                ))
            );
        }
        fune_server::serve(listener, ws, hosts).await?;
        out.ok("Stopped");
        anyhow::Ok(0)
    })
}

/// ブラウザで開く URL。ループバックと 0.0.0.0 で待ち受けているときは `localhost` の名前で開く
/// （Host の許可リストに入っている）。ほかのアドレスはそのアドレスで開く。
fn browser_url(host: IpAddr, addr: SocketAddr) -> String {
    if host.is_loopback() || host.is_unspecified() {
        format!("http://localhost:{}/", addr.port())
    } else {
        format!("http://{addr}/")
    }
}

/// `--port` を付けないときのポート。
const DEFAULT_PORT: u16 = 3000;
/// 既定のポートが使われているときに、続けて試すポートの数。
const FALLBACK_PORTS: u16 = 10;

/// 待ち受けるポートを確保する。`--port` で決めたポートが使われていればエラー。決めていなければ既定のポートから
/// 順に空いているものを探し、既定と違うポートになったらそう知らせる。
async fn bind(host: IpAddr, port: Option<u16>, out: &Out) -> anyhow::Result<TcpListener> {
    if let Some(port) = port {
        return fune_server::bind(host, port)
            .await
            .map_err(|e| bind_error(e, port));
    }
    for candidate in DEFAULT_PORT..DEFAULT_PORT + FALLBACK_PORTS {
        match fune_server::bind(host, candidate).await {
            Ok(listener) => {
                if candidate != DEFAULT_PORT {
                    out.warn(format!(
                        "Port {DEFAULT_PORT} is in use, so the GUI uses port {candidate}"
                    ));
                }
                return Ok(listener);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {}
            Err(e) => return Err(e.into()),
        }
    }
    anyhow::bail!(
        "Ports {DEFAULT_PORT} to {} are all in use. Choose one with --port",
        DEFAULT_PORT + FALLBACK_PORTS - 1
    )
}

/// 待ち受けに失敗したときのエラー。ポートが使われているときは `--port` で変えられると案内する。
fn bind_error(e: std::io::Error, port: u16) -> anyhow::Error {
    if e.kind() == std::io::ErrorKind::AddrInUse {
        anyhow::anyhow!("Port {port} is already in use. Choose another one with --port")
    } else {
        e.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_and_all_interfaces_open_as_localhost() {
        for host in ["127.0.0.1", "::1", "0.0.0.0"] {
            let host: IpAddr = host.parse().unwrap();
            let url = browser_url(host, SocketAddr::new(host, 3000));
            assert_eq!(url, "http://localhost:3000/", "{host}");
        }
    }

    #[test]
    fn another_address_opens_as_that_address() {
        let host: IpAddr = "192.168.1.5".parse().unwrap();
        let url = browser_url(host, SocketAddr::new(host, 3000));
        assert_eq!(url, "http://192.168.1.5:3000/");
    }

    #[test]
    fn a_port_in_use_points_to_the_port_flag() {
        let e = std::io::Error::from(std::io::ErrorKind::AddrInUse);
        assert!(bind_error(e, 3000).to_string().contains("--port"));
    }
}
