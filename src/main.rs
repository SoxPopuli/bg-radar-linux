#![allow(clippy::missing_safety_doc)]
#![allow(clippy::missing_transmute_annotations)]

#[macro_use]
extern crate static_assertions;

mod error;
mod padding;
mod process;
mod remote_ptr;
mod types;

#[cfg(test)]
mod tests;

use crate::{
    error::Error,
    padding::Padding,
    process::{get_process_procs, GameProcess, ProcessMemory}, remote_ptr::RemotePtr,
};
use std::{ffi::c_void, mem::MaybeUninit};

use rust_utils::byte_readers::ReadBytes;

mod entity_list {
    pub const OFFSET: usize = 0x27780;
    pub const ELEMENT_COUNT: usize = i16::MAX as usize;
    pub const LENGTH: usize = ELEMENT_COUNT * 16;
}

#[repr(C)]
#[derive(Debug)]
struct EntityPtr {
    id: u16,
    ptr: RemotePtr<c_void>,
}

#[allow(clippy::missing_transmute_annotations)]
fn get_static_entity_list(
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

fn find_game_process(first_open: bool) -> Result<GameProcess, Error> {
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

fn main() -> Result<(), Error> {
    let game_process = find_game_process(true)?;
    let entities = get_static_entity_list(&game_process)?;

    entities
        .into_iter()
        .filter(|x| x.id != u16::MAX)
        .map(|x| types::CGameAIBase::new(&game_process, &x))
        .for_each(|x| println!("{x:#?}"));

    Ok(())
}
