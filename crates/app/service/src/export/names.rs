//! TS に出す型名の重複の検査（docs/architecture/registry.md §4: 型名はワークスペース全体で一意）。
//! ts-rs は同じ名前の型を同じファイルに書くので、重複すると片方が黙って消える。それを書き出しの時点で止める。

use crate::export::ExportError;
use std::any::TypeId;
use std::collections::BTreeMap;
use ts_rs::{Config, Dependency, TS, TypeVisitor};

/// 型名 → Rust の型の対応を集め、別の型が同じ名前を使っていないか確かめる。
pub(crate) struct NameCheck<'a> {
    cfg: &'a Config,
    seen: BTreeMap<String, (TypeId, &'static str)>,
    duplicates: Vec<String>,
}

impl<'a> NameCheck<'a> {
    pub(crate) fn new(cfg: &'a Config) -> Self {
        Self {
            cfg,
            seen: BTreeMap::new(),
            duplicates: Vec::new(),
        }
    }

    /// 型と、それが参照する型をすべて登録する。
    pub(crate) fn visit<T: TS + 'static + ?Sized>(&mut self) {
        TypeVisitor::visit::<T>(self);
    }

    /// 重複があればエラー。
    pub(crate) fn finish(self) -> Result<(), ExportError> {
        if self.duplicates.is_empty() {
            Ok(())
        } else {
            Err(ExportError::DuplicateNames(self.duplicates.join(", ")))
        }
    }
}

impl TypeVisitor for NameCheck<'_> {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let Some(dep) = Dependency::from_ty::<T>(self.cfg) else {
            T::visit_generics(self);
            return;
        };
        let id = TypeId::of::<T>();
        let rust = std::any::type_name::<T>();
        match self.seen.get(&dep.ts_name) {
            Some((seen, _)) if *seen == id => return,
            // 総称型（`Authored<Scalar>` と `Authored<Value>`）は 1 つの宣言 `Authored<T>` を共有するので重複ではない。
            Some((_, other)) if generic_base(other) == generic_base(rust) => return,
            Some((_, other)) => {
                self.duplicates
                    .push(format!("{} ({other} と {rust})", dep.ts_name));
                return;
            }
            None => {
                self.seen.insert(dep.ts_name, (id, rust));
            }
        }
        T::visit_dependencies(self);
        T::visit_generics(self);
    }
}

/// `a::B<c::D>` → `a::B`（型引数を外した名前）。
fn generic_base(rust: &str) -> &str {
    rust.split('<').next().unwrap_or(rust)
}
