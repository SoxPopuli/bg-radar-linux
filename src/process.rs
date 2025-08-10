use std::{
    fs::read_to_string,
    num::{NonZero, NonZeroU32, NonZeroUsize},
    path::PathBuf,
};

use crate::error::Error;

pub trait ProcessMemory {
    fn read_mem(&self, address: usize, length: usize) -> Result<Vec<u8>, Error>;
    fn read_mem_into(
        &self,
        buffer: &mut [u8],
        address: usize,
        length: usize,
    ) -> Result<isize, Error>;
    unsafe fn read_mem_into_unsafe<T>(
        &self,
        buffer: *mut T,
        address: usize,
        length: usize,
    ) -> Result<isize, Error>;
}

#[derive(Debug)]
pub struct GameProcess {
    pub path: PathBuf,
    pub pid: NonZeroU32,
    pub base_address: NonZeroUsize,
    pub name: String,
}
impl GameProcess {
    pub fn exists(&self) -> bool {
        self.path.try_exists().unwrap_or(false)
    }

    pub fn new((path, pid): (PathBuf, NonZeroU32)) -> Result<Option<Self>, Error> {
        let name = read_to_string(path.join("comm"))?.trim_end().to_string();

        if !["BaldursGate", "BaldursGateII"].contains(&name.as_str()) {
            return Ok(None);
        }

        let maps = read_to_string(path.join("maps"))?;

        let base_address = maps
            .trim_end()
            .lines()
            .nth(3)
            .and_then(|x| Some(x.split_once('-')?.0))
            .and_then(|x| usize::from_str_radix(x, 16).ok())
            .and_then(NonZero::new)
            .ok_or(Error::Memory("Could not get base address".into()))?;

        Ok(Some(Self {
            path,
            name,
            pid,
            base_address,
        }))
    }
}

impl ProcessMemory for &GameProcess {
    unsafe fn read_mem_into_unsafe<T>(
        &self,
        buffer: *mut T,
        address: usize,
        length: usize,
    ) -> Result<isize, Error> {
        let pid = self.pid.get() as i32;

        unsafe {
            let src_iovec = libc::iovec {
                iov_len: length,
                iov_base: address as *mut libc::c_void,
            };

            let dst_iovec = libc::iovec {
                iov_len: length,
                iov_base: buffer as *mut libc::c_void,
            };

            let read = libc::process_vm_readv(pid, &dst_iovec, 1, &src_iovec, 1, 0);

            if read != (length as isize) {
                let err = std::io::Error::last_os_error();
                Err(err.into())
            } else {
                Ok(read)
            }
        }
    }

    fn read_mem_into(
        &self,
        buffer: &mut [u8],
        address: usize,
        length: usize,
    ) -> Result<isize, Error> {
        if buffer.len() < length {
            return Err(Error::InsufficentMemory {
                msg: "Insufficent memory for read_mem".into(),
                expected: length,
                actual: buffer.len(),
            });
        }

        unsafe { self.read_mem_into_unsafe(buffer.as_mut_ptr(), address, length) }
    }

    fn read_mem(&self, address: usize, length: usize) -> Result<Vec<u8>, Error> {
        let mut dst_buf = vec![0; length];

        self.read_mem_into(dst_buf.as_mut_slice(), address, length)
            .map(|_| dst_buf)
    }
}

pub fn get_process_procs()
-> Result<impl Iterator<Item = (std::path::PathBuf, NonZeroU32)>, std::io::Error> {
    let proc = std::fs::read_dir("/proc")?;

    Ok(proc.into_iter().filter_map(|entry| {
        if let Ok(entry) = entry
            && let Ok(metadata) = entry.metadata()
        {
            fn to_number(name: &std::ffi::OsString) -> Option<NonZeroU32> {
                name.to_str().and_then(|name| name.parse().ok())
            }

            if metadata.is_dir()
                && let Some(pid) = to_number(&entry.file_name())
            {
                Some((entry.path(), pid))
            } else {
                None
            }
        } else {
            None
        }
    }))
}
