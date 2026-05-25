//! Cross-project pattern matcher — finds isomorphic crate usage patterns
//! between decl-split archives using syn-based fingerprinting and group-theoretic
//! overlap scoring.

// ── Imports ──────────────────────────────────────────────────────────────
use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// mod fingerprint; // not used — we define PatternFingerprint locally with CrateUsage
// ── Core types ───────────────────────────────────────────────────────────

/// A single usage of a crate in a decl file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum UsageKind {
    Derive(String),
    ImportPath(String),
    TraitImpl(String),
    FunctionReturn(String),
    FieldType(String),
    MethodCall(String),
}

type CrateUsage = (String, UsageKind);

/// Fingerprint of a single decl file.
#[derive(Debug, Clone, Serialize)]
pub struct PatternFingerprint {
    pub file: PathBuf,
    pub project: String,
    pub usages: BTreeSet<CrateUsage>,
}

impl PatternFingerprint {
    /// Build a fingerprint from a syn-parsed file.
    fn from_file(path: &Path, project: &str) -> Result<Self> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let syntax = syn::parse_file(&source)
            .map_err(|e| anyhow::anyhow!("syn parse error in {}: {e}", path.display()))?;

        let mut usages = BTreeSet::new();

        // Walk items for use statements, derives, trait impls, fn signatures
        for item in &syntax.items {
            use syn::Item;
            match item {
                Item::Use(u) => {
                    // Extract crate name from use tree
                    let cn = use_tree_root_name(&u.tree);
                    if !cn.is_empty() {
                        usages.insert((cn, UsageKind::ImportPath("use".to_string())));
                    }
                }
                Item::Struct(s) => {
                    for attr in &s.attrs {
                        if attr.path().is_ident("derive") {
                            extract_derive_crates(attr, &mut usages);
                        }
                    }
                }
                Item::Enum(e) => {
                    for attr in &e.attrs {
                        if attr.path().is_ident("derive") {
                            extract_derive_crates(attr, &mut usages);
                        }
                    }
                }
                Item::Impl(i) => {
                    // Extract trait being implemented
                    if let Some((_, trait_path, _)) = &i.trait_ {
                        if let Some(seg) = trait_path.segments.first() {
                            let crate_name = seg.ident.to_string();
                            let segments: Vec<syn::PathSegment> = trait_path.segments.iter().cloned().collect();
                            let trait_str = path_segments_to_string(&segments);
                            usages.insert((crate_name, UsageKind::TraitImpl(trait_str)));
                        }
                    }
                    // Extract derives on the impl block itself
                    for attr in &i.attrs {
                        if attr.path().is_ident("derive") {
                            extract_derive_crates(attr, &mut usages);
                        }
                    }
                }
                Item::Fn(f) => {
                    for attr in &f.attrs {
                        if attr.path().is_ident("derive") {
                            extract_derive_crates(attr, &mut usages);
                        }
                    }
                    // Return type
                    let ret_ty_str = quote::quote!(#f).to_string();
                    for seg in extract_type_paths(&ret_ty_str) {
                        if !["std", "core", "alloc", "Self", "Result"].contains(&seg.as_str()) {
                            usages.insert((seg.clone(), UsageKind::FunctionReturn(seg)));
                        }
                    }
                }
                Item::Trait(t) => {
                    for attr in &t.attrs {
                        if attr.path().is_ident("derive") {
                            extract_derive_crates(attr, &mut usages);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(PatternFingerprint {
            file: path.to_path_buf(),
            project: project.to_string(),
            usages,
        })
    }
}

/// Convert a use tree (like `serde_json::Value`) to a path string.
fn path_to_string(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => {
            let rest = path_to_string(&p.tree);
            if rest.is_empty() {
                p.ident.to_string()
            } else {
                format!("{}::{}", p.ident, rest)
            }
        }
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => r.ident.to_string(),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(g) => {
            let items: Vec<String> = g.items.iter().map(|i| path_to_string(i)).collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

/// Extract crate names from a `#[derive(...)]` attribute.
fn extract_derive_crates(attr: &syn::Attribute, usages: &mut BTreeSet<CrateUsage>) {
    let meta = &attr.meta;
    if let syn::Meta::List(list) = meta {
        for token in list.tokens.clone().into_iter() {
            let s = quote::quote!(#token).to_string().trim().to_string();
            let crate_name = s.trim_end_matches(',').to_string();
            if !crate_name.is_empty() && !["Debug", "Clone", "Copy", "PartialEq", "Eq",
                "PartialOrd", "Ord", "Hash", "Default", "Serialize", "Deserialize",
                "Display", "From", "Into", "Deref", "AsRef", "AsMut",
                "Setters", "Strum", "EnumString", "IntoStaticStr", "Display",
            ].contains(&crate_name.as_str()) {
                // Derive macro — the crate is the derive name lowercased typically
                // But for known derive crates, add them explicitly
                if s.contains("Deserialize") || s.contains("Serialize") {
                    usages.insert(("serde".to_string(), UsageKind::Derive(crate_name)));
                } else if s.contains("From") || s.contains("Into") {
                    usages.insert(("thiserror".to_string(), UsageKind::Derive(crate_name)));
                }
            }
        }
    }
}

/// Naive extraction of type-like identifiers from a token string.
fn extract_type_paths(s: &str) -> Vec<String> {
    // Very simple heuristic: find identifiers that look like crate names
    let mut paths = Vec::new();
    for word in s.split(|c: char| !c.is_alphanumeric() && c != '_' && c != ':') {
        let trimmed = word.trim().to_string();
        if trimmed.len() >= 3 && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let lower = trimmed.to_lowercase();
            // Skip Rust keywords and common std types
            if !["self", "Self", "result", "Result", "option", "Option", "box", "Box",
                  "vec", "Vec", "string", "String", "str", "bool", "true", "false",
                  "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64",
                  "f32", "f64", "usize", "isize", "mut", "let", "fn", "impl",
                  "const", "static", "pub", "crate", "super", "self"
            ].contains(&lower.as_str()) {
                paths.push(trimmed);
            }
        }
    }
    paths
}

/// Convert a syn `UseTree` to its root crate name (e.g. "serde" from "serde::Serialize").
fn use_tree_root_name(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => use_tree_root_name(&p.tree),
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => r.ident.to_string(),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(_) => String::new(),
    }
}

/// Stringify path segments (e.g. `["serde", "Serialize"]` → "serde::Serialize").
fn path_segments_to_string(segments: &[syn::PathSegment]) -> String {
    segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Per-project aggregate report.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectReport {
    pub name: String,
    pub total_decls: usize,
    pub pattern_counts: BTreeMap<Vec<CrateUsage>, usize>,
    pub crate_frequencies: BTreeMap<String, usize>,
    pub usage_kind_frequencies: BTreeMap<CrateUsage, usize>,
}

/// A pair of matching decl files (forge, pi) with the same pattern.
#[derive(Debug, Clone, Serialize)]
pub struct MatchPair {
    pub forge_file: PathBuf,
    pub pi_file: PathBuf,
    pub forge_project: String,
    pub pi_project: String,
    pub pattern: Vec<CrateUsage>,
    pub score: f64,
}

// ── CLI ──────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "decl-patterns", about = "Cross-project pattern matcher for decl-split archives")]
struct Cli {
    /// Decl directories: name=path (e.g. "forge_domain=/path/to/decls")
    #[arg(long = "decls-dir", short = 'd', required = true)]
    decls_dirs: Vec<String>,

    /// Output JSON file (optional, defaults to stdout)
    #[arg(long = "output", short = 'o')]
    output: Option<PathBuf>,
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse directory args
    let mut projects: Vec<(String, PathBuf)> = Vec::new();
    for arg in &cli.decls_dirs {
        let parts: Vec<&str> = arg.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("  [warn] skipping invalid format: {arg} (expected name=path)");
            continue;
        }
        let path = PathBuf::from(parts[1]);
        if !path.is_dir() {
            eprintln!("  [warn] directory not found: {}", path.display());
            continue;
        }
        projects.push((parts[0].to_string(), path));
    }

    if projects.len() < 2 {
        anyhow::bail!("Need at least 2 project directories to compare patterns");
    }

    // Compute fingerprints for each project
    eprintln!("  Computing fingerprints across {} projects...", projects.len());
    let all_fingerprints = Mutex::new(Vec::new());
    let reports = Mutex::new(Vec::new());
    let match_pairs = Mutex::new(Vec::new());

    // Phase 1: extract fingerprints
    let mut project_fingerprints: Vec<(String, Vec<PatternFingerprint>)> = Vec::new();

    for (name, dir) in &projects {
        eprintln!("    Scanning {name} at {}...", dir.display());
        let files = find_decl_files(dir);
        eprintln!("    Found {} decl files", files.len());

        let mut fps = Vec::with_capacity(files.len());
        for file in &files {
            match PatternFingerprint::from_file(file, name) {
                Ok(fp) => fps.push(fp),
                Err(e) => eprintln!("      [warn] error processing {}: {e}", file.display()),
            }
        }
        project_fingerprints.push((name.clone(), fps));
    }

    // Phase 2: build reports
    for (name, fps) in &project_fingerprints {
        let report = build_report(name, fps);
        reports.lock().unwrap().push(report);
        all_fingerprints.lock().unwrap().extend(fps.iter().cloned());
    }

    // Phase 3: find forge↔pi match pairs (direct file-to-file comparisons)
    // Build pattern→files index for each project
    let pattern_index: Vec<HashMap<Vec<CrateUsage>, Vec<PatternFingerprint>>> = project_fingerprints
        .iter()
        .map(|(_, fps)| {
            let mut idx: HashMap<Vec<CrateUsage>, Vec<PatternFingerprint>> = HashMap::new();
            for fp in fps {
                let key: Vec<CrateUsage> = fp.usages.iter().cloned().collect();
                idx.entry(key).or_default().push(fp.clone());
            }
            idx
        })
        .collect();

    // For each pair of projects, find matching decls
    for i in 0..project_fingerprints.len() {
        for j in (i + 1)..project_fingerprints.len() {
            let (name_a, _) = &project_fingerprints[i];
            let (name_b, _) = &project_fingerprints[j];

            // Only compare forge→pi pairs
            let is_forge_pi = (name_a.starts_with("forge") && name_b.starts_with("pi"))
                || (name_a.starts_with("pi") && name_b.starts_with("forge"));

            if !is_forge_pi && !name_a.starts_with("forge") && !name_b.starts_with("forge") {
                continue;
            }

            let (forge_name, pi_name, forge_idx, pi_idx) = if name_a.starts_with("forge") {
                (name_a, name_b, &pattern_index[i], &pattern_index[j])
            } else {
                (name_b, name_a, &pattern_index[j], &pattern_index[i])
            };

            for (pattern, forge_files) in forge_idx {
                if let Some(pi_files) = pi_idx.get(pattern) {
                    // For each forge file, match with each pi file that shares the same pattern
                    for f_fp in forge_files {
                        for p_fp in pi_files {
                            // Score: how many of forge's usages are covered by pi's usages
                            let forge_usage_count = f_fp.usages.len();
                            let overlap_count = f_fp.usages.intersection(&p_fp.usages).count();
                            let score = if forge_usage_count > 0 {
                                overlap_count as f64 / forge_usage_count as f64 * 100.0
                            } else {
                                100.0 // Both empty = identical
                            };

                            match_pairs.lock().unwrap().push(MatchPair {
                                forge_file: f_fp.file.clone(),
                                pi_file: p_fp.file.clone(),
                                forge_project: forge_name.clone(),
                                pi_project: pi_name.clone(),
                                pattern: pattern.clone(),
                                score,
                            });
                        }
                    }
                }
            }
        }
    }

    // Phase 4: output
    let all_reports = reports.lock().unwrap();
    let all_pairs = match_pairs.lock().unwrap();

    // Sort match pairs by score descending
    let mut sorted_pairs = all_pairs.clone();
    sorted_pairs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Print main report
    print_main_report(&all_reports, &all_fingerprints.lock().unwrap());

    // Print top 10 forge↔pi matches
    print_top_matches(&sorted_pairs);

    // Optionally write JSON
    if let Some(output_path) = &cli.output {
        let reports_json: Vec<serde_json::Value> = all_reports
            .iter()
            .map(|r| {
                let usage_json: serde_json::Value = r
                    .usage_kind_frequencies
                    .iter()
                    .map(|((c, k), &count)| {
                        let kind_label = match k {
                            UsageKind::ImportPath(_) => "import",
                        };
                        (format!("{}({})", c, kind_label), count)
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>();
                let usage_map = serde_json::Value::Object(usage_json);
                let pattern_count = r.pattern_counts.len();
                serde_json::json!({
                    "name": r.name,
                    "total_decls": r.total_decls,
                    "crate_frequencies": r.crate_frequencies,
                    "usage_kind_frequencies": usage_map,
                    "pattern_count": pattern_count,
                })
            })
            .collect();
        let output = serde_json::json!({
            "reports": reports_json,
            "top_matches": &sorted_pairs.iter().take(50).map(|m| serde_json::json!({
                "source_project": m.source_project,
                "target_project": m.target_project,
                "score": m.score,
                "source_file": m.source_file,
                "target_file": m.target_file,
                "details": m.details,
            })).collect::<Vec<_>>(),
            "total_matches": all_pairs.len(),
            "total_decls": all_fingerprints.lock().unwrap().len(),
        });
        std::fs::write(output_path, serde_json::to_string_pretty(&output)?)
            .context("Failed to write output JSON")?;
        eprintln!("\n  Wrote detailed JSON to {}", output_path.display());
    }

    Ok(())
}

// ── Report building ──────────────────────────────────────────────────────

fn build_report(name: &str, fingerprints: &[PatternFingerprint]) -> ProjectReport {
    let mut pattern_counts: BTreeMap<Vec<CrateUsage>, usize> = BTreeMap::new();
    let mut crate_frequencies: BTreeMap<String, usize> = BTreeMap::new();
    let mut usage_kind_frequencies: BTreeMap<CrateUsage, usize> = BTreeMap::new();

    for fp in fingerprints {
        let key: Vec<CrateUsage> = fp.usages.iter().cloned().collect();
        *pattern_counts.entry(key).or_insert(0) += 1;

        for (crate_name, usage_kind) in &fp.usages {
            *crate_frequencies.entry(crate_name.clone()).or_insert(0) += 1;
            *usage_kind_frequencies
                .entry((crate_name.clone(), usage_kind.clone()))
                .or_insert(0) += 1;
        }
    }

    ProjectReport {
        name: name.to_string(),
        total_decls: fingerprints.len(),
        pattern_counts,
        crate_frequencies,
        usage_kind_frequencies,
    }
}

fn overlap_score(from: &ProjectReport, to: &ProjectReport) -> f64 {
    if from.total_decls == 0 {
        return 0.0;
    }
    let from_patterns: HashSet<Vec<CrateUsage>> =
        from.pattern_counts.keys().cloned().collect();
    let to_patterns: HashSet<Vec<CrateUsage>> =
        to.pattern_counts.keys().cloned().collect();
    let shared = from_patterns.intersection(&to_patterns).count();
    (shared as f64 / from.total_decls as f64) * 100.0
}

fn find_decl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name != "_decl_module_invocation.rs" {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    files
}

// ── Printing ─────────────────────────────────────────────────────────────

fn format_usage_kind_short(kind: &UsageKind) -> String {
    match kind {
        UsageKind::Derive(_) => "derive",
        UsageKind::ImportPath(_) => "import",
        UsageKind::TraitImpl(_) => "impl",
        UsageKind::FunctionReturn(_) => "fn_return",
        UsageKind::FieldType(_) => "field",
        UsageKind::MethodCall(_) => "method",
    }
    .to_string()
}

fn format_crate_usage((crate_name, kind): &CrateUsage) -> String {
    match kind {
        UsageKind::Derive(t) => format!("  {crate_name}: derive({t})"),
        UsageKind::ImportPath(p) => format!("  {crate_name}: use {p}"),
        UsageKind::TraitImpl(t) => format!("  {crate_name}: impl {t}"),
        UsageKind::FunctionReturn(t) => format!("  {crate_name}: fn returns {t}"),
        UsageKind::FieldType(t) => format!("  {crate_name}: field {t}"),
        UsageKind::MethodCall(m) => format!("  {crate_name}: .{m}()"),
    }
}

fn print_main_report(reports: &[ProjectReport], all_fingerprints: &[PatternFingerprint]) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║      Cross-Project Crate Usage Pattern Report              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    for report in reports {
        let total = report.total_decls;
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Project: {}", report.name);
        println!("  Declarations analyzed: {}", total);
        println!();

        // Most common crate usage patterns
        println!("  ── Most common crate usage patterns ──");
        let mut sorted_patterns: Vec<_> = report.pattern_counts.iter().collect();
        sorted_patterns.sort_by(|a, b| b.1.cmp(a.1));
        for (pattern, count) in sorted_patterns.iter().take(10) {
            let pct = (**count as f64 / total as f64) * 100.0;
            if pattern.is_empty() {
                println!("    [no external crate usage] — {} decls ({:.1}%)", count, pct);
            } else {
                println!("    {} decls ({:.1}%):", count, pct);
                for usage in pattern.iter().take(5) {
                    println!("{}", format_crate_usage(usage));
                }
                if pattern.len() > 5 {
                    println!("      ... and {} more", pattern.len() - 5);
                }
            }
            println!();
        }

        // Most commonly used crates
        println!("  ── Most frequently used crates ──");
        let mut sorted_crates: Vec<_> = report.crate_frequencies.iter().collect();
        sorted_crates.sort_by(|a, b| b.1.cmp(a.1));
        for (crate_name, count) in sorted_crates.iter().take(15) {
            let pct = (**count as f64 / total as f64) * 100.0;
            println!("    {:<25} {:>4} decls ({:.1}%)", crate_name, count, pct);
        }
        println!();

        // Most common (crate, usage_kind) pairs
        println!("  ── Most common (crate, usage_kind) pairs ──");
        let mut sorted_pairs: Vec<_> = report.usage_kind_frequencies.iter().collect();
        sorted_pairs.sort_by(|a, b| b.1.cmp(a.1));
        for ((crate_name, kind), count) in sorted_pairs.iter().take(15) {
            let pct = (**count as f64 / total as f64) * 100.0;
            println!("    {:<25} {:<20} {:>4} decls ({:.1}%)",
                crate_name,
                format_usage_kind_short(kind),
                count,
                pct
            );
        }
        println!();
    }

    // Cross-project shared patterns
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Cross-Project Shared Patterns");
    println!();

    if reports.len() >= 2 {
        let mut shared_across_all: Option<HashSet<Vec<CrateUsage>>> = None;
        for report in reports {
            let patterns: HashSet<Vec<CrateUsage>> =
                report.pattern_counts.keys().cloned().collect();
            shared_across_all = match shared_across_all {
                None => Some(patterns),
                Some(existing) => Some(existing.intersection(&patterns).cloned().collect()),
            };
        }

        if let Some(shared) = shared_across_all {
            let mut shared_count: BTreeMap<Vec<CrateUsage>, usize> = BTreeMap::new();
            for fp in all_fingerprints {
                let key: Vec<CrateUsage> = fp.usages.iter().cloned().collect();
                if shared.contains(&key) {
                    *shared_count.entry(key).or_insert(0) += 1;
                }
            }

            println!("  Patterns shared across ALL projects ({} unique patterns):", shared.len());
            let mut sorted_shared: Vec<_> = shared_count.iter().collect();
            sorted_shared.sort_by(|a, b| b.1.cmp(a.1));
            for (pattern, count) in sorted_shared.iter().take(10) {
                if pattern.is_empty() {
                    println!("    [empty pattern] — {} total decls", count);
                } else {
                    println!("    {} total decls:", count);
                    for usage in pattern.iter().take(4) {
                        println!("{}", format_crate_usage(usage));
                    }
                    if pattern.len() > 4 {
                        println!("      ... and {} more", pattern.len() - 4);
                    }
                }
                println!();
            }
        }

        // Pairwise overlap scores
        println!("  ── Pairwise Overlap Scores ──");
        for i in 0..reports.len() {
            for j in (i + 1)..reports.len() {
                let a = &reports[i];
                let b = &reports[j];
                let score_ab = overlap_score(a, b);
                let score_ba = overlap_score(b, a);
                println!(
                    "    {} → {}: {:.1}% of {} patterns found in {}",
                    a.name, b.name, score_ab, a.name, b.name
                );
                println!(
                    "    {} → {}: {:.1}% of {} patterns found in {}",
                    b.name, a.name, score_ba, b.name, a.name
                );
                println!();
            }
        }
    }
}

fn print_top_matches(pairs: &[MatchPair]) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Top Forge↔Pi-Agent Matching Declarations");
    println!();

    // Group by forge_project
    let mut by_project: BTreeMap<String, Vec<&MatchPair>> = BTreeMap::new();
    for pair in pairs {
        by_project
            .entry(pair.forge_project.clone())
            .or_default()
            .push(pair);
    }

    for (proj, project_pairs) in &by_project {
        // Deduplicate: for each forge file, keep only the highest-scoring pi match
        let mut best_per_forge_file: BTreeMap<PathBuf, &MatchPair> = BTreeMap::new();
        for pair in project_pairs {
            let entry = best_per_forge_file.entry(pair.forge_file.clone()).or_insert(pair);
            if pair.score > entry.score {
                *entry = pair;
            }
        }

        // Sort by score descending
        let mut sorted: Vec<_> = best_per_forge_file.values().collect();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let top_n = sorted.len().min(10);
        println!("  {proj} → pi_agent (top {top_n} of {} matches):", project_pairs.len());
        println!();

        for (rank, pair) in sorted.iter().enumerate().take(10) {
            let forge_stem = pair.forge_file.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
            let pi_stem = pair.pi_file.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
            let forge_rel = pair.forge_file.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("?");
            let pi_rel = pair.pi_file.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("?");

            println!("  #{:<2} Score: {:>5.1}%", rank + 1, pair.score);
            println!("      Forge: {proj}/decls/{forge_rel}/{forge_stem}.rs");
            println!("      Pi:    {}/decls/{}/{}.rs", pair.pi_project, pi_rel, pi_stem);
            println!("      Pattern ({} usages):", pair.pattern.len());
            for usage in pair.pattern.iter().take(6) {
                println!("{}", format_crate_usage(usage));
            }
            if pair.pattern.len() > 6 {
                println!("        ... and {} more", pair.pattern.len() - 6);
            }
            println!();
        }
    }

    println!();
    println!("  Total unique forge→pi match pairs: {}", pairs.len());
}
