// shoutout to chatgpt
extern crate winapi;
use winapi::um::memoryapi::VirtualQueryEx;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::winnt::{MEM_PRIVATE, MEM_COMMIT, MEMORY_BASIC_INFORMATION};
use winapi::um::handleapi::CloseHandle;
use std::ptr;
use std::mem;
use std::ffi::CString;
use std::os::windows::ffi::OsStrExt;

#[derive(Debug)]
struct MemoryRegion {
    base_address: usize,
    size: usize,
    protect: u32,
}

struct Luna {
    process_handle: isize, 
}

impl Luna {
    fn query_memory_regions(&self) -> Result<Vec<MemoryRegion>, String> {
        let mut regions = Vec::new();
        let mut address: usize = 0;

        if self.process_handle == 0 {
            return Ok(regions);
        }

        loop {
            let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { mem::zeroed() };
            let result = unsafe {
                VirtualQueryEx(
                    self.process_handle as isize,
                    address as *mut _,
                    &mut mbi,
                    mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };

            if result == 0 {
                break;
            }

            if mbi.State == MEM_COMMIT && mbi.Type == MEM_PRIVATE {
                regions.push(MemoryRegion {
                    base_address: mbi.BaseAddress as usize,
                    size: mbi.RegionSize as usize,
                    protect: mbi.Protect,
                });
            }

            address = mbi.BaseAddress as usize + mbi.RegionSize as usize;
        }

        Ok(regions)
    }
}
