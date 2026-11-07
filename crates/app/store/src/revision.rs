//! revision（追跡ファイルの (path, bytes) の BLAKE3）。楽観ロックに使う。

use crate::fileset::FileSet;
use fune_model::design::Revision;

/// ファイルの集合の revision（`b3:` ＋ 16 進）。パスの順に (パス, 長さ, 本文) を流し込む。
pub fn of(files: &FileSet) -> Revision {
    let mut hasher = blake3::Hasher::new();
    for (path, contents) in files.iter() {
        hasher.update(path.as_bytes());
        hasher.update(&[0]);
        hasher.update(&(contents.len() as u64).to_le_bytes());
        hasher.update(contents.as_bytes());
    }
    Revision(format!("b3:{}", hasher.finalize().to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_changes_with_content_and_path() {
        let mut a = FileSet::new();
        a.insert("parts/A.toml", "kind = \"obc\"\n");
        let mut b = a.clone();
        assert_eq!(of(&a), of(&b));
        b.insert("parts/A.toml", "kind = \"pcdu\"\n");
        assert_ne!(of(&a), of(&b));
        let mut c = FileSet::new();
        c.insert("parts/B.toml", "kind = \"obc\"\n");
        assert_ne!(of(&a), of(&c));
        assert!(of(&a).0.starts_with("b3:"));
    }
}
