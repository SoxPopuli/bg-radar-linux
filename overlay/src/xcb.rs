use std::num::{NonZero, NonZeroU32};

use x11rb::{protocol::xproto::ConnectionExt, rust_connection::RustConnection};

pub fn active_window_id(conn: &RustConnection) -> Option<NonZeroU32> {
    let active_window_cookie = conn.intern_atom(true, b"_NET_ACTIVE_WINDOW").unwrap();

    let atom = active_window_cookie.reply().unwrap().atom;

    NonZero::new(atom)
}

#[derive(Debug)]
struct Geometry {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

pub fn window_geometry(conn: &RustConnection, window_id: Option<NonZeroU32>) -> Geometry {
    let window_id = 
        window_id.or(
            active_window_id(conn)
        )
        .expect("Failed to get active window id");

    let geo = conn.get_geometry(window_id.get())
        .unwrap();

    let geo_reply = geo.reply().unwrap();

    Geometry {
        x: geo_reply.x,
        y: geo_reply.y,
        width: geo_reply.width,
        height: geo_reply.height,
    }
}
