// chatgpt wouldnt give me the whole thing so i gave up
extern crate winapi;

use std::{ptr, thread, time};
use winapi::um::memoryapi::VirtualQueryEx;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::winnt::{PAGE_READWRITE, MEMORY_BASIC_INFORMATION, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION};
use std::collections::HashMap;

#[derive(Debug)]
struct Instance {
    address: usize,
    mem: Option<RobloxInstances>,
}

#[derive(Debug)]
struct RobloxInstances {
    error: bool,
    injected: bool,
    username: String,
    pid: i64,
    exe_name: String,
    avatar: String,
    mem: Option<Memory>,
    instances: Instances,
    offsets: Offsets,
}

#[derive(Debug)]
struct Instances {
    render_view: u64,
    roblox_base: u64,
}

#[derive(Debug)]
struct Offsets {
    class_descriptor: u64,
    name: u64,
    parent: u64,
    local_player: u64,
    children: u64,
    bytecode: HashMap<String, u64>,
}

#[derive(Debug)]
struct Memory {
    pid: i32,
}

impl Memory {
    fn read_pointer(&self, address: usize) -> Option<usize> {
        // Unsafe implementation for reading a pointer
        unimplemented!()
    }

    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        // Unsafe implementation for reading raw bytes
        unimplemented!()
    }

    fn write_bytes(&self, address: usize, data: &[u8]) -> bool {
        // Unsafe implementation for writing bytes
        unimplemented!()
    }
}

impl Instance {
    fn new(address: usize, mem: Option<RobloxInstances>) -> Self {
        Instance { address, mem }
    }

    fn set_bytecode(&self, bytecode: Vec<u8>, size: u64) {
        let mem = self.mem.as_ref().unwrap(); // Assuming `mem` is initialized

        let offset = mem.offsets.bytecode.get(&self.class_name()).unwrap();
        let size_ptr = unsafe { mem.read_pointer(self.address + *offset) };

        // Write new bytecode to the given pointer
        if let Some(mut ptr) = size_ptr {
            unsafe { mem.write_bytes(ptr, &bytecode); }
        }
    }

    fn check_process_creation_time(&self, pid: i32) -> bool {
        unsafe {
            let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if process_handle.is_null() {
                return false;
            }

            let creation_time = 0u64; // Get process creation time (you might need to use GetProcessTimes)
            let current_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs();

            if current_time - creation_time > 20 {
                return false;
            }
        }

        true
    }
}

fn read_memory(pid: i32, address: usize) -> Option<Vec<u8>> {
    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let h_process = OpenProcess(PROCESS_VM_READ, 0, pid); // PROCESS_VM_READ
    if h_process.is_null() {
        return None;
    }

    if VirtualQueryEx(h_process, address as *const _, &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) == 0 {
        return None;
    }

    let mut buffer = vec![0u8; mbi.RegionSize as usize];
    let mut bytes_read = 0;
    if winapi::um::memoryapi::ReadProcessMemory(h_process, address as *const _, buffer.as_mut_ptr() as *mut _, buffer.len() as u64, &mut bytes_read) != 0 {
        Some(buffer)
    } else {
        None
    }
}

fn main() {
    let mem = Memory { pid: 12345 }; // Use actual process PID
    let instance = Instance {
        address: 0x123456, // Replace with actual address
        mem: Some(RobloxInstances {
            error: false,
            injected: false,
            username: "User".to_string(),
            pid: 12345,
            exe_name: "roblox.exe".to_string(),
            avatar: "avatar.png".to_string(),
            mem: None,
            instances: Instances {
                render_view: 0,
                roblox_base: 0,
            },
            offsets: Offsets {
                class_descriptor: 0,
                name: 0,
                parent: 0,
                local_player: 0,
                children: 0,
                bytecode: HashMap::new(),
            },
        }),
    };

    // Example usage of the set_bytecode method
    instance.set_bytecode(vec![1, 2, 3, 4], 100);

    // Check process creation time
    if instance.check_process_creation_time(12345) {
        println!("Process is running and has been up for less than 20 seconds.");
    }

    // Reading memory example
    if let Some(data) = read_memory(12345, 0x123456) {
        println!("Read memory: {:?}", data);
    }
}
