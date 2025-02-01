use std::ffi::{CStr, CString};
use std::io::Write;
use std::ptr;
use std::sync::Mutex;
use std::fs::File;
use std::io::Read;
use std::error::Error;
use std::convert::TryInto;

use libc::{c_char, c_void};
use xxhash_rust::xxh3::XxHash3;
use zstd::stream::copy_encoder;

lazy_static::lazy_static! {
    static ref LIMITER: Mutex<()> = Mutex::new(());
    static ref DLL: Option<libloading::Library> = unsafe { libloading::Library::new("Luna.dll").ok() };
}

fn mb(i: usize) -> usize {
    i * 1024 * 1024
}

#[derive(Debug)]
pub struct Bytecode;

impl Bytecode {
    pub fn compile(&self, source: &str) -> Result<Vec<u8>, i64> {
        if let Some(dll) = &*DLL {
            let get_bytecode: libloading::Symbol<unsafe extern fn(*const u8, *mut u8, *mut usize)> =
                unsafe { dll.get(b"getbytecode").unwrap() };

            let mut buffer = vec![0u8; mb(10)];
            let mut actual_size: usize = 0;
            unsafe {
                get_bytecode(
                    source.as_ptr(),
                    buffer.as_mut_ptr(),
                    &mut actual_size,
                );
            }

            // Trim any trailing zeros
            let trimmed_size = buffer.iter().rposition(|&x| x != 0).unwrap_or(0) + 1;

            return Ok(buffer[..trimmed_size].to_vec());
        }

        Err(-10)
    }

    pub fn decompress(&self, source: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(dll) = &*DLL {
            let decompress: libloading::Symbol<unsafe extern fn(*const u8, *mut u8)> =
                unsafe { dll.get(b"decompress").unwrap() };

            let mut buffer = Vec::with_capacity(1024);
            unsafe {
                decompress(
                    source.as_ptr(),
                    buffer.as_mut_ptr(),
                );
            }

            return Ok(buffer);
        }

        Err("Luna.DLL Error".to_string())
    }
}

pub fn compile_test() -> Vec<u8> {
    let code = r#"
        return function()
            local x = 10
            return x * 2
        end
    "#;

    let mut bytecode = Vec::new();

    // Serialize Lua bytecode here
    // This would require a Lua interpreter in Rust. Assuming you're using a library like rlua.
    // For simplicity, let's assume we have a method to generate Lua bytecode.

    bytecode.push(0); // This is just a placeholder
    bytecode
}

pub fn compress(bytecode: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let data_size = bytecode.len();

    let mut buffer = Vec::with_capacity(8);
    buffer.write_all(b"RSB1")?;

    buffer.write_all(&(data_size as u32).to_le_bytes())?;

    let mut encoder = zstd::Encoder::new(Vec::new(), 3)?;
    encoder.write_all(bytecode)?;
    let compressed_data = encoder.finish()?;

    buffer.extend(compressed_data);

    let size = buffer.len();

    let key = XxHash3::new().update(&buffer[..size]).finalize();
    let key_bytes = (key as u32).to_le_bytes();

    for (i, byte) in buffer.iter_mut().enumerate() {
        *byte ^= key_bytes[i % 4] + (i as u8 * 41);
    }

    Ok(buffer)
}

