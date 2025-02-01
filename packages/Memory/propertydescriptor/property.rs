// shoutout to chatgpt
extern crate winapi;
use std::collections::HashMap;
use std::ptr;
use std::mem;
use winapi::um::memoryapi::VirtualQueryEx;
use winapi::um::winnt::{MEM_COMMIT, MEM_PRIVATE, MEMORY_BASIC_INFORMATION};
use std::ffi::CString;
use std::os::windows::ffi::OsStrExt;

mod memory {
    use std::ptr;

    pub struct Luna {
        pub process_handle: isize, 
    }

    impl Luna {
        pub fn read_pointer(&self, address: usize) -> Option<usize> {
            Some(address + 0x8) // Simulated offset
        }

        pub fn read_int32(&self, address: usize) -> Option<i32> {
            Some(42)
        }

        pub fn write_int32(&self, address: usize, value: i32) {
            println!("Writing value {} to address {}", value, address);
        }

        pub fn mem_read(&self, address: usize, buffer: *mut u64, size: usize) {
            unsafe {
                ptr::write_bytes(buffer, 0, size);
            }
        }

        pub fn read_rbx_str(&self, address: usize) -> Option<String> {
            Some("Property Name".to_string())
        }
    }
}

use memory::Luna;

static mut OLD_ACCESSIBLE_FLAGS: HashMap<usize, i32> = HashMap::new();

#[derive(Debug)]
struct PropertyDescriptor {
    address: usize,
}

impl PropertyDescriptor {
    fn new(address: usize) -> Self {
        PropertyDescriptor { address }
    }

    fn name(&self, mem: &Luna) -> String {
        let name_pointer = mem.read_pointer(self.address + 0x8).unwrap();
        mem.read_rbx_str(name_pointer).unwrap()
    }

    fn capabilities(&self, mem: &Luna) -> i32 {
        mem.read_int32(self.address + 0x38).unwrap()
    }

    fn accessible_flags(&self, mem: &Luna) -> i32 {
        mem.read_int32(self.address + 0x40).unwrap()
    }

    fn is_hidden_value(&self, mem: &Luna) -> bool {
        let val = self.accessible_flags(mem);
        val < 32
    }

    fn set_scriptable(&self, mem: &Luna, scriptable: bool) {
        if scriptable {
            unsafe {
                if !OLD_ACCESSIBLE_FLAGS.contains_key(&self.address) {
                    OLD_ACCESSIBLE_FLAGS.insert(self.address, self.accessible_flags(mem));
                    mem.write_int32(self.address + 0x40, 63);
                }
            }
        } else {
            unsafe {
                if let Some(old_flag) = OLD_ACCESSIBLE_FLAGS.get(&self.address) {
                    mem.write_int32(self.address + 0x40, *old_flag);
                }
            }
        }
    }
}

#[derive(Debug)]
struct PropertyDescriptorContainer {
    address: usize,
}

impl PropertyDescriptorContainer {
    fn new(address: usize) -> Self {
        PropertyDescriptorContainer { address }
    }

    fn get_all_yield(&self, mem: &Luna) -> Vec<PropertyDescriptor> {
        let mut descriptors = Vec::new();
        let mut start: u64 = 0;
        let mut end: u64 = 0;

        mem.mem_read(self.address + 0x28, &mut start as *mut u64, mem::size_of::<u64>());
        mem.mem_read(self.address + 0x30, &mut end as *mut u64, mem::size_of::<u64>());

        for addr in start..end {
            let descriptor_addr = mem.read_pointer(addr as usize).unwrap();
            if descriptor_addr > 1000 {
                let new_descriptor = PropertyDescriptor::new(descriptor_addr);
                descriptors.push(new_descriptor);
            }
        }

        descriptors
    }

    fn get(&self, mem: &Luna, name: &str) -> PropertyDescriptor {
        for descriptor in self.get_all_yield(mem) {
            if descriptor.name(mem) == name {
                return descriptor;
            }
        }
        PropertyDescriptor::new(0)
    }
}

fn main() {
    let luna = Luna { process_handle: 0 };

    let pdc = PropertyDescriptorContainer::new(12345);
    let all_descriptors = pdc.get_all_yield(&luna);

    for descriptor in all_descriptors {
        println!("Found PropertyDescriptor: {:?}", descriptor);
    }

    let specific_descriptor = pdc.get(&luna, "Property Name");
    println!("Specific PropertyDescriptor: {:?}", specific_descriptor);
}
