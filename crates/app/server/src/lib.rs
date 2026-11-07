//! # fune-server — HTTP API（/api/v1）と SPA の配信
//!
//! ロジックは持たない。`fune_service` の関数を呼び、結果を JSON にするだけ。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `routes` | ルーティング |
//! | `error` | サービスのエラー → HTTP の状態と JSON |
//! | `host` | Host ヘッダーの許可リスト（DNS リバインディング対策） |
//! | `spa` | ビルド済みの web/dist を同梱して配信 |

mod error;
mod host;
mod routes;
mod spa;

pub use error::{ApiError, status_of};
pub use host::HostPolicy;
pub use routes::{AppState, router, router_with};

use fune_service::Workspace;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;

/// アドレスを確保する（既定は 127.0.0.1。ポート 0 なら空いているポート）。
pub async fn bind(host: IpAddr, port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::new(host, port)).await
}

/// 確保したアドレスで API と SPA を配信する。Host が `hosts` に無い要求は 403。
/// Ctrl-C（と SIGTERM）で受け付けを止め、処理中の要求を終えてから戻る。
pub async fn serve(
    listener: TcpListener,
    workspace: Arc<Workspace>,
    hosts: HostPolicy,
) -> std::io::Result<()> {
    axum::serve(listener, router_with(workspace, hosts))
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
