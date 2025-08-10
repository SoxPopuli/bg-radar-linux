use std::ffi::{c_char, c_void};

use crate::{EntityPtr, error::Error, process::GameProcess, remote_ptr::RemotePtr};

#[repr(u8)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum ObjectType {
    #[default]
    None = 0x00,
    AiBase = 0x01,
    Sound = 0x10,
    Container = 0x11,
    Spawning = 0x20,
    Door = 0x21,
    Static = 0x30,
    Sprite = 0x31,
    ObjectMarker = 0x40,
    Trigger = 0x41,
    TiledObject = 0x51,
    Temporal = 0x60,
    AreaAi = 0x61,
    Fireball = 0x70,
    GameAi = 0x71,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct CPoint {
    pub x: i32,
    pub y: i32,
}

struct ProcessPtr<'a> {
    process: &'a GameProcess,
    ptr: &'a RemotePtr<c_void>,
}
impl<'a> ProcessPtr<'a> {
    fn read<T>(&self, offset: isize) -> Result<T, Error> {
        unsafe { self.ptr.byte_offset(offset).cast().read(self.process) }
    }

    fn read_bytes(&self, offset: isize, len: usize) -> Result<Vec<u8>, Error> {
        unsafe { self.ptr.byte_offset(offset).read_bytes(self.process, len) }
    }

    fn read_array<T>(&self, offset: isize, len: usize) -> Result<Vec<T>, Error> {
        unsafe {
            self.ptr
                .byte_offset(offset)
                .cast()
                .read_array(self.process, len)
        }
    }
}

fn read_string(ptr: &ProcessPtr, offset: isize, strlen: usize) -> Result<String, Error> {
    let bytes: Vec<u8> = ptr.read_bytes(offset, strlen)?;
    let cstring = unsafe { std::ffi::CString::from_vec_unchecked(bytes) };

    Ok(cstring.to_string_lossy().to_string())
}

#[repr(C)]
#[derive(Debug, Default)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CA/index.html#caiobjecttype
pub struct CAIObjectType {
    name: String,
    enemy_ally: i8,
    general: i8,
    race: i8,
    class: i8,
    instance: i32,
    special_case: [i8; 5],
    specifics: i8,
    gender: i8,
    alignment: i8,
}
impl CAIObjectType {
    fn new(process: &GameProcess, ptr: &RemotePtr<c_void>) -> Result<Self, Error> {
        let p = ProcessPtr { process, ptr };

        Ok(Self {
            name: read_string(&p, 0x0, 8)?,
            ..Default::default()
        })
    }
}

#[repr(C)]
#[derive(Debug, Default)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CG/index.html#cgameobject
pub struct CGameObject {
    object_type: ObjectType,
    pos: CPoint,
    pos_z: i32,
    list_type: u8,
    type_ai: CAIObjectType,
    id: i32,
    can_be_seen: i16,
}

/// Type docs: https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CG/index.html#cgameaibase
#[repr(C)]
#[derive(Debug, Default)]
pub struct CGameAIBase {
    pub object: CGameObject,
}
impl CGameAIBase {
    pub fn new(
        process: &GameProcess,
        EntityPtr { id, ptr }: &EntityPtr,
    ) -> Result<Option<Self>, Error> {
        if *id == u16::MAX {
            return Ok(None);
        }

        let p = ProcessPtr { process, ptr };

        Ok(Some(Self {
            object: CGameObject {
                object_type: p.read(0x8)?,
                pos: p.read(0xC)?,
                pos_z: p.read(0x14)?,
                list_type: p.read(0x28)?,
                type_ai: CAIObjectType::new(process, &ptr.byte_offset(0x30))?,
                id: p.read(0x48)?,
                can_be_seen: p.read(0x4C)?,
            },
        }))
    }
}
