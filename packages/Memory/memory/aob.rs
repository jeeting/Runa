// shoutout to chatgpt
use std::collections::HashMap;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;
use windows::Win32::System::Memory::{VirtualQueryEx, MemoryBasicInformation};
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_ALL_ACCESS;
use windows::Win32::Foundation::{HANDLE, ERROR_ACCESS_DENIED};

#[derive(Debug, Clone)]
struct MemoryReg {
    base: usize,
    size: usize,
    state: u32,
    prot: u32,
    alloc: u32,
}

impl MemoryReg {
    fn new(base: usize, size: usize, state: u32, prot: u32, alloc: u32) -> Self {
        MemoryReg {
            base,
            size,
            state,
            prot,
            alloc,
        }
    }
}

pub struct Luna {
    pub process_handle: HANDLE,
}

impl Luna {
    pub fn new(process_handle: HANDLE) -> Self {
        Luna { process_handle }
    }

    pub fn plat(&self, aob: &str) -> Vec<u8> {
        let aob = aob.replace(" ", "");
        let mut true_b = Vec::new();
        let mut plat_list = Vec::new();

        for i in 0..aob.len() / 2 {
            plat_list.push(&aob[i * 2..i * 2 + 2]);
        }

        for item in plat_list {
            if item.contains("?") {
                true_b.push(0x00);
            } else {
                let byte = u8::from_str_radix(item, 16).unwrap();
                true_b.push(byte);
            }
        }

        true_b
    }

    pub fn find_pattern(data: &[u8], pattern: &[u8], base_address: usize) -> Option<usize> {
        let pattern_len = pattern.len();
        let data_len = data.len();

        for i in 0..=data_len - pattern_len {
            let mut found = true;
            for j in 0..pattern_len {
                if pattern[j] != 0x00 && pattern[j] != data[i + j] {
                    found = false;
                    break;
                }
            }
            if found {
                return Some(base_address + i);
            }
        }
        None
    }

    pub fn find_all_patterns(data: &[u8], pattern: &[u8], base_address: usize) -> Vec<usize> {
        let pattern_len = pattern.len();
        let data_len = data.len();
        let mut results = Vec::new();

        for i in 0..=data_len - pattern_len {
            let mut found = true;
            for j in 0..pattern_len {
                if pattern[j] != 0x00 && pattern[j] != data[i + j] {
                    found = false;
                    break;
                }
            }
            if found {
                results.push(base_address + i);
            }
        }
        results
    }

    pub fn aob_scan_all(
        &self,
        aob_hex_array: &str,
        xreturn_multiple: bool,
        stop_at_value: usize,
    ) -> Result<Vec<usize>, String> {
        let pattern = self.plat(aob_hex_array);
        let mut results = Vec::new();
        let mut regions = Vec::new();
        let mut mbi = MemoryBasicInformation::default();
        let mut address = 0usize;

        loop {
            let err = unsafe {
                VirtualQueryEx(self.process_handle, address as *const _, &mut mbi, std::mem::size_of::<MemoryBasicInformation>())
            };

            if err != 0 {
                break;
            }

            if mbi.State == 0x1000 && mbi.Protect == 0x04 && mbi.AllocationProtect == 0x04 {
                regions.push(MemoryReg::new(address, mbi.RegionSize as usize, mbi.State, mbi.Protect, mbi.AllocationProtect));
            }

            address += mbi.RegionSize as usize;
        }

        if regions.is_empty() {
            return Err("No readable memory regions found".to_string());
        }

        let results_ch = Arc::new(Mutex::new(Vec::new()));
        let err_ch = Arc::new(Mutex::new(Vec::new()));
        let mut threads = Vec::new();

        for region in regions {
            let results_ch = Arc::clone(&results_ch);
            let err_ch = Arc::clone(&err_ch);

            let handle = thread::spawn(move || {
                // Read the memory here (mocking a ReadMemory function)
                let data = vec![0u8; region.size]; // Replace with actual memory reading
                if let Some(local_results) = if xreturn_multiple {
                    Some(Luna::find_all_patterns(&data, &pattern, region.base))
                } else {
                    Luna::find_pattern(&data, &pattern, region.base).map(|res| vec![res])
                } {
                    let mut results_ch = results_ch.lock().unwrap();
                    results_ch.extend(local_results);
                } else {
                    let mut err_ch = err_ch.lock().unwrap();
                    err_ch.push(format!("Failed to read region at {:#x}", region.base));
                }
            });

            threads.push(handle);
        }

        for handle in threads {
            handle.join().unwrap();
        }

        let results_ch = results_ch.lock().unwrap();
        results.extend_from_slice(&results_ch);

        if !xreturn_multiple {
            if let Some(first) = results_ch.get(0) {
                results.push(*first);
            }
        }

        if stop_at_value > 0 && results.len() >= stop_at_value {
            results.truncate(stop_at_value);
        }

        results.sort();

        Ok(results)
    }
}
