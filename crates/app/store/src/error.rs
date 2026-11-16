//! store のエラー。

/// 読み書きのエラー。構文の誤りなど設計データ自体の問題は [`fune_model::Diagnostic`] として返し、ここには入れない。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("{path}: {message}")]
    Io { path: String, message: String },
    #[error("No project (fune.toml) found at or above {start}")]
    NotAProject { start: String },
    #[error("{path} already exists")]
    AlreadyExists { path: String },
    #[error("{path} not found")]
    Missing { path: String },
    #[error("{file}: cannot read TOML: {message}")]
    Toml { file: String, message: String },
    #[error("{file}: cannot edit: {message}")]
    Edit { file: String, message: String },
    #[error("Setting path `{path}` does not map to any file")]
    SettingPath { path: String },
    #[error("Another process is writing ({lock})")]
    Locked { lock: String },
    #[error("Write failed: {message}")]
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
