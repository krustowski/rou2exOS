#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        let mut i = 0;
        while i < n {
            *dst.add(i) = *src.add(i);
            i += 1;
        }
        dst
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(dst: *mut u8, val: i32, n: usize) -> *mut u8 {
    unsafe {
        let mut i = 0;
        while i < n {
            *dst.add(i) = val as u8;
            i += 1;
        }
        dst
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        if src < dst as *const u8 {
            let mut i = n;
            while i != 0 {
                i -= 1;
                *dst.add(i) = *src.add(i);
            }
        } else {
            let mut i = 0;
            while i < n {
                *dst.add(i) = *src.add(i);
                i += 1;
            }
        }
        dst
    }
}

