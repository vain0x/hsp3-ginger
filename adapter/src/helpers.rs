use logger;
use std;
use winapi;

fn message_box(message: String) {
    let message = std::ffi::CString::new(message).unwrap();
    let caption = std::ffi::CString::new("rust").unwrap();

    unsafe {
        winapi::um::winuser::MessageBoxA(
            std::ptr::null_mut(),
            message.as_ptr(),
            caption.as_ptr(),
            winapi::um::winuser::MB_OK,
        );
    }
}

/// Aborts with error message (for debug)
pub fn failwith<T: std::fmt::Debug>(error: T) -> ! {
    let message = format!("Error in hsp3debug: {:?}", error);
    logger::log(&message);
    message_box(message);
    panic!("Error")
}
