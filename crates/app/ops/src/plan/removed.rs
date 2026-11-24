//! 置き換える Op が、既にあるレコードから落としたキーの一覧（`fune case set` のような「全体を書く」Op の、書かなかった欄が
//! 黙って消えるのを知らせるため）。計画の結果（[`super::Plan`]・[`super::PlanSummary`]）に載り、CLI も HTTP も同じ一覧を返す。
//!
//! 見るのは、置き換える Op（[`replaces_record`]）が書き換えた既存のファイルだけ。新しく作るファイルや、キーを増やす・値を替えるだけの
//! 編集には何も載らない。キーの比べ方は `fune_store::write::removed`（TOML を読むのは store）。

use crate::op::Op;
use fune_store::FileSet;
use fune_store::write::removed::{has_key, removed_keys};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 置き換えで消えたキー（ファイル 1 つ分）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RemovedKeys {
    /// ルートからの相対パス（`/` 区切り）
    pub path: String,
    /// 消えたキー（ファイルの順。入れ子は `target.altitude` のようにドットでつなぐ）
    pub keys: Vec<String>,
}

/// Op が既にあるレコードを丸ごと置き換える（書かなかった欄は消える）ものか。
/// **どの Op が置き換えるかの一覧はここだけ**。部品の値・フィールド・設定のように 1 つの欄だけを書く Op は含めない。
pub(super) fn replaces_record(op: &Op) -> bool {
    matches!(
        op,
        Op::SetCase { .. }
            | Op::SetSource { .. }
            | Op::SetHarness { .. }
            | Op::SetManeuver { .. }
            | Op::SetPhase { .. }
            | Op::SetConfiguration { .. }
            | Op::SetRequirement { .. }
            | Op::SetSequence { .. }
            | Op::UpdateNet { .. }
            | Op::UpdateAttribute { .. }
            | Op::UpdateKind { .. }
    )
}

/// 置き換えの前後で、既存のファイルから消えたキー。
pub(super) fn between(before: &FileSet, after: &FileSet) -> Vec<RemovedKeys> {
    after
        .iter()
        .filter_map(|(path, now)| {
            let was = before.get(path).filter(|was| *was != now)?;
            let keys = removed_keys(was, now);
            (!keys.is_empty()).then(|| RemovedKeys {
                path: path.to_owned(),
                keys,
            })
        })
        .collect()
}

/// 全部の Op を当てた後の本文で、まだ消えているキーだけに絞る（あとの Op が書き戻したキーは、消えたと言わない）。
/// 同じファイルの知らせは 1 つにまとめる。
pub(super) fn settled(found: Vec<RemovedKeys>, finally: &FileSet) -> Vec<RemovedKeys> {
    let mut out: Vec<RemovedKeys> = Vec::new();
    for r in found {
        let Some(text) = finally.get(&r.path) else {
            continue;
        };
        let keys: Vec<String> = r.keys.into_iter().filter(|k| !has_key(text, k)).collect();
        if keys.is_empty() {
            continue;
        }
        match out.iter_mut().find(|o| o.path == r.path) {
            Some(o) => {
                for k in keys {
                    if !o.keys.contains(&k) {
                        o.keys.push(k);
                    }
                }
            }
            None => out.push(RemovedKeys { path: r.path, keys }),
        }
    }
    out
}
