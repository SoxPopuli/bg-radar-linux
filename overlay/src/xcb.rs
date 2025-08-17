use std::num::{NonZero, NonZeroU32};
use x11rb::{protocol::xproto::ConnectionExt, rust_connection::RustConnection};

pub fn active_window_id(conn: &RustConnection) -> Option<NonZeroU32> {
    let active_window_cookie = conn.intern_atom(true, b"_NET_ACTIVE_WINDOW").unwrap();

    let atom = active_window_cookie.reply().unwrap().atom;

    NonZero::new(atom)
}

#[derive(Debug)]
pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

pub fn window_geometry(conn: &RustConnection, window_id: NonZeroU32) -> Geometry {
    let geo = conn.get_geometry(window_id.get()).unwrap();

    let geo_reply = geo.reply().unwrap();

    Geometry {
        x: geo_reply.x,
        y: geo_reply.y,
        width: geo_reply.width,
        height: geo_reply.height,
    }
}
