//! 進行の報告。サービスは表示も端末の判定もせず、いま何をしているかを型で伝える（表示は CLI の仕事）。

/// 置き換えの途中経過（起きる順）。失敗したら、その手前の段階までで止まる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// 最新のリリースを問い合わせている（`--version` を付けたときは起きない）
    CheckingLatest,
    /// 成果物を取得している
    Downloading {
        /// 成果物の名前
        asset: String,
        /// 取得する版
        version: String,
    },
    /// チェックサムを取得して照合している
    Verifying,
    /// 展開して、取り出したバイナリの版を確かめている
    Unpacking,
    /// 実行中のバイナリを置き換えている
    Replacing {
        /// 置き換えるバイナリの場所
        path: String,
    },
}

/// 進行の受け取り先。
pub type OnStep<'a> = &'a mut dyn FnMut(Step);
