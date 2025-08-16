mod xcb;

use std::{
    cell::Cell,
    mem::ManuallyDrop,
    num::{NonZeroU32, NonZeroUsize},
    sync::{Arc, LazyLock},
    time::SystemTime,
};

use egui::{ClippedPrimitive, epaint::Primitive};
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

static CONTEXT: LazyLock<AppContext> = LazyLock::new(AppContext::new);

struct AppContext {
    egui_context: egui::Context,
    glow_context: Arc<glow::Context>,
    painter: egui_glow::Painter,
    start_time: SystemTime,

    window_id: Option<NonZeroU32>,
}
impl AppContext {
    pub fn new() -> Self {
        let egui_context = egui::Context::default();
        let glow_context = unsafe {
            let ctx = glow::Context::from_loader_function_cstr(|f| dlsym(RTLD_NEXT, f.as_ptr()));

            Arc::new(ctx)
        };
        let start_time = SystemTime::now();

        let painter = egui_glow::Painter::new(glow_context.clone(), "", None, true)
            .expect("Failed to create glow painter");

        let xconn = x11rb::connect(None).unwrap().0;
        let active_window_id = xcb::active_window_id(&xconn);

        Self {
            egui_context,
            glow_context,
            start_time,
            painter,

            window_id: active_window_id,
        }
    }

    /// Should be able to call this iteratively from our swap func
    pub fn step_overlay(&self) -> Vec<ClippedPrimitive> {
        let now = SystemTime::now();

        let time = now
            .duration_since(self.start_time)
            .ok()
            .map(|time| time.as_secs_f64());

        let raw_input = egui::RawInput {
            time,

            ..Default::default()
        };

        let output = self.egui_context.run(raw_input, |ui| {});

        self.egui_context
            .tessellate(output.shapes, output.pixels_per_point)
    }

    fn paint(&self, primitives: &[ClippedPrimitive]) {
    }
}

fn draw_primitive(p: ClippedPrimitive) {
    unsafe {
        match p.primitive {
            Primitive::Mesh(mesh) => {
                gl::Begin(gl::TRIANGLES);
                for [x, y, z] in mesh.triangles() {
                    gl::Vertex3i(x as i32, y as i32, z as i32);
                }
                gl::End();
            }
            Primitive::Callback(_f) => {
                unimplemented!("primitive callback function")
            }
        }
    }
}

#[unsafe(export_name = "glXSwapBuffers")]
#[allow(clippy::missing_safety_doc)]
extern "C" fn glx_swap_buffers(display: *mut Display, drawable: GLXDrawable) {
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::PushMatrix();

        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let triangles = CONTEXT.step_overlay();

        for t in triangles {
            draw_primitive(t);
        }

        gl::PopAttrib();
        gl::PopMatrix();

        GL_SWAP_FUNC(display, drawable);
    }
}
