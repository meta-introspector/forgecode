use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use petgraph::algo::{kosaraju_scc, toposort};
use petgraph::graph::DiGraph;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

// ── Data model ───────────────────────────────────────────────────────────

/// A single split declaration file analyzed for dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclNode {
    /// The file stem (e.g. "forge_domain_Model")
    pub id: String,
    /// Kind: "struct", "enum", "fn", "impl", "trait", "const", "type", "static"
    pub kind: String,
    /// The primary type name this decl defines or implements
    pub defines: Vec<String>,
    /// All type identifiers referenced in the body (potential internal deps)
    pub references: Vec<String>,
    /// External crate names used (e.g. "serde", "anyhow", "tokio")
    pub external_uses: Vec<String>,
    /// Source file path
    pub source_file: String,
    /// Original crate name
    pub crate_name: String,
    /// Source file contents (for CAR archival)
    pub source: String,
}

/// A layer in the topological sort (all nodes at the same depth).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeLayer {
    pub depth: usize,
    pub nodes: Vec<String>,
}

/// The full lattice output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lattice {
    pub crate_name: String,
    pub total_decls: usize,
    pub layers: Vec<LatticeLayer>,
    pub edges: Vec<(String, String)>,
    pub decl_info: BTreeMap<String, DeclNode>,
}

/// A generated plugin crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCrate {
    pub name: String,
    pub layer: usize,
    pub decls: Vec<String>,
    /// Internal plugin crate dependencies (other layers)
    pub dependencies: BTreeSet<String>,
    /// External crate dependencies (from crates.io)
    pub external_deps: BTreeSet<String>,
}

/// The full plugin crate plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPlan {
    pub source_crate: String,
    pub source_crate_dir: PathBuf,
    pub plugin_crates: Vec<PluginCrate>,
    pub ffi_boundary_types: BTreeMap<String, Vec<String>>,
    /// Ordered list of external crates, one per layer (index = layer depth)
    pub external_crate_layers: Vec<String>,
}

// ── Identifier extraction visitor ────────────────────────────────────────

/// Visitor that collects all identifiers used in type positions within a decl.
#[derive(Default)]
struct IdentCollector {
    /// All idents found in the AST
    idents: HashSet<String>,
}

impl<'ast> Visit<'ast> for IdentCollector {
    fn visit_ident(&mut self, i: &'ast proc_macro2::Ident) {
        let name = i.to_string();
        // Skip Rust keywords and common primitives
        match name.as_str() {
            "self" | "Self" | "super" | "crate" | "pub" | "fn" | "let" | "mut"
            | "const" | "static" | "struct" | "enum" | "trait" | "impl" | "type"
            | "where" | "async" | "await" | "use" | "mod" | "return" | "if"
            | "else" | "match" | "for" | "while" | "loop" | "break" | "continue"
            | "true" | "false" | "Some" | "None" | "Ok" | "Err" | "String"
            | "Vec" | "Option" | "Result" | "Box" | "Arc" | "Rc" | "bool"
            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16"
            | "i32" | "i64" | "i128" | "isize" | "f32" | "f64" | "str"
            | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "Cow"
            | "default" | "Default" | "Clone" | "Debug" | "Display"
            | "Serialize" | "Deserialize" | "Hash" | "Eq" | "PartialEq"
            | "Copy" | "Send" | "Sync" | "Unpin" | "RefUnpin" | "UnwindSafe"
            | "From" | "Into" | "AsRef" | "AsMut" | "TryFrom" | "TryInto"
            | "ToString" | "Iterator" | "IntoIterator" | "ExactSizeIterator"
            | "Fn" | "FnMut" | "FnOnce" | "Sized" | "ToOwned" | "Borrow"
            | "BorrowMut" | "Binary" | "Octal" | "LowerHex" | "UpperHex"
            | "Pointer" | "Write" | "Read" | "BufRead" | "Seek"
            | "FromStr" | "FromIterator" | "DoubleEndedIterator" => return,
            _ => {}
        }
        self.idents.insert(name);
    }
}

// ── External crate extraction ───────────────────────────────────────────

/// Known external crates that forge_domain depends on.
/// Proc-macro crates (derive_setters, derive_more, strum_macros, schemars, fake,
/// pretty_assertions) are included because they appear in `use` statements.
const KNOWN_EXTERNAL_CRATES: &[&str] = &[
    "anyhow", "async_trait", "base64", "chrono", "convert_case",
    "derive_getters", "derive_more", "derive_setters", "fake",
    "forge_json_repair", "futures", "html_escape", "markdown",
    "merge", "nom", "once_cell", "pretty_assertions", "rand",
    "regex", "reqwest", "schemars", "serde", "serde_json",
    "strum", "strum_macros", "thiserror", "tokio", "tracing",
    "url", "uuid",
];

/// Visitor that collects the root crate name from each `use` statement.
#[derive(Default)]
struct UseCrateCollector {
    /// External crate names found in use statements
    external_crates: HashSet<String>,
}

impl<'ast> Visit<'ast> for UseCrateCollector {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        // Extract the root of the use path
        if let syn::UseTree::Path(path) = &item.tree {
            let crate_name = path.ident.to_string();
            if KNOWN_EXTERNAL_CRATES.contains(&crate_name.as_str()) {
                self.external_crates.insert(crate_name);
            }
        } else if let syn::UseTree::Name(name) = &item.tree {
            let crate_name = name.ident.to_string();
            if KNOWN_EXTERNAL_CRATES.contains(&crate_name.as_str()) {
                self.external_crates.insert(crate_name);
            }
        }
        syn::visit::visit_item_use(self, item);
    }
}

// ── Decl analysis ────────────────────────────────────────────────────────

/// Parses a single split decl file and extracts its node info.
fn analyze_decl_file(
    path: &Path,
    crate_name: &str,
    all_defined_types: &HashSet<String>,
) -> Result<DeclNode> {
    let source = fs::read_to_string(path)
        .context(format!("Failed to read {}", path.display()))?;

    let syntax: syn::File = syn::parse_file(&source)
        .context(format!("Failed to parse {}", path.display()))?;

    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine what this decl defines
    let mut defines = Vec::new();
    for item in &syntax.items {
        match item {
            syn::Item::Struct(s) => defines.push(s.ident.to_string()),
            syn::Item::Enum(e) => defines.push(e.ident.to_string()),
            syn::Item::Trait(t) => defines.push(t.ident.to_string()),
            syn::Item::Type(t) => defines.push(t.ident.to_string()),
            syn::Item::Fn(f) => defines.push(f.sig.ident.to_string()),
            syn::Item::Const(c) => defines.push(c.ident.to_string()),
            syn::Item::Static(s) => defines.push(s.ident.to_string()),
            syn::Item::Impl(i) => {
                // For impl blocks, record the self type and trait
                if let syn::Type::Path(tp) = &*i.self_ty {
                    if let Some(seg) = tp.path.segments.last() {
                        defines.push(format!("impl_{}", seg.ident));
                    }
                }
                if let Some((_, path, _)) = &i.trait_ {
                    defines.push(format!("impl_{}", path.to_token_stream().to_string()));
                }
            }
            _ => {}
        }
    }

    // Collect all identifiers referenced in the body
 // Collect all identifiers referenced in the body
 let mut collector = IdentCollector::default();
 collector.visit_file(&syntax);

 // Collect external crate dependencies
 let mut ext_collector = UseCrateCollector::default();
 ext_collector.visit_file(&syntax);

 // Filter to only those that are defined within the crate's decls
 let references: Vec<String> = collector.idents
 .iter()
 .filter(|id| all_defined_types.contains(*id) || id.starts_with("Model") || id.starts_with("Context"))
 .cloned()
 .collect();

 let external_uses: Vec<String> = ext_collector.external_crates
 .iter()
 .cloned()
 .collect();
    // Determine kind from the file name pattern
    let kind = if stem.contains("_impl_for_") || stem.contains("_impl_") {
        "impl".to_string()
    } else if defines.iter().any(|d| d.starts_with("impl_")) {
        "impl".to_string()
    } else {
        // Infer from first item
        syntax.items.first().map(|item| match item {
            syn::Item::Struct(_) => "struct",
            syn::Item::Enum(_) => "enum",
            syn::Item::Fn(_) => "fn",
            syn::Item::Trait(_) => "trait",
            syn::Item::Type(_) => "type",
            syn::Item::Const(_) => "const",
            syn::Item::Static(_) => "static",
            syn::Item::Impl(_) => "impl",
            _ => "unknown",
        }).unwrap_or("unknown").to_string()
    };

    Ok(DeclNode {
 id: stem.clone(),
 kind,
 defines,
 references,
 external_uses,
 source_file: path.display().to_string(),
 crate_name: crate_name.to_string(),
 source,
 })
}

// ── Lattice building ────────────────────────────────────────────────────

/// Scans all decl files under a crate's src/decls/ directory.
fn scan_decls(crate_dir: &Path, crate_name: &str) -> Result<Vec<DeclNode>> {
    let decls_dir = crate_dir.join("src").join("decls");
    if !decls_dir.exists() {
        anyhow::bail!("No decls/ directory found in {}", crate_dir.display());
    }

    // First pass: collect all defined type names across the entire crate
    let all_decl_files: Vec<PathBuf> = walkdir::WalkDir::new(&decls_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "rs")
            && !e.path().file_name().map_or(false, |n| n == "_decl_module_invocation.rs")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    // Quick first pass to collect all type names defined in the crate
    let mut all_defined_types: HashSet<String> = HashSet::new();
    for path in &all_decl_files {
        if let Ok(source) = fs::read_to_string(path) {
            if let Ok(syntax) = syn::parse_file(&source) {
                for item in &syntax.items {
                    match item {
                        syn::Item::Struct(s) => { all_defined_types.insert(s.ident.to_string()); }
                        syn::Item::Enum(e) => { all_defined_types.insert(e.ident.to_string()); }
                        syn::Item::Trait(t) => { all_defined_types.insert(t.ident.to_string()); }
                        syn::Item::Type(t) => { all_defined_types.insert(t.ident.to_string()); }
                        _ => {}
                    }
                }
            }
        }
    }

    println!("Found {} defined types in crate {}", all_defined_types.len(), crate_name);
    for t in all_defined_types.iter() {
        println!("  type: {}", t);
    }

    // Second pass: full analysis
    let mut nodes = Vec::new();
    for path in &all_decl_files {
        match analyze_decl_file(path, crate_name, &all_defined_types) {
            Ok(node) => nodes.push(node),
            Err(e) => println!("  warning: failed to analyze {}: {}", path.display(), e),
        }
    }

    Ok(nodes)
}

/// Builds a dependency graph from decl nodes and topological sorts it.
fn build_lattice(nodes: &[DeclNode]) -> Result<Lattice> {
    let mut graph = DiGraph::<String, ()>::new();
    let mut node_indices: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    // Add all nodes to the graph
    for node in nodes {
        let idx = graph.add_node(node.id.clone());
        node_indices.insert(node.id.clone(), idx);
    }

    // Build a map from type name -> decl node id that defines it
    let mut type_to_decl: HashMap<String, String> = HashMap::new();
    for node in nodes {
        for def in &node.defines {
            type_to_decl.insert(def.clone(), node.id.clone());
        }
    }

    // Add edges: if node A references a type defined by node B, A depends on B
    let mut edges = Vec::new();
    for node in nodes {
        let node_idx = node_indices[&node.id];
        for ref_type in &node.references {
            if let Some(dep_id) = type_to_decl.get(ref_type) {
                if dep_id != &node.id {
                    if let Some(&dep_idx) = node_indices.get(dep_id) {
                        // Add edge from dep -> node (dep must come before node)
                        if !graph.contains_edge(dep_idx, node_idx) {
                            graph.add_edge(dep_idx, node_idx, ());
                            edges.push((dep_id.clone(), node.id.clone()));
                        }
                    }
                }
            }
        }
    }

    // Topological sort
    // Use Kosaraju's SCC to find cycles, then condense the graph
    // by merging each SCC into a single node before topological sorting.
    let sccs = kosaraju_scc(&graph);

    // Map each original node to its SCC index
    let mut node_to_scc: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();
    for (scc_idx, scc) in sccs.iter().enumerate() {
        for &node_idx in scc {
            node_to_scc.insert(node_idx, scc_idx);
        }
    }

    // Build condensed graph (one node per SCC)
    let mut condensed = DiGraph::<usize, ()>::new();
    for i in 0..sccs.len() {
        condensed.add_node(i);
    }
    for edge in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(edge).unwrap();
        let src_scc = node_to_scc[&src];
        let dst_scc = node_to_scc[&dst];
        if src_scc != dst_scc {
            let src_cidx = petgraph::graph::NodeIndex::new(src_scc);
            let dst_cidx = petgraph::graph::NodeIndex::new(dst_scc);
            if !condensed.contains_edge(src_cidx, dst_cidx) {
                condensed.add_edge(src_cidx, dst_cidx, ());
            }
        }
    }

    // Topological sort the condensed (acyclic) graph
    let sorted_sccs = toposort(&condensed, None)
        .expect("Condensed graph should be a DAG");

    // Compute layers on the condensed graph using longest path
    let mut scc_depth: HashMap<usize, usize> = HashMap::new();
    for cidx in &sorted_sccs {
        let scc_idx = condensed[*cidx];
        let max_dep_depth = condensed.neighbors_directed(*cidx, petgraph::Direction::Incoming)
            .filter_map(|dep_cidx| scc_depth.get(&condensed[dep_cidx]))
            .max()
            .copied()
            .unwrap_or(0);
        scc_depth.insert(scc_idx, max_dep_depth + 1);
    }

    // Map SCC depths back to original nodes
    let mut depth_map: HashMap<String, usize> = HashMap::new();
    for (scc_idx, scc) in sccs.iter().enumerate() {
        let depth = scc_depth[&scc_idx];
        for &node_idx in scc {
            let node_id = graph[node_idx].clone();
            depth_map.insert(node_id, depth);
        }
    }

    let max_depth = depth_map.values().copied().max().unwrap_or(0);
    let mut layers: Vec<LatticeLayer> = Vec::new();
    for depth in 0..=max_depth {
        let nodes_at_depth: Vec<String> = depth_map.iter()
            .filter(|(_, d)| **d == depth)
            .map(|(id, _)| id.clone())
            .collect();
        if !nodes_at_depth.is_empty() {
            layers.push(LatticeLayer {
                depth,
                nodes: nodes_at_depth,
            });
        }
    }

    let mut decl_info = BTreeMap::new();
    for node in nodes {
        decl_info.insert(node.id.clone(), node.clone());
    }

    Ok(Lattice {
        crate_name: nodes.first().map(|n| n.crate_name.clone()).unwrap_or_default(),
        total_decls: nodes.len(),
        layers,
        edges,
        decl_info,
    })
}

// ── Plugin crate generation ─────────────────────────────────────────────

/// Groups decl nodes into plugin crates based on the lattice layers.
/// The new layering strategy: each layer introduces exactly one new external crate.
/// Decls are assigned to layers by their external crate dependency set.
fn generate_plugin_plan(lattice: &Lattice, max_decls_per_crate: usize, source_crate_dir: &Path, exclude: &HashSet<String>) -> PluginPlan {
 let mut plugin_crates: Vec<PluginCrate> = Vec::new();
 let mut ffi_boundary_types: BTreeMap<String, Vec<String>> = BTreeMap::new();

 // ── New layering: group decls by external crate deps ──────────────
 // 1. Collect all external crates used across all decls
 let mut all_ext_crates: BTreeSet<String> = BTreeSet::new();
 for (_, info) in &lattice.decl_info {
 for ext in &info.external_uses {
 all_ext_crates.insert(ext.clone());
 }
 }

 // 2. Sort external crates by frequency (most-used first) to minimize layers
 let mut ext_crate_freq: Vec<(String, usize)> = all_ext_crates.iter()
 .map(|c| {
 let count = lattice.decl_info.values()
 .filter(|info| info.external_uses.contains(c))
 .count();
 (c.clone(), count)
 })
 .collect();
 ext_crate_freq.sort_by(|a, b| b.1.cmp(&a.1)); // most frequent first

 // 3. Build ordered list: layer N introduces external_crate_layers[N]
 let external_crate_layers: Vec<String> = ext_crate_freq.iter()
 .map(|(c, _)| c.clone())
 .collect();

 // 4. Assign each decl to a layer based on its external deps
 // Layer = max index of any external crate it uses
 // Layer 0 = no external deps, Layer 1 = first ext crate, etc.
 let ext_crate_to_layer: HashMap<String, usize> = external_crate_layers.iter()
 .enumerate()
 .map(|(i, c)| (c.clone(), i + 1)) // layer 0 = no deps, layer 1+ = ext crate index
 .collect();

 // Compute the external-deps-based layer for each decl
 let mut decl_ext_layer: HashMap<String, usize> = HashMap::new();
 for (decl_id, info) in &lattice.decl_info {
 let ext_layer = info.external_uses.iter()
 .filter_map(|c| ext_crate_to_layer.get(c))
 .max()
 .copied()
 .unwrap_or(0);
 decl_ext_layer.insert(decl_id.clone(), ext_layer);
 }

 // 5. Merge with topological layering: a decl's final layer is the
 // maximum of its external-deps layer and its topological depth
 // (must be >= topological depth to satisfy internal deps)
 let topo_depth: HashMap<String, usize> = {
 let mut map = HashMap::new();
 for layer in &lattice.layers {
 for node_id in &layer.nodes {
 map.insert(node_id.clone(), layer.depth);
 }
 }
 map
 };

 let mut decl_final_layer: HashMap<String, usize> = HashMap::new();
 for (decl_id, _) in &lattice.decl_info {
 let ext_l = decl_ext_layer.get(decl_id).copied().unwrap_or(0);
 let topo_l = topo_depth.get(decl_id).copied().unwrap_or(0);
 decl_final_layer.insert(decl_id.clone(), ext_l.max(topo_l));
 }

 // 6. Group decls by final layer
    // 6. Group decls by final layer, excluding broken decls
    let max_layer = decl_final_layer.values().copied().max().unwrap_or(0);
    let mut layer_groups: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for (decl_id, layer) in &decl_final_layer {
        if exclude.contains(decl_id) {
            continue;
        }
        layer_groups[*layer].push(decl_id.clone());
    }
 // 7. Create plugin crates from layer groups
 for (layer_idx, decls) in layer_groups.iter().enumerate() {
 if decls.is_empty() {
 continue;
 }

 let chunks: Vec<Vec<String>> = decls
 .chunks(max_decls_per_crate)
 .map(|c| c.to_vec())
 .collect();

 for (chunk_idx, chunk) in chunks.iter().enumerate() {
 let crate_name = if chunks.len() > 1 {
 format!("forge_{}_l{}_c{}", lattice.crate_name.replace("-", "_"), layer_idx, chunk_idx)
 } else {
 format!("forge_{}_l{}", lattice.crate_name.replace("-", "_"), layer_idx)
 };

 // Determine internal plugin crate dependencies
 let mut deps = BTreeSet::new();
 for decl_id in chunk {
 if let Some(info) = lattice.decl_info.get(decl_id) {
 for ref_type in &info.references {
 for (other_id, other_info) in &lattice.decl_info {
 if other_info.defines.iter().any(|d| d == ref_type)
 && !chunk.contains(other_id)
 {
 for prev_crate in &plugin_crates {
 if prev_crate.decls.contains(other_id) {
 deps.insert(prev_crate.name.clone());
 }
 }
 }
 }
 }
 }
 }

 // Determine external crate dependencies for this layer
 let mut ext_deps = BTreeSet::new();
 for decl_id in chunk {
 if let Some(info) = lattice.decl_info.get(decl_id) {
 for ext in &info.external_uses {
 ext_deps.insert(ext.clone());
 }
 }
 }

 // Collect FFI-safe types exported by this crate
 let exported_types: Vec<String> = chunk.iter()
 .filter_map(|decl_id| {
 lattice.decl_info.get(decl_id).and_then(|info| {
 if matches!(info.kind.as_str(), "struct" | "enum" | "trait" | "type") {
 info.defines.first().cloned()
 } else {
 None
 }
 })
 })
 .collect();
 ffi_boundary_types.insert(crate_name.clone(), exported_types);

 plugin_crates.push(PluginCrate {
 name: crate_name,
 layer: layer_idx,
 decls: chunk.clone(),
 dependencies: deps,
 external_deps: ext_deps,
 });
 }
 }

 PluginPlan {
 source_crate: lattice.crate_name.clone(),
 source_crate_dir: source_crate_dir.to_path_buf(),
 plugin_crates,
 ffi_boundary_types,
 external_crate_layers,
 }
}
/// Generates the actual crate files (Cargo.toml, lib.rs, ffi.rs) for each plugin.
/// Copies real decl content from the split files, rewriting use statements
/// to point at the correct plugin crate dependencies.
fn write_plugin_crates(plan: &PluginPlan, output_dir: &Path, dry_run: bool, plan_lattice: &Lattice, exclude: &HashSet<String>) -> Result<()> {
    // Build a map: decl_id -> which plugin crate name owns it
    let mut decl_to_crate: HashMap<String, String> = HashMap::new();
    for plugin in &plan.plugin_crates {
        for decl_id in &plugin.decls {
            decl_to_crate.insert(decl_id.clone(), plugin.name.clone());
        }
    }

    // Build a map: type_name -> plugin crate name that exports it
 // Use the lattice's decl_info to find which plugin crate defines each type
 let mut type_to_crate: HashMap<String, String> = HashMap::new();
 for plugin in &plan.plugin_crates {
 for decl_id in &plugin.decls {
 if let Some(info) = plan_lattice.decl_info.get(decl_id) {
 for type_name in &info.defines {
 type_to_crate.insert(type_name.clone(), plugin.name.clone());
 }
 }
 }
 }
    for plugin in &plan.plugin_crates {
        let crate_dir = output_dir.join(&plugin.name);

        if !dry_run {
            fs::create_dir_all(crate_dir.join("src"))?;
            fs::create_dir_all(crate_dir.join("ffi"))?;
        }

        // Generate Cargo.toml
 // Generate Cargo.toml
 let mut deps_toml = String::new();
 for dep in &plugin.dependencies {
 deps_toml.push_str(&format!(
 "{} = {{ path = \"../{}\" }}\n",
 dep, dep
 ));
 }
 // Add external crate dependencies based on what this layer actually uses
 // Version mappings for known external crates.
 // Key is the `use` name (underscore), value is (cargo_toml_name, version)
 let ext_crate_versions: HashMap<&str, (&str, &str)> = [
 ("anyhow", ("anyhow", "1")),
 ("async_trait", ("async-trait", "0.1")),
 ("base64", ("base64", "0.22")),
 ("chrono", ("chrono", "0.4")),
 ("convert_case", ("convert_case", "0.6")),
 ("derive_getters", ("derive-getters", "0.5")),
 ("derive_more", ("derive_more", "1")),
 ("derive_setters", ("derive_setters", "0.1")),
 ("fake", ("fake", "2")),
 ("futures", ("futures", "0.3")),
 ("html_escape", ("html_escape", "0.2")),
 ("markdown", ("markdown", "1.0")),
 ("merge", ("merge", "0.1")),
 ("nom", ("nom", "7")),
 ("once_cell", ("once_cell", "1")),
 ("pretty_assertions", ("pretty_assertions", "1")),
 ("rand", ("rand", "0.8")),
 ("regex", ("regex", "1")),
 ("reqwest", ("reqwest", "0.12")),
 ("schemars", ("schemars", "0.8")),
 ("serde", ("serde", "1.0")),
 ("serde_json", ("serde_json", "1")),
 ("strum", ("strum", "0.26")),
 ("strum_macros", ("strum_macros", "0.26")),
 ("thiserror", ("thiserror", "1")),
 ("tokio", ("tokio", "1")),
 ("tracing", ("tracing", "0.1")),
 ("url", ("url", "2")),
 ("uuid", ("uuid", "1")),
 ].iter().cloned().collect();

 // Local workspace crates that should be path dependencies instead of crates.io
 // The path will be recomputed relative to each output crate directory
 let local_crate_paths: HashMap<&str, &str> = [
 ("forge_json_repair", "crates/forge_json_repair"),
 ].iter().cloned().collect();

 // Always include serde since it's ubiquitous
 deps_toml.push_str("serde = { version = \"1.0\", features = [\"derive\"] }\n");

 for ext_dep in &plugin.external_deps {
 // Skip serde - already added above
 if ext_dep == "serde" || ext_dep == "serde_json" {
 if ext_dep == "serde_json" {
 deps_toml.push_str("serde_json = \"1\"\n");
 }
 continue;
 }
 // Check local workspace crates first (path deps)
 if let Some(sub_path) = local_crate_paths.get(ext_dep.as_str()) {
 // Compute absolute path from the workspace root
 // source_crate_dir is e.g. /path/to/forgecode/crates/forge_domain
 // We need /path/to/forgecode/ + sub_path
 let workspace_root = plan.source_crate_dir.parent() // crates/
 .and_then(|p| p.parent()) // forgecode root
 .unwrap_or(&plan.source_crate_dir);
 let abs_path = workspace_root.join(sub_path);
 // Make the path relative to the output crate dir
 let abs_path = if !abs_path.is_absolute() {
 fs::canonicalize(&abs_path).unwrap_or_else(|_| abs_path.to_path_buf())
 } else {
 abs_path.to_path_buf()
 };
 deps_toml.push_str(&format!("{} = {{ path = \"{}\" }}\n", ext_dep, abs_path.display()));
 continue;
 }
 if let Some((cargo_name, version)) = ext_crate_versions.get(ext_dep.as_str()) {
 // Handle features for specific crates
 match ext_dep.as_str() {
 "tokio" => deps_toml.push_str(&format!("{} = {{ version = \"{}\", features = [\"full\"] }}\n", cargo_name, version)),
 "chrono" => deps_toml.push_str(&format!("{} = {{ version = \"{}\", features = [\"serde\"] }}\n", cargo_name, version)),
 "uuid" => deps_toml.push_str(&format!("{} = {{ version = \"{}\", features = [\"v4\", \"serde\"] }}\n", cargo_name, version)),
 "schemars" => deps_toml.push_str(&format!("{} = {{ version = \"{}\", features = [\"chrono\", \"url\"] }}\n", cargo_name, version)),
 "derive_more" => deps_toml.push_str(&format!("{} = {{ version = \"{}\", features = [\"full\"] }}\n", cargo_name, version)),
 _ => deps_toml.push_str(&format!("{} = \"{}\"\n", cargo_name, version)),
 }
 } else {
 // Unknown crate - add with placeholder version (use underscore name as-is)
 deps_toml.push_str(&format!("{} = \"0.1\"\n", ext_dep));
 }
 }
        let cargo_toml = format!(
r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}"#,
            plugin.name, deps_toml
        );

        // Generate lib.rs with re-exports from sub-modules
        let mut lib_rs = String::from("// Auto-generated by decl-lattice\n\n");
        for decl_id in &plugin.decls {
            let safe_mod_name = decl_id.replace("-", "_");
            lib_rs.push_str(&format!("mod {};\n", safe_mod_name));
        }
        // Re-export all public items from sub-modules at the crate root
        lib_rs.push_str("\n// Re-exports\n");
        for decl_id in &plugin.decls {
            let safe_mod_name = decl_id.replace("-", "_");
            lib_rs.push_str(&format!("pub use {}::*;\n", safe_mod_name));
        }
        lib_rs.push_str("\npub mod ffi;\n");

        // Generate ffi.rs - FFI-safe wrappers
        let mut ffi_rs = String::from(
"//! FFI boundary module\n//! This module exposes C-compatible types and functions for cross-plugin communication.\n//! All types crossing the FFI boundary must be #[repr(C)] and own their data.\n\nuse std::ffi::CString;\nuse std::os::raw::c_char;\n\n"
        );

        if let Some(types) = plan.ffi_boundary_types.get(&plugin.name) {
            for typ in types {
                ffi_rs.push_str(&format!(
"/// FFI-safe wrapper for {}\n#[repr(C)]\npub struct Ffi{} {{\n    // TODO: define FFI-safe fields for {}\n    _marker: [u8; 0],\n}}\n\n",
                    typ, typ, typ
                ));
            }
        }

        // Generate per-decl modules with actual content from split files
        for decl_id in &plugin.decls {
            let safe_mod_name = decl_id.replace("-", "_");

            // Find the actual split decl source file
            let source_file = find_decl_source_file(&plan.source_crate_dir, decl_id);

            let mod_content = match source_file {
                Some(path) => {
                    match fs::read_to_string(&path) {
                        Ok(raw_content) => {
                            // Rewrite use statements to reference plugin crate deps
                            rewrite_decl_content(&raw_content, &type_to_crate, &plugin.name)
                        }
                        Err(e) => {
                            format!(
                                "// ERROR: failed to read {}: {}\n",
                                path.display(), e
                            )
                        }
                    }
                }
                None => {
                    format!(
                        "// WARNING: source file not found for decl '{}'\n// Searched in: {}/src/decls/\n",
                        decl_id,
                        plan.source_crate_dir.display()
                    )
                }
            };

            let mod_path = crate_dir.join("src").join(format!("{}.rs", safe_mod_name));
            if !dry_run {
                fs::write(&mod_path, mod_content)?;
            }
        }

        if !dry_run {
            fs::write(crate_dir.join("Cargo.toml"), &cargo_toml)?;
            fs::write(crate_dir.join("src").join("lib.rs"), &lib_rs)?;
            fs::write(crate_dir.join("src").join("ffi.rs"), &ffi_rs)?;
            println!("  generated: {} (layer {}, {} decls, {} deps)",
                plugin.name, plugin.layer, plugin.decls.len(), plugin.dependencies.len()
            );
        } else {
            println!("  dry-run: would generate {} (layer {}, {} decls, {} deps)",
                plugin.name, plugin.layer, plugin.decls.len(), plugin.dependencies.len()
            );
        }
    }

    Ok(())
}
// ── Decl source resolution ─────────────────────────────────────────────

/// Finds the actual split decl source file for a given decl_id.
///
/// Searches under `<source_crate_dir>/src/decls/` for a file whose stem
/// matches the decl_id.
fn find_decl_source_file(source_crate_dir: &Path, decl_id: &str) -> Option<PathBuf> {
    let decls_dir = source_crate_dir.join("src").join("decls");
    if !decls_dir.exists() {
        return None;
    }

    // Walk all subdirectories to find the matching file
    for entry in walkdir::WalkDir::new(&decls_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem == decl_id {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    None
}

/// Rewrites use statements in a decl's source content so that types
/// provided by other plugin crates are referenced via `dep_crate::Type`
/// instead of being assumed to be in the same crate.
///
/// Handles two patterns:
/// 1. `use crate::TypeName` -> `use dep_crate::TypeName`
/// 2. Bare references to external types get a `use dep_crate::TypeName;` prepended
///
/// # Arguments
/// * `content` - The raw source text of the split decl file
/// * `type_to_crate` - Map from type name to the plugin crate name that defines it
/// * `self_crate` - The name of the plugin crate this decl belongs to
fn rewrite_decl_content(
    content: &str,
    type_to_crate: &HashMap<String, String>,
    self_crate: &str,
) -> String {
    // Since the decl splitter output is often on a single line,
    // we work at the token/string level rather than line-by-line.

    // First, collect which external types this file references
    let syntax = match syn::parse_file(content) {
        Ok(s) => s,
        Err(_) => {
            return format!("// WARNING: failed to parse decl for use-rewriting\n{}", content);
        }
    };

    let mut external_types: BTreeMap<String, String> = BTreeMap::new();
    let mut collector = IdentCollector::default();
    collector.visit_file(&syntax);
    for ident in &collector.idents {
        if let Some(crate_name) = type_to_crate.get(ident) {
            if crate_name != self_crate {
                external_types.insert(ident.clone(), crate_name.clone());
            }
        }
    }

    if external_types.is_empty() {
        return content.to_string();
    }

    // Build replacement map for `use crate::X` -> `use dep_crate::X`
    // and collect types that need new use statements
    let mut result = content.to_string();

    // Replace `use crate :: { A , B , C }` patterns with individual dep-crate uses
    for (type_name, dep_crate) in &external_types {
        // Pattern: `use crate :: TypeName` or `use crate :: { ... TypeName ... }`
        // We handle the simple case first: replace `crate :: TypeName` with `dep_crate :: TypeName`
        let crate_pattern = format!("crate :: {}", type_name);
        let dep_pattern = format!("{} :: {}", dep_crate, type_name);
        result = result.replace(&crate_pattern, &dep_pattern);
    }

    // Handle `use crate :: { A , B , C }` group patterns
    // Replace `crate :: {` with individual imports from the respective crates
    // This is a simplified approach: expand grouped crate uses into individual dep uses
    if result.contains("crate :: {") {
        result = expand_grouped_crate_uses(&result, &external_types);
    }

    // For any external types not yet covered by a use statement, prepend one
    let mut needs_new_use: BTreeMap<String, String> = BTreeMap::new();
    for (type_name, dep_crate) in &external_types {
        let dep_use = format!("{} :: {}", dep_crate, type_name);
        if !result.contains(&dep_use) {
            needs_new_use.insert(type_name.clone(), dep_crate.clone());
        }
    }

    if !needs_new_use.is_empty() {
        let mut prepend = String::new();
        for (type_name, dep_crate) in &needs_new_use {
            prepend.push_str(&format!("use {} :: {} ; ", dep_crate, type_name));
        }
        result = format!("{} {}", prepend, result);
    }

    // Handle remaining `crate ::` references that weren't rewritten
    // (types that exist in the original crate but weren't split into their own decl file)
    // Replace `crate ::` with `self ::` since within a plugin crate, these are local types
    result = result.replace("crate :: ", "self :: ");

    result
}

/// Expands grouped `use crate :: { A , B , C }` patterns into individual
/// `use dep_crate_A :: A ; use dep_crate_B :: B ;` etc.
fn expand_grouped_crate_uses(
    content: &str,
    external_types: &BTreeMap<String, String>,
) -> String {
    // Simple regex-like approach: find `use crate :: { ... }` and expand
    let mut result = content.to_string();

    // Find patterns like `use crate :: { TypeA , TypeB }`
    while let Some(start) = result.find("crate :: {") {
        // Find the matching closing brace
        let brace_start = start + "crate :: ".len();
        if let Some(brace_end) = result[brace_start..].find('}') {
            let inner = &result[brace_start + 1..brace_start + brace_end];
            let type_names: Vec<&str> = inner.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let mut expanded = String::new();
            let mut remaining = String::new();
            for type_name in &type_names {
                if let Some(dep_crate) = external_types.get(*type_name) {
                    expanded.push_str(&format!("use {} :: {} ; ", dep_crate, type_name));
                } else {
                    // Type stays in this crate, keep as self reference
                    remaining.push_str(&format!("{} , ", type_name));
                }
            }

            // Replace the entire `use crate :: { ... }` segment
            let full_end = brace_start + brace_end + 1;
            // Find the start of the `use` keyword
            let use_start = if let Some(us) = result[..start].rfind("use ") {
                us
            } else {
                start
            };

            let replacement = if remaining.is_empty() {
                expanded.trim().to_string()
            } else {
                format!("{} use self :: {{ {} }} ; ", expanded.trim(), remaining.trim_end_matches(", ").trim())
            };

            result = format!("{}{}{}", &result[..use_start], replacement, &result[full_end..]);
            break; // Handle one group at a time to avoid index issues
        } else {
            break;
        }
    }

    result
}

// ── DAG-CBOR Vernacular Documents (CAR archive for excluded decls) ──────────

/// A CID v1 with:
/// - Multicodec: dag-cbor (0x71)
/// - Multihash: sha2-256 (0x12), 32 bytes
fn compute_cid(data: &[u8]) -> (Vec<u8>, String) {
    let hash = Sha256::digest(data);
    // CID v1 raw bytes: version(1) + multicodec(1) + multihash_type(1) + multihash_len(1) + hash(32)
    let mut cid_bytes = Vec::with_capacity(36);
    cid_bytes.push(0x01);  // CID version 1
    cid_bytes.push(0x71);  // dag-cbor codec
    cid_bytes.push(0x12);  // sha2-256
    cid_bytes.push(0x20);  // 32 bytes
    cid_bytes.extend_from_slice(&hash);
    
    // Base32 lower-case multibase encoding for human-readable CID
    use multibase::Base;
    let cid_str = multibase::encode(Base::Base32Lower, &cid_bytes);
    
    (cid_bytes, cid_str)
}

/// Vernacular document — a complete semantic snapshot of a syn declaration
/// that can be serialized as DAG-CBOR and archived in a CAR file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynVernacular {
    /// Unique declaration identifier (file stem, e.g. "forge_domain_Cause")
    pub decl_id: String,
    /// Kind: "struct", "enum", "impl", "fn", "trait", "type", "const", "static"
    pub kind: String,
    /// Type names defined by this decl
    pub defines: Vec<String>,
    /// For impl blocks: the self type (e.g. "Cause")
    pub impl_self_type: Option<String>,
    /// For impl blocks: the trait being implemented (e.g. "Display")
    pub impl_trait: Option<String>,
    /// All type identifiers referenced in the body
    pub references: Vec<String>,
    /// External crate names used (e.g. "serde", "thiserror")
    pub external_uses: Vec<String>,
    /// Full raw source content
    pub source: String,
    /// Source file path (relative to crate root)
    pub source_file: String,
    /// Human-readable hint about why this decl was excluded
    pub error_hint: String,
    /// CID of this document (self-referential, set after CBOR encoding)
    pub cid: String,
    /// Timestamp of archival
    pub timestamp: String,
}

impl SynVernacular {
    /// Build a vernacular document from a DeclNode that was excluded from generation.
    pub fn from_excluded(decl: &DeclNode, error_hint: &str) -> Self {
        // Extract impl-specific metadata from the source
        let (impl_self_type, impl_trait) = if decl.kind == "impl" {
            extract_impl_metadata(&decl.source)
        } else {
            (None, None)
        };

        SynVernacular {
            decl_id: decl.id.clone(),
            kind: decl.kind.clone(),
            defines: decl.defines.clone(),
            impl_self_type,
            impl_trait,
            references: decl.references.clone(),
            external_uses: decl.external_uses.clone(),
            source: decl.source.clone(),
            source_file: decl.source_file.clone(),
            error_hint: error_hint.to_string(),
            cid: String::new(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Extract impl self type and trait from source code.
fn extract_impl_metadata(source: &str) -> (Option<String>, Option<String>) {
    if let Ok(syntax) = syn::parse_file(source) {
        for item in &syntax.items {
            if let syn::Item::Impl(impl_item) = item {
                let self_type = if let syn::Type::Path(tp) = &*impl_item.self_ty {
                    tp.path.segments.last().map(|s| s.ident.to_string())
                } else {
                    None
                };
                let trait_name = impl_item.trait_.as_ref()
                    .and_then(|(_, path, _)| path.segments.last())
                    .map(|s| s.ident.to_string());
                return (self_type, trait_name);
            }
        }
    }
    (None, None)
}

/// Writes a CAR v1 file containing vernacular documents for the given decls.
///
/// CAR v1 format:
///   Header: <varint(len)> <cbor_header>
///   Block:  <varint(len)> <cid_bytes> <block_data>
///
/// Where cbor_header = {"version":1,"roots":[]}
fn write_car_archive(decls: &[DeclNode], excluded_ids: &HashSet<String>, 
                     output_path: &Path, full_plan_json: &str) -> Result<(usize, PathBuf)> {
    use std::io::Write;

    // Determine output file path
    let car_path = if output_path.is_dir() {
        output_path.join("excluded_decls.car")
    } else {
        let path = output_path.to_path_buf();
        path
    };
    
    // Collect excluded decls with their error hints
    let excluded: Vec<(String, DeclNode)> = decls.iter()
        .filter(|d| excluded_ids.contains(&d.id))
        .map(|d| (d.id.clone(), d.clone()))
        .collect();
    
    if excluded.is_empty() {
        return Ok((0, car_path));
    }

    let file = fs::File::create(&car_path)?;
    let mut writer = std::io::BufWriter::new(file);

    // ── Write CAR header ──────────────────────────────────────────────
    // Header CBOR: {"version": 1, "roots": []}
    let header_cbor = ciborium::value::Value::Map(vec![
        (
            ciborium::value::Value::Text("version".to_string()),
            ciborium::value::Value::Integer(1.into()),
        ),
        (
            ciborium::value::Value::Text("roots".to_string()),
            ciborium::value::Value::Array(vec![]),
        ),
    ]);
    let mut header_buf = Vec::new();
    ciborium::ser::into_writer(&header_cbor, &mut header_buf)
        .context("Failed to serialize CAR header")?;

    // Write LEB128-encoded header length
    write_leb128(&mut writer, header_buf.len() as u64)?;
    writer.write_all(&header_buf)?;

    // ── Write blocks (one per excluded decl) ──────────────────────────
    let mut blocks_written = 0;
    let mut all_roots: Vec<Vec<u8>> = Vec::new();

    for (decl_id, decl) in &excluded {
        // Build error hint based on decl kind and defines
        let error_hint = match decl.kind.as_str() {
            "impl" if deducible::is_cross_crate_impl(decl) => {
                "Cross-crate impl block (E0116): cannot impl a type defined in another plugin crate".to_string()
            }
            "impl" if deducible::has_nom_patterns(&decl.source) => {
                "Nom parser round-trip failure (E0277): syn tokenization of complex patterns".to_string()
            }
            _ => {
                format!("Excluded decl: {} ({}), defines={:?}", 
                    decl_id, decl.kind, decl.defines)
            }
        };

        let vernacular = SynVernacular::from_excluded(decl, &error_hint);

        // Serialize as CBOR (DAG-CBOR compatible)
        let mut cbor_buf = Vec::new();
        ciborium::ser::into_writer(&vernacular, &mut cbor_buf)
            .context(format!("Failed to serialize vernacular for {}", decl_id))?;

        // Compute CID
        let (cid_bytes, cid_str) = compute_cid(&cbor_buf);

        // Update the vernacular document's CID field and re-serialize with the CID
        // (This means the CID can never match the content — the CID is the pointer)
        let mut final_vernacular = vernacular;
        final_vernacular.cid = cid_str;
        let mut final_buf = Vec::new();
        ciborium::ser::into_writer(&final_vernacular, &mut final_buf)
            .context(format!("Failed to re-serialize vernacular for {}", decl_id))?;

        // Write CAR block: <leb128(len)> <cid_bytes> <data>
        let section_len = cid_bytes.len() as u64 + final_buf.len() as u64;
        write_leb128(&mut writer, section_len)?;
        writer.write_all(&cid_bytes)?;
        writer.write_all(&final_buf)?;

        blocks_written += 1;
        all_roots.push(cid_bytes);
    }

    // Flush and sync
    writer.flush()?;
    drop(writer);

    Ok((blocks_written, car_path))
}

/// Write a LEB128 unsigned integer.
fn write_leb128<W: Write>(writer: &mut W, mut value: u64) -> Result<()> {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte])?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}

/// Module for heuristic functions to avoid cluttering the main file.
mod deducible {
    use super::*;

    /// Does this decl look like a cross-crate impl? (E0116)
    pub fn is_cross_crate_impl(decl: &DeclNode) -> bool {
        if decl.kind != "impl" {
            return false;
        }
        // Check if the source has `impl ... for` pattern
        decl.source.contains("impl ") && decl.source.contains(" for ")
    }

    /// Does this decl use nom parser patterns that could cause round-trip issues?
    pub fn has_nom_patterns(source: &str) -> bool {
        source.contains("nom::") || source.contains("nom ")
    }
}

// ── CLI ──────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "decl-lattice", about = "Build dependency lattice from split declarations and generate FFI plugin crates")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze a crate's split decls and build the dependency lattice
    Analyze {
        /// Path to the crate directory (containing Cargo.toml and src/)
        #[arg(short, long)]
        crate_dir: PathBuf,

        /// Output JSON file for the lattice
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate plugin crates from the lattice
    Generate {
        /// Path to the crate directory (containing Cargo.toml and src/decls/)
        #[arg(short, long)]
        crate_dir: PathBuf,

        /// Output directory for generated plugin crates
        #[arg(short, long)]
        output: PathBuf,

        /// Maximum number of decls per plugin crate
        #[arg(long, default_value = "20")]
        max_decls: usize,

        /// Dry run: show what would be done without writing files
        #[arg(long)]
        dry_run: bool,

        /// Decl IDs to exclude (file stems, e.g. forge_domain_Cause)
        #[arg(long, num_args = 0..)]
        exclude: Vec<String>,

        /// File containing decl IDs to exclude (one per line)
        #[arg(long)]
        exclude_file: Option<PathBuf>,

        /// Output path for CAR archive of excluded decl verbal documents
        #[arg(long)]
        car_output: Option<PathBuf>,
    },
    /// Show the dependency graph in DOT format
    Dot {
        /// Path to the crate directory
        #[arg(short, long)]
        crate_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { crate_dir, output } => {
            let cargo_toml = crate_dir.join("Cargo.toml");
            let cargo_content = fs::read_to_string(&cargo_toml)?;
            let cargo_value: toml::Value = cargo_content.parse()?;
            let crate_name = cargo_value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();

            println!("Analyzing crate: {}", crate_name);
            let nodes = scan_decls(&crate_dir, &crate_name)?;
            println!("Analyzed {} declaration files", nodes.len());

            let lattice = build_lattice(&nodes)?;
            println!("Lattice: {} layers, {} edges", lattice.layers.len(), lattice.edges.len());

            for layer in &lattice.layers {
                println!("  Layer {}: {} decls", layer.depth, layer.nodes.len());
                for node_id in &layer.nodes {
                    if let Some(info) = lattice.decl_info.get(node_id) {
                        println!("    {} [{}] defines={:?} refs={}",
                            node_id, info.kind,
                            info.defines.iter().take(3).collect::<Vec<_>>(),
                            info.references.len()
                        );
                    }
                }
            }

            let json = serde_json::to_string_pretty(&lattice)?;
            match output {
                Some(path) => {
                    fs::write(&path, &json)?;
                    println!("Wrote lattice to {}", path.display());
                }
                None => println!("\n{}", json),
            }
        }
        Commands::Generate { crate_dir, output, max_decls, dry_run, exclude, exclude_file, car_output } => {
        let cargo_toml = crate_dir.join("Cargo.toml");
        let cargo_content = fs::read_to_string(&cargo_toml)?;
        let cargo_value: toml::Value = cargo_content.parse()?;
        let crate_name = cargo_value
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Build exclude set from CLI args + exclude file
        let mut exclude_set: HashSet<String> = exclude.into_iter().collect();
        if let Some(ref ef) = exclude_file {
            let file_content = fs::read_to_string(ef)?;
            for line in file_content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    exclude_set.insert(trimmed.to_string());
                }
            }
        }

        println!("Generating plugin crates for: {}", crate_name);
        println!("Excluding {} decls", exclude_set.len());
        let nodes = scan_decls(&crate_dir, &crate_name)?;
        let lattice = build_lattice(&nodes)?;

        println!("Lattice: {} layers", lattice.layers.len());
        let plan = generate_plugin_plan(&lattice, max_decls, &crate_dir, &exclude_set);
            println!("Plugin crates: {}", plan.plugin_crates.len());
            for plugin in &plan.plugin_crates {
                println!("  {} (layer {}, {} decls, deps: {:?})",
                    plugin.name, plugin.layer, plugin.decls.len(),
                    plugin.dependencies.iter().collect::<Vec<_>>()
                );
            }

            write_plugin_crates(&plan, &output, dry_run, &lattice, &exclude_set)?;

            // Write the plan as JSON
            let plan_json = serde_json::to_string_pretty(&plan)?;
            let plan_path = output.join("plugin_plan.json");
            if !dry_run {
                fs::create_dir_all(&output)?;
                fs::write(&plan_path, &plan_json)?;
                println!("Wrote plugin plan to {}", plan_path.display());
            }

            // Write CAR archive of excluded decls if requested
            if let Some(ref car_path) = car_output {
                let plan_json = serde_json::to_string_pretty(&plan)?;
                let (excluded_count, car_file) = write_car_archive(&nodes, &exclude_set, car_path, &plan_json)?;
                if excluded_count > 0 {
                    println!("Wrote CAR archive: {} ({} excluded decls)", car_file.display(), excluded_count);
                } else {
                    println!("No excluded decls to archive (CAR not created)");
                }
            }
        }
        Commands::Dot { crate_dir } => {
            let cargo_toml = crate_dir.join("Cargo.toml");
            let cargo_content = fs::read_to_string(&cargo_toml)?;
            let cargo_value: toml::Value = cargo_content.parse()?;
            let crate_name = cargo_value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();

            let nodes = scan_decls(&crate_dir, &crate_name)?;
            let lattice = build_lattice(&nodes)?;

            println!("digraph {} {{", crate_name.replace("-", "_"));
            println!("  rankdir=TB;");
            println!("  node [shape=box];");

            // Group nodes by layer for subgraph ranking
            for layer in &lattice.layers {
                println!("  subgraph layer_{} {{", layer.depth);
                println!("    rank=same;");
                for node_id in &layer.nodes {
                    if let Some(info) = lattice.decl_info.get(node_id) {
                        let label = format!("{}\\n{}", info.kind, info.defines.first().unwrap_or(&node_id.clone()));
                        println!("    {} [label=\"{}\"];", node_id.replace("-", "_"), label);
                    }
                }
                println!("  }}");
            }

            for (from, to) in &lattice.edges {
                println!("  {} -> {};",
                    from.replace("-", "_"),
                    to.replace("-", "_")
                );
            }

            println!("}}");
        }
    }

    Ok(())
}
