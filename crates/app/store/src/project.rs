//! プロジェクトのルート（fune.toml）の探索と初期化（`fune init`）、git のブランチ名。

use crate::error::StoreError;
use crate::layout::{ALLOCATION_FILE, POLICY_FILE, PROJECT_FILE, WORK_DIR, mode_file};
use crate::write::entities::new_mode_text;
use crate::write::project::{new_allocation_text, new_policy_text, new_project_text};
use crate::write::{FileChange, commit};
use fune_model::design::{
    Allocation, Allocations, BudgetPolicy, CatalogSource, HarnessAllowance, MaturityFactors, Mode,
    Project, ProjectMeta,
};
use fune_model::id::Id;
use std::collections::BTreeMap;
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

/// 新しいプロジェクトの雛形の中身（設定・配分・方針・モード）。本文は store の書き出し関数（`write::project`・
/// `write::entities::new_mode_text`）が作る。
fn template_design(name: &str) -> (Project, Allocations, BudgetPolicy, Mode) {
    let project = Project {
        meta: ProjectMeta {
            name: name.replace('"', ""),
            ..ProjectMeta::default()
        },
        modes: None,
        catalogs: vec![CatalogSource {
            name: "local".to_owned(),
            path: Some("catalog".to_owned()),
            git: None,
            rev: None,
        }],
    };
    let allocations = Allocations {
        mass: Some(Allocation {
            limit: None,
            system_margin: 0.1,
            alloc: BTreeMap::new(),
        }),
        power: None,
        data: None,
    };
    let factors = |off_the_shelf, modified, new| MaturityFactors {
        off_the_shelf: Some(off_the_shelf),
        modified: Some(modified),
        new: Some(new),
    };
    let policy = BudgetPolicy {
        growth: factors(0.05, 0.1, 0.2),
        uncertainty: factors(0.03, 0.1, 0.2),
        harness: HarnessAllowance { allowance: 0.05 },
    };
    let safe = Mode {
        id: Id::new("SAFE"),
        label: None,
        description: Some("最小限の機器だけを動かす".to_owned()),
        states: BTreeMap::new(),

        max_duration: None,
        eclipse: Default::default(),
    };
    (project, allocations, policy, safe)
}

/// 新しいプロジェクトの雛形（相対パス, 本文）。
pub fn template(name: &str) -> Result<Vec<(String, String)>, StoreError> {
    let (project, allocations, policy, safe) = template_design(name);
    Ok(vec![
        (PROJECT_FILE.to_owned(), new_project_text(&project)?),
        (
            ALLOCATION_FILE.to_owned(),
            new_allocation_text(&allocations),
        ),
        (POLICY_FILE.to_owned(), new_policy_text(&policy)?),
        (mode_file(safe.id.as_str()), new_mode_text(&safe)),
    ])
}

/// `dir` に新しいプロジェクトを作る（fune.toml が既にあれば誤り）。作ったファイルを返す。
pub fn init(dir: &Path, name: &str) -> Result<Vec<String>, StoreError> {
    if dir.join(PROJECT_FILE).exists() {
        return Err(StoreError::AlreadyExists {
            path: dir.join(PROJECT_FILE).display().to_string(),
        });
    }
    let mut files = template(name)?;
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

    /// 雛形は任意の質量上限やモードの表示名を持たず、正規形で書き出す。
    #[test]
    fn template_text_is_unchanged() {
        let files: std::collections::BTreeMap<String, String> =
            template("sample").unwrap().into_iter().collect();
        let expected = [
            (
                "fune.toml",
                "[project]\nname   = \"sample\"\nschema = 2\ntoml   = \"1.0\"\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n",
            ),
            (
                "budget/allocation.toml",
                "[mass]\nsystem_margin = 0.1\n\n[mass.alloc]\n",
            ),
            (
                "budget/policy.toml",
                "[growth]\noff_the_shelf = 0.05\nmodified      = 0.1\nnew           = 0.2\n\n[uncertainty]\noff_the_shelf = 0.03\nmodified      = 0.1\nnew           = 0.2\n\n[harness]\nallowance = 0.05\n",
            ),
            (
                "modes/SAFE.toml",
                "description = \"最小限の機器だけを動かす\"\n\n[states]\n",
            ),
        ];
        assert_eq!(files.len(), expected.len(), "{:?}", files.keys());
        for (path, text) in expected {
            assert_eq!(files.get(path).map(String::as_str), Some(text), "{path}");
        }
        // 名前の `"` は落とす（引用の中に書けないため）
        let quoted: std::collections::BTreeMap<String, String> =
            template("a\"b").unwrap().into_iter().collect();
        assert!(quoted["fune.toml"].contains("name   = \"ab\""));
        // 雛形は正規形（`fune fmt` で変わらない）
        for (path, text) in &files {
            assert_eq!(
                &crate::format::format_text(path, text).unwrap(),
                text,
                "{path}"
            );
        }
    }

    #[test]
    fn init_then_find_root_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let created = init(dir.path(), "demo").unwrap();
        assert!(created.contains(&"fune.toml".to_string()));
        std::fs::create_dir_all(dir.path().join("parts/aocs")).unwrap();
        let root = find_root(&dir.path().join("parts/aocs")).unwrap();
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
