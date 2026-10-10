//! WH_MOUSE_LL + WH_KEYBOARD_LL 저수준 훅으로 입력을 캡처한다. 전용 스레드 메시지 루프에서 돈다.

use std::ptr;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Instant;

use mox_engine::{CaptureGuard, InputError, RawEvent, RawKind};
use mox_format::{Button, Key, Os};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_INJECTED, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1,
};

use crate::keymap_win::vk_to_code;

/// 주입 표식(dwExtraInfo). "MOX".
const MOX_MAGIC: usize = 0x4D4F58;

struct CapState {
    tx: Sender<RawEvent>,
    start: Instant,
}

static CAP: Mutex<Option<CapState>> = Mutex::new(None);

fn emit(kind: RawKind, injected: bool) {
    if let Some(st) = CAP.lock().unwrap().as_ref() {
        let _ = st.tx.send(RawEvent {
            at: st.start.elapsed(),
            kind,
            injected,
        });
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let ms = &*(lparam as *const MSLLHOOKSTRUCT);
        let injected = (ms.flags & LLMHF_INJECTED) != 0 || ms.dwExtraInfo == MOX_MAGIC;
        let (x, y) = (ms.pt.x, ms.pt.y);
        let hiword = (ms.mouseData >> 16) as i16;
        let kind = match wparam as u32 {
            WM_MOUSEMOVE => Some(RawKind::MouseMove { x, y }),
            WM_LBUTTONDOWN => Some(RawKind::MouseDown {
                button: Button::Left,
                x,
                y,
            }),
            WM_LBUTTONUP => Some(RawKind::MouseUp {
                button: Button::Left,
                x,
                y,
            }),
            WM_RBUTTONDOWN => Some(RawKind::MouseDown {
                button: Button::Right,
                x,
                y,
            }),
            WM_RBUTTONUP => Some(RawKind::MouseUp {
                button: Button::Right,
                x,
                y,
            }),
            WM_MBUTTONDOWN => Some(RawKind::MouseDown {
                button: Button::Middle,
                x,
                y,
            }),
            WM_MBUTTONUP => Some(RawKind::MouseUp {
                button: Button::Middle,
                x,
                y,
            }),
            WM_XBUTTONDOWN => {
                let b = if hiword == XBUTTON1 as i16 {
                    Button::Back
                } else {
                    Button::Forward
                };
                Some(RawKind::MouseDown { button: b, x, y })
            }
            WM_XBUTTONUP => {
                let b = if hiword == XBUTTON1 as i16 {
                    Button::Back
                } else {
                    Button::Forward
                };
                Some(RawKind::MouseUp { button: b, x, y })
            }
            WM_MOUSEWHEEL => Some(RawKind::Wheel {
                dx: 0,
                dy: (hiword / 120) as i32,
                x,
                y,
            }),
            WM_MOUSEHWHEEL => Some(RawKind::Wheel {
                dx: (hiword / 120) as i32,
                dy: 0,
                x,
                y,
            }),
            _ => None,
        };
        if let Some(kind) = kind {
            emit(kind, injected);
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn kbd_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let ks = &*(lparam as *const KBDLLHOOKSTRUCT);
        let injected = (ks.flags & LLKHF_INJECTED) != 0 || ks.dwExtraInfo == MOX_MAGIC;
        let key = match vk_to_code(ks.vkCode as u16) {
            Some(c) => Key::Code(c),
            None => Key::Unknown {
                raw: ks.vkCode,
                os: Os::Windows,
            },
        };
        let kind = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(RawKind::KeyDown(key)),
            WM_KEYUP | WM_SYSKEYUP => Some(RawKind::KeyUp(key)),
            _ => None,
        };
        if let Some(kind) = kind {
            emit(kind, injected);
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

struct WinCaptureGuard {
    tid: u32,
    thread: Option<JoinHandle<()>>,
}

impl Drop for WinCaptureGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(self.tid, WM_QUIT, 0, 0)
        };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        *CAP.lock().unwrap() = None;
    }
}

pub fn start_capture(tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError> {
    {
        let mut g = CAP.lock().unwrap();
        if g.is_some() {
            return Err(InputError::Os("이미 캡처 중입니다".into()));
        }
        *g = Some(CapState {
            tx,
            start: Instant::now(),
        });
    }
    let (setup_tx, setup_rx) = mpsc::channel::<Result<u32, String>>();
    let thread = std::thread::spawn(move || {
        let hm: HHOOK =
            unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), ptr::null_mut(), 0) };
        let hk: HHOOK =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_proc), ptr::null_mut(), 0) };
        if hm.is_null() || hk.is_null() {
            if !hm.is_null() {
                unsafe { UnhookWindowsHookEx(hm) };
            }
            if !hk.is_null() {
                unsafe { UnhookWindowsHookEx(hk) };
            }
            let _ = setup_tx.send(Err("저수준 훅을 설치하지 못했습니다".into()));
            return;
        }
        let tid = unsafe { GetCurrentThreadId() };
        let _ = setup_tx.send(Ok(tid));
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) } > 0 {}
        unsafe {
            UnhookWindowsHookEx(hm);
            UnhookWindowsHookEx(hk);
        }
    });
    match setup_rx.recv() {
        Ok(Ok(tid)) => Ok(CaptureGuard::new(WinCaptureGuard {
            tid,
            thread: Some(thread),
        })),
        Ok(Err(msg)) => {
            let _ = thread.join();
            *CAP.lock().unwrap() = None;
            Err(InputError::Os(msg))
        }
        Err(_) => {
            let _ = thread.join();
            *CAP.lock().unwrap() = None;
            Err(InputError::Os("훅 스레드가 시작하지 못했습니다".into()))
        }
    }
}
