// shoutout to chatgpt
use std::ffi::OsString;
use std::io::Error;
use std::mem::size_of;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::psapi::EnumProcesses;
use winapi::um::tlhelp32::{CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32};
use winapi::um::winnt::{PROCESS_ALL_ACCESS};
use winapi::shared::minwindef::{DWORD, UINT, ULONG, LPVOID, LPDWORD};

pub struct Luna {
    process_handle: winapi::um::winnt::HANDLE,
}

impl Luna {
    pub fn new(pid: DWORD) -> Result<Self, String> {
        let process_handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        if process_handle.is_null() {
            return Err(format!("Failed to open process with PID: {}", pid));
        }

        Ok(Luna { process_handle })
    }

    fn utf16_ptr_to_string(u16: *const u16) -> String {
        if u16.is_null() {
            return "".to_string();
        }

        unsafe {
            let mut length = 0;
            while *u16.offset(length) != 0 {
                length += 1;
            }
            let slice = slice::from_raw_parts(u16, length as usize);
            OsString::from_wide(slice).to_string_lossy().to_string()
        }
    }

    pub fn get_base_addr(&self, pid: DWORD, target: &[&str]) -> Result<LPVOID, String> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err("CreateToolhelp32Snapshot failed".to_string());
            }

            let mut me: MODULEENTRY32 = std::mem::zeroed();
            me.dwSize = size_of::<MODULEENTRY32>() as DWORD;

            if Module32FirstW(snapshot, &mut me) == 0 {
                CloseHandle(snapshot);
                return Err("Module32FirstW failed".to_string());
            }

            loop {
                let mod_name = Self::utf16_ptr_to_string(me.szModule.as_ptr());
                for &name in target {
                    if mod_name.contains(name) {
                        let base_addr = me.modBaseAddr as LPVOID;
                        CloseHandle(snapshot);
                        return Ok(base_addr);
                    }
                }

                if Module32NextW(snapshot, &mut me) == 0 {
                    if Error::last_os_error().raw_os_error() == Some(18) {
                        break;
                    }
                    CloseHandle(snapshot);
                    return Err("Module32NextW failed".to_string());
                }
            }

            CloseHandle(snapshot);
            Err("Module not found in module list".to_string())
        }
    }
}

const INVALID_HANDLE_VALUE: winapi::um::winnt::HANDLE = -1isize as winapi::um::winnt::HANDLE;
