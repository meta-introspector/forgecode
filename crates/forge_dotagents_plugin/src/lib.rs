// Re-export the ZOS plugin symbols from the module
mod libdotagents_plugin;
pub use libdotagents_plugin::*;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

unsafe extern "C" {
    fn zos_plugin_suggest_skills(context: *const c_char) -> *const c_char;
    fn zos_plugin_free_string(s: *mut c_char);
}

/// Returns a vector of skill names.
pub fn suggest_skills(context: &str) -> Vec<String> {
    let c_context = CString::new(context).unwrap();
    unsafe {
        let raw = zos_plugin_suggest_skills(c_context.as_ptr());
        if raw.is_null() {
            return vec![];
        }
        let c_str = CStr::from_ptr(raw);
        let json = c_str.to_string_lossy();
        // Free the allocated string from the plugin side
        zos_plugin_free_string(raw as *mut c_char);
        serde_json::from_str(&json).unwrap_or_default()
    }
}