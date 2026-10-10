//! macOS 시스템 프레임워크 raw FFI. 필요한 심볼만 선언한다.
//! 모든 심볼은 macOS에서 안정적인 C API다(CoreGraphics / CoreFoundation / libSystem).

#![allow(non_snake_case, non_upper_case_globals)]

use std::os::raw::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

pub type CGDirectDisplayID = u32;
pub type CGEventRef = *mut c_void;
pub type CGEventSourceRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFStringRef = *const c_void;
pub type CFAllocatorRef = *const c_void;

/// CGEventTap 콜백. 반환한 이벤트가 시스템으로 전달된다(읽기 전용 탭에서는 보통 그대로 반환).
pub type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    etype: u32,
    event: CGEventRef,
    user: *mut c_void,
) -> CGEventRef;

// 이벤트 타입
pub const kCGEventLeftMouseDown: u32 = 1;
pub const kCGEventLeftMouseUp: u32 = 2;
pub const kCGEventRightMouseDown: u32 = 3;
pub const kCGEventRightMouseUp: u32 = 4;
pub const kCGEventMouseMoved: u32 = 5;
pub const kCGEventLeftMouseDragged: u32 = 6;
pub const kCGEventRightMouseDragged: u32 = 7;
pub const kCGEventKeyDown: u32 = 10;
pub const kCGEventKeyUp: u32 = 11;
pub const kCGEventFlagsChanged: u32 = 12;
pub const kCGEventScrollWheel: u32 = 22;
pub const kCGEventOtherMouseDown: u32 = 25;
pub const kCGEventOtherMouseUp: u32 = 26;
pub const kCGEventOtherMouseDragged: u32 = 27;
pub const kCGEventTapDisabledByTimeout: u32 = 0xFFFF_FFFE;
pub const kCGEventTapDisabledByUserInput: u32 = 0xFFFF_FFFF;

// 이벤트 필드
pub const kCGMouseEventClickState: u32 = 1;
pub const kCGMouseEventButtonNumber: u32 = 3;
pub const kCGKeyboardEventKeycode: u32 = 9;
pub const kCGScrollWheelEventDeltaAxis1: u32 = 11; // 세로
pub const kCGScrollWheelEventDeltaAxis2: u32 = 12; // 가로
pub const kCGEventSourceUserData: u32 = 42;

// 수정자 플래그(디바이스 독립)
pub const kCGEventFlagMaskShift: u64 = 0x0002_0000;
pub const kCGEventFlagMaskControl: u64 = 0x0004_0000;
pub const kCGEventFlagMaskAlternate: u64 = 0x0008_0000;
pub const kCGEventFlagMaskCommand: u64 = 0x0010_0000;

// 탭 생성 인자
pub const kCGSessionEventTap: u32 = 1;
pub const kCGHeadInsertEventTap: u32 = 0;
pub const kCGEventTapOptionListenOnly: u32 = 1;

// 마우스 버튼 번호
pub const kCGMouseButtonLeft: u32 = 0;
pub const kCGMouseButtonRight: u32 = 1;
pub const kCGMouseButtonCenter: u32 = 2;

// 스크롤 단위
pub const kCGScrollEventUnitLine: u32 = 1;

// CGEventPost 위치
pub const kCGHIDEventTap: u32 = 0;

// CGEventSource 상태
pub const kCGEventSourceStateHIDSystemState: i32 = 1;

/// 주입 이벤트 표식(필드 42). "MOX".
pub const MOX_MAGIC: i64 = 0x4D4F58;

#[repr(C)]
pub struct mach_timebase_info_data_t {
    pub numer: u32,
    pub denom: u32,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGMainDisplayID() -> CGDirectDisplayID;
    pub fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    pub fn CGDisplayPixelsWide(display: CGDirectDisplayID) -> usize;

    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        eventsOfInterest: u64,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventSourceCreate(stateID: i32) -> CGEventSourceRef;
    pub fn CGEventCreateMouseEvent(
        source: CGEventSourceRef,
        mouseType: u32,
        mouseCursorPosition: CGPoint,
        mouseButton: u32,
    ) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtualKey: u16,
        keyDown: bool,
    ) -> CGEventRef;
    // C 가변 인자 함수: wheel1은 고정, 그 뒤는 가변. arm64에서 가변 인자는 스택으로
    // 전달되므로 반드시 `...`로 선언해야 ABI가 맞는다.
    pub fn CGEventCreateScrollWheelEvent(
        source: CGEventSourceRef,
        units: u32,
        wheelCount: u32,
        wheel1: i32,
        ...
    ) -> CGEventRef;

    pub fn CGPreflightListenEventAccess() -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);
    pub fn CFRelease(cf: *const c_void);
}

extern "C" {
    pub fn mach_absolute_time() -> u64;
    pub fn mach_timebase_info(info: *mut mach_timebase_info_data_t) -> i32;
    pub fn mach_wait_until(deadline: u64) -> i32;
}
