#![allow(clippy::missing_safety_doc)]
#![allow(clippy::missing_transmute_annotations)]

#[macro_use]
extern crate static_assertions;

pub mod error;
pub mod padding;
pub mod process;
pub mod remote_ptr;
pub mod types;
pub mod ids;

#[cfg(test)]
mod tests;

use crate::{
    error::Error,
    process::{get_process_procs, GameProcess, ProcessMemory},
    remote_ptr::RemotePtr,
};
use std::{ffi::c_void, mem::MaybeUninit};

pub mod entity_list {
    pub const OFFSET: usize = 0x27780;
    pub const ELEMENT_COUNT: usize = i16::MAX as usize;
    pub const LENGTH: usize = ELEMENT_COUNT * 16;
}

#[repr(C)]
#[derive(Debug)]
pub struct EntityPtr {
    pub id: u16,
    pub ptr: RemotePtr<c_void>,
}
impl EntityPtr {
    pub fn is_valid(&self) -> bool {
        self.id != u16::MAX
    }
}

#[allow(clippy::missing_transmute_annotations)]
pub fn get_static_entity_list(
    process: &GameProcess,
) -> Result<[EntityPtr; entity_list::ELEMENT_COUNT], Error> {
    if !process.exists() {
        return Err(Error::GameProcessClosed);
    }

    let mut lst: [MaybeUninit<EntityPtr>; entity_list::ELEMENT_COUNT] =
        unsafe { MaybeUninit::uninit().assume_init() };

    unsafe {
        process.read_mem_into_unsafe(
            &mut lst,
            process.base_address.get() + entity_list::OFFSET,
            entity_list::LENGTH,
        )?;
        Ok(std::mem::transmute(lst))
    }
}

pub fn find_game_process(first_open: bool) -> Result<GameProcess, Error> {
    let mut procs = get_process_procs()?.map(GameProcess::new);
    procs
        .find_map(|b| match b {
            Ok(Some(b)) => Some(b),
            _ => None,
        })
        .ok_or(if first_open {
            Error::MissingGameProcess
        } else {
            Error::GameProcessClosed
        })
}
