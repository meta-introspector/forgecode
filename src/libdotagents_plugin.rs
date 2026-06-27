use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

/// ZOS plugin for dotagents - Agent Configuration Manager
/// This plugin exposes the dotagents functionality as a ZOS FFI plugin

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns the plugin name
#[no_mangle]
pub extern "C" fn zos_plugin_name() -> *const c_char {
    // SAFETY: The CString is leaked but that's fine for a plugin
    CString::new("dotagents-plugin").unwrap().into_raw()
}

/// Initialize the dotagents plugin
#[no_mangle]
pub extern "C" fn zos_plugin_init() -> i32 {
    if INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        // Already initialized
        return 0;
    }
    // Initialize plugin logic here - load dotagents skills, setup config, etc.
    0 // Success
}

/// Shutdown the dotagents plugin
#[no_mangle]
pub extern "C" fn zos_plugin_shutdown() -> i32 {
    if INITIALIZED.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        // Already shutdown
        return 0;
    }
    // Cleanup plugin logic here
    0 // Success
}

/// Returns the plugin version
#[no_mangle]
pub extern "C" fn zos_plugin_version() -> *const c_char {
    CString::new("0.1.2").unwrap().into_raw()
}

/// Returns the plugin description
#[no_mangle]
pub extern "C" fn zos_plugin_description() -> *const c_char {
    CString::new("Dotagents Agent Configuration Manager - Deploy and sync agent configuration across multiple AI coding agents").unwrap().into_raw()
}

/// Check if plugin is initialized
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zos_plugin_symbols_exist() {
        // Verify that the required symbols are exported
        assert!(!zos_plugin_name().is_null());
        assert!(!zos_plugin_version().is_null());
        assert!(!zos_plugin_description().is_null());
    }
}