use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use anyhow::Result;
use syn::{Item, UseTree, Visibility, Generics, TypeParamBound, GenericParam, WhereClause, Expr, Pat, Stmt, Field, Variant, FnArg, Receiver};

/// A canonical structural fingerprint for a Rust declaration.
/// Two declarations that produce the same fingerprint use the same
/// crate patterns and are structurally isomorphic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclFingerprint {
    /// The kind of declaration (struct, enum, fn, trait, impl, type, use, mod, const, static, macro)
    pub kind: String,
    /// Sorted list of external crate names used (from use statements)
    pub crates_used: Vec<String>,
    /// Structural shape signature — a hash of the AST shape (fields, generics, bounds, etc.)
    pub structural_shape: u64,
    /// Sorted list of trait bounds and generic constraints
    pub trait_bounds: Vec<String>,
    /// Visibility class (pub, pub(crate), pub(super), private)
    pub visibility_class: String,
    /// Whether the declaration has async, unsafe, or const markers
    pub qualifiers: Vec<String>,
}

/// High-level fingerprint with file and project context — used by the pattern matcher.
#[derive(Debug, Clone, Serialize)]
pub struct PatternFingerprint {
    pub file: std::path::PathBuf,
    pub project: String,
    pub usages: BTreeSet<(String, String)>,
}

impl PatternFingerprint {
    /// Compute a PatternFingerprint by reading and parsing a source file.
    pub fn from_file(file: &Path, project: &str) -> Result<Self> {
        let source = std::fs::read_to_string(file)?;
        let syntax = syn::parse_file(&source)?;
        let mut usages = BTreeSet::new();
        for item in &syntax.items {
            let fp = DeclFingerprint::from_item(item, &[]);
            for (cat, val) in fp.entries() {
                usages.insert((cat, val));
            }
        }
        Ok(PatternFingerprint {
            file: file.to_path_buf(),
            project: project.to_string(),
            usages,
        })
    }
}

impl DeclFingerprint {
    /// Compute a fingerprint from a syn Item by analyzing the AST shape.
    pub fn from_item(item: &Item, use_trees: &[UseTree]) -> Self {
        let kind = item_to_kind(item);
        let crates_used = extract_crate_uses(use_trees);
        let (bounds, _generics_info) = extract_generics(item);
        let shape_hash = compute_shape_hash(item);
        let vis_class = visibility_class(&extract_visibility(item));
        let qualifiers = extract_qualifiers(item);

        DeclFingerprint {
            kind,
            crates_used,
            structural_shape: shape_hash,
            trait_bounds: bounds,
            visibility_class: vis_class,
            qualifiers,
        }
    }
    /// Convert fingerprint fields into (category, value) pairs for comparison.
    pub fn entries(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        result.push(("kind".to_string(), self.kind.clone()));
        for c in &self.crates_used {
            result.push(("crate".to_string(), c.clone()));
        }
        for b in &self.trait_bounds {
            result.push(("trait_bound".to_string(), b.clone()));
        }
        result.push(("visibility".to_string(), self.visibility_class.clone()));
        for q in &self.qualifiers {
            result.push(("qualifier".to_string(), q.clone()));
        }
        result.push(("shape".to_string(), format!("{}", self.structural_shape)));
        result.sort();
        result
    }
}

fn extract_visibility(item: &Item) -> syn::Visibility {
    match item {
        Item::Struct(s) => s.vis.clone(),
        Item::Enum(e) => e.vis.clone(),
        Item::Fn(f) => f.vis.clone(),
        Item::Trait(t) => t.vis.clone(),
        Item::Impl(_) => syn::Visibility::Public(syn::VisPublic { pub_token: Default::default() }),
        Item::Type(t) => t.vis.clone(),
        Item::Mod(m) => m.vis.clone(),
        Item::Const(c) => c.vis.clone(),
        Item::Static(s) => s.vis.clone(),
        Item::Union(u) => u.vis.clone(),
        Item::Use(_) | Item::ExternCrate(_) | Item::ForeignMod(_) | Item::Macro(_) | _ => {
            syn::Visibility::Inherited
        }
    }
}

fn item_to_kind(item: &Item) -> String {
    match item {
        Item::Struct(_) => "struct".to_string(),
        Item::Enum(_) => "enum".to_string(),
        Item::Fn(_) => "fn".to_string(),
        Item::Trait(_) => "trait".to_string(),
        Item::Impl(_) => "impl".to_string(),
        Item::Type(_) => "type_alias".to_string(),
        Item::Use(_) => "use".to_string(),
        Item::Mod(_) => "mod".to_string(),
        Item::Const(_) => "const".to_string(),
        Item::Static(_) => "static".to_string(),
        Item::Macro(_) => "macro".to_string(),
        Item::Union(_) => "union".to_string(),
        Item::ForeignMod(_) => "foreign_mod".to_string(),
        Item::ExternCrate(_) => "extern_crate".to_string(),
        _ => "other".to_string(),
    }
}

fn visibility_class(vis: &Visibility) -> String {
    match vis {
        Visibility::Public(_) => "pub".to_string(),
        Visibility::Restricted(r) => format!("pub({})", r.path.segments.iter()
            .map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::")),
        Visibility::Inherited => "private".to_string(),
    }
}

fn extract_qualifiers(item: &Item) -> Vec<String> {
    let mut q = Vec::new();
    if let Item::Fn(f) = item {
        if f.sig.asyncness.is_some() { q.push("async".to_string()); }
        if f.sig.unsafety.is_some() { q.push("unsafe".to_string()); }
        if f.sig.constness.is_some() { q.push("const".to_string()); }
    }
    if let Item::Impl(i) = item {
        if i.unsafety.is_some() { q.push("unsafe".to_string()); }
    }
    if let Item::Trait(t) = item {
        if t.unsafety.is_some() { q.push("unsafe".to_string()); }
    }
    q.sort();
    q
}

fn extract_crate_uses(use_trees: &[UseTree]) -> Vec<String> {
    let mut crates = BTreeSet::new();
    for tree in use_trees {
        collect_crate_names(tree, &mut crates);
    }
    crates.into_iter().collect()
}

fn collect_crate_names(tree: &UseTree, crates: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(p) => {
            let name = p.ident.to_string();
            if is_external_crate_name(&name) {
                crates.insert(name);
            }
            collect_crate_names(&p.tree, crates);
        }
        UseTree::Name(n) => {
            let name = n.ident.to_string();
            if is_external_crate_name(&name) {
                crates.insert(name);
            }
        }
        UseTree::Rename(r) => {
            let name = r.ident.to_string();
            if is_external_crate_name(&name) {
                crates.insert(name);
            }
        }
        UseTree::Glob(_) => {}
        UseTree::Group(g) => {
            for item in &g.items {
                collect_crate_names(item, crates);
            }
        }
    }
}

fn is_external_crate_name(name: &str) -> bool {
    !matches!(name, "crate" | "self" | "super" | "std" | "core" | "alloc")
}

fn extract_generics(item: &Item) -> (Vec<String>, Vec<String>) {
    let (bounds, params) = match item {
        Item::Struct(s) => (collect_type_bounds(&s.generics, true), extract_generic_params(&s.generics)),
        Item::Enum(e) => (collect_type_bounds(&e.generics, true), extract_generic_params(&e.generics)),
        Item::Fn(f) => (collect_type_bounds(&f.sig.generics, false), extract_generic_params(&f.sig.generics)),
        Item::Trait(t) => (collect_type_bounds_for_trait(t), extract_generic_params(&t.generics)),
        Item::Impl(i) => (collect_type_bounds(&i.generics, true), extract_generic_params(&i.generics)),
        Item::Type(t) => (collect_type_bounds(&t.generics, true), extract_generic_params(&t.generics)),
        _ => (vec![], vec![]),
    };
    (bounds, params)
}

fn collect_type_bounds(generics: &Generics, _include_where: bool) -> Vec<String> {
    let mut bounds = BTreeSet::new();
    for param in &generics.params {
        if let GenericParam::Type(tp) = param {
            for bound in &tp.bounds {
                bounds.insert(format_bound(bound));
            }
        }
    }
    if let Some(wc) = &generics.where_clause {
        for pred in &wc.predicates {
            bounds.insert(format!("where:{}", quote::quote!(#pred).to_string()));
        }
    }
    bounds.into_iter().collect()
}

fn collect_type_bounds_for_trait(t: &syn::ItemTrait) -> Vec<String> {
    let mut bounds = BTreeSet::new();
    for param in &t.generics.params {
        if let GenericParam::Type(tp) = param {
            for bound in &tp.bounds {
                bounds.insert(format_bound(bound));
            }
        }
    }
    if let Some(wc) = &t.generics.where_clause {
        for pred in &wc.predicates {
            bounds.insert(format!("where:{}", quote::quote!(#pred).to_string()));
        }
    }
    bounds.into_iter().collect()
}

fn format_bound(bound: &TypeParamBound) -> String {
    quote::quote!(#bound).to_string()
}

fn extract_generic_params(generics: &Generics) -> Vec<String> {
    generics.params.iter().map(|p| {
        match p {
            GenericParam::Type(tp) => format!("T:{}", tp.ident),
            GenericParam::Lifetime(l) => format!("'{}", l.lifetime.ident),
            GenericParam::Const(c) => format!("const:{}", c.ident),
        }
    }).collect()
}

fn compute_shape_hash(item: &Item) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let shape = ast_shape_string(item);
    shape.hash(&mut hasher);
    hasher.finish()
}

fn ast_shape_string(item: &Item) -> String {
    match item {
        Item::Struct(s) => {
            let fields = s.fields.iter().map(|f| shape_field(f)).collect::<Vec<_>>().join(",");
            format!("struct({})", fields)
        }
        Item::Enum(e) => {
            let variants = e.variants.iter().map(|v| shape_variant(v)).collect::<Vec<_>>().join(",");
            format!("enum({})", variants)
        }
        Item::Fn(f) => {
            let args = f.sig.inputs.iter().map(|a| shape_fn_arg(a)).collect::<Vec<_>>().join(",");
            let ret = match &f.sig.output {
                syn::ReturnType::Default => "->()".to_string(),
                syn::ReturnType::Type(_, t) => format!("->{}", quote::quote!(#t)),
            };
            format!("fn({}){}", args, ret)
        }
        Item::Trait(t) => {
            let items = t.items.len();
            format!("trait(items:{})", items)
        }
        Item::Impl(i) => {
            let items = i.items.len();
            format!("impl(items:{})", items)
        }
        Item::Type(t) => {
            format!("type_alias({})", quote::quote!(#t.ty))
        }
        _ => format!("{:?}", std::mem::discriminant(item)),
    }
}

fn shape_field(f: &Field) -> String {
    let ty_str = quote::quote!(#f.ty).to_string();
    format!("{}:{}", ty_str.replace(' ', ""), f.ident.as_ref().map(|i| i.to_string()).unwrap_or("_unnamed".to_string()))
}

fn shape_variant(v: &Variant) -> String {
    let fields = v.fields.iter().map(|f| shape_field(f)).collect::<Vec<_>>().join(",");
    format!("{}({})", v.ident, fields)
}

fn shape_fn_arg(a: &FnArg) -> String {
    match a {
        FnArg::Receiver(r) => {
            if r.reference.is_some() { "&self".to_string() } else { "self".to_string() }
        }
        FnArg::Typed(t) => {
            let ty_str = quote::quote!(#t.ty).to_string();
            let pat_str = quote::quote!(#t.pat).to_string();
            format!("{}:{}", pat_str, ty_str.replace(' ', ""))
        }
    }
}
