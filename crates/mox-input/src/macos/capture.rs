//! CGEventTap으로 실제 입력을 캡처한다. 전용 스레드에서 CFRunLoop를 돌린다.

use std::cell::RefCell;
use std::collections::HashSet;
use std::os::raw::c_void;
use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Instant;

use mox_engine::{CaptureGuard, InputError, RawEvent, RawKind};
use mox_format::{Button, Key, Os};

use super::ffi;
use crate::keymap;

/// 콜백이 공유하는 상태. 탭 스레드에서만 접근하므로 RefCell로 충분하다.
struct TapCtx {
    tx: Sender<RawEvent>,
    start: Instant,
    /// 현재 눌려 있는 수정자 키코드(FlagsChanged로 up/down을 판별하기 위함).
    held_mods: RefCell<HashSet<u16>>,
}

fn now_dur(start: Instant) -> std::time::Duration {
    start.elapsed()
}

fn send(ctx: &TapCtx, event: ffi::CGEventRef, kind: RawKind) {
    let injected = unsafe { ffi::CGEventGetIntegerValueField(event, ffi::kCGEventSourceUserData) }
        == ffi::MOX_MAGIC;
    let _ = ctx.tx.send(RawEvent { at: now_dur(ctx.start), kind, injected });
}

fn location(event: ffi::CGEventRef) -> (i32, i32) {
    let p = unsafe { ffi::CGEventGetLocation(event) };
    (p.x.round() as i32, p.y.round() as i32)
}

fn key_of(event: ffi::CGEventRef) -> Key {
    let vk = unsafe { ffi::CGEventGetIntegerValueField(event, ffi::kCGKeyboardEventKeycode) } as u16;
    match keymap::keycode_to_code(vk) {
        Some(code) => Key::Code(code),
        None => Key::Unknown { raw: vk as u32, os: Os::Macos },
    }
}

fn other_button(event: ffi::CGEventRef) -> Button {
    match unsafe { ffi::CGEventGetIntegerValueField(event, ffi::kCGMouseEventButtonNumber) } {
        2 => Button::Middle,
        3 => Button::Back,
        4 => Button::Forward,
        _ => Button::Middle,
    }
}

extern "C" fn tap_callback(
    _proxy: ffi::CGEventTapProxy,
    etype: u32,
    event: ffi::CGEventRef,
    user: *mut c_void,
) -> ffi::CGEventRef {
    // user는 start_capture가 넘긴 TapCtx. 탭이 유효한 동안 살아 있다.
    let ctx = unsafe { &*(user as *const TapCtx) };

    if etype == ffi::kCGEventTapDisabledByTimeout || etype == ffi::kCGEventTapDisabledByUserInput {
        // 시스템이 탭을 껐다. 다시 켠다. (CaptureGuard가 보관한 포트는 여기서 알 수 없으므로
        // 전역 TLS 대신, 재활성화는 아래 RE_ENABLE로 처리한다.)
        RE_ENABLE.with(|p| {
            let port = *p.borrow();
            if !port.is_null() {
                unsafe { ffi::CGEventTapEnable(port, true) };
            }
        });
        return event;
    }

    let (x, y) = location(event);
    let kind = match etype {
        ffi::kCGEventMouseMoved
        | ffi::kCGEventLeftMouseDragged
        | ffi::kCGEventRightMouseDragged
        | ffi::kCGEventOtherMouseDragged => Some(RawKind::MouseMove { x, y }),
        ffi::kCGEventLeftMouseDown => Some(RawKind::MouseDown { button: Button::Left, x, y }),
        ffi::kCGEventLeftMouseUp => Some(RawKind::MouseUp { button: Button::Left, x, y }),
        ffi::kCGEventRightMouseDown => Some(RawKind::MouseDown { button: Button::Right, x, y }),
        ffi::kCGEventRightMouseUp => Some(RawKind::MouseUp { button: Button::Right, x, y }),
        ffi::kCGEventOtherMouseDown => {
            Some(RawKind::MouseDown { button: other_button(event), x, y })
        }
        ffi::kCGEventOtherMouseUp => Some(RawKind::MouseUp { button: other_button(event), x, y }),
        ffi::kCGEventScrollWheel => {
            let dy = unsafe {
                ffi::CGEventGetIntegerValueField(event, ffi::kCGScrollWheelEventDeltaAxis1)
            } as i32;
            let dx = unsafe {
                ffi::CGEventGetIntegerValueField(event, ffi::kCGScrollWheelEventDeltaAxis2)
            } as i32;
            Some(RawKind::Wheel { dx, dy, x, y })
        }
        ffi::kCGEventKeyDown => Some(RawKind::KeyDown(key_of(event))),
        ffi::kCGEventKeyUp => Some(RawKind::KeyUp(key_of(event))),
        ffi::kCGEventFlagsChanged => {
            // 수정자 키: 이미 눌려 있었으면 이번엔 뗌, 아니면 누름.
            let vk =
                unsafe { ffi::CGEventGetIntegerValueField(event, ffi::kCGKeyboardEventKeycode) }
                    as u16;
            let key = key_of(event);
            let mut held = ctx.held_mods.borrow_mut();
            if held.remove(&vk) {
                Some(RawKind::KeyUp(key))
            } else {
                held.insert(vk);
                Some(RawKind::KeyDown(key))
            }
        }
        _ => None,
    };
    if let Some(kind) = kind {
        send(ctx, event, kind);
    }
    event
}

thread_local! {
    /// 콜백이 탭을 재활성화할 때 쓰는 포트. 탭 스레드에서만 설정·사용.
    static RE_ENABLE: RefCell<ffi::CFMachPortRef> = const { RefCell::new(ptr::null_mut()) };
}

/// 캡처를 멈추는 가드. drop 시 런루프를 멈추고 스레드를 join한다.
struct MacCaptureGuard {
    run_loop: usize, // CFRunLoopRef 주소(스레드 간 전달용)
    thread: Option<JoinHandle<()>>,
    _ctx: Box<TapCtx>,
}

impl Drop for MacCaptureGuard {
    fn drop(&mut self) {
        unsafe { ffi::CFRunLoopStop(self.run_loop as ffi::CFRunLoopRef) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// 모든 입력 이벤트 타입 비트마스크(1<<etype)의 합. 관심 타입만 켠다.
fn event_mask() -> u64 {
    let types = [
        ffi::kCGEventLeftMouseDown,
        ffi::kCGEventLeftMouseUp,
        ffi::kCGEventRightMouseDown,
        ffi::kCGEventRightMouseUp,
        ffi::kCGEventMouseMoved,
        ffi::kCGEventLeftMouseDragged,
        ffi::kCGEventRightMouseDragged,
        ffi::kCGEventKeyDown,
        ffi::kCGEventKeyUp,
        ffi::kCGEventFlagsChanged,
        ffi::kCGEventScrollWheel,
        ffi::kCGEventOtherMouseDown,
        ffi::kCGEventOtherMouseUp,
        ffi::kCGEventOtherMouseDragged,
    ];
    types.iter().fold(0u64, |m, &t| m | (1u64 << t))
}

pub fn start_capture(tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError> {
    let ctx = Box::new(TapCtx {
        tx,
        start: Instant::now(),
        held_mods: RefCell::new(HashSet::new()),
    });
    let ctx_ptr: *const TapCtx = &*ctx;
    // 탭 스레드 설정 결과(런루프 주소 또는 오류)를 받는 채널.
    let (setup_tx, setup_rx) = mpsc::channel::<Result<usize, String>>();
    // raw 포인터는 Send가 아니므로 주소를 usize로 넘겨 스레드 안에서 되돌린다.
    let user_addr = ctx_ptr as usize;

    let thread = std::thread::spawn(move || {
        let user = user_addr as *mut c_void;
        let port = unsafe {
            ffi::CGEventTapCreate(
                ffi::kCGSessionEventTap,
                ffi::kCGHeadInsertEventTap,
                ffi::kCGEventTapOptionListenOnly,
                event_mask(),
                tap_callback,
                user,
            )
        };
        if port.is_null() {
            let _ = setup_tx.send(Err("입력 모니터링 권한이 없거나 탭을 만들 수 없습니다".into()));
            return;
        }
        RE_ENABLE.with(|p| *p.borrow_mut() = port);
        let source = unsafe {
            ffi::CFMachPortCreateRunLoopSource(ptr::null(), port, 0)
        };
        let run_loop = unsafe { ffi::CFRunLoopGetCurrent() };
        unsafe {
            ffi::CFRunLoopAddSource(run_loop, source, ffi::kCFRunLoopCommonModes);
            ffi::CGEventTapEnable(port, true);
        }
        let _ = setup_tx.send(Ok(run_loop as usize));
        unsafe { ffi::CFRunLoopRun() };
        // 런루프 종료 후 정리.
        unsafe {
            ffi::CGEventTapEnable(port, false);
            ffi::CFRelease(source);
            ffi::CFRelease(port);
        }
        RE_ENABLE.with(|p| *p.borrow_mut() = ptr::null_mut());
    });

    match setup_rx.recv() {
        Ok(Ok(run_loop)) => Ok(CaptureGuard::new(MacCaptureGuard {
            run_loop,
            thread: Some(thread),
            _ctx: ctx,
        })),
        Ok(Err(msg)) => {
            let _ = thread.join();
            Err(InputError::Permission(msg))
        }
        Err(_) => {
            let _ = thread.join();
            Err(InputError::Os("탭 스레드가 시작하지 못했습니다".into()))
        }
    }
}
