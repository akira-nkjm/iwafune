//! store のエラー。

/// 読み書きのエラー。構文の誤りなど設計データ自体の問題は [`fune_model::Diagnostic`] として返し、ここには入れない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("{path}: {message}")]
    Io { path: String, message: String },
    #[error("{start} から上にプロジェクト（fune.toml）が見つかりません")]
    NotAProject { start: String },
    #[error("{path} は既にあります")]
    AlreadyExists { path: String },
    #[error("{path} がありません")]
    Missing { path: String },
    #[error("{file}: TOML を読めません: {message}")]
    Toml { file: String, message: String },
    #[error("{file}: 編集できません: {message}")]
    Edit { file: String, message: String },
    #[error("設定のパス `{path}` はどのファイルにも対応しません")]
    SettingPath { path: String },
    #[error("ほかのプロセスが書き込み中です（{lock}）")]
    Locked { lock: String },
    #[error("書き込みに失敗しました: {message}")]
    Commit { message: String },
}

impl StoreError {
    /// 入出力のエラーを包む。
    pub(crate) fn io(path: &std::path::Path, error: std::io::Error) -> Self {
        StoreError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        }
    }

    /// 編集のエラー。
    pub(crate) fn edit(file: &str, message: impl Into<String>) -> Self {
        StoreError::Edit {
            file: file.to_string(),
            message: message.into(),
        }
    }
}
