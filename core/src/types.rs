use std::ffi::{CStr, c_char, c_void};

use crate::{
    EntityPtr,
    error::Error,
    ids::{
        alignment::Alignment,
        classes::{Class, ClassLevels},
        effect::Effect,
        enemy_ally::EnemyAlly,
        gender::Gender,
        general::General,
        race::Race,
    },
    process::ProcessMemory,
    remote_ptr::RemotePtr,
};

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
pub enum Lookup<T, U> {
    Found(T),
    Unknown(U),
}
impl<T, U> Lookup<T, U> {
    pub fn to_option(self) -> Option<T> {
        match self {
            Self::Found(f) => Some(f),
            Self::Unknown(_) => None,
        }
    }

    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Found(f) => Some(f),
            Self::Unknown(_) => None,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
/// https://eeex-docs.readthedocs.io/en/latest/EE%20Game%20Structures%20%28x64%29/CA/index.html#caiobjecttype
pub struct CAIObjectType {
    pub name: Option<String>,
    pub enemy_ally: Lookup<EnemyAlly, u8>,
    pub general: Lookup<General, u8>,
    pub race: Lookup<Race, u8>,
    pub class: Lookup<Class, u8>,
    pub instance: i32,
    pub special_case: [u8; 5],
    pub specifics: u8,
    pub gender: Lookup<Gender, u8>,
    pub alignment: Lookup<Alignment, u8>,
}
impl CAIObjectType {
    fn new(process: impl ProcessMemory + Copy, ptr: RemotePtr<c_void>) -> Result<Self, Error> {
        let name = read_string(process, ptr, 0x0, 8)?;

        macro_rules! to_lookup {
            ($enum_type: ty, $offset: expr) => {{
                read(process, ptr, $offset).map(|x| {
                    <$enum_type>::try_from(x)
                        .map(Lookup::Found)
                        .unwrap_or(Lookup::Unknown(x))
                })
            }};
        }

        Ok(Self {
            name,
            enemy_ally: to_lookup!(EnemyAlly, 0x8)?,
            general: to_lookup!(General, 0x9)?,
            race: to_lookup!(Race, 0xA)?,
            class: to_lookup!(Class, 0xB)?,
            instance: read(process, ptr, 0xC)?,
            special_case: read(process, ptr, 0x10)?,
            specifics: read(process, ptr, 0x15)?,
            gender: to_lookup!(Gender, 0x16)?,
            alignment: to_lookup!(Alignment, 0x17)?,
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
    pub max_hp: i16,
    pub ac: i16,
    pub thac0: i16,

    pub ac_crush_mod: i16,
    pub ac_missile_mod: i16,
    pub ac_pierce_mod: i16,
    pub ac_slash_mod: i16,

    pub number_of_attacks: i16,

    pub save_vs_death: i16,
    pub save_vs_wands: i16,
    pub save_vs_poly: i16,
    pub save_vs_breath: i16,
    pub save_vs_spell: i16,

    pub resist_fire: i16,
    pub resist_cold: i16,
    pub resist_electricity: i16,
    pub resist_acid: i16,
    pub resist_magic: i16,
    pub resist_magic_fire: i16,
    pub resist_magic_cold: i16,
    pub resist_slashing: i16,
    pub resist_crushing: i16,
    pub resist_piercing: i16,
    pub resist_missile: i16,

    pub level1: i16,
    pub level2: i16,
    pub level3: i16,

    pub str: i16,
    /// e.g. exceptional strength
    pub str_extra: i16,
    pub dex: i16,
    pub con: i16,
    pub int: i16,
    pub wis: i16,
    pub chr: i16,
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
    pub hp: i16,
    pub level1: i8,
    pub level2: i8,
    pub level3: i8,
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
pub struct CGameEffect {
    version: String,
    res: String,
    res_2: String,
    res_3: String,
    effect_id: Effect,
    duration: u32,
    duration_type: u32,
    spell_level: i32,
    source_res: String,
}
impl CGameEffect {
    pub fn new(process: impl ProcessMemory + Copy, ptr: RemotePtr<c_void>) -> Result<Self, Error> {
        let base_ptr = ptr.byte_offset(0x8);

        let version = read_res_ref(process, base_ptr, 0x0)?;
        let res = read_res_ref(process, base_ptr, 0x28)?;
        let spell_level = read(process, base_ptr, 0x10)?;

        Ok(Self {
            version,
            res,
            res_2: read_res_ref(process, base_ptr, 0x68)?,
            res_3: read_res_ref(process, base_ptr, 0x70)?,
            effect_id: read(process, base_ptr, 0x8)?,
            duration_type: read(process, base_ptr, 0x1C)?,
            duration: read(process, base_ptr, 0x20)?,
            spell_level,
            source_res: read_res_ref(process, base_ptr, 0x8C)?,
        })
    }
}

fn read_ptr_list<T, P: ProcessMemory + Copy>(
    process: P,
    base_ptr: RemotePtr<c_void>,
    read_func: impl Fn(P, RemotePtr<c_void>) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let mut lst = vec![];
    let mut head: RemotePtr<c_void> = unsafe { base_ptr.byte_offset(0x8).cast().read(process)? };

    let count: u32 = unsafe { base_ptr.byte_offset(0x18).cast().read(process)? };

    for _ in 0..count {
        let next = unsafe { head.cast().read(process)? };

        let data_ptr: RemotePtr<c_void> = unsafe { head.byte_offset(0x10).cast().read(process)? };

        let x = read_func(process, data_ptr.cast())?;
        lst.push(x);

        head = next;
    }

    Ok(lst)
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

    pub class_levels: ClassLevels,
    pub equipped_effects: Vec<CGameEffect>,
    pub timed_effects: Vec<CGameEffect>,
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

            let derived_stats = CDerivedStats::new(process, entity.ptr.byte_offset(0x1120))?;
            let class = base.object.type_ai.class.clone().to_option().unwrap();
            let levels = class.get_levels(&derived_stats);

            // 0x18 before value in docs?
            let name = read_string(process, entity.ptr, 0x3910, 64)?.unwrap();
            let current_area = read_res_ref(process, entity.ptr, 0x3A20 - 0x18)?;

            let equipped_effects = {
                let offset = entity.ptr.byte_offset(0x49B0 - 0x18);
                read_ptr_list(process, offset, CGameEffect::new)
            }?;

            let timed_effects = {
                let offset = entity.ptr.byte_offset(0x4A00 - 0x18);
                read_ptr_list(process, offset, CGameEffect::new)
            }?;

            Ok(Some(Self {
                base,
                res_ref,
                base_stats: CCreatureFileHeader::new(process, entity.ptr.byte_offset(0x560))?,
                name,
                derived_stats,
                current_area,

                class_levels: levels,
                equipped_effects,
                timed_effects,
            }))
        }
    }
}
