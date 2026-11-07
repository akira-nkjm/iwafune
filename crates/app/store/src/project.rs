//! プロジェクトのルート（fune.toml）の探索と初期化（`fune init`）、git のブランチ名。

use crate::error::StoreError;
use crate::layout::{ALLOCATION_FILE, POLICY_FILE, PROJECT_FILE, WORK_DIR};
use crate::write::{FileChange, commit};
use std::path::{Path, PathBuf};

/// `start` から上へ fune.toml を探し、見つかったディレクトリを返す。
pub fn find_root(start: &Path) -> Result<PathBuf, StoreError> {
    let start = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| StoreError::io(start, e))?
            .join(start)
    };
    let mut dir = Some(start.as_path());
    while let Some(d) = dir {
        if d.join(PROJECT_FILE).is_file() {
            return Ok(d.to_path_buf());
        }
        dir = d.parent();
    }
    Err(StoreError::NotAProject {
        start: start.display().to_string(),
    })
}

/// git のブランチ名（`root` から上の `.git/HEAD` を読む。git でなければ None）。
/// 切り離された HEAD は commit の先頭 12 文字。worktree（`.git` がファイルで `gitdir: …`）にも対応する。
pub fn git_branch(root: &Path) -> Option<String> {
    let dot_git = root
        .ancestors()
        .map(|d| d.join(".git"))
        .find(|g| g.exists())?;
    let git_dir = if dot_git.is_file() {
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let target = text.trim().strip_prefix("gitdir:")?.trim().to_string();
        let target = PathBuf::from(target);
        if target.is_absolute() {
            target
        } else {
            dot_git.parent()?.join(target)
        }
    } else {
        dot_git
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    Some(head.strip_prefix("ref: refs/heads/").map_or_else(
        || head.get(..12).unwrap_or(head).to_string(),
        str::to_string,
    ))
}

/// 新しいプロジェクトの雛形（相対パス, 本文）。
pub fn template(name: &str) -> Vec<(String, String)> {
    let name = name.replace('"', "");
    vec![
        (
            PROJECT_FILE.to_string(),
            format!(
                "[project]\nname   = \"{name}\"\nschema = 2\ntoml   = \"1.0\"\n\n[groups]\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n"
            ),
        ),
        (
            ALLOCATION_FILE.to_string(),
            "[mass]\nlimit         = { value = 12.0, unit = \"kg\" }\nsystem_margin = 0.1\n\n[mass.alloc]\n".to_string(),
        ),
        (
            POLICY_FILE.to_string(),
            "[growth]\noff_the_shelf = 0.05\nmodified      = 0.1\nnew           = 0.2\n\n[uncertainty]\noff_the_shelf = 0.03\nmodified      = 0.1\nnew           = 0.2\n\n[harness]\nallowance = 0.05\n".to_string(),
        ),
        (
            "modes/SAFE.toml".to_string(),
            "label       = \"セーフ\"\ndescription = \"最小限の機器だけを動かす\"\n\n[states]\n".to_string(),
        ),
    ]
}

/// `dir` に新しいプロジェクトを作る（fune.toml が既にあれば誤り）。作ったファイルを返す。
pub fn init(dir: &Path, name: &str) -> Result<Vec<String>, StoreError> {
    if dir.join(PROJECT_FILE).exists() {
        return Err(StoreError::AlreadyExists {
            path: dir.join(PROJECT_FILE).display().to_string(),
        });
    }
    let mut files = template(name);
    files.push((".gitignore".to_string(), format!("{WORK_DIR}/\n")));
    let changes: Vec<FileChange> = files
        .iter()
        .filter(|(path, _)| !dir.join(path).exists())
        .map(|(path, text)| FileChange {
            path: path.clone(),
            before: None,
            after: Some(text.clone()),
        })
        .collect();
    commit(dir, &changes)?;
    Ok(changes.into_iter().map(|c| c.path).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_git_branch() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(git_branch(dir.path()), None);
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::create_dir_all(dir.path().join("sat")).unwrap();
        assert_eq!(git_branch(&dir.path().join("sat")).as_deref(), Some("main"));
        std::fs::write(dir.path().join(".git/HEAD"), "0123456789abcdef0123\n").unwrap();
        assert_eq!(git_branch(dir.path()).as_deref(), Some("0123456789ab"));
        let wt = tempfile::tempdir().unwrap();
        std::fs::write(
            wt.path().join(".git"),
            format!("gitdir: {}\n", dir.path().join(".git").display()),
        )
        .unwrap();
        assert_eq!(git_branch(wt.path()).as_deref(), Some("0123456789ab"));
    }

    #[test]
    fn init_then_find_root_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let created = init(dir.path(), "demo").unwrap();
        assert!(created.contains(&"fune.toml".to_string()));
        std::fs::create_dir_all(dir.path().join("parts/AOCS")).unwrap();
        let root = find_root(&dir.path().join("parts/AOCS")).unwrap();
        assert_eq!(root, dir.path());
        let snap = crate::read::load(&root).unwrap();
        assert_eq!(snap.design.project.meta.name, "demo");
        assert!(
            snap.design.load_diagnostics.is_empty(),
            "{:?}",
            snap.design.load_diagnostics
        );
        assert!(init(dir.path(), "again").is_err());
    }
}
