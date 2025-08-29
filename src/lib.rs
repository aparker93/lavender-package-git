use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use git2::{RemoteCallbacks, Cred, FetchOptions};
use std::path::Path;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref LAST_ERR: Mutex<Option<String>> = Mutex::new(None);
    // Optional stored credentials (username, password) for HTTPS auth.
    static ref CREDENTIALS: Mutex<Option<(String, String)>> = Mutex::new(None);
}

fn set_err(s: String) {
    *LAST_ERR.lock().unwrap() = Some(s);
}

#[no_mangle]
pub extern "C" fn gitffi_clone(url: *const c_char, path: *const c_char) -> c_int {
    if url.is_null() || path.is_null() {
        set_err("null pointer".to_string());
        return -1;
    }
    eprintln!("git_ffi: git_clone called");
    let c_url = unsafe { CStr::from_ptr(url) };
    let c_path = unsafe { CStr::from_ptr(path) };
    let url = match c_url.to_str() { Ok(s) => s, Err(e) => { set_err(format!("bad url utf8: {}", e)); return -2; } };
    let path = match c_path.to_str() { Ok(s) => s, Err(e) => { set_err(format!("bad path utf8: {}", e)); return -3; } };

    // Prepare the repo builder. Only configure credential callbacks if we have
    // explicit credentials stored; otherwise let libgit2 perform anonymous
    // access (which is appropriate for public repositories).
    let mut builder = git2::build::RepoBuilder::new();
    if CREDENTIALS.lock().unwrap().is_some() {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            // First try explicitly-set credentials
            if let Some((u, p)) = &*CREDENTIALS.lock().unwrap() {
                return Cred::userpass_plaintext(&u, &p);
            }
            // Fallback: if username is suggested by URL, try ssh agent (if applicable)
            if let Some(user) = username_from_url {
                return Cred::ssh_key_from_agent(user);
            }
            // No credentials available
            Err(git2::Error::from_str("no credentials available"))
        });
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(callbacks);
        builder.fetch_options(fo);
    }

    let dest = Path::new(path);
    match builder.clone(url, dest) {
        Ok(_) => 0,
        Err(e) => { set_err(e.to_string()); -4 }
    }
}

#[no_mangle]
pub extern "C" fn gitffi_last_error_len() -> usize {
    if let Some(s) = &*LAST_ERR.lock().unwrap() {
        s.len()
    } else { 0 }
}

#[no_mangle]
pub extern "C" fn gitffi_last_error(buf: *mut c_char, buflen: usize) -> usize {
    if buf.is_null() { return 0; }
    if let Some(s) = &*LAST_ERR.lock().unwrap() {
        let bytes = s.as_bytes();
        let copy_len = std::cmp::min(bytes.len(), buflen.saturating_sub(1));
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
            *buf.add(copy_len) = 0;
        }
        bytes.len()
    } else { 0 }
}

/// Set global credentials for HTTPS operations.
#[no_mangle]
pub extern "C" fn gitffi_set_credentials(user: *const c_char, pass: *const c_char) -> c_int {
    if user.is_null() || pass.is_null() {
        return -1;
    }
    let u = unsafe { CStr::from_ptr(user) };
    let p = unsafe { CStr::from_ptr(pass) };
    let u = match u.to_str() { Ok(s) => s.to_string(), Err(_) => return -2 };
    let p = match p.to_str() { Ok(s) => s.to_string(), Err(_) => return -3 };
    *CREDENTIALS.lock().unwrap() = Some((u, p));
    0
}

/// Clear any stored credentials.
#[no_mangle]
pub extern "C" fn gitffi_clear_credentials() {
    *CREDENTIALS.lock().unwrap() = None;
}

// Provide a direct registration function matching the core's expectation.
// Returns ownership of a boxed Vec<FunctionDef> which the loader will take.
// Advertise provided functions to the interpreter via lavender_provide
// This returns a pointer to a NUL-terminated JSON string describing functions.
#[no_mangle]
pub extern "C" fn lavender_provide() -> *const c_char {
    // Example: [{"name":"clone","symbol":"git_clone","params":["url","path"]}]
    // Return a NUL-terminated C string (leaked for the lifetime of the process).
    // The core will read this pointer as a C string.
    let js = r#"[{"name":"clone","symbol":"gitffi_clone","params":["url","path"]}]"#;
    let cstring = std::ffi::CString::new(js).expect("CString::new failed");
    // Leak the CString so the pointer remains valid for the process lifetime.
    let ptr = cstring.into_raw();
    ptr as *const c_char
}
