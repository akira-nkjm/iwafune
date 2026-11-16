//! 本物の [`Host`]: `curl`・`tar`・実行中のバイナリの `--version`。

use super::Host;
use crate::error::{Result, ServiceError};
use std::path::Path;
use std::process::Command;

/// このコンピュータで実際に取得・展開・実行する。
pub(super) struct SystemHost;

impl Host for SystemHost {
    fn fetch_text(&self, url: &str) -> Result<String> {
        crate::fetch::text(url)
    }

    fn download(&self, url: &str, to: &Path, progress: bool) -> Result<()> {
        crate::fetch::download(url, to, progress)
    }

    fn extract(&self, archive: &Path, into: &Path) -> Result<()> {
        let output = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(into)
            .output()
            .map_err(|e| ServiceError::SelfUpdate(format!("Cannot start tar: {e}")))?;
        if output.status.success() {
            return Ok(());
        }
        Err(ServiceError::SelfUpdate(format!(
            "Cannot extract the downloaded archive: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }

    fn run_version(&self, binary: &Path) -> Result<String> {
        let output = Command::new(binary)
            .arg("--version")
            .output()
            .map_err(|e| {
                ServiceError::SelfUpdate(format!("Cannot run the downloaded binary: {e}"))
            })?;
        if !output.status.success() {
            return Err(ServiceError::SelfUpdate(
                "The downloaded binary failed to report its version".into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
