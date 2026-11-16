//! `fune gui`: HTTP サーバー（fune-server）を起動し、ブラウザで開く。

use crate::output::Out;
use fune_service::Workspace;
use std::net::IpAddr;
use std::sync::Arc;

/// サーバーを起動して、Ctrl-C まで待つ。
pub fn run(
    ws: &Workspace,
    host: &str,
    port: u16,
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
        let listener = fune_server::bind(host, port).await?;
        let addr = listener.local_addr()?;
        // 0.0.0.0 で待ち受けても、ブラウザはローカルホストの名前で開く（Host の許可リストに入っているため）。
        let url = if host.is_unspecified() {
            format!("http://127.0.0.1:{}/", addr.port())
        } else {
            format!("http://{addr}/")
        };
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
