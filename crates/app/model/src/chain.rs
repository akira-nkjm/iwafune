//! 連鎖のたどり方（取付の `mount.on`・製品ツリーの親・種別の `extends`）。**循環を見つけて止める連鎖の走査はここだけ**。
//!
//! 部品の取付の連鎖（`Resolved::pose_chain`・`Resolved::mount_root`・検査の `mount.cycle`）、製品ツリーの祖先
//! （`Design::ancestors`）、種別の継承（`Design::kind_chain`）が使う。

/// `first` から `next` で次をたどる反復子。同じキーのものに戻ったら（循環）そこで止まり、[`Chain::cycled`] が true になる。
pub struct Chain<T, K, KF, NF> {
    current: Option<T>,
    seen: Vec<K>,
    key: KF,
    next: NF,
    cycled: bool,
}

/// 連鎖をたどる。`key` は循環の判定に使う名前（ID）、`next` は次のもの（無ければ終わり）。
pub fn chain<T, K, KF, NF>(first: Option<T>, key: KF, next: NF) -> Chain<T, K, KF, NF>
where
    K: PartialEq,
    KF: Fn(&T) -> K,
    NF: FnMut(&T) -> Option<T>,
{
    Chain {
        current: first,
        seen: Vec::new(),
        key,
        next,
        cycled: false,
    }
}

impl<T, K, KF, NF> Chain<T, K, KF, NF> {
    /// 循環で止まったか（たどり終えてから読む）。
    pub fn cycled(&self) -> bool {
        self.cycled
    }
}

impl<T, K, KF, NF> Iterator for Chain<T, K, KF, NF>
where
    K: PartialEq,
    KF: Fn(&T) -> K,
    NF: FnMut(&T) -> Option<T>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let item = self.current.take()?;
        let key = (self.key)(&item);
        if self.seen.contains(&key) {
            self.cycled = true;
            return None;
        }
        self.seen.push(key);
        self.current = (self.next)(&item);
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::chain;
    use std::collections::BTreeMap;

    fn links(pairs: &[(&'static str, &'static str)]) -> BTreeMap<&'static str, &'static str> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn follows_until_the_end_or_a_cycle() {
        let parent = links(&[("a", "b"), ("b", "c")]);
        let mut it = chain(Some("a"), |s| *s, |s| parent.get(s).copied());
        assert_eq!(it.by_ref().collect::<Vec<_>>(), ["a", "b", "c"]);
        assert!(!it.cycled());

        let looped = links(&[("a", "b"), ("b", "c"), ("c", "a")]);
        let mut it = chain(Some("a"), |s| *s, |s| looped.get(s).copied());
        assert_eq!(it.by_ref().collect::<Vec<_>>(), ["a", "b", "c"]);
        assert!(it.cycled());

        let selfish = links(&[("a", "a")]);
        let mut it = chain(Some("a"), |s| *s, |s| selfish.get(s).copied());
        assert_eq!(it.by_ref().count(), 1);
        assert!(it.cycled());

        let mut empty = chain(None::<&str>, |s| *s, |_| None);
        assert_eq!(empty.next(), None);
        assert!(!empty.cycled());
    }
}
