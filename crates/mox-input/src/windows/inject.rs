//! SendInput으로 입력을 주입한다.

use mox_engine::{InjectEvent, InputError};
use mox_format::Button;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    XBUTTON1, XBUTTON2,
};

use crate::keymap_win::{code_to_vk, is_extended};

const MOX_MAGIC: usize = 0x4D4F58;

fn send_one(input: INPUT) {
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
}

/// 픽셀 좌표를 가상 데스크탑 기준 0..65535로 정규화.
fn normalize(x: i32, y: i32) -> (i32, i32) {
    let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let vw = (unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }).max(2);
    let vh = (unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }).max(2);
    let nx = ((x - vx) as i64 * 65535 / (vw - 1) as i64) as i32;
    let ny = ((y - vy) as i64 * 65535 / (vh - 1) as i64) as i32;
    (nx, ny)
}

fn mouse(flags: u32, nx: i32, ny: i32, data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: nx,
                dy: ny,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MOX_MAGIC,
            },
        },
    }
}

fn key(vk: u16, up: bool, extended: bool) -> INPUT {
    let mut flags = 0u32;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MOX_MAGIC,
            },
        },
    }
}

/// (down 플래그, up 플래그, xbutton 데이터) — 일반 버튼은 데이터 0.
fn button_flags(b: Button) -> (u32, u32, u32) {
    match b {
        Button::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        Button::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        Button::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        Button::Back => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON1 as u32),
        Button::Forward => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON2 as u32),
    }
}

pub fn inject(ev: &InjectEvent) -> Result<(), InputError> {
    match *ev {
        // 드래그는 Windows에서 버튼을 누른 채 이동하면 자동으로 일어난다 → 이동과 동일.
        InjectEvent::MoveTo { x, y } | InjectEvent::DragTo { x, y, .. } => {
            let (nx, ny) = normalize(x, y);
            send_one(mouse(
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                nx,
                ny,
                0,
            ));
        }
        InjectEvent::Down { button, x, y, .. } => {
            let (nx, ny) = normalize(x, y);
            let (down, _, data) = button_flags(button);
            send_one(mouse(
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | down,
                nx,
                ny,
                data,
            ));
        }
        InjectEvent::Up { button, x, y, .. } => {
            let (nx, ny) = normalize(x, y);
            let (_, up, data) = button_flags(button);
            send_one(mouse(
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | up,
                nx,
                ny,
                data,
            ));
        }
        InjectEvent::Wheel { dx, dy, x, y } => {
            let (nx, ny) = normalize(x, y);
            send_one(mouse(
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                nx,
                ny,
                0,
            ));
            if dy != 0 {
                send_one(mouse(MOUSEEVENTF_WHEEL, 0, 0, (dy * 120) as u32));
            }
            if dx != 0 {
                send_one(mouse(MOUSEEVENTF_HWHEEL, 0, 0, (dx * 120) as u32));
            }
        }
        // Windows는 수정자 키를 실제로 눌러 상태를 만든다(매크로가 수정자 이벤트를 따로 담음) → mods 무시.
        InjectEvent::KeyDown { key: code, .. } => {
            if let Some(vk) = code_to_vk(code) {
                send_one(key(vk, false, is_extended(code)));
            }
        }
        InjectEvent::KeyUp { key: code, .. } => {
            if let Some(vk) = code_to_vk(code) {
                send_one(key(vk, true, is_extended(code)));
            }
        }
        InjectEvent::RawKey { raw, down } => send_one(key(raw as u16, !down, false)),
    }
    Ok(())
}
