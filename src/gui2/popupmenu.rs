use super::*;
use crate::evaluator::object::Object;

#[link(name = "user32")]
extern "stdcall" {
    fn TrackPopupMenu(hmenu: isize, uflags: u32, x: i32, y: i32, nreserved: i32, hwnd: isize, prcrect: isize) -> i32;
}

use std::sync::OnceLock;

static REGISTER_CLASS: OnceLock<UWindowResult<()>> = OnceLock::new();

pub struct PopupParentWin {
    hwnd: HWND,
}
impl PopupParentWin {
    fn new() -> UWindowResult<Self> {
        let hwnd = Self::create_window("PopupDummy")?;
        Ok(Self { hwnd })
    }
}
impl UWindow<()> for PopupParentWin {
    const CLASS_NAME: PCWSTR = w!("UWSCR.Popup");

    fn create_window(title: &str) -> UWindowResult<HWND> {
        Self::register_window_class(&REGISTER_CLASS)?;
        let hwnd = WindowBuilder::new(title, Self::CLASS_NAME)
            .size(None, None, Some(1), Some(1))
            .build()?;
        Ok(hwnd)
    }

    unsafe extern "system"
    fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            msg => wm::DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
    fn draw(&self) -> UWindowResult<()> {
        unimplemented!()
    }
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
    fn font(&self) -> Gdi::HFONT {
        unimplemented!()
    }
}

pub struct PopupMenu {
    pub hmenu: wm::HMENU,
    items: Vec<MenuItem>,
}
impl PopupMenu {
    pub fn new(list: Vec<ItemName>) -> UWindowResult<Self> {
        let mut id = 1;
        let mut items = vec![];
        unsafe {
            let hmenu = Self::create_menu()?;
            let mut index = 0;
            loop {
                let Some(item) = list.get(index) else {break;};
                if let ItemName::Item(name) = item {
                    let name = HSTRING::from(name);
                    // 次のアイテムがSubItemsであればサブメニューを作成
                    if let Some(ItemName::SubItems(names)) = list.get(index+1) {
                        let sub = Self::create_menu()?;
                        Self::append_submenu(hmenu, &sub, &name)?;
                        Self::set_submenu(sub, names, &mut id, &mut items)?;
                        index += 1;
                    } else {
                        items.push(MenuItem::new(id, name.to_string()));
                        Self::append_menu_item(hmenu, &mut id, &name)?;
                    }
                }
                index += 1;
            }
            Ok(Self { hmenu, items })
        }
    }
    pub fn show(self, x: Option<i32>, y: Option<i32>) -> UWindowResult<Option<String>> {
        unsafe {
            let dummy = PopupParentWin::new()?;
            let (x, y) = {
                if x.is_some() || x.is_some() {
                    (x.unwrap(), y.unwrap())
                } else {
                    let mut point = POINT::default();
                    let _ = wm::GetCursorPos(&mut point);
                    (x.unwrap_or(point.x), y.unwrap_or(point.y))
                }
            };
            wm::SetForegroundWindow(dummy.hwnd);

            let uflags = (wm::TPM_TOPALIGN|wm::TPM_RETURNCMD|wm::TPM_NONOTIFY).0;
            let id = TrackPopupMenu(self.hmenu.0, uflags, x, y, 0, dummy.hwnd.0, 0);
            dummy.destroy();

            let selected = self.items.into_iter().find_map(|item| (item.id as i32 == id).then_some(item.name));
            Ok(selected)
        }
    }
    unsafe fn set_submenu(parent: wm::HMENU, names: &Vec<ItemName>, id: &mut usize, items: &mut Vec<MenuItem>) -> UWindowResult<()> {
        let mut index = 0usize;
        loop {
            let Some(item) = names.get(index) else {break;};
            if let ItemName::Item(name) = item {
                let name = HSTRING::from(name);
                if let Some(ItemName::SubItems(names)) = names.get(index+1) {
                    let sub = Self::create_menu()?;
                    Self::append_submenu(parent, &sub, &name)?;
                    Self::set_submenu(sub, names, id, items)?;
                    index += 1;
                } else {
                    items.push(MenuItem::new(*id, name.to_string()));
                    Self::append_menu_item(parent, id, &name)?;
                }
            }
            index += 1;
        }
        Ok(())
    }
    unsafe fn create_menu() -> UWindowResult<wm::HMENU> {
        wm::CreatePopupMenu().map_err(|_| UWindowError::PopupMenuCreateError)
    }
    unsafe fn append_submenu(parent: wm::HMENU, sub: &wm::HMENU, item: &HSTRING) -> UWindowResult<()> {
        wm::AppendMenuW(parent, wm::MF_POPUP, sub.0 as usize, item)
            .map_err(|_| UWindowError::PopupMenuAppendError)?;
        Ok(())
    }
    unsafe fn append_menu_item(hmenu: wm::HMENU, id: &mut usize, item: &HSTRING) -> UWindowResult<()> {
        wm::AppendMenuW(hmenu, wm::MF_ENABLED|wm::MF_STRING, *id, item)
            .map_err(|_| UWindowError::PopupMenuAppendError)?;
        *id += 1;
        Ok(())
    }
}

pub struct MenuItem {
    id: usize,
    name: String,
}
impl MenuItem {
    fn new(id: usize, name: String) -> Self {
        Self { id, name }
    }
}

pub enum ItemName {
    Item(String),
    SubItems(Vec<ItemName>)
}

impl From<Object> for ItemName {
    fn from(obj: Object) -> Self {
        match obj {
            Object::Array(vec) => {
                let items = vec.into_iter()
                    .map(|o| o.into())
                    .collect();
                Self::SubItems(items)
            },
            obj => Self::Item(obj.to_string())
        }
    }
}