use std::{ffi::c_void, fs::File, io::Read, num::NonZero, path::Path};

use crate::{
    entity_list, error::Error, ids::classes::Class, process::{GameProcess, ProcessMemory}, remote_ptr::RemotePtr, types::{CGameAIBase, CGameObject, CGameSprite, ObjectType}, EntityPtr
};

#[derive(Debug)]
struct MemoryRegion {
    start: usize,
    end: usize,
    mem: Vec<u8>,
}
impl MemoryRegion {
    pub fn load_all(base_path: &Path) -> Vec<MemoryRegion> {
        let archive_file = base_path.join("memory_dump.tar.zst");

        let mut archive = {
            let file = File::open(archive_file).expect("Failed to open mem dump file");
            let d = zstd::Decoder::new(file).expect("zstd decoder error");

            tar::Archive::new(d)
        };

        archive
            .entries()
            .unwrap()
            .map(|file| {
                let mut file = file.unwrap();

                let data = {
                    let mut buf = vec![0; file.size() as usize];
                    file.read_exact(&mut buf).unwrap();
                    buf
                };

                let path = file.path().unwrap();

                let filename = path.file_stem().unwrap().to_str().unwrap();
                let (start, end) = filename.split_once('-').unwrap();

                MemoryRegion {
                    start: usize::from_str_radix(start, 16).expect("Invalid region start"),
                    end: usize::from_str_radix(end, 16).expect("Invalid region end"),
                    mem: data,
                }
            })
            .collect()
    }

    pub fn from_absolute_address(
        regions: &[MemoryRegion],
        address: usize,
        length: usize,
    ) -> Option<&[u8]> {
        let region = regions
            .iter()
            .find(|r| address >= r.start && address <= r.end)?;

        let offset = address - region.start;

        Some(&region.mem[offset..offset + length])
    }
}

#[derive(Debug)]
struct MockProcess {
    process: GameProcess,
    memory_regions: Vec<MemoryRegion>,
    maps: &'static str,
}
impl ProcessMemory for &MockProcess {
    fn read_mem(&self, address: usize, length: usize) -> Result<Vec<u8>, Error> {
        let slice = MemoryRegion::from_absolute_address(&self.memory_regions, address, length)
            .ok_or(Error::Memory(format!("Invalid address: 0x{address:x}")))?;

        let mut buffer = vec![0; length];
        buffer.copy_from_slice(slice);

        Ok(buffer)
    }

    fn read_mem_into(
        &self,
        buffer: &mut [u8],
        address: usize,
        length: usize,
    ) -> Result<isize, Error> {
        let memory = self.read_mem(address, length)?;

        buffer.copy_from_slice(&memory);

        Ok(memory.len() as isize)
    }

    unsafe fn read_mem_into_unsafe<T>(
        &self,
        buffer: *mut T,
        address: usize,
        length: usize,
    ) -> Result<isize, Error> {
        let memory = self.read_mem(address, length)?;

        let count = length / size_of::<T>();
        unsafe { buffer.copy_from_nonoverlapping(memory.as_ptr().cast(), count) };

        Ok(memory.len() as isize)
    }
}

const BASE_DIR: &str = env!("CARGO_MANIFEST_DIR");
const MEMORY_MAP: &str = include_str!("../../dumps/bgee-memmap");

fn get_mock_process() -> MockProcess {
    let base_path = Path::new(BASE_DIR);

    let dump_file_path = base_path.join("dumps");

    MockProcess {
        process: GameProcess {
            path: dump_file_path.clone(),
            pid: NonZero::new(1).unwrap(),
            base_address: crate::process::get_base_address_from_memory_map(MEMORY_MAP)
                .expect("Failed to read memory map"),
            name: "Mock Process".to_string(),
        },
        memory_regions: MemoryRegion::load_all(&dump_file_path),
        maps: MEMORY_MAP,
    }
}

#[test]
fn read_mem_test() -> Result<(), Error> {
    let process = get_mock_process();

    let data = (&process).read_mem(
        process.process.base_address.get() + entity_list::OFFSET,
        entity_list::LENGTH,
    )?;
    let mut entities = vec![];

    for i in (0..data.len()).step_by(16) {
        let id = u16::from_ne_bytes(data[i..=i + 1].try_into().unwrap());

        let ptr = usize::from_ne_bytes(data[i + 8..=i + 15].try_into().unwrap());

        entities.push(EntityPtr {
            id,
            ptr: RemotePtr::new(ptr as *const c_void),
        });
    }

    let ai = entities
        .into_iter()
        .filter(|e| e.is_valid())
        .map(|e| {
            let base = CGameAIBase::new(&process, &e);
            base.map(|base| (e, base))
        })
        .filter_map(|base| match base {
            Ok((e, Some(base))) if base.object.object_type == ObjectType::Sprite => {
                Some(CGameSprite::new(&process, &e, base))
            }
            _ => None,
        })
        .collect::<Result<Vec<_>, Error>>()?;

    panic!("{ai:#?}");
}
