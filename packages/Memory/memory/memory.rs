// chatgpt wouldnt give the whole code so i gave up
use std::ffi::CString;
use std::ptr;
use std::mem;
use winapi::um::processthreadsapi::{OpenProcess, GetExitCodeProcess};
use winapi::um::memoryapi::{VirtualQueryEx, VirtualAllocEx, WriteProcessMemory, NtReadVirtualMemory};
use winapi::um::psapi::{QueryWorkingSetEx};
use winapi::um::winuser::SendMessageA;
use winapi::um::handleapi::CloseHandle;
use winapi::um::winnt::{PROCESS_ALL_ACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE, MEM_RELEASE, MEM_DECOMMIT, MEMORY_BASIC_INFORMATION};
use winapi::shared::minwindef::{DWORD, LPVOID, BOOL};
use winapi::shared::ntdef::LPWSTR;

#[derive(Debug)]
struct Luna {
    process_handle: isize,
    is_64bit: bool,
    roblox_base: usize,
    alloc_addr: usize,
    pid: u32,
}

impl Luna {
    fn new(pid: u32) -> Result<Luna, String> {
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        if handle == 0 {
            return Err("Failed to open process".to_string());
        }

        let base = get_base_addr(pid)?;
        
        Ok(Luna {
            process_handle: handle,
            is_64bit: true, 
            roblox_base: base,
            alloc_addr: 0,
            pid,
        })
    }

    fn mem_read(&self, address: usize, buffer: *mut u8, size: usize) -> Result<(), String> {
        let mut mbi = MEMORY_BASIC_INFORMATION { ..unsafe { mem::zeroed() } };
        let mbi_size = mem::size_of::<MEMORY_BASIC_INFORMATION>();
        
        unsafe {
            VirtualQueryEx(self.process_handle as isize, address as LPVOID, &mut mbi, mbi_size);
            let status = NtReadVirtualMemory(self.process_handle as isize, address as LPVOID, buffer, size as DWORD, 0);
            if status == 0 {
                return Err("Failed to read memory".to_string());
            }
        }
        Ok(())
    }

    fn mem_write(&self, address: usize, buffer: *const u8, size: usize) -> Result<(), String> {
        let mut bytes_written = 0;
        unsafe {
            let status = WriteProcessMemory(self.process_handle as isize, address as LPVOID, buffer as LPVOID, size as DWORD, &mut bytes_written);
            if status == 0 || bytes_written != size {
                return Err("Failed to write memory".to_string());
            }
        }
        Ok(())
    }

    fn get_base_addr(pid: u32) -> Result<usize, String> {
        Ok(0)
    }

    fn send_message(hwnd: isize, msg: u32, wparam: usize, lparam: usize) -> Result<(), String> {
        let result = unsafe { SendMessageA(hwnd as isize, msg, wparam as DWORD, lparam as DWORD) };
        if result == 0 {
            return Err("Failed to send message".to_string());
        }
        Ok(())
    }

    fn is_handle_valid(&self) -> bool {
        let mut exit_code: DWORD = 0;
        unsafe {
            let status = GetExitCodeProcess(self.process_handle as isize, &mut exit_code);
            if status == 0 || exit_code != 259 {
                return false;
            }
        }
        true
    }
}

fn get_processes() -> Result<Vec<Processes>, String> {
    Ok(vec![
        Processes {
            name: "RobloxPlayerBeta.exe".to_string(),
            pid: 1234,
        }
    ])
}

#[derive(Debug)]
struct Processes {
    name: String,
    pid: u32,
}

fn remove_euro(data: Vec<Processes>) -> Vec<Processes> {
    let mut removed_robloxs = Vec::new();
    for mut proc in data {
        if proc.name != "RobloxPlayerBeta.exe" && proc.name != "Windows10Universal.exe" {
            proc.name = "RobloxPlayerBeta.exe".to_string();
        }
        removed_robloxs.push(proc);
    }
    removed_robloxs
}

fn is_process_running() -> Result<(bool, Vec<Processes>), String> {
    let processes = get_processes()?;
    let filtered_processes = remove_euro(processes);
    Ok((filtered_processes.len() > 0, filtered_processes))
}
