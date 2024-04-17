mod acc;
mod clkitem;
mod win32;
mod monitor;
mod uia;

use crate::{Evaluator, MouseOrg, MorgTarget, MorgContext, LOGPRINTWIN};
use crate::object::*;
use crate::builtins::*;
use crate::builtins::{
    window_low,
    system_controls::is_64bit_os,
    text_control::ErrConst,
    dialog::THREAD_LOCAL_BALLOON,
};
use crate::gui::UWindow;
pub use monitor::Monitor;
pub use acc::U32Ext;
use util::winapi::get_console_hwnd;
use util::clipboard::Clipboard;

#[cfg(feature="chkimg")]
use crate::builtins::chkimg::{ChkImg, ScreenShot, CheckColor};
#[cfg(feature="chkimg")]
use util::settings::USETTINGS;

#[allow(unused_braces)]
use windows::{
    Win32::{
        Foundation::{
            MAX_PATH,
            BOOL, HANDLE, HMODULE,
            HWND, WPARAM, LPARAM, POINT, RECT,
            CloseHandle,
        },
        System::{
            Threading::{
                PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
                OpenProcess, WaitForInputIdle, IsWow64Process,
            },
            ProcessStatus::GetModuleFileNameExW,
            Ole::CF_BITMAP,
            DataExchange::{
                OpenClipboard, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
            }
        },
        UI::{
            WindowsAndMessaging::{
                SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
                SW_SHOWNORMAL, SW_SHOW, SW_HIDE, SW_MINIMIZE, SW_MAXIMIZE,
                WINDOWPLACEMENT,
                WM_CLOSE, WM_DESTROY, HWND_TOPMOST, HWND_NOTOPMOST,
                WindowFromPoint, GetParent, IsWindowVisible, GetClientRect,
                GetForegroundWindow, GetWindowTextW, GetClassNameW, EnumWindows,
                IsWindow, PostMessageW, SetForegroundWindow, ShowWindow,
                SetWindowPos, GetWindowRect, MoveWindow, GetWindowPlacement,
                GetWindowThreadProcessId, IsIconic, IsHungAppWindow,
                EnumChildWindows, GetMenu, GetSystemMenu,
                GetCursorInfo, CURSORINFO,
            },
            Input::KeyboardAndMouse::{
                SendInput, INPUT
            },
        },
        Graphics::{
            Gdi::{
                ClientToScreen,
                GetDC, ReleaseDC, DeleteDC,
                GetPixel,
                HBITMAP,
                SelectObject, DeleteObject,
                CreateCompatibleDC,
                RedrawWindow, RDW_FRAME, RDW_INVALIDATE, RDW_ERASE, RDW_UPDATENOW, RDW_ALLCHILDREN
            },
            Dwm::{
                DWMWA_EXTENDED_FRAME_BOUNDS,
                DwmIsCompositionEnabled, DwmGetWindowAttribute,
            },
        }
    },
};

use std::{ffi::c_void, fmt};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use strum_macros::{EnumString, EnumProperty, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::ToPrimitive;
use once_cell::sync::Lazy;

#[cfg(feature="chkimg")]
use std::sync::OnceLock;

#[derive(Clone)]
pub struct WindowControl {
    next_id: Arc<Mutex<i32>>,
    windows: Arc<Mutex<HashMap<i32, HWND>>>
}

static WINDOW_CONTROL_SINGLETON: Lazy<WindowControl> = Lazy::new(||{
    WindowControl {
        next_id: Arc::new(Mutex::new(1)),
        windows: Arc::new(Mutex::new(HashMap::new()))
    }
});

pub fn get_next_id() -> i32 {
    let mut next_id = WINDOW_CONTROL_SINGLETON.next_id.lock().unwrap();
    let id = next_id.clone();
    *next_id += 1;

    id
}

pub fn set_new_window(id: i32, handle: HWND, to_zero: bool) {
    let mut list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    list.insert(id, handle);
    if to_zero {
        list.insert(0, handle);
    }
}

fn set_id_zero(hwnd: HWND) {
    let mut list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    list.insert(0, hwnd);
}

#[derive(PartialEq, Clone, Debug)]
pub struct Window {
    pub id: i32
}

impl Eq for Window {}
impl Hash for Window {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("getid", getid, get_desc!(getid));
    sets.add("idtohnd", idtohnd, get_desc!(idtohnd));
    sets.add("hndtoid", hndtoid, get_desc!(hndtoid));
    sets.add("clkitem", clkitem, get_desc!(clkitem));
    sets.add("ctrlwin", ctrlwin, get_desc!(ctrlwin));
    sets.add("status", status, get_desc!(status));
    sets.add("acw", acw, get_desc!(acw));
    sets.add("getallwin", getallwin, get_desc!(getallwin));
    sets.add("getctlhnd", getctlhnd, get_desc!(getctlhnd));
    sets.add("getitem", getitem, get_desc!(getitem));
    sets.add("posacc", posacc, get_desc!(posacc));
    sets.add("muscur", muscur, get_desc!(muscur));
    sets.add("peekcolor", peekcolor, get_desc!(peekcolor));
    sets.add("sckey", sckey, get_desc!(sckey));
    sets.add("setslider", setslider, get_desc!(setslider));
    sets.add("getslider", getslider, get_desc!(getslider));
    sets.add("chkbtn", chkbtn, get_desc!(chkbtn));
    sets.add("getstr", getstr, get_desc!(getstr));
    sets.add("sendstr", sendstr, get_desc!(sendstr));
    sets.add("getslctlst", getslctlst, get_desc!(getslctlst));
    sets.add("monitor", monitor, get_desc!(monitor));
    sets.add("mouseorg", mouseorg, get_desc!(mouseorg));
    sets.add("chkmorg", chkmorg, get_desc!(chkmorg));
    #[cfg(feature="chkimg")]
    sets.add("chkimg", chkimg, get_desc!(chkimg));
    #[cfg(feature="chkimg")]
    sets.add("saveimg", saveimg, get_desc!(saveimg));
    #[cfg(feature="chkimg")]
    sets.add("chkclr", chkclr, get_desc!(chkclr));
    sets
}

// GETID
#[allow(non_camel_case_types)]
#[derive(Debug, VariantNames, EnumString, EnumProperty)]
pub enum SpecialWindowId {
    #[strum(props(prefix="__", suffix="__", desc="アクティブウィンドウ"))]
    GET_ACTIVE_WIN,    // __GET_ACTIVE_WIN__
    #[strum(props(prefix="__", suffix="__", desc="マウス座標のウィンドウ"))]
    GET_FROMPOINT_WIN, // __GET_FROMPOINT_WIN__
    #[strum(props(prefix="__", suffix="__", desc="マウス座標の子ウィンドウ"))]
    GET_FROMPOINT_OBJ, // __GET_FROMPOINT_OBJ__
    #[strum(props(prefix="__", suffix="__"))]
    GET_THISUWSC_WIN,  // __GET_THISUWSC_WIN__
    #[strum(props(prefix="__", suffix="__", desc="プリントウィンドウ"))]
    GET_LOGPRINT_WIN,  // __GET_LOGPRINT_WIN__
    #[strum(props(prefix="__", suffix="__", desc="吹き出し"))]
    GET_BALLOON_WIN,   // __GET_BALLOON_WIN__
    #[strum(props(prefix="__", suffix="__", desc="吹き出し"))]
    GET_FUKIDASI_WIN,  // __GET_FUKIDASI_WIN__
    #[strum(props(prefix="__", suffix="__"))]
    GET_FORM_WIN,      // __GET_FORM_WIN__
    #[strum(props(prefix="__", suffix="__"))]
    GET_FORM_WIN2,     // __GET_FORM_WIN2__
    #[strum(props(prefix="__", suffix="__"))]
    GET_SCHEDULE_WIN,  // __GET_SCHEDULE_WIN__
    #[strum(props(prefix="__", suffix="__"))]
    GET_STOPFORM_WIN,  // __GET_STOPFORM_WIN__
    #[strum(props(prefix="__", suffix="__", desc="UWSCR実行中のコンソールウィンドウ"))]
    GET_CONSOLE_WIN    // __GET_CONSOLE_WIN__
}

#[builtin_func_desc(
    desc="ウィンドウのIDを得る",
    rtype={desc="ウィンドウID、見つからない場合-1",types="数値"}
    sets=[
        "タイトル-クラス名指定",
        [
            {n="タイトル",t="文字列",d="ウィンドウタイトル (部分一致)"},
            {o,n="クラス名",t="文字列",d="ウィンドウクラス名 (部分一致)"},
            {o,n="待ち時間",t="数値",d="タイムアウト秒、-1で無限待ち"},
        ],
        "定数指定",
        [
            {n="定数",t="定数",d=r#"以下のいずれかを指定
- GET_ACTIVE_WIN: アクティブウィンドウ
- GET_FROMPOINT_WIN: マウスカーソル下のウィンドウ
- GET_FROMPOINT_OBJ: マウスカーソル下の子ウィンドウ
- GET_LOGPRINT_WIN: Printウィンドウ
- GET_BALLOON_WIN, GET_FUKIDASI_WIN: 吹き出し
- GET_THISUWSC_WIN, GET_CONSOLE_WIN: UWSCRを実行しているコンソールウィンドウ
            "#},
        ],
    ],
)]
pub fn getid(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let title = args.get_as_string(0, None)?;
    let hwnd = match title.as_str() {
        "__GET_ACTIVE_WIN__" => unsafe {
            GetForegroundWindow()
        },
        "__GET_FROMPOINT_WIN__" => get_hwnd_from_mouse_point(true)?,
        "__GET_FROMPOINT_OBJ__" => get_hwnd_from_mouse_point(false)?,
        "__GET_CONSOLE_WIN__" |
        "__GET_THISUWSC_WIN__" => {
            get_console_hwnd()
        },
        "__GET_LOGPRINT_WIN__" => {
            match LOGPRINTWIN.get() {
                Some(m) => {
                    let guard = m.lock().unwrap();
                    let lp = guard.as_ref()
                        .map_err(|e| builtin_func_error(e.message.clone()))?;
                    lp.hwnd()
                },
                None => HWND::default(),
            }
        },
        "__GET_BALLOON_WIN__" => {
            let cell = THREAD_LOCAL_BALLOON.with(|b| b.clone());
            let hwnd = cell.borrow().as_ref().map(|b| b.hwnd());
            hwnd.unwrap_or_default()
        },
        "__GET_FORM_WIN__" => {
            HWND::default()
        },
        "__GET_FORM_WIN2__" => {
            HWND::default()
        },
        "__GET_SCHEDULE_WIN__" => {
            HWND::default()
        },
        "__GET_STOPFORM_WIN__" => {
            HWND::default()
        },
        _ => {
            let class_name = args.get_as_string(1, Some("".into()))?;
            let wait = args.get_as_num(2, Some(0.0))?;
            let _mdi_title = args.get_as_string(3, Some("".into()))?;
            find_window(title, class_name, wait)?
        },
    };
    if hwnd.0 > 0 {
        let id = get_id_from_hwnd(hwnd);
        // if id == -1.0 {
        //     let new_id = get_next_id();
        //     set_new_window(new_id, hwnd, false);
        //     id = new_id as f64;
        // }
        Ok(Object::Num(id))
    } else {
        Ok(Object::Num(-1.0))
    }
}

const MAX_NAME_SIZE: usize = 512;

#[derive(Debug)]
struct TargetWindow {
    hwnd: HWND,
    title: String,
    class_name: String,
    found: bool,
}

unsafe extern "system"
fn callback_find_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut title_buffer = [0; MAX_NAME_SIZE];
    let mut class_buffer = [0; MAX_NAME_SIZE];
    // let target = &mut *(lparam as *mut TargetWindow) as &mut TargetWindow;
    let target = &mut *(lparam.0 as *mut TargetWindow);

    let len = GetWindowTextW(hwnd, &mut title_buffer);
    let title = String::from_utf16_lossy(&title_buffer[..len as usize]);
    match title.to_ascii_lowercase().find(target.title.to_ascii_lowercase().as_str()) {
        Some(_) => {
            let len = GetClassNameW(hwnd, &mut class_buffer);
            let class = String::from_utf16_lossy(&class_buffer[..len as usize]);

            match class.to_ascii_lowercase().find(target.class_name.to_ascii_lowercase().as_str()) {
                Some(_) => {
                    target.title = title;
                    target.class_name = class;
                    target.hwnd = hwnd;
                    target.found = true;
                    return false.into();
                },
                None => ()
            }
        },
        None => ()
    }
    true.into() // 次のウィンドウへ
}

fn find_window(title: String, class_name: String, timeout: f64) -> windows::core::Result<HWND> {
    let mut target = TargetWindow {
        hwnd: HWND::default(),
        title,
        class_name,
        found: false,
    };
    let now = Instant::now();
    let limit = if timeout < 0.0 {
        // 0以下なら無限待ち
        None
    } else if timeout == 0.0 {
        // デフォルト値
        // 0.1～10秒まで状況や負荷により自動判断
        let auto_detected_timeout = 1.0;
        Some(Duration::from_secs_f64(auto_detected_timeout))
    } else {
        Some(Duration::from_secs_f64(timeout))
    };
    unsafe {
        let lparam = &mut target as *mut TargetWindow as isize;
        loop {
            let _ = EnumWindows(Some(callback_find_window), LPARAM(lparam));
            if target.found {
                let h = get_process_handle_from_hwnd(target.hwnd)?;
                WaitForInputIdle(h, 1000); // 入力可能になるまで最大1秒待つ
                let _ = CloseHandle(h);
                break;
            }
            if limit.is_some() && now.elapsed() >= limit.unwrap() {
                break;
            }
        }
        Ok(target.hwnd)
    }
}

fn get_hwnd_from_mouse_point(toplevel: bool) -> BuiltInResult<HWND> {
    unsafe {
        let point = window_low::get_current_pos()?;
        let mut hwnd = WindowFromPoint(point);
        if toplevel {
            loop {
                let parent = GetParent(hwnd);
                if parent.0 == 0 || ! IsWindowVisible(parent).as_bool(){
                    break;
                } else {
                    hwnd = parent;
                }
            }
        }
        Ok(hwnd)
    }
}

// IDTOHND
#[builtin_func_desc(
    desc="ウィンドウIDからHWNDを得る",
    rtype={desc="ウィンドウのHWND、該当ウィンドウがなければ0",types="数値"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
    ],
)]
pub fn idtohnd(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int::<i32>(0, None)?;
    if id < 0 {
        return Ok(Object::Num(0.0));
    }
    let h = get_hwnd_from_id(id);
    if h.0 > 0 {
        unsafe {
            if IsWindow(h).as_bool() {
                return Ok(Object::Num(h.0 as f64));
            }
        }
    }
    Ok(Object::Num(0.0))
}

pub fn get_hwnd_from_id(id: i32) -> HWND {
    let list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
    match list.get(&id) {
        Some(h) => *h,
        None => HWND::default()
    }
}

// HNDTOID
#[builtin_func_desc(
    desc="HWNDからウィンドウIDを得る",
    rtype={desc="ウィンドウID",types="数値"}
    args=[
        {n="HWND",t="数値",d="ウィンドウのHWND"},
    ],
)]
pub fn hndtoid(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let h = args.get_as_int::<isize>(0, None)?;
    let hwnd = HWND(h);
    let id = get_id_from_hwnd(hwnd);
    Ok(Object::Num(id))
}

pub fn get_id_from_hwnd(hwnd: HWND) -> f64 {
    let id = {
        let list = WINDOW_CONTROL_SINGLETON.windows.lock().unwrap();
        list.iter().find_map(
            |(key, &val)| if val == hwnd {
                Some(*key as f64)
            } else {
                None
            }
        )
    };
    // リストにない場合は新たなIDを発行する
    // hwndが無効なら-1
    match id {
        Some(id) => id,
        None => if unsafe { IsWindow(hwnd).as_bool() } {
            let new_id = get_next_id();
            set_new_window(new_id, hwnd, false);
            new_id as f64
        } else {
            -1.0
        }
    }
}

// ACW
#[builtin_func_desc(
    desc="ウィンドウの位置やサイズを変更",
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {o,n="X",t="数値",d="移動先X座標、省略時は現在のX座標"},
        {o,n="Y",t="数値",d="移動先Y座標、省略時は現在のY座標"},
        {o,n="高さ",t="数値",d="ウィンドウ高さ、省略時は現在の高さを維持"},
        {o,n="幅",t="数値",d="ウィンドウ幅、省略時は現在の幅を維持"},
        {o,n="待機秒",t="数値",d="ウィンドウに変更を加えるまでの待機時間をミリ秒で指定"},
    ],
)]
pub fn acw(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int::<i32>(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd.0 == 0 {
        return Ok(Object::Empty);
    }
    let x = args.get_as_int(1, None).ok();
    let y = args.get_as_int(2, None).ok();
    let w = args.get_as_int(3, None).ok();
    let h = args.get_as_int(4, None).ok();
    let ms= args.get_as_int(5, Some(0)).unwrap_or(0);
    thread::sleep(Duration::from_millis(ms));
    set_window_size(hwnd, x, y, w, h);
    set_id_zero(hwnd);
    Ok(Object::Empty)
}


// CLKITEM
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum ClkConst {
    #[strum[props(desc="ボタン、チェックボックス、ラジオ")]]
    CLK_BTN       = 1,
    #[strum[props(desc="リストボックス、コンボボックス")]]
    CLK_LIST      = 2,
    #[strum[props(desc="メニュー")]]
    CLK_MENU      = 4,
    #[strum[props(desc="タブ")]]
    CLK_TAB       = 8,
    #[strum(props(desc="ツリービュー", alias="CLK_TREEVEW"))]
    CLK_TREEVIEW  = 16,
    #[strum(props(desc="リストビュー", alias="CLK_LSTVEW"))]
    CLK_LISTVIEW  = 32,
    #[strum[props(desc="ツールバー")]]
    CLK_TOOLBAR   = 64,
    #[strum[props(desc="リンク")]]
    CLK_LINK      = 128,
    #[strum[props(desc="アイテム名部分一致")]]
    CLK_SHORT     = 256,
    #[strum[props(desc="バックグラウンド処理")]]
    CLK_BACK      = 512,
    #[strum(props(desc="クリック位置にマウスカーソル移動", alias="CLK_MUSMOVE"))]
    CLK_MOUSEMOVE = 1024,
    #[strum[props(desc="右クリック")]]
    CLK_RIGHTCLK  = 4096,
    #[strum[props(desc="左クリック")]]
    CLK_LEFTCLK   = 2048,
    #[strum[props(desc="ダブルクリック")]]
    CLK_DBLCLK    = 8192,
    #[strum[props(desc="ACC逆順サーチ")]]
    CLK_FROMLAST  = 65536,
    #[strum[props(desc="ACCによるクリック")]]
    CLK_ACC       = 32768,
    #[strum[props(desc="APIによるクリック")]]
    CLK_API       = 536870912,
    #[strum[props(desc="UIAによるクリック")]]
    CLK_UIA       = 1073741824,
    #[strum[props(desc="戻り値をコントロールのハンドルにする")]]
    CLK_HWND      = 262144,
}

#[builtin_func_desc(
    desc="ウィンドウのボタン等をクリック",
    rtype={desc="クリックの成否、または対象のHWND",types="真偽値または数値"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="アイテム名",t="文字列",d="クリック対象のタイトル"},
        {o,n="CLK定数",t="文字列",d=r#"以下の定数の組み合わせ(OR連結)により指定
- アイテム種別 (未指定時は全種別が対象となる)
    - CLK_BTN: ボタン、チェックボックス、ラジオボタン
    - CLK_LIST: リストボックス、コンボボックス
    - CLK_MENU: メニュー
    - CLK_TAB: タブ
    - CLK_TREEVEW, CLK_TREEVIEW: ツリービュー
    - CLK_LSTVEW, CLK_LISTVIEW: リストビュー、ヘッダ
    - CLK_TOOLBAR: ツールバー
    - CLK_LINK: リンク
- クリック方式
    - CLK_API: Win32 API (メッセージ送信)
    - CLK_ACC: アクセシビリティコントロール
    - CLK_UIA: UI Automation
- マウスボタン指定 (未指定時は方式毎のデフォルトクリック動作)
    - CLK_RIGHTCLK: 右クリック
    - CLK_LEFTCLK: 左クリック
    - CLK_DBLCLK: CLK_LEFTCLKと組み合わせてダブルクリック
- オプション
    - CLK_SHORT: アイテム名を部分一致とする
    - CLK_BACK: バックグラウンド処理
    - CLK_MUSMOVE, CLK_MOUSEMOVE: クリック位置にマウスカーソルを移動
    - CLK_FROMLAST: CLK_ACC時に逆順サーチ
    - CLK_HWND: クリック成功時にクリックしたコントロールのHWNDを返す"#},
        {o,n="チェック状態",t="真偽値および2",d="TRUE: チェックオン、FALSE: チェックオフ: 2: グレー状態"},
        {o,n="n番目",t="数値",d="同名アイテムが複数ある場合その順番"},
    ],
)]
pub fn clkitem(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    // let name = args.get_as_string(1, None)?;
    let names = args.get_as_string_array(1)?;
    let clk_const = args.get_as_int(2, Some(0_usize))?;
    let check = args.get_as_three_state(3, Some(ThreeState::True))?;
    let order = args.get_as_int(4, Some(1))?;
    let order = if order < 1 {1_u32} else {order as u32};

    let hwnd = get_hwnd_from_id(id);

    let name = if names.len() > 1 {
        names.join("\t")
    } else {
        names[0].to_string()
    };

    let ci = clkitem::ClkItem::new(name, clk_const, order);
    let result = ci.click(hwnd, check);

    Ok(result)
}

// CTRLWIN
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum CtrlWinCmd {
    #[strum[props(desc="ウィンドウを閉じる")]]
    CLOSE     = 2,
    #[strum[props(desc="ウィンドウを強制的に閉じる")]]
    CLOSE2    = 3,
    #[strum[props(desc="ウィンドウをアクティブにする")]]
    ACTIVATE  = 1,
    #[strum[props(desc="ウィンドウを隠す")]]
    HIDE      = 4,
    #[strum[props(desc="ウィンドウを表示する")]]
    SHOW      = 5,
    #[strum[props(desc="ウィンドウを最小化")]]
    MIN       = 6,
    #[strum[props(desc="ウィンドウを最大化")]]
    MAX       = 7,
    #[strum[props(desc="ウィンドウを通常サイズ表示")]]
    NORMAL    = 8,
    #[strum[props(desc="ウィンドウを最前面に固定")]]
    TOPMOST   = 9,
    #[strum[props(desc="ウィンドウの最前面固定を解除")]]
    NOTOPMOST = 10,
    #[strum[props(desc="ウィンドウを最前面に移動するがアクティブにはしない")]]
    TOPNOACTV = 11,
}

#[builtin_func_desc(
    desc="ウィンドウに命令を送信",
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="コマンド",t="定数",d=r#"命令を以下のいずれかから指定
- CLOSE: ウィンドウを閉じる
- CLOSE2: ウィンドウを強制的に閉じる
- ACTIVATE: ウィンドウをアクティブにする
- HIDE: ウィンドウを非表示にする
- SHOW: ウィンドウの非表示を解除する
- MIN: ウィンドウを最小化する
- MAX: ウィンドウを最大化する
- NORMAL: ウィンドウを通常サイズに戻す
- TOPMOST: ウィンドウを最前面に固定する
- NOTOPMOST: ウィンドウの最前面固定を解除
- TOPNOACTV: ウィンドウを最前面に移動するがアクティブにはしない"#},
    ],
)]
pub fn ctrlwin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    if hwnd.0 == 0 {
        return Ok(Object::Empty);
    }
    if let Some(cmd) = args.get_as_const(1, true)? {
        match cmd {
            CtrlWinCmd::CLOSE => unsafe {
                let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            CtrlWinCmd::CLOSE2 => unsafe {
                let _ = PostMessageW(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
            },
            CtrlWinCmd::ACTIVATE => unsafe {
                SetForegroundWindow(hwnd);
            },
            CtrlWinCmd::HIDE => unsafe {
                ShowWindow(hwnd, SW_HIDE);
            },
            CtrlWinCmd::SHOW => unsafe {
                ShowWindow(hwnd, SW_SHOW);
            },
            CtrlWinCmd::MIN => unsafe {
                ShowWindow(hwnd, SW_MINIMIZE);
            },
            CtrlWinCmd::MAX => unsafe {
                ShowWindow(hwnd, SW_MAXIMIZE);
            },
            CtrlWinCmd::NORMAL => unsafe {
                ShowWindow(hwnd, SW_SHOWNORMAL);
            },
            CtrlWinCmd::TOPMOST => unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE
                );
            },
            CtrlWinCmd::NOTOPMOST => unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE
                );
            },
            CtrlWinCmd::TOPNOACTV => unsafe {
                for h in vec![HWND_TOPMOST, HWND_NOTOPMOST] {
                    let _ = SetWindowPos(
                        hwnd,
                        h,
                        0, 0, 0, 0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
                    );
                }
            },
        }
    }
    set_id_zero(hwnd);
    Ok(Object::Empty)
}

// STATUS
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, PartialEq, Clone, Copy)]
pub enum StatusEnum {
    #[strum[props(desc="すべての状態を得る、他の定数と併用不可")]]
    ST_ALL       = 0,
    #[strum[props(desc="ウィンドウタイトルを取得")]]
    ST_TITLE     = 9,
    #[strum[props(desc="ウィンドウクラスを取得")]]
    ST_CLASS     = 14,
    #[strum[props(desc="ウィンドウのX座標")]]
    ST_X         = 1,
    #[strum[props(desc="ウィンドウのY座標")]]
    ST_Y         = 2,
    #[strum[props(desc="ウィンドウの幅")]]
    ST_WIDTH     = 3,
    #[strum[props(desc="ウィンドウの高さ")]]
    ST_HEIGHT    = 4,
    #[strum[props(desc="ウィンドウのクライアントX座標")]]
    ST_CLX       = 5,
    #[strum[props(desc="ウィンドウのクライアントY座標")]]
    ST_CLY       = 6,
    #[strum[props(desc="ウィンドウのクライアント領域幅")]]
    ST_CLWIDTH   = 7,
    #[strum[props(desc="ウィンドウのクライアント領域高さ")]]
    ST_CLHEIGHT  = 8,
    #[strum[props(desc="ウィンドウの親ウィンドウのID")]]
    ST_PARENT    = 16,
    #[strum[props(desc="ウィンドウが最小化しているか")]]
    ST_ICON      = 10,
    #[strum[props(desc="ウィンドウが最大化しているか")]]
    ST_MAXIMIZED = 11,
    #[strum[props(desc="ウィンドウが可視か")]]
    ST_VISIBLE   = 12,
    #[strum[props(desc="ウィンドウがアクティブか")]]
    ST_ACTIVE    = 13,
    #[strum[props(desc="ウィンドウがビジー状態か")]]
    ST_BUSY      = 15,
    #[strum[props(desc="ウィンドウが有効か")]]
    ST_ISID      = 21,
    #[strum[props(desc="ウィンドウプロセスが64ビットか")]]
    ST_WIN64     = 19,
    #[strum[props(desc="ウィンドウプロセスの実行ファイルパス")]]
    ST_PATH      = 17,
    #[strum[props(desc="ウィンドウプロセスのプロセスID")]]
    ST_PROCESS   = 18,
    #[strum[props(desc="ウィンドウが表示されているモニタ番号")]]
    ST_MONITOR   = 20,
    #[strum[props(desc="ウィンドウの見た目補正なしX座標")]]
    ST_WX        = 101,
    #[strum[props(desc="ウィンドウの見た目補正なしY座標")]]
    ST_WY        = 102,
    #[strum[props(desc="ウィンドウの見た目補正なし幅")]]
    ST_WWIDTH    = 103,
    #[strum[props(desc="ウィンドウの見た目補正なし高さ")]]
    ST_WHEIGHT   = 104,
}

struct WindowSize(i32, i32, i32, i32); // x, y, with, height
impl WindowSize {
    fn x(&self) -> i32 {
        self.0
    }
    fn y(&self) -> i32 {
        self.1
    }
    fn width(&self) -> i32 {
        self.2
    }
    fn height(&self) -> i32 {
        self.3
    }
}

fn get_window_size(h: HWND) -> WindowSize {
    unsafe {
        let mut rect = RECT::default();
        let pvattribute = &mut rect as *mut RECT as *mut c_void;
        let cbattribute = std::mem::size_of::<RECT>() as u32;
        if DwmIsCompositionEnabled().unwrap_or(BOOL(0)).as_bool() {
            // 見た目のRectを取る
            if DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_err() {
                // 失敗時はGetWindowRect
                let _ = GetWindowRect(h, &mut rect);
            }
        } else {
            // AEROがオフならGetWindowRect
            let _ = GetWindowRect(h, &mut rect);
        };
        WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
    }
}

fn get_window_rect(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        let _ = GetWindowRect(h, &mut rect);
    }
    WindowSize(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
}

pub fn set_window_size(hwnd: HWND, x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) {
    let default_rect = get_window_size(hwnd);

    let x = x.unwrap_or(default_rect.x());
    let y = y.unwrap_or(default_rect.y());
    let w = w.unwrap_or(default_rect.width());
    let h = h.unwrap_or(default_rect.height());
    unsafe {
        let _ = MoveWindow(hwnd, x, y, w, h, true);
        if DwmIsCompositionEnabled().unwrap_or(BOOL(0)).as_bool() {
            // 見た目のRectを取る
            let mut drect = RECT::default();
            let pvattribute = &mut drect as *mut RECT as *mut c_void;
            let cbattribute = std::mem::size_of::<RECT>() as u32;
            if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, pvattribute, cbattribute).is_ok() {
                // 実際のRectを取る
                let mut wrect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut wrect);

                // 見た目と実際の差分から最適な移動位置を得る
                let fix= |o, v| {
                    o - v
                };
                let new_x = fix(x, drect.left - wrect.left);
                let new_y = fix(y, drect.top - wrect.top);
                let new_w = fix(w, (drect.right - drect.left) - (wrect.right - wrect.left));
                let new_h = fix(h, (drect.bottom - drect.top) - (wrect.bottom - wrect.top));
                // 移動し直し
                let _ = MoveWindow(hwnd, new_x, new_y, new_w, new_h, true);
            }
        }
    }
}


fn get_client_size(h: HWND) -> WindowSize {
    let mut rect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    unsafe {
        let _ = GetClientRect(h, &mut rect);
        let mut point = POINT {x: rect.left, y: rect.top};
        ClientToScreen(h, &mut point);
        WindowSize(
            point.x,
            point.y,
            rect.right - rect.left,
            rect.bottom - rect.top
        )
    }
}

fn get_window_text(hwnd: HWND) -> BuiltInResult<Object> {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetWindowTextW(hwnd, &mut buffer);
        let s = String::from_utf16_lossy(&buffer[..len as usize]);
        Ok(Object::String(s))
    }
}

fn get_class_name(hwnd: HWND) -> BuiltInResult<Object> {
    unsafe {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetClassNameW(hwnd, &mut buffer);
        let name = String::from_utf16_lossy(&buffer[..len as usize]);
        Ok(Object::String(name))
    }
}

fn get_parent(hwnd: HWND) -> Object {
    unsafe {
        let parent = GetParent(hwnd);
        Object::Num(get_id_from_hwnd(parent))
    }
}

fn is_maximized(hwnd: HWND)-> Object {
    let mut wp = WINDOWPLACEMENT::default();
    unsafe {
        let _ = GetWindowPlacement(hwnd, &mut wp);
        Object::Bool(wp.showCmd == SW_MAXIMIZE.0 as u32)
    }
}

fn is_active_window(hwnd: HWND) -> Object {
    unsafe {
        Object::Bool(GetForegroundWindow() == hwnd)
    }
}

fn is_window(hwnd: HWND) -> bool {
    unsafe {
        IsWindow(hwnd).as_bool()
    }
}

fn get_process_id_from_hwnd(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

fn is_process_64bit(hwnd: HWND) -> BuiltInResult<Object> {
    if ! is_64bit_os().unwrap_or(true) {
        // 32bit OSなら必ずfalse
        return Ok(Object::Bool(false));
    }
    let h = get_process_handle_from_hwnd(hwnd)?;
    let mut is_wow64 = false.into();
    unsafe {
        let _ = IsWow64Process(h, &mut is_wow64);
    }
    let is_64 = ! is_wow64.as_bool();
    Ok(Object::Bool(is_64))
}

fn get_process_handle_from_hwnd(hwnd: HWND) -> windows::core::Result<HANDLE> {
    let pid = get_process_id_from_hwnd(hwnd);
    unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false, pid
        )
    }
}

fn get_process_path_from_hwnd(hwnd: HWND) -> BuiltInResult<Object> {
    let mut buffer = [0; MAX_PATH as usize];
    unsafe {
        let handle = get_process_handle_from_hwnd(hwnd)?;
        GetModuleFileNameExW(handle, HMODULE::default(), &mut buffer);
        let _ = CloseHandle(handle);
    }
    let path = String::from_utf16_lossy(&buffer);
    Ok(Object::String(path))
}

fn get_monitor_index_from_hwnd(hwnd: HWND) -> Object {
    match Monitor::from_hwnd(hwnd) {
        Some(m) => m.index().into(),
        None => Object::Empty,
    }
}


fn get_status_result(hwnd: HWND, stat: StatusEnum) -> BuiltInResult<Object> {
    let obj = match stat {
        StatusEnum::ST_TITLE => get_window_text(hwnd)?,
        StatusEnum::ST_CLASS => get_class_name(hwnd)?,
        StatusEnum::ST_X |
        StatusEnum::ST_Y |
        StatusEnum::ST_WIDTH |
        StatusEnum::ST_HEIGHT => {
            let wsize = get_window_size(hwnd);
            match stat {
                StatusEnum::ST_X => Object::Num(wsize.x() as f64),
                StatusEnum::ST_Y => Object::Num(wsize.y() as f64),
                StatusEnum::ST_WIDTH => Object::Num(wsize.width() as f64),
                StatusEnum::ST_HEIGHT => Object::Num(wsize.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_CLX |
        StatusEnum::ST_CLY |
        StatusEnum::ST_CLWIDTH |
        StatusEnum::ST_CLHEIGHT => {
            let csize = get_client_size(hwnd);
            match stat {
                StatusEnum::ST_CLX => Object::Num(csize.x() as f64),
                StatusEnum::ST_CLY => Object::Num(csize.y() as f64),
                StatusEnum::ST_CLWIDTH => Object::Num(csize.width() as f64),
                StatusEnum::ST_CLHEIGHT => Object::Num(csize.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_PARENT => get_parent(hwnd),
        StatusEnum::ST_ICON => unsafe {
            Object::Bool(IsIconic(hwnd).as_bool())
        },
        StatusEnum::ST_MAXIMIZED => is_maximized(hwnd),
        StatusEnum::ST_VISIBLE => unsafe {
            Object::Bool(IsWindowVisible(hwnd).as_bool())
        },
        StatusEnum::ST_ACTIVE => is_active_window(hwnd),
        StatusEnum::ST_BUSY => unsafe {
            Object::Bool(IsHungAppWindow(hwnd).as_bool())
        },
        StatusEnum::ST_ISID => unsafe {
            Object::Bool(IsWindow(hwnd).as_bool())
        },
        StatusEnum::ST_WIN64 => is_process_64bit(hwnd)?,
        StatusEnum::ST_PATH => get_process_path_from_hwnd(hwnd)?,
        StatusEnum::ST_PROCESS => Object::Num(get_process_id_from_hwnd(hwnd) as f64),
        StatusEnum::ST_MONITOR => get_monitor_index_from_hwnd(hwnd),
        StatusEnum::ST_WX |
        StatusEnum::ST_WY |
        StatusEnum::ST_WWIDTH |
        StatusEnum::ST_WHEIGHT => {
            let size = get_window_rect(hwnd);
            match stat {
                StatusEnum::ST_WX => Object::Num(size.x() as f64),
                StatusEnum::ST_WY => Object::Num(size.y() as f64),
                StatusEnum::ST_WWIDTH => Object::Num(size.width() as f64),
                StatusEnum::ST_WHEIGHT => Object::Num(size.height() as f64),
                _ => Object::Empty
            }
        },
        StatusEnum::ST_ALL => Object::Empty
    };
    Ok(obj)
}

fn get_all_status(hwnd: HWND) -> BuiltinFuncResult {
    let mut stats = HashTbl::new(true, false);
    stats.insert((StatusEnum::ST_TITLE as u8).to_string(), get_window_text(hwnd)?);
    stats.insert((StatusEnum::ST_CLASS as u8).to_string(), get_class_name(hwnd)?);
    let wsize = get_window_size(hwnd);
    stats.insert((StatusEnum::ST_X as u8).to_string(), Object::Num(wsize.x() as f64));
    stats.insert((StatusEnum::ST_Y as u8).to_string(), Object::Num(wsize.y() as f64));
    stats.insert((StatusEnum::ST_WIDTH as u8).to_string(), Object::Num(wsize.width() as f64));
    stats.insert((StatusEnum::ST_HEIGHT as u8).to_string(), Object::Num(wsize.height() as f64));
    let csize = get_client_size(hwnd);
    stats.insert((StatusEnum::ST_CLX as u8).to_string(), Object::Num(csize.x() as f64));
    stats.insert((StatusEnum::ST_CLY as u8).to_string(), Object::Num(csize.y() as f64));
    stats.insert((StatusEnum::ST_CLWIDTH as u8).to_string(), Object::Num(csize.width() as f64));
    stats.insert((StatusEnum::ST_CLHEIGHT as u8).to_string(), Object::Num(csize.height() as f64));
    stats.insert((StatusEnum::ST_PARENT as u8).to_string(), get_parent(hwnd));
    stats.insert((StatusEnum::ST_ICON as u8).to_string(), unsafe{ Object::Bool(IsIconic(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_MAXIMIZED as u8).to_string(), is_maximized(hwnd));
    stats.insert((StatusEnum::ST_VISIBLE as u8).to_string(), unsafe{ Object::Bool(IsWindowVisible(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_ACTIVE as u8).to_string(), is_active_window(hwnd));
    stats.insert((StatusEnum::ST_BUSY as u8).to_string(), unsafe{ Object::Bool(IsHungAppWindow(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_ISID as u8).to_string(), unsafe{ Object::Bool(IsWindow(hwnd).as_bool()) });
    stats.insert((StatusEnum::ST_WIN64 as u8).to_string(), is_process_64bit(hwnd)?);
    stats.insert((StatusEnum::ST_PATH as u8).to_string(), get_process_path_from_hwnd(hwnd)?);
    stats.insert((StatusEnum::ST_PROCESS as u8).to_string(), Object::Num(get_process_id_from_hwnd(hwnd) as f64));
    stats.insert((StatusEnum::ST_MONITOR as u8).to_string(), get_monitor_index_from_hwnd(hwnd));
    Ok(Object::HashTbl(Arc::new(Mutex::new(stats))))
}

#[builtin_func_desc(
    desc="ウィンドウの情報や状態を得る",
    rtype={desc="状態による、複数指定時はST定数をキーとした連想配列",types="文字列/数値/真偽値または連想配列"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {v=21,n="状態",t="定数",d=r#"以下から指定
- ST_TITLE: ウィンドウタイトル (文字列)
- ST_CLASS: ウィンドウクラス名 (文字列)
- ST_X: ウィンドウ左上のX座標 (数値)
- ST_Y: ウィンドウ左上のY座標 (数値)
- ST_WIDTH: ウィンドウの幅 (数値)
- ST_HEIGHT: ウィンドウの高さ (数値)
- ST_CLX: ウィンドウのクライアント領域左上のX座標 (数値)
- ST_CLY: ウィンドウのクライアント領域左上のY座標 (数値)
- ST_CLWIDTH: ウィンドウのクライアント領域の幅 (数値)
- ST_CLHEIGHT: ウィンドウのクライアント領域の高さ (数値)
- ST_PARENT: 親ウィンドウのID (数値)
- ST_ICON: 最小化してればTRUE (真偽値)
- ST_MAXIMIZED: 最大化してればTRUE (真偽値)
- ST_VISIBLE: ウィンドウが可視ならTRUE (真偽値)
- ST_ACTIVE: ウィンドウがアクティブならTRUE (真偽値)
- ST_BUSY: ウィンドウが応答なしならTRUE (真偽値)
- ST_ISID: ウィンドウが有効ならTRUE (真偽値)
- ST_WIN64: プロセスが64ビットかどうか (真偽値)
- ST_PATH: プロセスの実行ファイルのパス (文字列)
- ST_PROCESS: プロセスID (数値)
- ST_MONITOR: ウィンドウが表示されているモニタ番号 (数値)
- ST_ALL: 上記すべて"#},],
)]
pub fn status(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let hwnd = get_hwnd_from_id(
        args.get_as_int(0, None)?
    );
    if args.len() > 2 {
        let mut i = 1;
        // let mut stats = vec![Object::Empty; 22];
        let mut stats = HashTbl::new(true, false);
        while i < args.len() {
            if let Some(cmd) = args.get_as_const::<StatusEnum>(i, true)? {
                let value = get_status_result(hwnd, cmd)?;
                let name = (cmd as u8).to_string();
                stats.insert(name, value);
            }
            i += 1;
        }
        Ok(Object::HashTbl(Arc::new(Mutex::new(stats))))
    } else {
        if let Some(cmd) = args.get_as_const::<StatusEnum>(1, true)?{
            if cmd == StatusEnum::ST_ALL {
                Ok(get_all_status(hwnd)?)
            } else {
                let st = get_status_result(hwnd, cmd)?;
                Ok(st)
            }
        } else {
            Ok(Object::Empty)
        }
    }
}

// monitor
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum MonitorEnum {
    #[strum[props(desc="モニタX座標")]]
    MON_X           = 0,
    #[strum[props(desc="モニタY座標")]]
    MON_Y           = 1,
    #[strum[props(desc="モニタ幅")]]
    MON_WIDTH       = 2,
    #[strum[props(desc="モニタ高さ")]]
    MON_HEIGHT      = 3,
    #[strum(props(alias="MON_ISMAIN", desc="プライマリモニタかどうか"))]
    MON_PRIMARY     = 4,
    #[strum[props(desc="モニタ名")]]
    MON_NAME        = 5,
    #[strum[props(desc="作業エリアX座標")]]
    MON_WORK_X      = 10,
    #[strum[props(desc="作業エリアY座標")]]
    MON_WORK_Y      = 11,
    #[strum[props(desc="作業エリア幅")]]
    MON_WORK_WIDTH  = 12,
    #[strum[props(desc="作業エリア高さ")]]
    MON_WORK_HEIGHT = 13,
    #[strum[props(desc="モニタのDPI")]]
    MON_DPI         = 15,
    #[strum[props(desc="モニタのスケーリング倍率")]]
    MON_SCALING     = 16,
    #[strum[props(desc="すべての情報を得る、他の定数と併用不可")]]
    MON_ALL         = 20,
}
impl fmt::Display for MonitorEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}", ToPrimitive::to_f64(self).unwrap_or_default())
    }
}

#[builtin_func_desc(
    desc="モニタ情報を得る",
    rtype={desc="情報による",types="文字列/数値または連想配列"}
    args=[
        {n="モニタ番号",t="数値",d="モニタを示す番号 (0から)"},
        {n="情報種別",t="定数",d=r#"以下のいずれかを指定
- MON_X: モニタのX座標 (数値)
- MON_Y: モニタのY座標 (数値)
- MON_WIDTH: モニタの幅 (数値)
- MON_HEIGHT: モニタの高さ (数値)
- MON_PRIMARY, MON_ISMAIN: プライマリ(メイン)モニタかどうか (真偽値)
- MON_NAME: モニタ名 (文字列)
- MON_WORK_X: 作業エリアのX座標 (数値)
- MON_WORK_Y: 作業エリアのY座標 (数値)
- MON_WORK_WIDTH: 作業エリアの幅 (数値)
- MON_WORK_HEIGHT: 作業エリアの高さ (数値)
- MON_DPI: 画面のDPI (数値)
- MON_SCALING: スケーリング倍率 (数値)
- MON_ALL: 上記すべて (連想配列、キーはMON定数)"#},
    ],
)]
pub fn monitor(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if args.len() == 0 {
        let count = Monitor::get_count();
        Ok(count.into())
    } else {
        let index = args.get_as_int(0, None)?;
        let Some(monitor) = Monitor::from_index(index) else {
            return Ok(false.into())
        };
        let mon_enum = args.get_as_const::<MonitorEnum>(1, false)?
            .unwrap_or(MonitorEnum::MON_ALL);
        let obj = match mon_enum {
            MonitorEnum::MON_X => monitor.x().into(),
            MonitorEnum::MON_Y => monitor.y().into(),
            MonitorEnum::MON_WIDTH => monitor.width().into(),
            MonitorEnum::MON_HEIGHT => monitor.height().into(),
            MonitorEnum::MON_PRIMARY => monitor.is_primary().into(),
            MonitorEnum::MON_NAME => monitor.name().into(),
            MonitorEnum::MON_WORK_X => monitor.work_x().into(),
            MonitorEnum::MON_WORK_Y => monitor.work_y().into(),
            MonitorEnum::MON_WORK_WIDTH => monitor.work_width().into(),
            MonitorEnum::MON_WORK_HEIGHT => monitor.work_height().into(),
            MonitorEnum::MON_DPI => monitor.dpi().into(),
            MonitorEnum::MON_SCALING => monitor.scaling().into(),
            MonitorEnum::MON_ALL => {
                let mut map = HashTbl::new(false, false);
                map.insert(MonitorEnum::MON_X.to_string(), monitor.x().into());
                map.insert(MonitorEnum::MON_Y.to_string(), monitor.y().into());
                map.insert(MonitorEnum::MON_WIDTH.to_string(), monitor.width().into());
                map.insert(MonitorEnum::MON_HEIGHT.to_string(), monitor.height().into());
                map.insert(MonitorEnum::MON_PRIMARY.to_string(), monitor.is_primary().into());
                map.insert(MonitorEnum::MON_NAME.to_string(), monitor.name().into());
                map.insert(MonitorEnum::MON_WORK_X.to_string(), monitor.work_x().into());
                map.insert(MonitorEnum::MON_WORK_Y.to_string(), monitor.work_y().into());
                map.insert(MonitorEnum::MON_WORK_WIDTH.to_string(), monitor.work_width().into());
                map.insert(MonitorEnum::MON_WORK_HEIGHT.to_string(), monitor.work_height().into());
                map.insert(MonitorEnum::MON_DPI.to_string(), monitor.dpi().into());
                map.insert(MonitorEnum::MON_SCALING.to_string(), monitor.scaling().into());
                Object::HashTbl(Arc::new(Mutex::new(map)))
            },
        };
        Ok(obj)
    }
}

#[cfg(feature="chkimg")]
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum ChkImgOption {
    #[strum[props(desc="グレースケール化せず探索を行う")]]
    CHKIMG_NO_GRAY = 1,
    #[strum[props(desc="GraphicCaptureAPIでキャプチャする")]]
    CHKIMG_USE_WGCAPI = 2,
}

#[cfg(feature="chkimg")]
impl ChkImgOption {
    fn gray_scale(opt: i32) -> bool {
        1 & opt != 1
    }
    fn use_wgcapi(opt: i32) -> bool {
        2 & opt == 2
    }
}

#[cfg(feature="chkimg")]
static SAVE_SS: OnceLock<bool> = OnceLock::new();
#[cfg(feature="chkimg")]
fn should_save_ss() -> bool {
    let b = SAVE_SS.get_or_init(|| {
        let settings = USETTINGS.lock().unwrap();
        settings.chkimg.save_ss
    });
    *b
}

#[cfg(feature="chkimg")]
#[builtin_func_desc(
    desc="スクリーン上の画像の位置を返す",
    rtype={desc="画像位置情報 [X,Y,スコア] の配列",types="配列"}
    args=[
        {n="画像",t="文字列",d="画像ファイルのパス"},
        {o,n="スコア",t="数値",d="一致率を0-100で指定、100なら完全一致 (デフォルト95)"},
        {o,n="最大検索数",t="数値",d="指定した数の座標が見つかり次第探索を打ち切る、指定数に満たない場合全体を探索"},
        {o,n="left",t="数値",d="探索範囲の左上X座標、省略時はスクリーンまたはウィンドウ左上X座標"},
        {o,n="top",t="数値",d="探索範囲の左上Y座標、省略時はスクリーンまたはウィンドウ左上Y座標"},
        {o,n="right",t="数値",d="探索範囲の右下X座標、省略時はスクリーンまたはウィンドウ右下X座標"},
        {o,n="bottom",t="数値",d="探索範囲の右下Y座標、省略時はスクリーンまたはウィンドウ右下Y座標"},
        {o,n="オプション",t="定数",d=r#"探索オプションを以下から指定、OR連結可
- CHKIMG_NO_GRAY: 画像をグレースケール化せず探索を行う
- CHKIMG_USE_WGCAPI: デスクトップまたはウィンドウの画像取得にGraphicsCaptureAPIを使う"#},
        {o,n="モニタ番号",t="数値",d="CHKIMG_USE_WGCAPI指定時かつmouseorg未使用時に探索するモニタ番号を0から指定"},
    ],
)]
pub fn chkimg(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let default_score = 95;
    let path = args.get_as_string(0, None)?;
    let score = args.get_as_int::<i32>(1, Some(default_score))?;
    if score < 1 && score > 100 {
        return Err(builtin_func_error(UErrorMessage::GivenNumberIsOutOfRange(1.0, 100.0)));
    }
    let score = score as f64 / 100.0;
    let count = args.get_as_int::<u8>(2, Some(5))?;
    let (left, top, right, bottom, opt, monitor) = match args.get_as_int_or_array_or_empty(3)? {
        Some(two) => match two {
            TwoTypeArg::T(n) => {
                let left = Some(n as i32);
                let top = args.get_as_int_or_empty(4)?;
                let right = args.get_as_int_or_empty(5)?;
                let bottom = args.get_as_int_or_empty(6)?;
                let opt = args.get_as_int(7, Some(0))?;
                let monitor = args.get_as_int(8, Some(0))?;
                (left, top, right, bottom, opt, monitor)
            },
            TwoTypeArg::U(arr) => {
                let left = arr.get(0).map(|o| o.as_f64(false)).flatten().map(|n| n as i32);
                let top = arr.get(1).map(|o| o.as_f64(false)).flatten().map(|n| n as i32);
                let right = arr.get(2).map(|o| o.as_f64(false)).flatten().map(|n| n as i32);
                let bottom = arr.get(3).map(|o| o.as_f64(false)).flatten().map(|n| n as i32);
                let opt = args.get_as_int(4, Some(0))?;
                let monitor = args.get_as_int(5, Some(0))?;
                (left, top, right, bottom, opt, monitor)
            },
        },
        None => {
            let left = None;
            let top = args.get_as_int_or_empty(4)?;
            let right = args.get_as_int_or_empty(5)?;
            let bottom = args.get_as_int_or_empty(6)?;
            let opt = args.get_as_int(7, Some(0))?;
            let monitor = args.get_as_int(8, Some(0))?;
            (left, top, right, bottom, opt, monitor)
        },
    };


    let mi = MorgImg::from(&evaluator.mouseorg);
    let ss = match mi.hwnd {
        Some(hwnd) => {
            let client = mi.is_client();
            let style = if mi.is_back {
                ImgConst::IMG_BACK
            } else {
                ImgConst::IMG_FORE
            };
            if ChkImgOption::use_wgcapi(opt) {
                ScreenShot::get_window_wgcapi(hwnd, left, top, right, bottom, client)?
            } else {
                ScreenShot::get_window(hwnd, left, top, right, bottom, client, style)?
            }
        },
        None => {
            if ChkImgOption::use_wgcapi(opt) {
                ScreenShot::get_screen_wgcapi(monitor, left, top, right, bottom)?
            } else {
                ScreenShot::get_screen(left, top, right, bottom)?
            }
        },
    };


    if should_save_ss() {
        ss.save(None)?;
    }
    let chk = ChkImg::from_screenshot(ss, ChkImgOption::gray_scale(opt))?;
    let result = chk.search(&path, score, Some(count))?;
    let arr = result
        .into_iter()
        .map(|m| {
            // let (x, y) = mi.fix_point(m.x, m.y);
            let vec = vec![
                Object::Num(m.x as f64),
                Object::Num(m.y as f64),
                Object::Num(m.score * 100.0)
            ];
            Object::Array(vec)
        })
        .collect::<Vec<_>>();
    Ok(Object::Array(arr))
}

unsafe extern "system"
fn callback_getallwin(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = lparam.0 as *mut HwndList;
    match hwnd {
        HWND(0) => false.into(),
        h => {
            (*list).0.push(h);
            true.into()
        },
    }
}

#[derive(Debug)]
struct HwndList(Vec<HWND>);

#[builtin_func_desc(
    desc="すべてのウィンドウ、またはウィンドウの子ウィンドウのIDを得る",
    rtype={desc="ID配列",types="配列"}
    args=[
        {o,n="ウィンドウID",t="数値",d="子ウィンドウを得たいウィンドウ"},
    ],
)]
pub fn getallwin(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let target = match args.get_as_int_or_empty::<i32>(0)? {
        Some(id) => match get_hwnd_from_id(id) {
            HWND(0) => return Ok(Object::Array(vec![])),
            h => Some(h)
        },
        None => None,
    };
    let id_list = unsafe {
        let mut list = HwndList(vec![]);
        let lparam = LPARAM(&mut list as *mut HwndList as isize);
        match target {
            Some(h) => {
                EnumChildWindows(h, Some(callback_getallwin), lparam);
            },
            None => {
                let _ = EnumWindows(Some(callback_getallwin), lparam);
            },
        };

        list.0.into_iter()
            .map(|h| {
                let id = get_id_from_hwnd(h);
                Object::Num(id)
            })
            .collect()
    };
    Ok(Object::Array(id_list))
}

#[allow(non_camel_case_types)]
#[derive(Debug, VariantNames, EnumString, EnumProperty)]
pub enum GetHndConst {
    #[strum(props(prefix="__", suffix="__", desc="メニューハンドルを取得"))]
    GET_MENU_HND,   // __GET_MENU_HND__
    #[strum(props(prefix="__", suffix="__", desc="システムメニューハンドルを取得"))]
    GET_SYSMENU_HND // __GET_SYSMENU_HND__
}

unsafe extern "system"
fn callback_getctlhnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctlhnd = &mut *(lparam.0 as *mut CtlHnd);
    let pat = ctlhnd.target.to_ascii_lowercase();

    let mut buffer = [0; MAX_NAME_SIZE];
    let len = GetWindowTextW(hwnd, &mut buffer);
    let title = String::from_utf16_lossy(&buffer[..len as usize]);
    if let Some(_) = title.to_ascii_lowercase().find(&pat) {
        ctlhnd.order -= 1;
        if ctlhnd.order == 0 {
            ctlhnd.hwnd = hwnd;
            return false.into()
        }
    } else {
        let mut buffer = [0; MAX_NAME_SIZE];
        let len = GetClassNameW(hwnd, &mut buffer);
        let name = String::from_utf16_lossy(&buffer[..len as usize]);
        if let Some(_) = name.to_ascii_lowercase().find(&pat) {
            ctlhnd.order-= 1;
            if ctlhnd.order == 0 {
                ctlhnd.hwnd = hwnd;
                return false.into()
            }
        }
    }
    true.into()
}

struct CtlHnd{target: String, hwnd: HWND, order: u32}

#[builtin_func_desc(
    desc="子ウィンドウのHWNDまたはメニューハンドルを得る",
    rtype={desc="ハンドル値",types="数値"}
    sets=[
        "名前指定",
        [
            {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
            {n="アイテム名",t="文字列",d="子ウィンドウのタイトルまたはクラス名 (部分一致)"},
            {o,n="n番目",t="数値",d="該当子ウィンドウが複数ある場合その順番"},
        ],
        "定数指定",
        [
            {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
            {n="メニュー種別",t="定数",d=r#"ハンドルを得たいメニューを以下のいずれかで指定
- GET_MENU_HND: メニューハンドル
- GET_SYSMENU_HND: システムメニューハンドル"#},
        ],
    ],
)]
pub fn getctlhnd(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let parent = get_hwnd_from_id(id);
    let target = args.get_as_string(1, None)?;

    let hwnd = match target.to_ascii_uppercase().as_str() {
        "__GET_MENU_HND__" => unsafe {
            let menu = GetMenu(parent);
            menu.0 as f64
        },
        "__GET_SYSMENU_HND__" => unsafe {
            let menu = GetSystemMenu(parent, false);
            menu.0 as f64
        },
        _ => {
            let n = args.get_as_int(2, Some(1))?;
            let order = if n < 1 {1_u32} else {n as u32};

            let mut ctlhnd = CtlHnd {target, hwnd: HWND::default(), order};
            let lparam = LPARAM(&mut ctlhnd as *mut CtlHnd as isize);
            unsafe {
                EnumChildWindows(parent, Some(callback_getctlhnd), lparam);
            }
            ctlhnd.hwnd.0 as f64
        }
    };
    Ok(hwnd.into())
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum GetItemConst {
    #[strum[props(desc="ボタン、チェックボックス、ラジオ")]]
    ITM_BTN       = 1,
    #[strum[props(desc="リストボックス、コンボボックス")]]
    ITM_LIST      = 2,
    #[strum[props(desc="タブ")]]
    ITM_TAB       = 8,
    #[strum[props(desc="メニュー")]]
    ITM_MENU      = 4,
    #[strum(props(desc="ツリービュー", alias="ITM_TREEVEW"))]
    ITM_TREEVIEW  = 16,
    #[strum(props(desc="リストビュー", alias="ITM_LSTVEW"))]
    ITM_LISTVIEW  = 32,
    #[strum[props(desc="メニュー")]]
    ITM_EDIT      = 131072,
    #[strum[props(desc="スタティックコントロール")]]
    ITM_STATIC    = 262144,
    #[strum[props(desc="ステータスバー")]]
    ITM_STATUSBAR = 524288,
    #[strum[props(desc="ツールバー")]]
    ITM_TOOLBAR   = 64,
    #[strum[props(desc="リンク")]]
    ITM_LINK      = 128,
    #[strum[props(desc="ACCクリック可能なアイテム")]]
    ITM_ACCCLK    = 4194304,
    #[strum[props(desc="ACCクリック可能または選択可能テキスト")]]
    ITM_ACCCLK2   = 272629760,
    #[strum[props(desc="ACCスタティックテキスト")]]
    ITM_ACCTXT    = 8388608,
    #[strum[props(desc="ACCエディット可能テキスト")]]
    ITM_ACCEDIT   = 16777216,
    #[strum[props(desc="ACCで逆順検索")]]
    ITM_FROMLAST  = 65536,
    // ITM_BACK      = 512,
}
impl Into<u32> for GetItemConst {
    fn into(self) -> u32 {
        ToPrimitive::to_u32(&self).unwrap_or(0)
    }
}

#[builtin_func_desc(
    desc="該当アイテム名の一覧を得る",
    rtype={desc="アイテム名一覧",types="配列"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="種別",t="定数",d=r#"アイテム種別を以下から指定、OR連結可
- ITM_BTN: ボタン、チェックボックス、ラジオボタン
- ITM_LIST: リストボックス、コンボボックス
- ITM_TAB: タブコントロール
- ITM_MENU: メニュー
- ITM_TREEVIEW (ITM_TREEVEW): ツリービュー
- ITM_LISTVIEW (ITM_LSTVEW): リストビュー
- ITM_EDIT: エディットボックス
- ITM_STATIC: スタティックコントロール
- ITM_STATUSBAR: ステータスバー
- ITM_TOOLBAR: ツールバー
- ITM_LINK: リンク
- ITM_ACCCLK: ACCによりクリック可能なもの
- ITM_ACCCLK2: ACCによりクリック可能なものおよび選択可能テキスト
- ITM_ACCTXT: ACCスタティックテキスト
- ITM_ACCEDIT: ACCエディット可能テキスト
- ITM_FROMLAST: ACCで検索順序を逆にする (最後のアイテムから取得)"#},
        {o,n="n番目",t="数値",d="リスト、リストビュー、ツリービューが複数ある場合その順番、-1ならすべて取得"},
        {o,n="列",t="数値",d="取得するリストビューの列を指定、0なら全て、-1ならカラム名"},
        {o,n="無効無視",t="真偽値",d="TRUEならディセーブル状態のコントロールは取得しない"},
        {o,n="ACC最大数",t="数値",d="ACC系で取得数の上限を指定、0なら全て、-nなら逆順でn個取得"},
    ],
)]
pub fn getitem(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    let target = args.get_as_int(1, None)?;
    let nth = args.get_as_int(2, Some(1))?;
    let column = args.get_as_int(3, Some(1))?;
    let ignore_disabled = args.get_as_bool(4, Some(false))?;
    let acc_max = args.get_as_int(5, Some(0))?;

    // api
    let mut items = win32::Win32::getitem(hwnd, target, nth, column, ignore_disabled);
    // acc
    let acc_items = acc::Acc::getitem(hwnd, target, acc_max);

    items.extend(acc_items);
    let arr = items.into_iter().map(|s| s.into()).collect();
    Ok(Object::Array(arr))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum AccConst {
    #[strum[props(desc="表示文字列")]]
    ACC_ACC         = 1,
    #[strum[props(desc="DrawTextやTextOutで描画されたテキスト")]]
    ACC_API         = 2,
    #[strum[props(desc="ACCオブジェクト名")]]
    ACC_NAME        = 3,
    #[strum[props(desc="ACCオブジェクトの値")]]
    ACC_VALUE       = 4,
    #[strum[props(desc="ACCオブジェクトの役割名")]]
    ACC_ROLE        = 5,
    #[strum[props(desc="ACCオブジェクトの状態")]]
    ACC_STATE       = 6,
    #[strum[props(desc="ACCオブジェクトの説明")]]
    ACC_DESCRIPTION = 7,
    #[strum[props(desc="ACCオブジェクト表示位置")]]
    ACC_LOCATION    = 8,
    #[strum[props(desc="対象ウィンドウをアクティブにしない")]]
    ACC_BACK        = 512,
}

#[builtin_func_desc(
    desc="指定座標のアクセシビリティオブジェクトから情報を得る",
    rtype={desc="種別による",types="文字列または配列"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="CX",t="数値",d="クライアントX座標"},
        {n="CY",t="数値",d="クライアントY座標"},
        {o,n="種別",t="定数",d=r#"以下のいずれかを選択、ACC_BACKをOR連結することで対象をアクティブにしない
- 0: ACC_ACC を実行し、取得できなければ ACC_API を実行 (デフォルト)
- ACC_ACC: 表示文字列の取得 (文字列)
- ACC_API: DrawText, TextOut等のAPIで描画されたテキストを取得 (未実装)
- ACC_NAME: オブジェクトの表示名 (文字列)
- ACC_VALUE: エディットボックス等の値 (文字列)
- ACC_ROLE: オブジェクトの役割名 (文字列)
- ACC_STATE: オブジェクトの状態 (配列)
- ACC_DESCRIPTION: オブジェクトの説明 (文字列)
- ACC_LOCATION: オブジェクトの位置情報 ([x, y, 幅, 高さ])
        "#},
    ],
)]
pub fn posacc(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let hwnd = get_hwnd_from_id(id);
    let clx = args.get_as_int(1, None::<i32>)?;
    let cly = args.get_as_int(2, None::<i32>)?;
    let mode = args.get_as_int(3, Some(0_u16))?;
    let mode = if (mode & AccConst::ACC_BACK as u16) > 0 {
        // ACC_BACKを除去
        mode - AccConst::ACC_BACK as u16
    } else {
        // ACC_BACKがないので対象ウィンドウをアクティブにする
        unsafe { SetForegroundWindow(hwnd); }
        mode
    };
    let obj = match acc::Acc::from_point(hwnd, clx, cly) {
        Some(acc) => match mode {
            0 => {
                match acc.get_name().map(|name|name.into()) {
                    Some(obj) => obj,
                    None => acc.get_api_text().map(|api| api.into()).unwrap_or_default(),
                }
            }
            1 | 3 => {
                acc.get_name().map(|name|name.into()).unwrap_or_default()
            },
            2 => {
                acc.get_api_text().map(|api| api.into()).unwrap_or_default()
            },
            4 => {
                acc.get_value().map(|val| val.into()).unwrap_or_default()
            },
            5 => {
                acc.get_role_text().map(|role| role.into()).unwrap_or_default()
            },
            6 => {
                let vec2obj = |vec: Vec<String>| {
                    let arr = vec.into_iter()
                        .map(|text| text.into())
                        .collect();
                    Object::Array(arr)
                };
                acc.get_state_texts().map(vec2obj).unwrap_or_default()
            },
            7 => {
                acc.get_description().map(|desc| desc.into()).unwrap_or_default()
            },
            8 => {
                let vec2obj = |vec: Vec<i32>| {
                    let arr = vec.into_iter()
                        .map(|n| n.into())
                        .collect();
                    Object::Array(arr)
                };
                acc.get_location(hwnd).map(vec2obj).unwrap_or_default()
            },
            _ => Object::Empty
        },
        None => Object::Empty,
    };
    Ok(obj)
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum CurConst {
    #[strum[props(desc="砂時計付き矢印")]]
    CUR_APPSTARTING = 1,
    #[strum[props(desc="標準矢印")]]
    CUR_ARROW       = 2,
    #[strum[props(desc="十字")]]
    CUR_CROSS       = 3,
    #[strum[props(desc="手のひら")]]
    CUR_HAND        = 4,
    #[strum[props(desc="？マーク付き矢印")]]
    CUR_HELP        = 5,
    #[strum[props(desc="Iビーム")]]
    CUR_IBEAM       = 6,
    #[strum[props(desc="禁止")]]
    CUR_NO          = 8,
    #[strum[props(desc="4方向矢印")]]
    CUR_SIZEALL     = 10,
    #[strum[props(desc="両方向矢印斜め左下がり")]]
    CUR_SIZENESW    = 11,
    #[strum[props(desc="両方向矢印上下")]]
    CUR_SIZENS      = 12,
    #[strum[props(desc="両方向矢印斜め右下がり")]]
    CUR_SIZENWSE    = 13,
    #[strum[props(desc="両方向矢印左右")]]
    CUR_SIZEWE      = 14,
    #[strum[props(desc="垂直矢印")]]
    CUR_UPARROW     = 15,
    #[strum[props(desc="砂時計")]]
    CUR_WAIT        = 16,
}

#[builtin_func_desc(
    desc="マウスカーソル種別を得る",
    rtype={desc=r#"以下のいずれかを返す
- CUR_APPSTARTING (1): 砂時計付き矢印
- CUR_ARROW (2): 標準矢印
- CUR_CROSS (3): 十字
- CUR_HAND (4): ハンド
- CUR_HELP (5): クエスチョンマーク付き矢印
- CUR_IBEAM (6): アイビーム (テキスト上のカーソル)
- CUR_NO (8): 禁止
- CUR_SIZEALL (10): ４方向矢印
- CUR_SIZENESW (11): 斜め左下がりの両方向矢印
- CUR_SIZENS (12): 上下両方向矢印
- CUR_SIZENWSE (13): 斜め右下がりの両方向矢印
- CUR_SIZEWE (14): 左右両方向矢印
- CUR_UPARROW (15): 垂直の矢印
- CUR_WAIT (16): 砂時計
- 0: 上記以外"#,types="定数"}
)]
pub fn muscur(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = unsafe {
        let mut pci = CURSORINFO::default();
        pci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        let _ = GetCursorInfo(&mut pci);
        pci.hCursor.0
    };
    let cursor = match id {
        65563 => CurConst::CUR_APPSTARTING,
        65541 => CurConst::CUR_ARROW,
        65547 => CurConst::CUR_CROSS,
        65569 => CurConst::CUR_HAND,
        65565 => CurConst::CUR_HELP,
        65543 => CurConst::CUR_IBEAM,
        65561 => CurConst::CUR_NO,
        65559 => CurConst::CUR_SIZEALL,
        65553 => CurConst::CUR_SIZENESW,
        65557 => CurConst::CUR_SIZENS,
        65551 => CurConst::CUR_SIZENWSE,
        65555 => CurConst::CUR_SIZEWE,
        65549 => CurConst::CUR_UPARROW,
        65545 => CurConst::CUR_WAIT,
        _ => return Ok(Object::Num(0.0)),
    };
    let n = cursor as i32 as f64;
    Ok(Object::Num(n))
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum ColConst {
    #[default]
    #[strum[props(desc="BGR値を返す")]]
    COL_BGR = 0,
    #[strum[props(desc="RGB値を返す")]]
    COL_RGB = 3,
    #[strum[props(desc="R値のみを返す")]]
    COL_R   = 4,
    #[strum[props(desc="G値のみを返す")]]
    COL_G   = 5,
    #[strum[props(desc="B値のみを返す")]]
    COL_B   = 6,
}

#[builtin_func_desc(
    desc="指定座標の色を返す",
    rtype={desc="色を示す数値、失敗時-1",types="数値"}
    args=[
        {n="X",t="数値",d="X座標"},
        {n="Y",t="数値",d="Y座標"},
        {o,n="取得値",t="定数",d=r#"戻り値を指定
- COL_BGR: BGR値
- COL_RGB: RGB値
- COL_R: R値
- COL_G: G値
- COL_B: B値"#},
        {o,n="クリップボード",t="真偽値",d="TRUEなら画面ではなくクリップボード画像から取得"},
    ],
)]
pub fn peekcolor(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let x = args.get_as_int(0, None::<i32>)?;
    let y = args.get_as_int(1, None::<i32>)?;
    let colconst = args.get_as_const::<ColConst>(2, false)?.unwrap_or_default();
    let clipboard = args.get_as_bool(3, Some(false))?;
    unsafe {
        let bgr = if clipboard {
            if IsClipboardFormatAvailable(CF_BITMAP.0 as u32).is_ok() && OpenClipboard(HWND(0)).is_ok() {
                let h = GetClipboardData(CF_BITMAP.0 as u32)?;
                let hbitmap = HBITMAP(h.0);
                let hdc = CreateCompatibleDC(None);
                let old = SelectObject(hdc, hbitmap);
                let colorref = GetPixel(hdc, x, y);
                SelectObject(hdc, old);
                let _ = CloseHandle(h);
                DeleteObject(hbitmap);
                DeleteDC(hdc);
                let _ = CloseClipboard();
                colorref.0
            } else {
                0xFFFFFFFF
            }
        } else {
            let mi = MorgImg::from(&evaluator.mouseorg);
            let hwnd = if mi.is_back {
                mi.hwnd.as_ref()
            } else {
                None
            };
            let (x, y) = mi.fix_point(x, y);
            mi.redraw_window();
            let hdc = GetDC(hwnd);
            let colorref = GetPixel(hdc, x, y);
            ReleaseDC(mi.hwnd.as_ref(), hdc);
            colorref.0
        };
        if bgr > 0xFFFFFF {
            Ok(Object::Num(-1.0))
        } else {
            let r = |c: u32| c & 0xFF;
            let g = |c: u32| (c >> 8) & 0xFF;
            let b = |c: u32| (c >> 16) & 0xFF;
            let color = match colconst {
                ColConst::COL_BGR => bgr,
                ColConst::COL_RGB => {
                    r(bgr) << 16 |
                    g(bgr) << 8 |
                    b(bgr)
                },
                ColConst::COL_R => r(bgr),
                ColConst::COL_G => g(bgr),
                ColConst::COL_B => b(bgr),
            };
            Ok(Object::Num(color as f64))
        }
    }
}

#[builtin_func_desc(
    desc="ウィンドウにショートカットキーを送信",
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {v=35,n="キー1-35",t="定数",d="VK定数、指定順に入力"},
    ],
)]
pub fn sckey(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None::<i32>)?;
    let hwnd = get_hwnd_from_id(id);
    let keys = args.get_sckey_codes(1)?;
    let pinputs: Vec<INPUT> = SCKeyCode::codes_to_input(keys);
    unsafe {
        if hwnd.0 != 0 {
            SetForegroundWindow(hwnd);
        }
        SendInput(&pinputs, std::mem::size_of::<INPUT>() as i32);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(Object::default())
}

struct Slider {
    slider: win32::Slider,
}

impl Slider {
    fn new(hwnd: HWND, nth: u32) -> Option<Self> {
        win32::Win32::get_slider(hwnd, nth)
            .map(|slider| Self { slider })
    }
    fn get(&self, param: SldConst) -> i32 {

        match param {
            SldConst::SLD_POS => self.slider.get_pos(),
            SldConst::SLD_MIN => self.slider.get_min(),
            SldConst::SLD_MAX => self.slider.get_max(),
            SldConst::SLD_PAGE => self.slider.get_page(),
            SldConst::SLD_BAR => self.slider.get_bar(),
            SldConst::SLD_X => self.slider.get_point().0,
            SldConst::SLD_Y => self.slider.get_point().1,
        }
    }
    fn set(&self, pos: i32, smooth: bool) -> bool {
        self.slider.set_pos(pos, smooth)
    }
}

#[builtin_func_desc(
    desc="スライダーを動かす",
    rtype={desc="成功時TRUE",types="真偽値"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="スライダー位置",t="数値",d="スライダーの移動先"},
        {o,n="n番目",t="数値",d="スライダーが複数存在する場合その順番"},
        {o,n="スクロール",t="真偽値",d="TRUEならスクロールバーを徐々に動かす"},
    ],
)]
pub fn setslider(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    let value = args.get_as_int(1, None)?;
    let nth = args.get_as_int(2, Some(1))?;
    let smooth = args.get_as_bool(3, Some(true))?;

    let result = if let Some(slider) = Slider::new(hwnd, nth) {
        slider.set(value, smooth)
    } else {
        false
    };
    Ok(result.into())

}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum SldConst {
    #[default]
    #[strum[props(desc="スライダー現在値")]]
    SLD_POS  = 0,
    #[strum[props(desc="スライダー最小値")]]
    SLD_MIN  = 1,
    #[strum[props(desc="スライダー最大値")]]
    SLD_MAX  = 2,
    #[strum[props(desc="スライダー移動量")]]
    SLD_PAGE = 3,
    #[strum[props(desc="スライダー表示方向")]]
    SLD_BAR  = 4,
    #[strum[props(desc="スライダーのクライアントX座標")]]
    SLD_X    = 5,
    #[strum[props(desc="スライダーのクライアントY座標")]]
    SLD_Y    = 6,
}

#[builtin_func_desc(
    desc="スライダーの値を得る",
    rtype={desc="スライダー値、なければERR_VALUE",types="数値"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {o,n="n番目",t="数値",d="スライダーが複数ある場合その順番"},
        {o,n="種別",t="数値",d=r#"以下のいずれかを指定
- SLD_POS: 現在値
- SLD_MIN: 最小値
- SLD_MAX: 最大値
- SLD_PAGE: 1ページ移動量
- SLD_BAR: 表示方向 (横なら0、縦なら1)
- SLD_X: クライアントX座標
- SLD_Y: クライアントY座標"#},
    ],
)]
pub fn getslider(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let hwnd = get_hwnd_from_id(id);
    let nth = args.get_as_nth(1)?;
    let param = args.get_as_const(2, false)?.unwrap_or_default();

    if let Some(slider) = Slider::new(hwnd, nth) {
        let val = slider.get(param);
        Ok(Object::Num(val as f64))
    } else {
        let error_value = Object::Num(ErrConst::ERR_VALUE as i32 as f64);
        Ok(error_value)
    }
}

#[builtin_func_desc(
    desc="チェックボックスやラジオボタンの状態を得る",
    rtype={desc=r#"状態を示す値

-- 1: 存在しないか無効
- 0: チェックされていない
- 1: チェックされている
- 2: チェックボックスがグレー状態
- FALSE: ウィンドウが存在しない"#,types="数値またはFALSE"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {n="アイテム名",t="文字列",d="ボタン名 (部分一致)"},
        {o,n="n番目",t="数値",d="該当ボタンが複数ある場合その順番"},
        {o,n="ACC",t="真偽値",d="TRUE: ACCを使用, FALSE: APIまたはUIAを使用"},
    ],
)]
pub fn chkbtn(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let name = args.get_as_string(1, None)?;
    let nth = args.get_as_nth(2)?;
    let acc = args.get_as_bool(3, Some(false))?;

    let hwnd = get_hwnd_from_id(id);
    if unsafe {IsWindow(hwnd).as_bool()} {
        let result = if acc {
            acc::Acc::get_check_state(hwnd, name, nth).unwrap_or(-1)
        } else {
            let state = win32::Win32::get_check_state(hwnd, name.clone(), nth);
            if state < 0 {
                uia::UIA::chkbtn(hwnd, name, nth).unwrap_or(-1)
            } else {
                state
            }
        } as f64;

        Ok(result.into())
    } else {
        Ok(false.into())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum GetStrConst {
    #[strum[props(desc="エディットコントロール")]]
    STR_EDIT       = 0,
    #[strum[props(desc="スタティックコントロール")]]
    STR_STATIC     = 1,
    #[strum[props(desc="ステータスバー")]]
    STR_STATUS     = 2,
    #[strum[props(desc="エディット可能ACCオブジェクト")]]
    STR_ACC_EDIT   = 3,
    #[strum[props(desc="スタティックテキストACCオブジェクト")]]
    STR_ACC_STATIC = 4,
    #[strum[props(desc="DataGridViewのセル値")]]
    STR_ACC_CELL   = 5,
    STR_UIA        = 6,
}

#[builtin_func_desc(
    desc="ウィンドウから文字列を得る",
    rtype={desc="取得文字列",types="文字列"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ、0ならクリップボード文字列を得る"},
        {o,n="n番目",t="数値",d="該当コントロールが複数ある場合その順番"},
        {o,n="コントロール種別",t="定数",d=r#"取得対象コントロールを以下のいずれかで指定
- STR_EDIT: エディットコントロール
- STR_STATIC: スタティックコントロール
- STR_STATUS: ステータスバー
- STR_ACC_EDIT: 文字入力欄 (ACC使用)
- STR_ACC_STATIC: 入力欄以外のテキスト (ACC使用)
- STR_ACC_CELL: DataGridView内のセルの値 (ACC使用)"#},
        {o,n="マウス移動",t="真偽値",d="TRUEなら該当コントロール位置にマウスカーソルを移動"},
    ],
)]
pub fn getstr(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let nth = args.get_as_nth(1)?;
    let item_type = args.get_as_const(2, false)?.unwrap_or(GetStrConst::STR_EDIT);
    let mouse = args.get_as_bool(3, Some(false))?;

    if id == 0 {
        // クリップボードから
        let str = Clipboard::new().map_err(|e| UError::from(e))?.get_str();
        Ok(str.into())
    } else {
        let hwnd = get_hwnd_from_id(id);
        if is_window(hwnd) {
            let str = match item_type {
                GetStrConst::STR_EDIT => win32::Win32::get_edit_str(hwnd, nth, mouse),
                GetStrConst::STR_STATIC => win32::Win32::get_static_str(hwnd, nth, mouse),
                GetStrConst::STR_STATUS => win32::Win32::get_status_str(hwnd, nth, mouse),
                GetStrConst::STR_ACC_EDIT => acc::Acc::get_edit_str(hwnd, nth, mouse),
                GetStrConst::STR_ACC_STATIC => acc::Acc::get_static_str(hwnd, nth, mouse),
                GetStrConst::STR_ACC_CELL => acc::Acc::get_cell_str(hwnd, nth, mouse),
                GetStrConst::STR_UIA => None,
            };
            Ok(str.into())
        } else {
            Ok(Object::Empty)
        }
    }

}

#[derive(Clone, Copy)]
pub enum SendStrMode {
    /// キャレット位置に挿入
    Append,
    /// 元の内容を消してから入力
    Replace,
    /// キャレット位置に挿入
    /// ただし1文字ずつ送信
    OneByOne,
}
impl From<i32> for SendStrMode {
    fn from(n: i32) -> Self {
        match n {
            0 => Self::Append,
            2 => Self::OneByOne,
            _ => Self::Replace,
        }
    }
}

#[builtin_func_desc(
    desc="エディットボックスまたはクリップボードに文字列を送信する",
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ、0ならクリップボードに送る"},
        {n="送信文字列",t="文字列",d="送る文字列"},
        {o,n="n番目",t="数値",d="エディットボックスが複数存在する場合その順番"},
        {o,n="送信モード",t="真偽値または数値",d="FALSE: 追記, TRUE: 置換, 2: 一文字ずつ送信 (ACC時使用不可)"},
        {o,n="ACC設定",t="真偽値または定数",d=r#"以下のいずれかを指定
- FALSE: APIまたはUIAで送信
- TRUE: ACCで送信
- STR_ACC_CELL: DataGridView内のCellに送信
- STR_UIA: UIAで送信、送信モードは無視され常に置換される
"#},
    ],
)]
pub fn sendstr(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let str = args.get_as_string(1, None)?;
    let nth = args.get_as_int(2, Some(0))?;
    let mode = args.get_as_bool_or_int(3, Some(0))?;
    let acc = args.get_as_bool_or_int(4, Some(0))?;

    if id == 0 {
        // クリップボードに挿入
        Clipboard::new().map_err(|e| UError::from(e))?.send_str(str);
    } else {
        let hwnd = get_hwnd_from_id(id);
        let mode = SendStrMode::from(mode);
        match acc {
            0 => {
                if win32::Win32::sendstr(hwnd, nth, &str, mode).is_none() {
                    uia::UIA::sendstr(hwnd, nth, str);
                }
            },
            5 => acc::Acc::sendstr_cell(hwnd, nth, &str, mode), // cell
            6 => uia::UIA::sendstr(hwnd, nth, str), // uia
            _ => acc::Acc::sendstr(hwnd, nth, &str, mode), // acc
        };
    }
    Ok(Object::Empty)
}

#[builtin_func_desc(
    desc="コンボボックス、リストボックス、ツリービュー、リストビューから選択項目の値を得る",
    rtype={desc="項目の値、複数選択なら配列",types="文字列または配列"}
    args=[
        {n="ウィンドウID",t="数値",d="対象ウィンドウ"},
        {o,n="n番目",t="数値",d="該当コントロールが複数ある場合はその順番"},
        {o,n="列",t="数値",d="リストビューの場合取得する列の番号を指定"},
    ],
)]
pub fn getslctlst(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let nth = args.get_as_nth(1)?;
    let column = args.get_as_nth(2)? as isize - 1;

    let hwnd = get_hwnd_from_id(id);

    let mut found = win32::Win32::getslctlst(hwnd, nth, column);
    let obj = match found.len() {
        0 => Object::Empty,
        1 => found.pop().into(),
        _ => {
            let arr = found.into_iter().map(|s| s.into()).collect();
            Object::Array(arr)
        }
    };

    Ok(obj)
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum ImgConst {
    #[strum[props(desc="ウィンドウ状態によりIMG_FOREかIMG_BACKが適用される")]]
    IMG_AUTO = 0,
    #[strum[props(desc="スクリーン全体からウィンドウ位置の画像を切り出す")]]
    IMG_FORE = 1,
    #[strum[props(desc="ウィンドウから直接画像取得")]]
    IMG_BACK = 2,
}
#[cfg(feature="chkimg")]
#[builtin_func_desc(
    desc="ウィンドウ画像を保存",
    args=[
        {o,n="ファイル名",t="文字列",d="保存先ファイルのパス、省略時はクリップボードにコピー"},
        {o,n="ウィンドウID",t="数値",d="対象ウィンドウ、0ならスクリーン全体"},
        {o,n="X",t="数値",d="取得範囲の起点となるX座標、省略時はウィンドウまたはスクリーンの左上X座標"},
        {o,n="Y",t="数値",d="取得範囲の起点となるY座標、省略時はウィンドウまたはスクリーンの左上Y座標"},
        {o,n="幅",t="数値",d="取得範囲の幅、省略時はウィンドウまたはスクリーンの幅"},
        {o,n="高さ",t="数値",d="取得範囲の高さ、省略時はウィンドウまたはスクリーンの高さ"},
        {o,n="クライアント領域",t="真偽値",d="TRUEならウィンドウ全体ではなくクライアント領域のみを得る"},
        {o,n="オプション",t="数値",d=r#"画像形式により以下を指定
- jpg: 画質を0-100で指定、デフォルト95
- png: 圧縮度合いを0-9で指定、デフォルトは1
- bmp: この値は無視される"#},
        {o,n="取得方法",t="定数",d=r#"画面取得方法を以下から指定
- IMG_FORE: スクリーン全体から対象ウィンドウの座標を元に画像を切り出す
- IMG_BACK: 対象ウィンドウの画像を直接取得、バックグラウンド可
- IMG_AUTO: ウィンドウが完全に見えているならIMG_FORE、隠れているならIMG_BACKで取得 (デフォルト)"#},
        {o,n="WGCAPI",t="真偽値",d="TRUEならGraphicsCaptureAPIでキャプチャする"},
        {o,n="モニタ番号",t="真偽値",d="WGCAPI利用かつスクリーンを取得する場合にモニタ番号を0から指定"},
    ],
)]
pub fn saveimg(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let filename = args.get_as_string_or_empty(0)?;
    let id = args.get_as_int(1, Some(0))?;
    let left = args.get_as_int_or_empty(2)?;
    let top = args.get_as_int_or_empty(3)?;
    let width = args.get_as_int_or_empty(4)?;
    let height = args.get_as_int_or_empty(5)?;
    let client = args.get_as_bool(6, Some(false))?;
    let param = args.get_as_int_or_empty(7)?;
    let style = args.get_as_const(8, false)?.unwrap_or(ImgConst::IMG_AUTO);
    let wgcapi = args.get_as_bool(9, Some(false))?;

    let ss = if id > 0 {
        let hwnd = get_hwnd_from_id(id);
        if wgcapi {
            ScreenShot::get_window_wgcapi_wh(hwnd, left, top, width, height, client)?
        } else {
            ScreenShot::get_window_wh(hwnd, left, top, width, height, client, style)?
        }
    } else if id < 0 {
        return Ok(Object::Empty);
    } else {
        if wgcapi {
            let monitor = args.get_as_int(10, Some(0))?;
            let right = match left {
                Some(l) => match width {
                    Some(w) => {
                        if l.is_negative() {
                            Some(w)
                        } else {
                            Some(l + w)
                        }
                    },
                    None => None,
                },
                None => width,
            };
            let bottom = match top {
                Some(t) => match height {
                    Some(h) => {
                        if t.is_negative() {
                            Some(h)
                        } else {
                            Some(t + h)
                        }
                    },
                    None => None,
                },
                None => height,
            };
            ScreenShot::get_screen_wgcapi(monitor, left, top, right, bottom)?
        } else {
            ScreenShot::get_screen_wh(left, top, width, height)?
        }
    };
    if let Some(filename) = filename {
        let mut path = std::path::PathBuf::from(filename);
        let ext = path.extension().map(|os| os.to_str()).flatten();
        let (jpg_quality, png_compression) = match ext {
            Some("jpg") | Some("jpeg") => {
                (param.filter(|n| n >= &0 && n <= &100), None)
            },
            Some("png") => {
                (None, param.filter(|n| n >= &0 && n <= &9))
            },
            Some(_) => (None, None),
            None => {
                path.set_extension("png");
                (None, param.filter(|n| n >= &0 && n <= &9))
            }
        };
        let filename = path.to_string_lossy();
        ss.save_to(&filename, jpg_quality, png_compression)?;
    } else {
        ss.to_clipboard()?;
    }

    Ok(Object::Empty)
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum MorgTargetConst {
    #[default]
    #[strum[props(desc="起点座標をウィンドウ左上にする")]]
    MORG_WINDOW = 0,
    #[strum[props(desc="起点座標をウィンドウのクライアント領域の左上にする")]]
    MORG_CLIENT = 1,
    #[strum[props(desc="起点座標をウィンドウのクライアント領域の左上にし、直接送信を有効にする")]]
    MORG_DIRECT = 2,
}
impl Into<MorgTarget> for MorgTargetConst {
    fn into(self) -> MorgTarget {
        match self {
            MorgTargetConst::MORG_WINDOW => MorgTarget::Window,
            MorgTargetConst::MORG_CLIENT => MorgTarget::Client,
            MorgTargetConst::MORG_DIRECT => MorgTarget::Direct,
        }
    }
}
#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive, Default)]
pub enum MorgContextConst {
    #[default]
    #[strum[props(desc="スクリーン上から画像や色を取得")]]
    MORG_FORE = 1,
    #[strum[props(desc="ウィンドウから直接画像や色を取得")]]
    MORG_BACK = 2,
}
impl Into<MorgContext> for MorgContextConst {
    fn into(self) -> MorgContext {
        match self {
            MorgContextConst::MORG_FORE => MorgContext::Fore,
            MorgContextConst::MORG_BACK => MorgContext::Back,
        }
    }
}
struct MorgImg {
    input: window_low::Input,
    is_back: bool,
    hwnd: Option<HWND>,
}
impl From<&Option<MouseOrg>> for MorgImg {
    fn from(morg: &Option<MouseOrg>) -> Self {
        let input = window_low::Input::from(morg);
        let (is_back, hwnd) = match &morg {
            Some(morg) => {
                (morg.is_back(), Some(morg.hwnd))
            },
            None => (false, None),
        };
        Self { input, is_back, hwnd }
    }
}
impl MorgImg {
    #[cfg(feature="chkimg")]
    fn is_client(&self) -> bool {
        self.input.is_client()
    }
    /// MORG_FOREならmouseorgの座標、MORG_BACKならそのまま返す
    fn fix_point(&self, x: i32, y: i32) -> (i32, i32) {
        if self.is_back {
            (x, y)
        } else {
            self.input.fix_point(x, y)
        }
    }
    /// MORG_BACKならウィンドウを再描画
    fn redraw_window(&self) {
        unsafe {
            if self.is_back {
                let flags = RDW_FRAME|RDW_INVALIDATE|RDW_ERASE|RDW_UPDATENOW|RDW_ALLCHILDREN;
                RedrawWindow(self.hwnd.as_ref(), None, None, flags);
            }
        }
    }
}

#[builtin_func_desc(
    desc="特定関数のおける起点座標を指定ウィンドウ基準とする",
    rtype={desc="成功時TRUE",types="真偽値"}
    args=[
        {n="ID",t="数値",d="対象ウィンドウのウィンドウIDまたはHWND"},
        {o,n="起点",t="定数",d=r#"起点座標を以下のいずれから指定
- MORG_WINDOW: 対象ウィンドウのウィンドウ領域左上を起点とする (デフォルト)
- MORG_CLIENT: 対象ウィンドウのクライアント領域左上を起点とする
- MORG_DIRECT: 対象ウィンドウのクライアント領域左上を起点とし、入力を直接送信する"#},
        {o,n="取得方法",t="定数",d=r#"ウィンドウ画像の取得方法を指定
- MORG_WINDOW: 対象ウィンドウのウィンドウ領域左上を起点とする (デフォルト)
- MORG_CLIENT: 対象ウィンドウのクライアント領域左上を起点とする
- MORG_DIRECT: 対象ウィンドウのクライアント領域左上を起点とし入力を直接送信する、HWND指定に対応"#},
        {o,n="HWNDフラグ",t="真偽値",d=r#"IDの扱いを指定
- FALSE: 入力値をウィンドウIDとして扱うが、対象ウィンドウが存在しない場合かつMORG_DIRECTであればHWNDとして扱う
- TRUE: MORG_DIRECTであれば入力値をHWNDとして扱う
"#},
    ],
)]
pub fn mouseorg(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let id = args.get_as_int(0, None)?;
    let target = args.get_as_const::<MorgTargetConst>(1, false)?.unwrap_or_default();
    let context = args.get_as_const::<MorgContextConst>(2, false)?.unwrap_or_default();
    let hwnd_flg = args.get_as_bool(3, Some(false))?;

    if id == 0 {
        evaluator.clear_mouseorg();
        Ok(true.into())
    } else {
        let hwnd = match target {
            MorgTargetConst::MORG_DIRECT => {
                if hwnd_flg {
                    HWND(id as isize)
                } else {
                    let hwnd = get_hwnd_from_id(id);
                    if let HWND(0) = hwnd {
                        HWND(id as isize)
                    } else {
                        hwnd
                    }
                }
            },
            _ => {
                get_hwnd_from_id(id)
            },
        };

        if is_window(hwnd) {
            evaluator.set_mouseorg(hwnd, target, context);
            Ok(true.into())
        } else {
            Ok(false.into())
        }
    }
}

#[builtin_func_desc(
    desc="mousemorg時の起点座標を得る",
    rtype={desc="起点座標を[x, y]で得る、mouseorg未実行ならEMPTY",types="配列"}
)]
pub fn chkmorg(evaluator: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    match window_low::get_morg_point(&evaluator.mouseorg) {
        Some((x, y)) => {
            let arr = vec![ x.into(), y.into() ];
            Ok(Object::Array(arr))
        },
        None => Ok(Object::Empty),
    }
}

#[cfg(feature="chkimg")]
fn obj_vec_to_u8_slice(arr: Vec<Object>) -> [u8; 3] {
    let to_u8 = |i: usize| arr.get(i).map(|o| o.as_f64(true)).flatten().unwrap_or(0.0) as u8;
    [ to_u8(0), to_u8(1), to_u8(2) ]
}

#[cfg(feature="chkimg")]
#[builtin_func_desc(
    desc="指定色の座標を得る",
    rtype={desc="座標と色の情報 `[X, Y, [B, G, R]]` の配列",types="配列"}
    args=[
        {n="探索色",t="数値または配列",d="BGR値、または [B,G,R]"},
        {o,n="閾値",t="数値または配列",d="BGRそれぞれに対する閾値、または [B,G,R] で個別指定"},
        {o,n="範囲",t="配列",d="[左上X,左上Y,右下X,右下Y] で指定、省略時はモニタまたはウィンドウ全体"},
        {o,n="モニタ番号",t="数値",d="mouseorg未使用時に探索対象となるモニタを指定(0から)"},
    ],
)]
pub fn chkclr(evaluator: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {

    let threshold = args.get_as_int_or_array_or_empty(1)?.map(|two| {
        match two {
            TwoTypeArg::T(n) => {
                let t = n as u8;
                [t; 3]
            },
            TwoTypeArg::U(arr) => {
                obj_vec_to_u8_slice(arr)
            },
        }
    });
    let check_color = match args.get_as_int_or_array(0, None)? {
        TwoTypeArg::T(bgr) => {
            let color = bgr as u32;
            CheckColor::new_from_bgr(color, threshold)
        },
        TwoTypeArg::U(arr) => {
            let color = obj_vec_to_u8_slice(arr);
            CheckColor::new(color, threshold)
        },
    };
    let range = args.get_as_array(2, Some(vec![]))?;
    let to_i32 = |i: usize| range.get(i).map(|o| o.as_f64(false).map(|n| n as i32)).flatten();
    let left = to_i32(0);
    let top = to_i32(1);
    let right = to_i32(2);
    let bottom = to_i32(3);

    let mi = MorgImg::from(&evaluator.mouseorg);

    let ss = match mi.hwnd {
        Some(hwnd) => {
            let client = mi.is_client();
            ScreenShot::get_window_wgcapi(hwnd, left, top, right, bottom, client)?
        },
        None => {
            let monitor = args.get_as_int(3, Some(0))?;
            ScreenShot::get_screen_wgcapi(monitor, left, top, right, bottom)?
        },
    };

    if should_save_ss() {
        ss.save(Some("chkclr.png"))?;
    }

    let found = check_color.search(&ss)?.into_iter()
        .map(|(x, y, (b, g, r))| {
            let color = Object::Array(vec![b.into(), g.into(), r.into()]);
            Object::Array(vec![x.into(), y.into(), color])
        })
        .collect();

    Ok(Object::Array(found))

}