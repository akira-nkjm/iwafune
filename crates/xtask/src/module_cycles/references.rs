//! Rust paths in imports, expressions, types and macro tokens.
use super::{Module, is_test};
use syn::visit::{self, Visit};
use syn::{Item, UseTree};
pub(super) fn absolute(module: &[String], path: &[String], local: bool) -> Option<Module> {
    let (head, tail) = path.split_first()?;
    match head.as_str() {
        "crate" => Some(tail.to_vec()),
        "self" => {
            let mut p = module.to_vec();
            p.extend_from_slice(tail);
            Some(p)
        }
        "super" => {
            let mut p = module.to_vec();
            p.pop();
            absolute(&p, tail, true)
        }
        _ if local => {
            let mut p = module.to_vec();
            p.extend_from_slice(path);
            Some(p)
        }
        _ => None,
    }
}

pub(super) fn use_paths(tree: &UseTree, mut prefix: Module, out: &mut Vec<(Module, String)>) {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            use_paths(&p.tree, prefix, out);
        }
        UseTree::Name(n) => {
            let name = n.ident.to_string();
            if name != "self" {
                prefix.push(name.clone());
            }
            out.push((prefix, name));
        }
        UseTree::Rename(n) => {
            prefix.push(n.ident.to_string());
            out.push((prefix, n.rename.to_string()));
        }
        UseTree::Glob(_) => out.push((prefix, "*".into())),
        UseTree::Group(g) => {
            for t in &g.items {
                use_paths(t, prefix.clone(), out);
            }
        }
    }
}

#[derive(Default)]
pub(super) struct References {
    pub(super) paths: Vec<Module>,
}
impl<'ast> Visit<'ast> for References {
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {} // Includes doc comments.
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        let mut paths = Vec::new();
        use_paths(&u.tree, Vec::new(), &mut paths);
        self.paths.extend(paths.into_iter().map(|(p, _)| p));
    }
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Fn(f) = item
            && is_test(&f.attrs)
        {
            return;
        }
        visit::visit_item(self, item);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        self.macro_paths(m.tokens.clone());
    }
    fn visit_path(&mut self, p: &'ast syn::Path) {
        self.paths
            .push(p.segments.iter().map(|s| s.ident.to_string()).collect());
        visit::visit_path(self, p);
    }
}

impl References {
    fn macro_paths(&mut self, tokens: proc_macro2::TokenStream) {
        use proc_macro2::TokenTree;
        let tokens: Vec<_> = tokens.into_iter().collect();
        for (i, token) in tokens.iter().enumerate() {
            if let TokenTree::Group(g) = token {
                self.macro_paths(g.stream());
            }
            let TokenTree::Ident(head) = token else {
                continue;
            };
            if head != "crate" && head != "super" {
                continue;
            }
            let mut path = vec![head.to_string()];
            let mut tail = &tokens[i + 1..];
            while let [
                TokenTree::Punct(a),
                TokenTree::Punct(b),
                TokenTree::Ident(name),
                rest @ ..,
            ] = tail
            {
                if a.as_char() != ':' || b.as_char() != ':' {
                    break;
                }
                path.push(name.to_string());
                tail = rest;
            }
            if path.len() > 1 {
                self.paths.push(path);
            }
        }
    }
}
