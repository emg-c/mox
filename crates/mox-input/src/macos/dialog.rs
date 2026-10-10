//! CFUserNotification 기반 확인 창. GUI 프레임워크 없이 CoreFoundation만으로 모달 알림을 띄운다.

use std::os::raw::c_void;

use super::ffi;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringCreateWithBytes(
        alloc: ffi::CFAllocatorRef,
        bytes: *const u8,
        num_bytes: isize,
        encoding: u32,
        is_external: bool,
    ) -> ffi::CFStringRef;
    fn CFUserNotificationDisplayAlert(
        timeout: f64,
        flags: u32,
        icon_url: *const c_void,
        sound_url: *const c_void,
        localization_url: *const c_void,
        alert_header: ffi::CFStringRef,
        alert_message: ffi::CFStringRef,
        default_button: ffi::CFStringRef,
        alternate_button: ffi::CFStringRef,
        other_button: ffi::CFStringRef,
        response_flags: *mut u32,
    ) -> i32;
}

const UTF8_ENCODING: u32 = 0x0800_0100;

struct CFStr(ffi::CFStringRef);

impl CFStr {
    fn new(s: &str) -> Self {
        let r = unsafe {
            CFStringCreateWithBytes(
                std::ptr::null(),
                s.as_ptr(),
                s.len() as isize,
                UTF8_ENCODING,
                false,
            )
        };
        CFStr(r)
    }
}

impl Drop for CFStr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::CFRelease(self.0) };
        }
    }
}

/// 확인 창을 띄운다. 기본 버튼(재생)을 누르면 true, 취소/닫기면 false.
pub fn confirm(header: &str, message: &str) -> bool {
    let h = CFStr::new(header);
    let m = CFStr::new(message);
    let yes = CFStr::new("재생");
    let no = CFStr::new("취소");
    let mut response: u32 = 0;
    let rc = unsafe {
        CFUserNotificationDisplayAlert(
            0.0,
            0,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            h.0,
            m.0,
            yes.0,
            no.0,
            std::ptr::null(),
            &mut response,
        )
    };
    // rc != 0 이면 창을 띄우지 못한 것 → 안전하게 취소로 본다.
    // response 0 = 기본 버튼(재생), 그 외 = 취소.
    rc == 0 && response == 0
}
