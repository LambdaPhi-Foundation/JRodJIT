use std::ffi::CString;

pub struct CLibrary {
    handle: isize,
}

impl CLibrary {
    pub fn load(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path).map_err(|e| format!("Invalid library path: {}", e))?;
        let handle = unsafe {
            windows_sys::Win32::System::LibraryLoader::LoadLibraryA(c_path.as_ptr() as *const u8)
        };
        if handle == 0 {
            return Err(format!("LoadLibraryA failed for '{}'", path));
        }
        Ok(Self { handle })
    }

    pub fn get_proc(&self, symbol: &str) -> Result<*mut std::ffi::c_void, String> {
        let c_sym = CString::new(symbol).map_err(|e| format!("Invalid symbol: {}", e))?;
        let addr: Option<unsafe extern "system" fn() -> isize> = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                self.handle,
                c_sym.as_ptr() as *const u8,
            )
        };
        match addr {
            Some(f) => Ok(f as *mut std::ffi::c_void),
            None => Err(format!("GetProcAddress failed for '{}'", symbol)),
        }
    }
}

unsafe impl Send for CLibrary {}
unsafe impl Sync for CLibrary {}
