use std::ffi::{CStr, c_char, c_void};

use crate::{error::Error, ids::classes::Class, process::ProcessMemory, remote_ptr::RemotePtr, EntityPtr};

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

fn read<T>(process: impl ProcessMemory, ptr: RemotePtr<c_void>, offset: isize) -> Result<T, Error> {
    unsafe { ptr.byte_offset(offset).cast().read(process) }
}

fn read_bytes(
    process: impl ProcessMemory,
    ptr: RemotePtr<c_void>,
    offset: isize,
    len: usize,
) -> Result<Vec<u8>, Error> {
    unsafe { ptr.byte_offset(offset).read_bytes(process, len) }
}

fn read_array<T>(
    process: impl ProcessMemory,
    ptr: RemotePtr<c_void>,
    offset: isize,
    len: usize,
) -> Result<Vec<T>, Error> {
    unsafe { ptr.byte_offset(offset).cast().read_array(process, len) }
}

fn read_res_ref(
    process: impl ProcessMemory,
    ptr: RemotePtr<c_void>,
    offset: isize,
) -> Result<String, Error> {
    let bytes = read_bytes(process, ptr, offset, 8)?;

    let null_byte = bytes
        .iter()
        .enumerate()
        .find_map(|(x, c)| if *c == b'\0' { Some(x) } else { None });

    match null_byte {
        Some(end) => {
            let cs =
                CStr::from_bytes_with_nul(&bytes[..=end]).map_err(|e| Error::InvalidString {
                    msg: e.to_string(),
                    bytes: bytes.clone(),
                })?;
            Ok(cs.to_string_lossy().to_string())
        }
        None => {
            let s = str::from_utf8(&bytes).map_err(|e| Error::InvalidString {
                msg: e.to_string(),
                bytes: bytes.clone(),
            })?;

            Ok(s.to_string())
        }
    }
}

fn read_string(
    process: impl ProcessMemory + Copy,
    ptr: RemotePtr<c_void>,
    offset: isize,
    strlen: usize,
) -> Result<Option<String>, Error> {
    let char_ptr: *mut c_char = read(process, ptr, offset)?;
    let char_ptr = RemotePtr::new(char_ptr);
    let bytes = unsafe { char_ptr.read_bytes(process, strlen)? };

    Ok({
        CStr::from_bytes_until_nul(&bytes)
            .ok()
            .map(|slice| slice.to_str().unwrap().to_string())
    })
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Lookup<F, U> {
    Found(F),
    Unknown(U),
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CA/index.html#caiobjecttype
pub struct CAIObjectType {
    pub name: Option<String>,
    pub enemy_ally: i8,
    pub general: i8,
    pub race: i8,
    pub class: Lookup<Class, i8>,
    pub instance: i32,
    pub special_case: [i8; 5],
    pub specifics: i8,
    pub gender: i8,
    pub alignment: i8,
}
impl CAIObjectType {
    fn new(process: impl ProcessMemory + Copy, ptr: RemotePtr<c_void>) -> Result<Self, Error> {
        let name = read_string(process, ptr, 0x0, 8)?;
        let class = read(process, ptr, 0xB)?;

        let class = 
            Class::try_from(class as u8)
            .map(Lookup::Found)
            .unwrap_or(Lookup::Unknown(class));

        Ok(Self {
            name,
            enemy_ally: read(process, ptr, 0x8)?,
            general: read(process, ptr, 0x9)?,
            race: read(process, ptr, 0xA)?,
            class,
            instance: read(process, ptr, 0xC)?,
            special_case: read(process, ptr, 0x10)?,
            specifics: read(process, ptr, 0x15)?,
            gender: read(process, ptr, 0x16)?,
            alignment: read(process, ptr, 0x17)?,
        })
    }
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CG/index.html#cgameobject
pub struct CGameObject {
    pub object_type: ObjectType,
    pub pos: CPoint,
    pub pos_z: i32,
    pub list_type: u8,
    pub type_ai: CAIObjectType,
    pub id: i32,
    pub can_be_seen: i16,
}

/// Type docs: https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CG/index.html#cgameaibase
#[repr(C)]
#[derive(Debug)]
pub struct CGameAIBase {
    pub object: CGameObject,
}
impl CGameAIBase {
    pub fn new(
        process: impl ProcessMemory + Copy,
        entity: &EntityPtr,
    ) -> Result<Option<Self>, Error> {
        if !entity.is_valid() {
            return Ok(None);
        }

        Ok(Some(Self {
            object: CGameObject {
                object_type: read(process, entity.ptr, 0x8)?,
                pos: read(process, entity.ptr, 0xC)?,
                pos_z: read(process, entity.ptr, 0x14)?,
                list_type: read(process, entity.ptr, 0x28)?,
                type_ai: CAIObjectType::new(process, entity.ptr.byte_offset(0x30))?,
                id: read(process, entity.ptr, 0x48)?,
                can_be_seen: read(process, entity.ptr, 0x4C)?,
            },
        }))
    }
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CD/index.html#cderivedstats
pub struct CDerivedStats {
    max_hp: i16,
    ac: i16,
    thac0: i16,

    ac_crush_mod: i16,
    ac_missile_mod: i16,
    ac_pierce_mod: i16,
    ac_slash_mod: i16,

    number_of_attacks: i16,

    save_vs_death: i16,
    save_vs_wands: i16,
    save_vs_poly: i16,
    save_vs_breath: i16,
    save_vs_spell: i16,

    resist_fire: i16,
    resist_cold: i16,
    resist_electricity: i16,
    resist_acid: i16,
    resist_magic: i16,
    resist_magic_fire: i16,
    resist_magic_cold: i16,
    resist_slashing: i16,
    resist_crushing: i16,
    resist_piercing: i16,
    resist_missile: i16,

    level1: i16,
    level2: i16,
    level3: i16,

    str: i16,
    /// e.g. exceptional strength
    str_extra: i16,
    dex: i16,
    con: i16,
    int: i16,
    wis: i16,
    chr: i16,
}
impl CDerivedStats {
    pub fn new(process: impl ProcessMemory + Copy, ptr: RemotePtr<c_void>) -> Result<Self, Error> {
        Ok(Self {
            max_hp: read(process, ptr, 0x4)?,
            ac: read(process, ptr, 0x6)?,
            thac0: read(process, ptr, 0x10)?,

            ac_crush_mod: read(process, ptr, 0x8)?,
            ac_missile_mod: read(process, ptr, 0xA)?,
            ac_pierce_mod: read(process, ptr, 0xC)?,
            ac_slash_mod: read(process, ptr, 0xE)?,

            number_of_attacks: read(process, ptr, 0x12)?,

            save_vs_death: read(process, ptr, 0x14)?,
            save_vs_wands: read(process, ptr, 0x16)?,
            save_vs_poly: read(process, ptr, 0x18)?,
            save_vs_breath: read(process, ptr, 0x1A)?,
            save_vs_spell: read(process, ptr, 0x1C)?,

            resist_fire: read(process, ptr, 0x1E)?,
            resist_cold: read(process, ptr, 0x20)?,
            resist_electricity: read(process, ptr, 0x22)?,
            resist_acid: read(process, ptr, 0x24)?,
            resist_magic: read(process, ptr, 0x26)?,
            resist_magic_fire: read(process, ptr, 0x28)?,
            resist_magic_cold: read(process, ptr, 0x2A)?,

            resist_slashing: read(process, ptr, 0x2C)?,
            resist_crushing: read(process, ptr, 0x2E)?,
            resist_piercing: read(process, ptr, 0x30)?,
            resist_missile: read(process, ptr, 0x32)?,

            level1: read(process, ptr, 0x46)?,
            level2: read(process, ptr, 0x48)?,
            level3: read(process, ptr, 0x4A)?,

            str: read(process, ptr, 0x4E)?,
            str_extra: read(process, ptr, 0x50)?,
            int: read(process, ptr, 0x52)?,
            wis: read(process, ptr, 0x54)?,
            dex: read(process, ptr, 0x56)?,
            con: read(process, ptr, 0x58)?,
            chr: read(process, ptr, 0x5A)?,
        })
    }
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CC/index.html#ccreaturefileheader
pub struct CCreatureFileHeader {
    hp: i16,
    level1: i8,
    level2: i8,
    level3: i8,
}
impl CCreatureFileHeader {
    pub fn new(process: impl ProcessMemory + Copy, ptr: RemotePtr<c_void>) -> Result<Self, Error> {
        Ok(Self {
            hp: read(process, ptr, 0x1C)?,
            level1: read(process, ptr, 0x22C)?,
            level2: read(process, ptr, 0x22D)?,
            level3: read(process, ptr, 0x22E)?,
        })
    }
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CG/index.html#cgamesprite
pub struct CGameSprite {
    pub base: CGameAIBase,
    pub res_ref: String,
    pub base_stats: CCreatureFileHeader,
    pub name: String,
    pub derived_stats: CDerivedStats,
    pub current_area: String,
}
impl CGameSprite {
    pub fn new(
        process: impl ProcessMemory + Copy,
        entity: &EntityPtr,
        base: CGameAIBase,
    ) -> Result<Option<Self>, Error> {
        if !entity.is_valid() || base.object.object_type != ObjectType::Sprite {
            Ok(None)
        } else {
            let res_ref = read_res_ref(process, entity.ptr, 0x540)?;

            // 0x18 before value in docs?
            let name = read_string(process, entity.ptr, 0x3910, 64)?.unwrap();
            let current_area = read_res_ref(process, entity.ptr, 0x3A20 - 0x18)?;

            Ok(Some(Self {
                base,
                res_ref,
                base_stats: CCreatureFileHeader::new(process, entity.ptr.byte_offset(0x560))?,
                name,
                derived_stats: CDerivedStats::new(process, entity.ptr.byte_offset(0x1120))?,
                current_area,
            }))
        }
    }
}
