// shoutout to chatgpt
use std::{env, fs::{self, File}, io::{self, Read}, collections::HashMap, path::{Path, PathBuf}, process, str};
use std::cmp::Ordering;
use std::ptr;
use std::mem;
use std::thread;
use std::sync::Arc;

mod memory {
    pub struct Luna {
        pub roblox_base: usize,
    }

    impl Luna {
        pub fn mem_read(&self, address: usize, buffer: &mut u64, size: usize) -> io::Result<()> {
            unsafe {
                ptr::write_bytes(buffer, 0, size);
            }
            Ok(())
        }

        pub fn read_pointer(&self, address: usize) -> Option<usize> {
            Some(address + 0x8)
        }

        pub fn read_rbx_str(&self, address: usize) -> Option<String> {
            Some("Property Name".to_string())
        }

        pub fn read_string(&self, address: usize, length: usize) -> Option<String> {
            Some("RenderJob".to_string()) 
        }
    }
}

use memory::Luna;

#[derive(Debug, Clone, Copy)]
pub struct Offsets {
    pub render_view_from_render_job: u64,
    pub data_model_holder: u64,
    pub data_model: u64,
    pub visual_data_model: u64,
    pub name: u64,
    pub children: u64,
    pub parent: u64,
    pub class_descriptor: u64,
    pub local_player: u64,
    pub value_base: u64,
    pub module_flags: u64,
    pub is_core: u64,
    pub place_id: u64,
    pub bytecode_size: u64,
    pub bytecode: HashMap<String, u64>,
    pub offset_task_scheduler: u64,
    pub offset_jobs_container: u64,
}

pub static mut OFFSETS_DATA_PLAYER: Offsets = Offsets {
    render_view_from_render_job: 0x1E8,
    data_model_holder: 0x118,
    data_model: 0x1A8,
    visual_data_model: 0x720,
    name: 0x68,
    children: 0x70,
    parent: 0x50,
    class_descriptor: 0x18,
    local_player: 0x118,
    value_base: 0xC8,
    module_flags: 0x1B0 - 0x4,
    is_core: 0x1B0,
    place_id: 0x170,
    bytecode_size: 0xA8,
    bytecode: HashMap::from([
        ("LocalScript".to_string(), 0x1C0),
        ("ModuleScript".to_string(), 0x168),
    ]),
    offset_task_scheduler: 0x5C71FC8,
    offset_jobs_container: 0x1C8,
};

pub static mut OFFSETS_DATA_UWP: Offsets = Offsets {
    data_model_holder: 0x118,
    data_model: 0x1A8,
    visual_data_model: 0x720,
    name: 0x68,
    children: 0x70,
    parent: 0x50,
    class_descriptor: 0x18,
    local_player: 0x118,
    value_base: 0xC8,
    module_flags: 0x1B0 - 0x4,
    is_core: 0x1B0,
    place_id: 0x170,
    bytecode_size: 0xA8,
    bytecode: HashMap::from([
        ("LocalScript".to_string(), 0x1C0),
        ("ModuleScript".to_string(), 0x168),
    ]),
    offset_task_scheduler: 0x5C71FC8,
    offset_jobs_container: 0x1C8,
};

static mut UWP_LOGS: Option<String> = None;

pub fn init() {
    thread::spawn(move || {
        let appdata = env::var("LOCALAPPDATA").unwrap_or_default();
        let uwp_logs = {
            let mut data: HashMap<String, String> = HashMap::new();
            if let Ok(entries) = fs::read_dir(Path::new(&appdata).join("Packages")) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.to_str().unwrap_or_default().contains("ROBLOXCORPORATION.ROBLOX") {
                            let package_name = path.parent().unwrap().file_name().unwrap().to_str().unwrap();
                            let modified_time = entry.metadata().unwrap().modified().unwrap();
                            data.insert(package_name.to_string(), modified_time.to_string());
                        }
                    }
                }
            }
            data.iter().map(|(name, _)| format!("{}/LocalState/logs", name)).next().unwrap_or_default()
        };
        unsafe {
            UWP_LOGS = Some(uwp_logs);
        }
    });
}

pub fn retrieve_rv(mem: &Luna, offsets: Offsets, logs: String) -> (u64, u64) {
    let logs_dir = Path::new(&logs);
    if !logs_dir.exists() || !logs_dir.is_dir() {
        println!("Logs directory doesn't exist");
        return (0, 0);
    }

    let mut log_files: Vec<PathBuf> = Vec::new();
    if let Ok(files) = fs::read_dir(logs_dir) {
        for file in files.filter_map(Result::ok) {
            if file.path().extension().map_or(false, |ext| ext == "log") {
                log_files.push(file.path());
            }
        }
    }

    if log_files.is_empty() {
        println!("No log files found");
        return (0, 0);
    }

    log_files.sort_by(|a, b| {
        let a_metadata = a.metadata().unwrap();
        let b_metadata = b.metadata().unwrap();
        b_metadata.modified().unwrap().cmp(&a_metadata.modified().unwrap())
    });

    let mut locked_files: Vec<PathBuf> = Vec::new();
    for log_path in log_files {
        if fs::remove_file(&log_path).is_err() {
            locked_files.push(log_path);
        }
    }

    if locked_files.is_empty() {
        println!("No locked files found");
        return (0, 0);
    }

    for log_path in locked_files {
        if let Ok(data) = fs::read(log_path) {
            for line in data.split(|&c| c == b'\n').filter_map(|line| str::from_utf8(line).ok()) {
                if let Some(pos) = line.find("view(") {
                    if let Ok(render_view) = u64::from_str_radix(&line[pos + 5..pos + 21], 16) {
                        let mut fake_data_model = 0u64;
                        let mut real_data_model = 0u64;

                        mem.mem_read((render_view + offsets.data_model_holder) as usize, &mut fake_data_model, mem::size_of::<u64>())?;
                        mem.mem_read((fake_data_model + offsets.data_model) as usize, &mut real_data_model, mem::size_of::<u64>())?;

                        if real_data_model != 0 {
                            if let Some(name_ptr) = mem.read_pointer(real_data_model as usize) {
                                if let Some(name) = mem.read_rbx_str(name_ptr as usize) {
                                    if ["Ugc", "LuaApp", "App", "Game"].contains(&name.as_str()) {
                                        return (render_view, real_data_model);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (0, 0)
}

pub fn get_rv(mem: &Luna, offsets: Offsets, uwp: bool) -> (u64, u64) {
    unsafe {
        if let Some(uwp_logs) = &UWP_LOGS {
            if uwp {
                return retrieve_rv(mem, offsets, uwp_logs.clone());
            }
        }
    }
    retrieve_rv(mem, offsets, format!("{}/Roblox/logs", env::var("LOCALAPPDATA").unwrap()))
}

pub fn get_render_vdm(pid: u32, mem: &Luna, offsets: Offsets, uwp: bool) -> u64 {
    if mem == &Luna { return 0 }

    if let Some(task_ptr) = mem.read_pointer(mem.roblox_base + offsets.offset_task_scheduler as usize) {
        if let Some(task_list_ptr) = mem.read_pointer(task_ptr + offsets.offset_jobs_container as usize) {
            for i in (0..0x500).step_by(0x10) {
                if let Some(task_container_ptr) = mem.read_pointer(task_list_ptr + i) {
                    if let Some(name) = mem.read_string(task_container_ptr + 0x90, 10) {
                        if name.eq_ignore_ascii_case("RenderJob") {
                            return task_container_ptr;
                        }
                    }
                }
            }
        }
    }
    0
}

fn main() {
    let luna = Luna { roblox_base: 0 };
    let offsets = OFFSETS_DATA_PLAYER;
    init();
    let (render_view, data_model) = get_rv(&luna, offsets, true);
    println!("RenderView: 0x{:X}, DataModel: 0x{:X}", render_view, data_model);
}
