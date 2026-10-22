use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
};
use std::ptr::null_mut;

pub struct ExecBuffer {
    ptr: *mut u8,
    size: usize,
    capacity: usize,
    committed: bool,
}

impl ExecBuffer {
    pub fn new(initial_size: usize) -> Result<Self, String> {
        let size = initial_size.next_power_of_two().max(4096);
        let ptr = unsafe {
            VirtualAlloc(
                null_mut(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };
        if ptr.is_null() {
            return Err(format!(
                "VirtualAlloc failed for {} bytes: error {}",
                size,
                unsafe { GetLastError() }
            ));
        }
        Ok(Self {
            ptr: ptr as *mut u8,
            size: 0,
            capacity: size,
            committed: true,
        })
    }

    pub fn write(&mut self, byte: u8) {
        if self.size >= self.capacity {
            self.grow();
        }
        unsafe {
            *self.ptr.add(self.size) = byte;
        }
        self.size += 1;
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write(b);
        }
    }

    pub fn write_u32_le(&mut self, val: u32) {
        self.write_bytes(&val.to_le_bytes());
    }

    pub fn write_i32_le(&mut self, val: i32) {
        self.write_bytes(&val.to_le_bytes());
    }

    pub fn write_u64_le(&mut self, val: u64) {
        self.write_bytes(&val.to_le_bytes());
    }

    pub fn current_offset(&self) -> usize {
        self.size
    }

    pub fn patch_rel32(&mut self, offset: usize, target: usize) {
        let delta = target as isize - (offset as isize + 4);
        unsafe {
            let p = self.ptr.add(offset) as *mut u32;
            *p = delta as u32;
        }
    }

    fn grow(&mut self) {
        let new_cap = self.capacity * 2;
        let new_ptr = unsafe {
            VirtualAlloc(
                null_mut(),
                new_cap,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };
        if new_ptr.is_null() {
            panic!(
                "ExecBuffer grow failed: error {}",
                unsafe { GetLastError() }
            );
        }
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr, new_ptr as *mut u8, self.size);
            VirtualFree(self.ptr as *mut _, 0, MEM_RELEASE);
        }
        self.ptr = new_ptr as *mut u8;
        self.capacity = new_cap;
    }

    pub fn finalize(&mut self) -> Result<*mut u8, String> {
        let mut old_protect = 0u32;
        let ok = unsafe {
            VirtualProtect(
                self.ptr as *mut _,
                self.capacity,
                PAGE_EXECUTE_READ,
                &mut old_protect as *mut u32,
            )
        };
        if ok == 0 {
            return Err(format!(
                "VirtualProtect failed: error {}",
                unsafe { GetLastError() }
            ));
        }
        Ok(self.ptr)
    }

    pub fn data(&self) -> *mut u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl Drop for ExecBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                VirtualFree(self.ptr as *mut _, 0, MEM_RELEASE);
            }
        }
    }
}

unsafe impl Send for ExecBuffer {}
unsafe impl Sync for ExecBuffer {}

pub fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
