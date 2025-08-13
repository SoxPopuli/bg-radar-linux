use std::{
    ffi::{CString, c_void},
    mem::ManuallyDrop,
    sync::{Arc, LazyLock, Mutex},
};

use libc::{RTLD_NEXT, dlsym};

#[allow(warnings)]
mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

struct Display;
type GLXDrawable = u32;
type GLXSwapBuffers = fn(*mut Display, GLXDrawable);

static GL_SWAP_FUNC: LazyLock<ManuallyDrop<Box<GLXSwapBuffers>>> = LazyLock::new(|| {
    let func = "glXSwapBuffers\0";
    unsafe {
        let f = dlsym(RTLD_NEXT, func.as_ptr().cast());
        ManuallyDrop::new(Box::from_raw(f.cast()))
    }
});

fn load_fn() {
    GL_SWAP_FUNC(std::ptr::null_mut(), 2);
}
