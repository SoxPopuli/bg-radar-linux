mod xcb;
mod gui;

use std::{
    mem::ManuallyDrop,
    num::NonZeroU32,
    sync::{Arc, LazyLock, Mutex},
    time::SystemTime,
};

use egui::{ClippedPrimitive, Event, PointerButton};
use libc::{RTLD_NEXT, dlsym};
use x11rb::{
    connection::Connection,
    protocol::xproto::{ButtonMask, KeyButMask, ModMask},
};

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

static CONTEXT: LazyLock<Mutex<AppContext>> = LazyLock::new(|| {
    let ctx = AppContext::new();
    Mutex::new(ctx)
});

struct EventCollectorData {
    active: bool,
    events: Vec<Event>,
}

fn button_event(x: i16, y: i16, state: KeyButMask, pressed: bool) -> Option<Event> {
    let button = {
        if state.contains(ButtonMask::M1) {
            Some(PointerButton::Primary)
        } else if state.contains(ButtonMask::M2) {
            Some(PointerButton::Secondary)
        } else if state.contains(ButtonMask::M3) {
            Some(PointerButton::Middle)
        } else if state.contains(ButtonMask::M4) {
            Some(PointerButton::Extra1)
        } else if state.contains(ButtonMask::M5) {
            Some(PointerButton::Extra2)
        } else {
            None
        }
    };

    button.map(|button| egui::Event::PointerButton {
        pos: egui::Pos2 {
            x: x.into(),
            y: y.into(),
        },
        button,
        pressed,
        modifiers: egui::Modifiers {
            shift: state.contains(ModMask::SHIFT),
            ctrl: state.contains(ModMask::CONTROL),
            ..Default::default()
        },
    })
}

struct EventCollector {
    data: Arc<Mutex<EventCollectorData>>,
    handle: Option<std::thread::JoinHandle<()>>,
}
impl EventCollector {
    fn spawn_collection_thread(
        data: Arc<Mutex<EventCollectorData>>,
        window_id: u32,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            use x11rb::protocol::Event as XEvent;

            let (conn, _) = x11rb::connect(None).unwrap();

            loop {
                let mut data = data.lock().unwrap();
                if !data.active {
                    break;
                }

                let e = conn.poll_for_event().expect("Failed to poll for X Event");
                match e {
                    Some(XEvent::ButtonPress(e)) => {
                        if e.event != window_id {
                            continue;
                        }

                        button_event(e.event_x, e.event_y, e.state, true)
                            .into_iter()
                            .for_each(|e| data.events.push(e));
                    }
                    Some(XEvent::ButtonRelease(e)) => {
                        if e.event != window_id {
                            continue;
                        }

                        button_event(e.event_x, e.event_y, e.state, false)
                            .into_iter()
                            .for_each(|e| data.events.push(e));
                    }

                    Some(XEvent::MotionNotify(e)) => {
                        if e.event != window_id {
                            continue;
                        }

                        let evt = Event::PointerMoved(egui::Pos2 {
                            x: e.event_x.into(),
                            y: e.event_y.into(),
                        });

                        data.events.push(evt);
                    }

                    _ => continue,
                }
            }
        })
    }

    pub fn new(window_id: u32) -> Self {
        let data = Arc::new(Mutex::new(EventCollectorData {
            active: true,
            events: vec![],
        }));

        let handle = Self::spawn_collection_thread(data.clone(), window_id);

        Self {
            data,
            handle: Some(handle),
        }
    }

    pub fn take_events(&self) -> Vec<Event> {
        let mut lock = self.data.lock().unwrap();
        std::mem::take(&mut lock.events)
    }
}
impl Drop for EventCollector {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.active = false;
        drop(data);

        let handle = std::mem::take(&mut self.handle);
        if let Some(handle) = handle {
            handle.join().unwrap()
        }
    }
}

struct AppContext {
    egui_context: egui::Context,
    glow_context: Arc<glow::Context>,
    painter: egui_glow::Painter,
    start_time: SystemTime,

    x_connection: x11rb::rust_connection::RustConnection,
    window_id: NonZeroU32,

    event_collector: EventCollector,
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
        let active_window_id = xcb::active_window_id(&xconn).expect("Missing window id");

        let event_collector = EventCollector::new(active_window_id.get());

        Self {
            egui_context,
            glow_context,
            start_time,
            painter,

            x_connection: xconn,
            window_id: active_window_id,
            event_collector,
        }
    }

    /// Should be able to call this iteratively from our swap func
    pub fn step_overlay(&self) -> (Vec<ClippedPrimitive>, f32) {
        let now = SystemTime::now();

        let time = now
            .duration_since(self.start_time)
            .ok()
            .map(|time| time.as_secs_f64());

        let raw_input = egui::RawInput {
            time,
            events: self.event_collector.take_events(),

            ..Default::default()
        };

        let output = self.egui_context.run(raw_input, gui::Gui::run);

        let p = self
            .egui_context
            .tessellate(output.shapes, output.pixels_per_point);

        (p, output.pixels_per_point)
    }

    fn paint(&mut self, primitives: &[ClippedPrimitive], pixels_per_point: f32) {
        let xcb::Geometry { width, height, .. } =
            xcb::window_geometry(&self.x_connection, self.window_id);
        let screen_size = [width as u32, height as u32];

        self.painter
            .paint_primitives(screen_size, pixels_per_point, primitives);
    }
}

#[unsafe(export_name = "glXSwapBuffers")]
#[allow(clippy::missing_safety_doc)]
extern "C" fn glx_swap_buffers(display: *mut Display, drawable: GLXDrawable) {
    let mut context_lock = CONTEXT.lock().expect("Couldn't acquire context lock");

    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::PushMatrix();

        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let (triangles, pixels_per_point) = context_lock.step_overlay();
        context_lock.paint(&triangles, pixels_per_point);

        gl::PopAttrib();
        gl::PopMatrix();

        GL_SWAP_FUNC(display, drawable);
    }
}
