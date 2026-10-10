//! InjectEvent를 CGEvent로 만들어 주입한다.

use mox_engine::{InjectEvent, InputError};
use mox_format::{Button, Mods};

use super::ffi;
use crate::keymap;

/// 공유 CGEventSource. 주입 이벤트에 재녹화 방지 표식을 달기 위해 백엔드가 하나 들고 쓴다.
pub struct Injector {
    source: ffi::CGEventSourceRef,
}

// CGEventSourceRef는 스레드 간 공유해도 안전하다(재생은 단일 스레드에서 호출).
unsafe impl Send for Injector {}
unsafe impl Sync for Injector {}

impl Injector {
    pub fn new() -> Result<Self, InputError> {
        let source = unsafe { ffi::CGEventSourceCreate(ffi::kCGEventSourceStateHIDSystemState) };
        if source.is_null() {
            return Err(InputError::Os("CGEventSource를 만들 수 없습니다".into()));
        }
        Ok(Self { source })
    }

    fn point(x: i32, y: i32) -> ffi::CGPoint {
        ffi::CGPoint {
            x: x as f64,
            y: y as f64,
        }
    }

    fn post(&self, event: ffi::CGEventRef) {
        if event.is_null() {
            return;
        }
        unsafe {
            ffi::CGEventSetIntegerValueField(event, ffi::kCGEventSourceUserData, ffi::MOX_MAGIC);
            ffi::CGEventPost(ffi::kCGHIDEventTap, event);
            ffi::CFRelease(event);
        }
    }

    fn mouse(&self, etype: u32, x: i32, y: i32, button: u32, clicks: Option<u32>) {
        let event =
            unsafe { ffi::CGEventCreateMouseEvent(self.source, etype, Self::point(x, y), button) };
        if let Some(c) = clicks {
            unsafe {
                ffi::CGEventSetIntegerValueField(event, ffi::kCGMouseEventClickState, c as i64)
            };
        }
        self.post(event);
    }

    fn flags_of(mods: Mods) -> u64 {
        let mut f = 0u64;
        if mods.shift {
            f |= ffi::kCGEventFlagMaskShift;
        }
        if mods.control {
            f |= ffi::kCGEventFlagMaskControl;
        }
        if mods.alt {
            f |= ffi::kCGEventFlagMaskAlternate;
        }
        if mods.meta {
            f |= ffi::kCGEventFlagMaskCommand;
        }
        f
    }

    fn key(&self, vk: u16, down: bool, mods: Mods) {
        let event = unsafe { ffi::CGEventCreateKeyboardEvent(self.source, vk, down) };
        if event.is_null() {
            return;
        }
        unsafe { ffi::CGEventSetFlags(event, Self::flags_of(mods)) };
        self.post(event);
    }

    pub fn inject(&self, ev: &InjectEvent) -> Result<(), InputError> {
        match *ev {
            InjectEvent::MoveTo { x, y } => {
                self.mouse(ffi::kCGEventMouseMoved, x, y, ffi::kCGMouseButtonLeft, None)
            }
            InjectEvent::DragTo { button, x, y } => {
                let (etype, btn) = drag_type(button);
                self.mouse(etype, x, y, btn, None);
            }
            InjectEvent::Down {
                button,
                x,
                y,
                clicks,
            } => {
                let (down_type, _, btn) = button_types(button);
                self.mouse(down_type, x, y, btn, Some(clicks));
            }
            InjectEvent::Up {
                button,
                x,
                y,
                clicks,
            } => {
                let (_, up_type, btn) = button_types(button);
                self.mouse(up_type, x, y, btn, Some(clicks));
            }
            InjectEvent::Wheel { dx, dy, x, y } => {
                // 좌표 이동 후 스크롤(스크롤 이벤트에는 위치가 없다).
                self.mouse(ffi::kCGEventMouseMoved, x, y, ffi::kCGMouseButtonLeft, None);
                let event = if dx == 0 {
                    unsafe {
                        ffi::CGEventCreateScrollWheelEvent(
                            self.source,
                            ffi::kCGScrollEventUnitLine,
                            1,
                            dy,
                        )
                    }
                } else {
                    unsafe {
                        ffi::CGEventCreateScrollWheelEvent(
                            self.source,
                            ffi::kCGScrollEventUnitLine,
                            2,
                            dy,
                            dx,
                        )
                    }
                };
                self.post(event);
            }
            InjectEvent::KeyDown { key, mods } => {
                if let Some(vk) = keymap::code_to_keycode(key) {
                    self.key(vk, true, mods);
                }
            }
            InjectEvent::KeyUp { key, mods } => {
                if let Some(vk) = keymap::code_to_keycode(key) {
                    self.key(vk, false, mods);
                }
            }
            InjectEvent::RawKey { raw, down } => self.key(raw as u16, down, Mods::default()),
        }
        Ok(())
    }
}

impl Drop for Injector {
    fn drop(&mut self) {
        unsafe { ffi::CFRelease(self.source) };
    }
}

/// (down 타입, up 타입, 버튼 번호)
fn button_types(b: Button) -> (u32, u32, u32) {
    match b {
        Button::Left => (
            ffi::kCGEventLeftMouseDown,
            ffi::kCGEventLeftMouseUp,
            ffi::kCGMouseButtonLeft,
        ),
        Button::Right => (
            ffi::kCGEventRightMouseDown,
            ffi::kCGEventRightMouseUp,
            ffi::kCGMouseButtonRight,
        ),
        Button::Middle => (
            ffi::kCGEventOtherMouseDown,
            ffi::kCGEventOtherMouseUp,
            ffi::kCGMouseButtonCenter,
        ),
        Button::Back => (ffi::kCGEventOtherMouseDown, ffi::kCGEventOtherMouseUp, 3),
        Button::Forward => (ffi::kCGEventOtherMouseDown, ffi::kCGEventOtherMouseUp, 4),
    }
}

/// (드래그 타입, 버튼 번호)
fn drag_type(b: Button) -> (u32, u32) {
    match b {
        Button::Left => (ffi::kCGEventLeftMouseDragged, ffi::kCGMouseButtonLeft),
        Button::Right => (ffi::kCGEventRightMouseDragged, ffi::kCGMouseButtonRight),
        Button::Middle => (ffi::kCGEventOtherMouseDragged, ffi::kCGMouseButtonCenter),
        Button::Back => (ffi::kCGEventOtherMouseDragged, 3),
        Button::Forward => (ffi::kCGEventOtherMouseDragged, 4),
    }
}
