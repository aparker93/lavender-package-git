git_ffi
=======

A small C-compatible dynamic library exposing minimal git functionality (clone + last error) to be used by Lavender via the `ffi` module.

Build:

    cd git_ffi
    cargo build --release

The produced library will be in target/release:
- macOS: libgit_ffi.dylib (or git_ffi.dylib depending on platform)
- Linux: libgit_ffi.so
- Windows: git_ffi.dll

Usage from Lavender:

- Copy the produced library to ~/.lavender/libs as libgit_ffi.so (or .dylib/.dll).
- In a Lavender script:

    load "ffi"
    ffi::load_library("gitffi", "/home/user/.lavender/libs/libgit_ffi.so")
    var ptr = ffi::load_symbol("gitffi", "git_clone")

Note: The FFI currently returns raw function pointers as integers; you will need a thin native wrapper or an added interpreter helper to call the function pointers directly. Alternatively, we can add higher-level native registration so the library registers functions into Lavender automatically; tell me if you want that.
