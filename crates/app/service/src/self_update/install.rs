//! 置き換えの手順。どこで失敗しても実行中のバイナリには触れず、一時ディレクトリを消す。

use super::asset::{archive_name, dir_name, download_url, parse_sha256_file};
use super::{Context, Host, OnStep, Step, Version};
use crate::data::sha256_file;
use crate::error::{Result, ServiceError};
use std::path::{Path, PathBuf};

fn err(message: impl Into<String>) -> ServiceError {
    ServiceError::SelfUpdate(message.into())
}

/// 実行中のバイナリの隣に作る一時ディレクトリ（同じファイルシステムなので最後の `rename` が不可分になる）。
/// 落としたときも（成功・失敗とも）消す。
struct TempDir(PathBuf);

impl TempDir {
    fn create_beside(exe: &Path) -> Result<Self> {
        let dir = exe
            .parent()
            .ok_or_else(|| err("The fune binary has no parent directory"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let path = dir.join(format!(".fune-update-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&path).map_err(|e| {
            err(format!(
                "Cannot write to {} ({e}). fune is installed in a location this user cannot write to; run the update with the right permissions or reinstall another way",
                dir.display()
            ))
        })?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `version` を取得して検証し、実行中のバイナリを置き換える。置き換えたバイナリの場所を返す。
/// 各段階の始まりを `on_step` に伝える。`progress` は成果物の取得だけに効く（小さなチェックサムには出さない）。
pub(super) fn install(
    ctx: &Context,
    host: &dyn Host,
    version: Version,
    progress: bool,
    on_step: OnStep,
) -> Result<PathBuf> {
    let tmp = TempDir::create_beside(&ctx.exe)?;
    let file = archive_name(version, ctx.platform);
    let archive = tmp.0.join(&file);
    on_step(Step::Downloading {
        asset: file.clone(),
        version: version.to_string(),
    });
    host.download(&download_url(&ctx.repo, version, &file), &archive, progress)
        .map_err(|e| not_found_or(ctx, version, e))?;
    on_step(Step::Verifying);
    verify_checksum(ctx, host, version, &tmp.0, &file)?;
    on_step(Step::Unpacking);
    let new_binary = extract_binary(ctx, host, version, &tmp.0, &archive)?;
    check_version(host, &new_binary, version)?;
    on_step(Step::Replacing {
        path: ctx.exe.display().to_string(),
    });
    replace(&new_binary, &ctx.exe)?;
    Ok(ctx.exe.clone())
}

/// 取得の失敗が 404 なら、リリースが無いことを伝える（そうでなければそのまま）。
fn not_found_or(ctx: &Context, version: Version, e: ServiceError) -> ServiceError {
    if e.to_string().contains("404") {
        return err(format!(
            "Release {version} was not found (see {}/releases)",
            ctx.repo
        ));
    }
    e
}

/// アーカイブの SHA-256 を `.sha256` と照合する。
fn verify_checksum(
    ctx: &Context,
    host: &dyn Host,
    version: Version,
    tmp: &Path,
    file: &str,
) -> Result<()> {
    let sum_name = format!("{file}.sha256");
    let sum_path = tmp.join(&sum_name);
    host.download(
        &download_url(&ctx.repo, version, &sum_name),
        &sum_path,
        false,
    )?;
    let text = std::fs::read_to_string(&sum_path)
        .map_err(|e| err(format!("Cannot read {sum_name}: {e}")))?;
    let expected = parse_sha256_file(&text, file)?;
    let actual = sha256_file(&tmp.join(file))
        .map_err(|e| err(format!("Cannot read the downloaded archive: {e}")))?;
    if actual != expected {
        return Err(err(format!(
            "The SHA-256 of {file} does not match its checksum file (expected {expected}, got {actual}). Nothing was replaced"
        )));
    }
    Ok(())
}

/// 展開して `fune-<版>-<環境>/fune` の場所を返す。
fn extract_binary(
    ctx: &Context,
    host: &dyn Host,
    version: Version,
    tmp: &Path,
    archive: &Path,
) -> Result<PathBuf> {
    let out = tmp.join("extracted");
    std::fs::create_dir(&out).map_err(|e| err(format!("Cannot create {}: {e}", out.display())))?;
    host.extract(archive, &out)?;
    let dir = dir_name(version, ctx.platform);
    let binary = out.join(&dir).join("fune");
    let is_file = std::fs::symlink_metadata(&binary).is_ok_and(|m| m.is_file());
    if !is_file {
        return Err(err(format!("The archive does not contain {dir}/fune")));
    }
    Ok(binary)
}

/// 取り出したバイナリが `fune <版>` と名乗ることを確かめる。
fn check_version(host: &dyn Host, binary: &Path, version: Version) -> Result<()> {
    set_executable(binary)?;
    let reported = host.run_version(binary)?;
    let expected = format!("fune {version}");
    if reported.trim() != expected {
        return Err(err(format!(
            "The downloaded binary reports '{}' instead of '{expected}'. Nothing was replaced",
            reported.trim()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| err(format!("Cannot make {} executable: {e}", path.display())))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// 新しいバイナリを実行中のバイナリの上へ `rename` する（Unix では実行中でも安全）。
/// 一時ディレクトリは実行中のバイナリと同じディレクトリにあるので、同じファイルシステムで不可分に入れ替わる。
fn replace(new_binary: &Path, exe: &Path) -> Result<()> {
    std::fs::rename(new_binary, exe).map_err(|e| {
        err(format!(
            "Cannot replace {} ({e}). Nothing was replaced",
            exe.display()
        ))
    })
}
