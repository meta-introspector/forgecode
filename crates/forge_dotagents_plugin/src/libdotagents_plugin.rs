/* ZOS Plugin for dotagents Integration */

use std::ffi::{CString, CStr};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_name() -> *const c_char {
    CString::new("dotagents").unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_init() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_shutdown() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_metadata() -> *const c_char {
    CString::new("ZOS Plugin for dotagents integration").unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_suggest_skills(context: *const c_char) -> *const c_char {
    unsafe {
        let context_str = if context.is_null() {
            "general".to_string()
        } else {
            CStr::from_ptr(context).to_string_lossy().to_string()
        };
        
        // Determine skill suggestions based on context
        let skills = match context_str.as_ref() {
            "math" | "formal" => vec![
                "aristo-mathlib",
                "aristo-consolidation",
                "proof-cid",
                "aristo-tools",
                "lean4-2-category-formalization"
            ],
            "code" | "engineering" => vec![
                "cargo-vendormod",
                "cargo-vendormod-graph",
                "vendormod-landscape",
                "vendormod-mcp",
                "forge_task_manager_plugin"
            ],
            "lean" | "formal-methods" => vec![
                "lean4",
                "lean4-declaration-splitter",
                "lean4-fuzz",
                "lean4-repl-server",
                "lean4-2-category-formalization"
            ],
            "task-planning" | "goap" => vec![
                "task-manager",
                "backlog",
                "qa-team-tile",
                "fuzzing-team-tile",
                "vax"
            ],
            _ => vec![
                "dotagents-providers",
                "system-manager",
                "task-manager",
                "backlog",
                "qa-team-tile"
            ]
        };
        
        // Convert to JSON string
        let json = serde_json::to_string(&skills).unwrap_or_default();
        CString::new(json).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            CString::from_raw(s);
        }
    }
}