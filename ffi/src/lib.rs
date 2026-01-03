//! C FFI bindings for Charybdis benchmarking library.
//!
//! All strings returned must be freed using `charybdis_free_string`.
//! All session pointers must be freed using `charybdis_session_free`.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

use charybdis::{BenchmarkConfig, BenchmarkSession, ProgressEvent, WorkloadConfig};

/// Global tokio runtime
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Opaque session handle
pub struct CharybdisSession {
    inner: BenchmarkSession,
}

/// Progress callback function pointer type.
/// Called with JSON-encoded ProgressEvent and user-provided data pointer.
/// Pass NULL to disable progress updates.
pub type CharybdisProgressCallback = Option<extern "C" fn(json: *const c_char, user_data: *mut c_void)>;

/// Free a string allocated by charybdis
#[no_mangle]
pub extern "C" fn charybdis_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}

/// Get charybdis version
#[no_mangle]
pub extern "C" fn charybdis_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    CString::new(version).unwrap().into_raw()
}

/// Create a new benchmark session
///
/// Returns NULL on error. Use `charybdis_last_error` to get error message.
#[no_mangle]
pub extern "C" fn charybdis_session_new(
    config_json: *const c_char,
    workload_json: *const c_char,
) -> *mut CharybdisSession {
    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let workload_str = match unsafe { CStr::from_ptr(workload_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let config: BenchmarkConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return ptr::null_mut(),
    };
    let workload: WorkloadConfig = match serde_json::from_str(workload_str) {
        Ok(w) => w,
        Err(_) => return ptr::null_mut(),
    };

    match BenchmarkSession::new(config, workload) {
        Ok(session) => Box::into_raw(Box::new(CharybdisSession { inner: session })),
        Err(_) => ptr::null_mut(),
    }
}

/// Get session ID
#[no_mangle]
pub extern "C" fn charybdis_session_id(session: *const CharybdisSession) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    let session = unsafe { &*session };
    CString::new(session.inner.id()).unwrap().into_raw()
}

/// Run the benchmark with progress callback
///
/// Returns JSON results string or NULL on error.
#[no_mangle]
pub extern "C" fn charybdis_session_run(
    session: *mut CharybdisSession,
    callback: CharybdisProgressCallback,
    user_data: *mut c_void,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    let session = unsafe { &*session };

    // Wrap callback data for thread safety
    // SAFETY: The caller guarantees the callback and user_data are valid
    // for the duration of the benchmark run
    struct CallbackWrapper {
        cb: extern "C" fn(*const c_char, *mut c_void),
        data: *mut c_void,
    }
    unsafe impl Send for CallbackWrapper {}
    unsafe impl Sync for CallbackWrapper {}

    let wrapper = callback.map(|cb| CallbackWrapper { cb, data: user_data });

    let progress_callback = move |event: ProgressEvent| {
        if let Some(ref w) = wrapper {
            if let Ok(json) = serde_json::to_string(&event) {
                if let Ok(cstr) = CString::new(json) {
                    (w.cb)(cstr.as_ptr(), w.data);
                }
            }
        }
    };

    let result = get_runtime().block_on(session.inner.run(progress_callback));

    match result {
        Ok(results) => {
            match serde_json::to_string(&results) {
                Ok(json) => CString::new(json).unwrap().into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Cancel a running benchmark
#[no_mangle]
pub extern "C" fn charybdis_session_cancel(session: *mut CharybdisSession) {
    if !session.is_null() {
        let session = unsafe { &*session };
        session.inner.cancel();
    }
}

/// Free a session
#[no_mangle]
pub extern "C" fn charybdis_session_free(session: *mut CharybdisSession) {
    if !session.is_null() {
        unsafe { let _ = Box::from_raw(session); }
    }
}

/// Validate a Rune script
///
/// Returns NULL if valid, or JSON error array if invalid.
/// The JSON array contains objects with: line, column, message, severity
#[no_mangle]
pub extern "C" fn charybdis_validate_script(script: *const c_char) -> *mut c_char {
    let script_str = match unsafe { CStr::from_ptr(script) }.to_str() {
        Ok(s) => s,
        Err(_) => return CString::new(r#"[{"line":1,"column":1,"message":"Invalid UTF-8","severity":"error"}]"#).unwrap().into_raw(),
    };

    match charybdis::validate_script(script_str) {
        Ok(()) => ptr::null_mut(),
        Err(e) => {
            let err_str = e.to_string();
            // Check if error already contains structured DIAG_JSON
            if err_str.starts_with("DIAG_JSON:") {
                // Pass through the structured JSON directly
                CString::new(err_str).unwrap().into_raw()
            } else {
                // Wrap plain error message in structured format
                let errors = vec![serde_json::json!({
                    "line": 1,
                    "column": 1,
                    "message": err_str,
                    "severity": "error"
                })];
                CString::new(serde_json::to_string(&errors).unwrap()).unwrap().into_raw()
            }
        }
    }
}

/// Get built-in workload templates as JSON
#[no_mangle]
pub extern "C" fn charybdis_builtin_workloads() -> *mut c_char {
    let templates = charybdis::builtin_workloads();
    match serde_json::to_string(&templates) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
