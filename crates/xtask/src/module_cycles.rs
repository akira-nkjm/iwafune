//! Sibling-module dependency cycles in production Rust. Re-exports are transparent aliases:
//! references resolve to their defining module, not to the public facade. Parent/child
//! references stay inside that module; edges are projected at their common ancestor.
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::Item;
use syn::visit::Visit;
mod references;
use references::{References, absolute, use_paths};

type Module = Vec<String>;
type Graph = BTreeMap<Module, BTreeSet<Module>>;

#[derive(Default)]
struct Sources {
    items: BTreeMap<Module, Vec<Item>>,
    aliases: BTreeMap<Module, Module>,
    globs: BTreeMap<Module, Vec<Module>>,
}

/// Check every crate separately, without compiling it or reading target directories.
pub fn check(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    for family in ["crates/lib", "crates/app", "crates"] {
        for entry in std::fs::read_dir(root.join(family))? {
            let dir = entry?.path();
            let src = dir.join("src");
            if !src.is_dir() {
                continue;
            }
            let mut sources = Sources::default();
            // lib and bin are separate module trees, even in the same package.
            for main in ["lib.rs", "main.rs"] {
                let path = src.join(main);
                if !path.exists() {
                    continue;
                }
                sources.load(&path, Vec::new())?;
                let graph = sources.graph();
                for cycle in cycles(&graph) {
                    errors.push(format!(
                        "{} ({main}): module cycle: {}",
                        dir.display(),
                        cycle
                            .iter()
                            .map(|p| p.join("::"))
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    ));
                }
                sources = Sources::default();
            }
        }
    }
    Ok(())
}

fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.parse_args::<syn::Path>()
                .is_ok_and(|p| p.is_ident("test"))
    })
}

impl Sources {
    fn load(&mut self, file: &Path, module: Module) -> Result<()> {
        let text = std::fs::read_to_string(file)?;
        let parsed = syn::parse_file(&text).with_context(|| file.display().to_string())?;
        let dir = if matches!(
            file.file_name().and_then(|n| n.to_str()),
            Some("mod.rs" | "lib.rs" | "main.rs")
        ) {
            file.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            file.with_extension("")
        };
        self.collect(parsed.items, module, &dir, file.parent().unwrap_or(&dir))
    }

    fn collect(
        &mut self,
        items: Vec<Item>,
        module: Module,
        dir: &Path,
        file_dir: &Path,
    ) -> Result<()> {
        let mut production = Vec::new();
        for item in items {
            if let Item::Mod(m) = &item {
                if is_test(&m.attrs) {
                    continue;
                }
                let mut child = module.clone();
                child.push(m.ident.to_string());
                if let Some((_, items)) = &m.content {
                    self.collect(items.clone(), child, &dir.join(m.ident.to_string()), dir)?;
                } else {
                    let path = module_file(m, dir, file_dir);
                    if path.exists() {
                        self.load(&path, child)?;
                    }
                }
            } else {
                if let Item::Use(u) = &item {
                    if is_test(&u.attrs) {
                        continue;
                    }
                    let mut paths = Vec::new();
                    use_paths(&u.tree, Vec::new(), &mut paths);
                    for (path, name) in paths {
                        if let Some(target) = absolute(&module, &path, true) {
                            if name == "*" {
                                self.globs.entry(module.clone()).or_default().push(target);
                            } else {
                                let mut alias = module.clone();
                                alias.push(name);
                                self.aliases.insert(alias, target);
                            }
                        }
                    }
                }
                production.push(item);
            }
        }
        self.items.insert(module, production);
        Ok(())
    }

    fn resolve(&self, mut path: Module) -> Module {
        let mut seen = BTreeSet::new();
        while seen.insert(path.clone()) {
            let mut next = None;
            for i in (1..=path.len()).rev() {
                if self.items.contains_key(&path[..i]) {
                    continue;
                }
                if let Some(target) = self.aliases.get(&path[..i]) {
                    let mut p = target.clone();
                    p.extend_from_slice(&path[i..]);
                    if p != path {
                        next = Some(p);
                        break;
                    }
                }
            }
            if next.is_none() {
                for i in (0..path.len()).rev() {
                    if self.items.contains_key(&path[..i]) {
                        if let Some(globs) = self.globs.get(&path[..i]) {
                            for glob in globs {
                                let mut candidate = glob.clone();
                                candidate.extend_from_slice(&path[i..]);
                                if self.known(&candidate) {
                                    next = Some(candidate);
                                    break;
                                }
                            }
                        }
                        break;
                    }
                }
            }
            match next {
                Some(p) => path = p,
                None => break,
            }
        }
        while !self.items.contains_key(&path) && !path.is_empty() {
            path.pop();
        }
        path
    }

    fn known(&self, path: &[String]) -> bool {
        if self.aliases.contains_key(path) || self.items.contains_key(path) {
            return true;
        }
        let Some((name, parent)) = path.split_last() else {
            return false;
        };
        self.items
            .get(parent)
            .is_some_and(|items| items.iter().any(|i| item_name(i).as_deref() == Some(name)))
    }

    fn graph(&self) -> Graph {
        let mut graph = Graph::new();
        for (module, items) in &self.items {
            let mut refs = References::default();
            for item in items {
                // Re-exports declare public paths, not implementation dependencies.
                if matches!(item, Item::Use(u) if !matches!(u.vis, syn::Visibility::Inherited)) {
                    continue;
                }
                refs.visit_item(item);
            }
            for path in refs.paths {
                let Some(target) = absolute(module, &path, false) else {
                    continue;
                };
                let target = self.resolve(target);
                let common = module
                    .iter()
                    .zip(&target)
                    .take_while(|(a, b)| a == b)
                    .count();
                if common == module.len() || common == target.len() {
                    continue;
                }
                graph
                    .entry(module[..=common].to_vec())
                    .or_default()
                    .insert(target[..=common].to_vec());
            }
        }
        graph
    }
}

fn item_name(item: &Item) -> Option<String> {
    Some(
        match item {
            Item::Struct(i) => &i.ident,
            Item::Enum(i) => &i.ident,
            Item::Trait(i) => &i.ident,
            Item::Type(i) => &i.ident,
            Item::Fn(i) => &i.sig.ident,
            Item::Const(i) => &i.ident,
            Item::Static(i) => &i.ident,
            _ => return None,
        }
        .to_string(),
    )
}

fn cycles(graph: &Graph) -> Vec<Vec<Module>> {
    fn walk(
        node: &Module,
        graph: &Graph,
        stack: &mut Vec<Module>,
        done: &mut BTreeSet<Module>,
        out: &mut Vec<Vec<Module>>,
    ) {
        if let Some(i) = stack.iter().position(|n| n == node) {
            let mut cycle = stack[i..].to_vec();
            cycle.push(node.clone());
            out.push(cycle);
            return;
        }
        if done.contains(node) {
            return;
        }
        stack.push(node.clone());
        if let Some(next) = graph.get(node) {
            for n in next {
                walk(n, graph, stack, done, out);
            }
        }
        stack.pop();
        done.insert(node.clone());
    }
    let mut out = Vec::new();
    let mut done = BTreeSet::new();
    for node in graph.keys() {
        walk(node, graph, &mut Vec::new(), &mut done, &mut out);
    }
    out
}

fn module_file(m: &syn::ItemMod, dir: &Path, file_dir: &Path) -> std::path::PathBuf {
    let explicit = m.attrs.iter().find_map(|a| {
        if !a.path().is_ident("path") {
            return None;
        }
        let syn::Meta::NameValue(n) = &a.meta else {
            return None;
        };
        let syn::Expr::Lit(l) = &n.value else {
            return None;
        };
        let syn::Lit::Str(s) = &l.lit else {
            return None;
        };
        Some(file_dir.join(s.value()))
    });
    let flat = dir.join(format!("{}.rs", m.ident));
    explicit.unwrap_or_else(|| {
        if flat.exists() {
            flat
        } else {
            dir.join(m.ident.to_string()).join("mod.rs")
        }
    })
}

#[cfg(test)]
mod tests;
