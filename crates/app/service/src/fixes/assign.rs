//! 種別を替える候補（`Fix::AssignPart`）の絞り込み。**替えられるかは、`fune set <部品>.kind <種別>` と同じ Op を
//! `fune_ops::plan` で計画して決める**（検査の規則はここに写さない）。新しい検査のエラーが出る組（カタログ品を選んだ部品、
//! 種別に無い状態ごとの値を持つ部品など）は、選ばせても確定できないので候補に出さない。

use fune_model::design::PartField;
use fune_ops::Op;
use fune_store::Snapshot as Stored;

use std::collections::HashMap;
use std::sync::Mutex;

use fune_model::design::Revision;

use crate::workspace::Snapshot;

/// 絞り込みの結果の覚え（ファイルの revision・候補の部品・種別が同じなら、同じ結果）。
/// 1 回の計画は設計を読み直すので（試した範囲ではこれが時間の大半）、解析・カード・表が同じ不足を出すたびに繰り返さない。
type Key = (Revision, Vec<String>, Vec<String>);
type Narrowed = (Vec<String>, Vec<String>);
static MEMO: Mutex<Option<HashMap<Key, Narrowed>>> = Mutex::new(None);
/// 覚えておく数の上限（超えたら捨てて作り直す。編集のたびに revision が変わるので、古いものは使われない）
const MEMO_LIMIT: usize = 32;

/// 候補の部品と種別のうち、少なくとも 1 つの相手と組んで計画が通るものだけ（順は元のまま）。
pub(super) fn narrow(s: &Snapshot, parts: Vec<String>, kinds: Vec<String>) -> Narrowed {
    let key = (
        fune_store::revision::of(&s.files),
        parts.clone(),
        kinds.clone(),
    );
    if let Some(hit) = MEMO
        .lock()
        .ok()
        .and_then(|m| m.as_ref()?.get(&key).cloned())
    {
        return hit;
    }
    let out = compute(s, parts, kinds);
    if let Ok(mut m) = MEMO.lock() {
        let m = m.get_or_insert_with(HashMap::new);
        if m.len() >= MEMO_LIMIT {
            m.clear();
        }
        m.insert(key, out.clone());
    }
    out
}

/// 同時に動かす計画の数の上限（1 回の計画は独立で、設計を読み直すのが重いので、コアに分けて回す）
const MAX_THREADS: usize = 8;

/// 計画して絞り込む（覚えは [`narrow`]）。部品ごとの計画は互いに独立なので、部品を分けて並べて回す。
fn compute(s: &Snapshot, parts: Vec<String>, kinds: Vec<String>) -> Narrowed {
    let stored = Stored {
        root: std::path::PathBuf::new(),
        files: s.files.clone(),
        design: s.design.clone(),
    };
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, MAX_THREADS);
    let chunk = parts.len().div_ceil(threads).max(1);
    // 部品ごとの、替えられる種別の添字
    let ok: Vec<Vec<usize>> = std::thread::scope(|scope| {
        let handles: Vec<_> = parts
            .chunks(chunk)
            .map(|group| {
                let (stored, kinds) = (&stored, &kinds);
                scope.spawn(move || {
                    group
                        .iter()
                        .map(|part| {
                            (0..kinds.len())
                                .filter(|&k| can_assign(stored, part, &kinds[k]))
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    let kept = parts
        .into_iter()
        .zip(&ok)
        .filter_map(|(p, k)| (!k.is_empty()).then_some(p))
        .collect();
    let kinds = kinds
        .into_iter()
        .enumerate()
        .filter_map(|(i, k)| ok.iter().any(|o| o.contains(&i)).then_some(k))
        .collect();
    (kept, kinds)
}

/// 部品の種別を替える Op が、新しい検査のエラーなしに計画できるか。
fn can_assign(stored: &Stored, part: &str, kind: &str) -> bool {
    let op = Op::SetField {
        part: part.to_owned(),
        field: PartField::Kind(Some(kind.to_owned())),
    };
    fune_ops::plan(stored, &[op]).is_ok()
}
