use sha2::{Digest, Sha256};
use std::cell::Cell;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW};

use crate::error::{Mt5Error, Result};

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

/// Transport abstraction over the MT5 IPC pipe.
///
/// `NamedPipeClient` is the real Windows implementation; tests can inject an
/// in-memory mock to exercise the full request/response flow without a
/// running terminal (mirrors go-mt5's mock pipe).
pub trait Transport: Send {
    /// Send a command with its payload and return the response data
    /// (after the 8-byte cmd_echo + success header).
    fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>>;
}

pub struct NamedPipeClient {
    handle: HANDLE,
    // Set when the pipe handle is no longer usable (e.g. the terminal closed
    // it after an unsupported command). Subsequent calls fail fast with a
    // clear error instead of reusing a dead handle; the caller must create a
    // new client (re-initialize). Interior mutability so existing &self API
    // keeps working.
    broken: Cell<bool>,
}

impl NamedPipeClient {
    pub fn new(pipe_name: Option<&str>) -> Result<Self> {
        let name = match pipe_name {
            Some(n) => n.to_string(),
            None => {
                return Err(Mt5Error::ConnectionFailed(
                    "Pipe name must be provided. Use initialize(Some(\"pipe_name\"))".into(),
                ))
            }
        };

        let pipe_name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            WaitNamedPipeW(pipe_name_wide.as_ptr(), 500);
        }

        let handle = unsafe {
            CreateFileW(
                pipe_name_wide.as_ptr(),
                0x80000000 | 0x40000000,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(Mt5Error::ConnectionFailed(format!(
                "Failed to connect to pipe: {}",
                name
            )));
        }

        Ok(Self {
            handle,
            broken: Cell::new(false),
        })
    }

    fn mark_broken(&self, err: Mt5Error) -> Mt5Error {
        if !self.broken.get() {
            self.broken.set(true);
            unsafe {
                CloseHandle(self.handle);
            }
        }
        err
    }

    pub fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
        if self.broken.get() {
            return Err(Mt5Error::ConnectionFailed(
                "Pipe connection was closed by the terminal (broken pipe); re-initialize the client".into(),
            ));
        }

        let total_len = 4 + data.len();
        let mut request = Vec::with_capacity(8 + data.len());
        request.extend_from_slice(&(total_len as u32).to_le_bytes());
        request.extend_from_slice(&cmd.to_le_bytes());
        request.extend_from_slice(data);

        unsafe {
            let mut bytes_written = 0u32;
            let result = WriteFile(
                self.handle,
                request.as_ptr(),
                request.len() as u32,
                &mut bytes_written,
                std::ptr::null_mut(),
            );

            if result == 0 {
                return Err(self.mark_broken(Mt5Error::IoError(std::io::Error::last_os_error())));
            }
        }

        self.read_response()
    }

    fn read_response(&self) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        let mut bytes_read = 0u32;

        unsafe {
            let result = ReadFile(
                self.handle,
                len_buf.as_mut_ptr(),
                len_buf.len() as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            );

            if result == 0 {
                return Err(self.mark_broken(Mt5Error::IoError(std::io::Error::last_os_error())));
            }
        }

        let payload_len = u32::from_le_bytes(len_buf) as usize;

        if payload_len < 8 {
            return Err(Mt5Error::InvalidResponse(format!(
                "Payload too small: {} bytes",
                payload_len
            )));
        }

        let mut payload = vec![0u8; payload_len];
        let mut total_read = 0usize;

        while total_read < payload_len {
            let mut bytes_read = 0u32;
            unsafe {
                let result = ReadFile(
                    self.handle,
                    payload[total_read..].as_mut_ptr(),
                    (payload_len - total_read) as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                );

                if result == 0 {
                    return Err(
                        self.mark_broken(Mt5Error::IoError(std::io::Error::last_os_error()))
                    );
                }

                total_read += bytes_read as usize;
            }
        }

        let _cmd_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let _success = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) != 0;

        if payload.len() > 8 {
            Ok(payload[8..].to_vec())
        } else {
            Ok(Vec::new())
        }
    }
}

impl Transport for NamedPipeClient {
    fn send(&self, cmd: u32, data: &[u8]) -> Result<Vec<u8>> {
        NamedPipeClient::send(self, cmd, data)
    }
}

impl Drop for NamedPipeClient {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

unsafe impl Send for NamedPipeClient {}

pub fn compute_pipe_name(terminal_path: &str) -> String {
    let input = format!(r"\\?\{}", terminal_path.to_lowercase());
    let input_utf16: Vec<u16> = input.encode_utf16().collect();

    let mut buf = Vec::with_capacity(input_utf16.len() * 2);
    for c in input_utf16 {
        buf.push(c as u8);
        buf.push((c >> 8) as u8);
    }

    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let result = hasher.finalize();

    format!(
        r"\\.\pipe\MT5.Terminal.{}",
        hex::encode(result).to_uppercase()
    )
}

pub fn discover_mt5_pipe() -> String {
    let paths = find_terminal64_paths().unwrap_or_default();

    for path in &paths {
        let pipe_name = compute_pipe_name(path);
        if test_pipe_connection(&pipe_name) {
            return pipe_name;
        }
    }

    panic!("No responding MT5 pipe found");
}

fn test_pipe_connection(pipe_name: &str) -> bool {
    let pipe_name_wide: Vec<u16> = pipe_name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        WaitNamedPipeW(pipe_name_wide.as_ptr(), 500);
    }

    let handle = unsafe {
        CreateFileW(
            pipe_name_wide.as_ptr(),
            0x80000000 | 0x40000000,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };

    if handle != INVALID_HANDLE_VALUE {
        unsafe {
            CloseHandle(handle);
        }
        true
    } else {
        false
    }
}

fn find_terminal64_paths() -> Result<Vec<String>> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(Mt5Error::ConnectionFailed(
            "Failed to create process snapshot".into(),
        ));
    }

    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut pe = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    let mut result = unsafe { Process32FirstW(snapshot, &mut pe) };
    while result != 0 {
        let exe_name = String::from_utf16_lossy(&pe.szExeFile)
            .trim_end_matches('\0')
            .to_lowercase();

        if exe_name == "terminal64.exe" {
            if let Ok(path) = get_process_path(pe.th32ProcessID) {
                if seen.insert(path.clone()) {
                    paths.push(path);
                }
            }
        }

        result = unsafe { Process32NextW(snapshot, &mut pe) };
    }

    unsafe { CloseHandle(snapshot) };

    if paths.is_empty() {
        return Err(Mt5Error::ConnectionFailed(
            "No running terminal64.exe found".into(),
        ));
    }

    Ok(paths)
}

fn get_process_path(pid: u32) -> Result<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(Mt5Error::ConnectionFailed(format!(
            "Failed to open process {}",
            pid
        )));
    }

    let mut buf = [0u16; 32768];
    let mut size = buf.len() as u32;

    let result = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };

    unsafe { CloseHandle(handle) };

    if result == 0 {
        return Err(Mt5Error::ConnectionFailed(format!(
            "Failed to get process image name for PID {}",
            pid
        )));
    }

    Ok(String::from_utf16_lossy(&buf[..size as usize]))
}
