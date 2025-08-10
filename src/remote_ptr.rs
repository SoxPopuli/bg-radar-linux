use crate::{error::Error, process::GameProcess};
use std::mem::MaybeUninit;

#[repr(transparent)]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct RemotePtr<T>(*const T);
impl<T> RemotePtr<T> {
    pub fn byte_offset(&self, offset: isize) -> RemotePtr<T> {
        unsafe { RemotePtr(self.0.byte_offset(offset)) }
    }

    pub fn cast<U>(&self) -> RemotePtr<U> {
        RemotePtr(self.0.cast())
    }

    pub unsafe fn read(&self, process: &GameProcess) -> Result<T, Error> {
        let mut output = MaybeUninit::uninit();
        unsafe {
            process.read_mem_into_unsafe(output.as_mut_ptr(), self.0.addr(), size_of::<T>())?;
            Ok(output.assume_init())
        }
    }

    pub unsafe fn read_bytes(
        &self,
        process: &GameProcess,
        length: usize,
    ) -> Result<Vec<u8>, Error> {
        process.read_mem(self.0.addr(), length)
    }

    pub unsafe fn read_array(&self, process: &GameProcess, length: usize) -> Result<Vec<T>, Error> {
        let mut buffer = Vec::new();
        buffer.resize_with(length, MaybeUninit::<T>::uninit);

        unsafe {
            process.read_mem_into_unsafe(
                buffer.as_mut_ptr(),
                self.0.addr(),
                size_of::<T>() * length,
            )?;
            Ok(std::mem::transmute(buffer))
        }
    }
}
