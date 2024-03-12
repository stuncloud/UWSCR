use std::sync::{
    mpsc::{channel, Receiver, RecvError, Sender},
    OnceLock,
};

use windows::core::HSTRING;
use windows::Win32::{
    Foundation::{WPARAM, LPARAM, LRESULT, POINT, HWND},
    UI::WindowsAndMessaging::{
        SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx,
        HHOOK,
        WH_MOUSE_LL, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, KBDLLHOOKSTRUCT,
        HC_ACTION,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        KBDLLHOOKSTRUCT_FLAGS,
        WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        WM_MBUTTONDOWN, WM_MBUTTONUP,

        MessageBoxW, MB_OKCANCEL, IDOK,
        WindowFromPoint, GetParent, IsWindowVisible, GetClassNameW, GetWindowTextW,
    }
};

type Win32Result<T> = windows::core::Result<T>;

static DESKTOP_SENDER: OnceLock<Sender<DesktopRecord>> = OnceLock::new();

#[derive(Debug)]
enum DesktopRecordDetail {
    LeftClick {
        control: Control,
        window: Control,
    },
    LeftDrag {
        from: Point,
        to: Point,
    },
    RightClick {
        control: Control,
        window: Control,
    },
    RightDrag {
        from: Point,
        to: Point,
    },
    LLMouseMove(Point),
    LLMouseWheel(i16),
    LLMouseLeftDown(Point),
    LLMouseLeftUp(Point),
    LLMouseRightDown(Point),
    LLMouseRightUp(Point),
    LLMouseMiddleDown(Point),
    LLMouseMiddleUp(Point),
    LLMouseUnknown(u32),
    LLKeyboardDown(u32),
    LLKeyboardUp(u32),
    LLKeyboardDownSys(u32),
    LLKeyboardUpSys(u32),
    LLKeyboarUnknown(u32),
}
#[derive(Debug)]
struct Control {
    hwnd: HWND,
    title: String,
    class: String,
}
impl From<HWND> for Control {
    fn from(hwnd: HWND) -> Self {
        let (title, class) = get_text_and_class(hwnd);
        Self { hwnd, title, class }
    }
}
impl From<Point> for Control {
    fn from(point: Point) -> Self {
        let hwnd = window_from_point(point.0);
        Self::from(hwnd)
    }
}

#[derive(Debug, PartialEq)]
struct Point(POINT);
impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.0.x, self.0.y)
    }
}
impl From<POINT> for Point {
    fn from(point: POINT) -> Self {
        Self(point)
    }
}

pub enum RecordLevel {
    Low,
    High,
}

fn record_low_level(receiver: Receiver<DesktopRecord>) -> Vec<DesktopRecordDetail> {
    let mut details = Vec::new();
    loop {
        if let Ok(record) = receiver.recv() {
            let detail = match record {
                DesktopRecord::Mouse(e, t) => {
                    match t {
                        MouseEventType::LeftDown => DesktopRecordDetail::LLMouseLeftDown(e.point),
                        MouseEventType::LeftUp => DesktopRecordDetail::LLMouseLeftUp(e.point),
                        MouseEventType::RightDown => DesktopRecordDetail::LLMouseRightDown(e.point),
                        MouseEventType::RightUp => DesktopRecordDetail::LLMouseRightUp(e.point),
                        MouseEventType::Move => DesktopRecordDetail::LLMouseMove(e.point),
                        MouseEventType::Wheel => {
                            let delta = ((e.data & 0xFFFF0000) >> 16) as u16 as i16;
                            DesktopRecordDetail::LLMouseWheel(delta)
                        },
                        MouseEventType::MiddleDown => DesktopRecordDetail::LLMouseMiddleDown(e.point),
                        MouseEventType::MiddleUp => DesktopRecordDetail::LLMouseMiddleUp(e.point),
                    }
                },
                DesktopRecord::MouseUnknown(n) => DesktopRecordDetail::LLMouseUnknown(n),
                DesktopRecord::Keyboard(e, t) => {
                    match t {
                        KeyboardEventType::Up => DesktopRecordDetail::LLKeyboardUp(e.vk),
                        KeyboardEventType::SysUp => DesktopRecordDetail::LLKeyboardUpSys(e.vk),
                        KeyboardEventType::Down => DesktopRecordDetail::LLKeyboardDown(e.vk),
                        KeyboardEventType::SysDown => DesktopRecordDetail::LLKeyboardDownSys(e.vk),
                    }
                },
                DesktopRecord::KeyboardUnknown(n) => DesktopRecordDetail::LLKeyboarUnknown(n),
                DesktopRecord::StopRecording => break,
            };
            details.push(detail);
        }
    }
    details
}
fn record_high_level(receiver: Receiver<DesktopRecord>) -> Vec<DesktopRecordDetail> {
    let mut left_button = None::<MouseEvent>;
    let mut right_button = None::<MouseEvent>;
    let mut details = Vec::new();
    loop {
        if let Ok(record) = receiver.recv() {
            match record {
                DesktopRecord::Mouse(e, t) => {
                    match t {
                        MouseEventType::LeftDown => {
                            left_button = Some(e);
                        },
                        MouseEventType::LeftUp => {
                            match left_button {
                                Some(down) => {
                                    let detail = if e.point == down.point {
                                        // クリック
                                        let control = Control::from(e.point);
                                        let parent = get_parent_win(control.hwnd);
                                        let window = Control::from(parent);
                                        DesktopRecordDetail::LeftClick { control, window }
                                    } else {
                                        // ドラッグ
                                        DesktopRecordDetail::LeftDrag { from: down.point, to: e.point }
                                    };
                                    left_button = None;
                                    details.push(detail);
                                },
                                None => {
                                    // 来ないはず
                                },
                            }
                        },
                        MouseEventType::RightDown => {
                            right_button = Some(e);
                        },
                        MouseEventType::RightUp => {
                            match right_button {
                                Some(down) => {
                                    let detail = if e.point == down.point {
                                        // クリック
                                        let control = Control::from(e.point);
                                        let parent = get_parent_win(control.hwnd);
                                        let window = Control::from(parent);
                                        DesktopRecordDetail::RightClick { control, window }
                                    } else {
                                        // ドラッグ
                                        DesktopRecordDetail::RightDrag { from: down.point, to: e.point }
                                    };
                                    right_button = None;
                                    details.push(detail);
                                },
                                None => {
                                    // 来ないはず
                                },
                            }
                        },
                        MouseEventType::Wheel => {
                            println!("\u{001b}[35m[debug] wheel: {e:?}\u{001b}[0m");
                        },
                        MouseEventType::MiddleDown => {}
                        MouseEventType::MiddleUp => {}
                        MouseEventType::Move => {},
                    }
                },
                DesktopRecord::MouseUnknown(n) => {
                    println!("\u{001b}[32m[debug] unknown mouse: {n}\u{001b}[0m");
                },
                DesktopRecord::Keyboard(e, t) => {
                    match t {
                        KeyboardEventType::Up => {
                            println!("\u{001b}[90m[debug] Up: {e:?}\u{001b}[0m");
                        },
                        KeyboardEventType::Down => {
                            println!("\u{001b}[90m[debug] Down: {e:?}\u{001b}[0m");
                        },
                        KeyboardEventType::SysUp => {
                            println!("\u{001b}[90m[debug] SysUp: {e:?}\u{001b}[0m");
                        },
                        KeyboardEventType::SysDown => {
                            println!("\u{001b}[90m[debug] SysDown: {e:?}\u{001b}[0m");
                        },
                    }
                },
                DesktopRecord::KeyboardUnknown(n) => {
                    println!("\u{001b}[36m[debug] unknown kbd: {n}\u{001b}[0m");
                },
                DesktopRecord::StopRecording => break,
            }
        }
    }
    details
}

fn details_to_script(details: Vec<DesktopRecordDetail>) -> Vec<String> {
    let f = |detail| {
        match detail {
            // DesktopRecordDetail::LeftClick { control, window } => todo!(),
            // DesktopRecordDetail::LeftDrag { from, to } => todo!(),
            // DesktopRecordDetail::RightClick { control, window } => todo!(),
            // DesktopRecordDetail::RightDrag { from, to } => todo!(),
            DesktopRecordDetail::LLMouseMove(p) => format!("mmv({p})"),
            DesktopRecordDetail::LLMouseWheel(delta) => format!("btn(WHEEL), {delta}"),
            DesktopRecordDetail::LLMouseLeftDown(p) => format!("btn(LEFT, DOWN, {p})"),
            DesktopRecordDetail::LLMouseLeftUp(p) => format!("btn(LEFT, UP, {p})"),
            DesktopRecordDetail::LLMouseRightDown(p) => format!("btn(RIGHT, DOWN, {p})"),
            DesktopRecordDetail::LLMouseRightUp(p) => format!("btn(RIGHT, UP, {p})"),
            DesktopRecordDetail::LLMouseMiddleDown(p) => format!("btn(MIDDLE, DOWN, {p})"),
            DesktopRecordDetail::LLMouseMiddleUp(p) => format!("btn(MIDDLE, UP, {p})"),
            // DesktopRecordDetail::LLMouseUnknown(n) => format!(""),
            DesktopRecordDetail::LLKeyboardDown(vk) => format!("kbd({vk}, DOWN)"),
            DesktopRecordDetail::LLKeyboardUp(vk) => format!("kbd({vk}, UP)"),
            DesktopRecordDetail::LLKeyboardDownSys(vk) => format!("kbd({vk}, DOWN)"),
            DesktopRecordDetail::LLKeyboardUpSys(vk) => format!("kbd({vk}, UP)"),
            // DesktopRecordDetail::LLKeyboarUnknown(p) => format!(""),
            d => format!("{d:?}"),
        }
    };
    details.into_iter().map(f).collect()
}
pub fn record_desktop(level: RecordLevel) -> RecordResult<Option<Vec<String>>> {
    unsafe {
        let (sender, receiver) = channel();
        let sender2 = sender.clone();
        DESKTOP_SENDER.set(sender2).map_err(|_| RecorderError::Send)?;

        let recorder = DesktopRecorder::new()?;

        let handle = std::thread::spawn(move || {
            let records = match level {
                RecordLevel::Low => record_low_level(receiver),
                RecordLevel::High => record_high_level(receiver),
            };
            records
        });

        let title = HSTRING::from("UWSCR デスクトップ操作記録");
        let message = HSTRING::from("操作後にOKで保存");
        let r = MessageBoxW(None, &message, &title, MB_OKCANCEL);
        recorder.unhook()?;
        sender.send(DesktopRecord::StopRecording).map_err(|_| RecorderError::Send)?;

        if r == IDOK {
            let details = handle.join().map_err(|_| RecorderError::JoinHandle)?;
            let script = details_to_script(details);
            Ok(Some(script))
        } else {
            Ok(None)
        }
    }
}
fn window_from_point(point: POINT) -> HWND {
    unsafe {
        WindowFromPoint(point)
    }
}
fn get_parent_win(child: HWND) -> HWND {
    unsafe {
        let mut hwnd = child;
        loop {
            let parent = GetParent(hwnd);
            if parent.0 == 0 || ! IsWindowVisible(parent).as_bool() {
                break hwnd;
            } else {
                hwnd = parent;
            }
        }
    }
}
fn get_text_and_class(hwnd: HWND) -> (String, String) {
    unsafe {
        let mut buf = [0; 512];
        let len = GetWindowTextW(hwnd, &mut buf) as usize;
        let text = String::from_utf16_lossy(&buf[..len]);
        let mut buf = [0; 512];
        let len = GetClassNameW(hwnd, &mut buf) as usize;
        let class = String::from_utf16_lossy(&buf[..len]);
        (text, class)
    }
}

type RecordResult<T> = Result<T, RecorderError>;
pub enum RecorderError {
    Win32(windows::core::Error),
    Recv(RecvError),
    Send,
    JoinHandle,
}
impl From<windows::core::Error> for RecorderError {
    fn from(err: windows::core::Error) -> Self {
        Self::Win32(err)
    }
}
impl From<RecvError> for RecorderError {
    fn from(err: RecvError) -> Self {
        Self::Recv(err)
    }
}
impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecorderError::Win32(e) => write!(f, "{e}"),
            RecorderError::Recv(e) => write!(f, "{e}"),
            RecorderError::Send => write!(f, "Sender error"),
            RecorderError::JoinHandle => write!(f, "JoinHandle error"),
        }
    }
}

#[derive(Debug)]
enum DesktopRecord {
    Mouse(MouseEvent, MouseEventType),
    MouseUnknown(u32),
    Keyboard(KeyboardEvent, KeyboardEventType),
    KeyboardUnknown(u32),
    StopRecording,
}
#[derive(Debug)]
enum MouseEventType {
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    MiddleDown,
    MiddleUp,
    Move,
    Wheel,
}
#[derive(Debug)]
struct MouseEvent {
    point: Point,
    data: u32,
    flags: u32,
    time: u32,
}
impl From<&MSLLHOOKSTRUCT> for MouseEvent {
    fn from(mhs: &MSLLHOOKSTRUCT) -> Self {
        Self {
            point: mhs.pt.into(),
            data: mhs.mouseData,
            flags: mhs.flags,
            time: mhs.time,
        }
    }
}
#[derive(Debug)]
enum KeyboardEventType {
    Up,
    SysUp,
    Down,
    SysDown,
}
#[derive(Debug)]
struct KeyboardEvent {
    vk: u32,
    scan_code: u32,
    flags: KBDLLHOOKSTRUCT_FLAGS,
    time: u32
}
impl From<&KBDLLHOOKSTRUCT> for KeyboardEvent {
    fn from(khs: &KBDLLHOOKSTRUCT) -> Self {
        Self {
            vk: khs.vkCode,
            scan_code: khs.scanCode,
            flags: khs.flags,
            time: khs.time,
        }
    }
}

struct DesktopRecorder {
    mouse: HHOOK,
    keyboard: HHOOK,
}
impl Drop for DesktopRecorder {
    fn drop(&mut self) {
        unsafe {
            let _ = self.unhook();
        }
    }
}
impl DesktopRecorder {

    unsafe fn new() -> Win32Result<Self> {
        let (mouse, keyboard) = Self::hook()?;
        let recorder = Self { mouse, keyboard };
        Ok(recorder)
    }
    unsafe fn hook() -> Win32Result<(HHOOK, HHOOK)>{
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(Self::ll_mouse_hook), None, 0)?;
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::ll_keyboard_hook), None, 0)?;
        Ok((mouse, keyboard))
    }
    unsafe fn unhook(&self) -> Win32Result<()> {
        UnhookWindowsHookEx(self.mouse)?;
        UnhookWindowsHookEx(self.keyboard)?;
        Ok(())
    }

    unsafe extern "system"
    fn ll_mouse_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {

        if ncode == HC_ACTION as i32 {
            if let Some(sender) = DESKTOP_SENDER.get() {
                let p = lparam.0 as *const MSLLHOOKSTRUCT;
                let event = MouseEvent::from(&*p);
                let record = match wparam.0 as u32 {
                    WM_LBUTTONDOWN => DesktopRecord::Mouse(event, MouseEventType::LeftDown),
                    WM_LBUTTONUP => DesktopRecord::Mouse(event, MouseEventType::LeftUp),
                    WM_RBUTTONDOWN => DesktopRecord::Mouse(event, MouseEventType::RightDown),
                    WM_RBUTTONUP => DesktopRecord::Mouse(event, MouseEventType::RightUp),
                    WM_MOUSEMOVE => DesktopRecord::Mouse(event, MouseEventType::Move),
                    WM_MOUSEWHEEL => DesktopRecord::Mouse(event, MouseEventType::Wheel),
                    WM_MBUTTONDOWN => DesktopRecord::Mouse(event, MouseEventType::MiddleDown),
                    WM_MBUTTONUP => DesktopRecord::Mouse(event, MouseEventType::MiddleUp),
                    msg => DesktopRecord::MouseUnknown(msg),
                };
                let _ = sender.send(record);
            }
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
    unsafe extern "system"
    fn ll_keyboard_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if ncode == HC_ACTION as i32 {
            if let Some(sender) = DESKTOP_SENDER.get() {
                let p = lparam.0 as *const KBDLLHOOKSTRUCT;
                let event = KeyboardEvent::from(&*p);
                let record = match wparam.0 as u32 {
                    WM_KEYDOWN => DesktopRecord::Keyboard(event, KeyboardEventType::Down),
                    WM_KEYUP => DesktopRecord::Keyboard(event, KeyboardEventType::Up),
                    WM_SYSKEYDOWN => DesktopRecord::Keyboard(event, KeyboardEventType::SysDown),
                    WM_SYSKEYUP => DesktopRecord::Keyboard(event, KeyboardEventType::SysUp),
                    msg => DesktopRecord::KeyboardUnknown(msg),
                };
                let _ = sender.send(record);
            }
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }

}