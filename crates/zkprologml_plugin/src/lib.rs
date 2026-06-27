/* ZOS Plugin for ZKPrologML Integration */

use std::borrow::Cow;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_name() -> *const c_char {
    CString::new("zkprologml").unwrap().into_raw()
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
    CString::new("ZOS Plugin for ZKPrologML Integration").unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_suggest_skills(context: *const c_char) -> *const c_char {
    unsafe {
        let context_str = if context.is_null() {
            Cow::Borrowed("general")
        } else {
            CStr::from_ptr(context).to_string_lossy()
        };
        
        let skills = match context_str.as_ref() {
            "math" => vec![
                "zkprologml-math-logic",
                "zkprologml-formal-proofs",
                "zkprologml-arithmetization"
            ],
            "code" => vec![
                "zkprologml-code-integration",
                "zkprologml-types",
                "zkprologml-rs-to-prolog"
            ],
            "formal" => vec![
                "zkprologml-theorem-proving",
                "zkprologml-model-checking",
                "zkprologml-verification"
            ],
            "task-planning" => vec![
                "zkprologml-goap",
                "zkprologml-decision-tree",
                "zkprologml-plan-optimization"
            ],
            "zkprologml" => vec![
                "zkprologml-core",
                "zkprologml-plugins",
                "zkprologml-interfaces"
            ],
            _ => vec![]
        };
        
        let json = serde_json::to_string(&skills).unwrap_or_default();
        CString::new(json).unwrap().into_raw()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn zos_plugin_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}