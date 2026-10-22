use std::ffi::c_void;

/// Runtime helper: print an i64 value followed by newline.
#[no_mangle]
pub extern "C" fn jr_print_int(v: i64) {
    println!("{}", v);
}

/// Runtime helper: print an f64 value followed by newline.
#[no_mangle]
pub extern "C" fn jr_print_float(v: f64) {
    if v.fract() == 0.0 && !v.is_infinite() && !v.is_nan() {
        println!("{:.1}", v);
    } else {
        println!("{}", v);
    }
}

/// Runtime helper: print a raw pointer value followed by newline.
#[no_mangle]
pub extern "C" fn jr_print_ptr(v: *mut c_void) {
    println!("0x{:p}", v);
}

/// Runtime helper: print a bool value followed by newline.
#[no_mangle]
pub extern "C" fn jr_print_bool(v: i8) {
    println!("{}", if v != 0 { "true" } else { "false" });
}
