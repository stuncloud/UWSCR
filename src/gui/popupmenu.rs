use super::*;

use crate::evaluator::object::Object;
use crate::winapi::{WString, PcwstrExt};

static POPUP_CLASS: OnceCell<Result<String, UWindowError>> = OnceCell::new();

pub struct PoupupDummyWin {
    hwnd: HWND
}
impl PoupupDummyWin {
    fn new() -> UWindowResult<Self> {
        let class_name = Window::get_class_name("UWSCR.PoupupDummyWin", &POPUP_CLASS, Some(Self::wndproc))?;
        let hwnd = Window::create_window(
            None,
            &class_name,
            "",
            WINDOW_EX_STYLE::default(),
            WINDOW_STYLE::default(),
            0, 0, 1, 1, None
        )?;
        unsafe {SetForegroundWindow(hwnd);}
        Ok(Self {hwnd})
    }
    fn destroy(&self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}
impl UWindow<()> for PoupupDummyWin {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

#[derive(Debug)]
pub struct PopupMenu {
    menu: Menu
}

impl PopupMenu {
    pub fn new(list: Vec<Object>) -> UWindowResult<Self> {
        let menu = Self::create_menu(list)?;
        Ok(PopupMenu { menu })
    }
    fn get_point(x: Option<i32>, y: Option<i32>) -> (i32, i32) {
        unsafe {
            let mut p = POINT::default();
            GetCursorPos(&mut p);
            match (x, y) {
                (None, None) => (p.x, p.y),
                (None, Some(mut y)) => {
                    if let Some(m) = Monitor::from_point(p.x, y) {
                        y = m.to_scaled(y);
                    }
                    (p.x, y)
                },
                (Some(mut x), None) => {
                    if let Some(m) = Monitor::from_point(x, p.y) {
                        x = m.to_scaled(x);
                    }
                    (x, p.y)
                },
                (Some(mut x), Some(mut y)) => {
                    if let Some(m) = Monitor::from_point(x, y) {
                        x = m.to_scaled(x);
                        y = m.to_scaled(y);
                    }
                    (x, y)
                },
            }
        }
    }
    pub fn show(&self, x: Option<i32>, y: Option<i32>) -> UWindowResult<Option<String>> {
        unsafe {
            let dummy = PoupupDummyWin::new()?;
            let (x, y) = Self::get_point(x, y);
            let b = TrackPopupMenu(
                self.menu.hmenu(),
                TPM_TOPALIGN|TPM_RETURNCMD|TPM_NONOTIFY,
                x, y, 0,
                dummy.hwnd(),
                None
            );
            dummy.destroy();
            Ok(self.menu.get_item(b.0))
        }
    }
    fn create_menu(list: Vec<Object>) -> UWindowResult<Menu> {
        let mut menu = Menu::new()?;
        for i in 0..list.len() {
            match list.get(i) {
                Some(o) => match o {
                    Object::Array(_) => continue,
                    o => {
                        let item = o.to_string();
                        if let Some(Object::Array(vec)) = list.get(i+1) {
                            let mut sub = Self::create_submenu(item, vec, menu.get_submenu_id())?;
                            menu.set_submenu(&mut sub);
                        } else {
                            menu.append(item);
                        }
                    },
                },
                None => break,
            }
        }
        Ok(menu)
    }
    fn create_submenu(name: String, list: &Vec<Object>, id: usize) -> UWindowResult<SubMenu> {
        let mut menu = SubMenu::new(name, id)?;
        for i in 0..list.len() {
            match list.get(i) {
                Some(o) => match o {
                    Object::Array(_) => continue,
                    o => {
                        let item = o.to_string();
                        if let Some(Object::Array(vec)) = list.get(i+1) {
                            let mut sub = Self::create_submenu(item, vec, menu.get_submenu_id())?;
                            menu.set_submenu(&mut sub);
                        } else {
                            menu.append(item);
                        }
                    },
                },
                None => break,
            }
        }
        Ok(menu)
    }
}

type ItemList = Vec<(usize, String)>;

trait MenuTrait {
    fn get_submenu_id(&self) -> usize {
        self.id() * 100
    }
    fn set_submenu(&mut self, sub: &mut SubMenu) {
        unsafe {
            let lpnewitem = sub.get_name().to_wide_null_terminated().to_pcwstr();
            AppendMenuW(self.hmenu(), MF_POPUP, sub.get_id(), lpnewitem);
            let list = self.get_mut_list();
            sub.push_list_to_parent(list);
        }
    }
    fn append(&mut self, item: String) {
        unsafe {
            let id = self.id();
            self.set_list(id, &item);
            let lpnewitem = item.to_wide_null_terminated().to_pcwstr();
            AppendMenuW(self.hmenu(), MF_ENABLED|MF_STRING, self.id(), lpnewitem);
        }
        self.increase_id();
    }
    fn set_list(&mut self, id: usize, name: &String) {
        let list = self.get_mut_list();
        list.push((id, name.to_string()));
    }
    fn get_mut_list(&mut self) -> &mut ItemList;
    fn hmenu(&self) -> HMENU;
    fn increase_id(&mut self);
    fn id(&self) -> usize;
}

#[derive(Debug, Clone)]
struct Menu {
    pub hmenu: HMENU,
    id: usize,
    list: ItemList
}

impl Menu {
    fn new() -> UWindowResult<Self> {
        let hmenu = unsafe {
            CreatePopupMenu()
                .map_err(|e| UWindowError::FailedToCreatePopupMenu(e.to_string()))?
        };
        Ok(Self { hmenu, id: 1, list: vec![] })
    }
    fn get_item(&self, id: i32) -> Option<String> {
        let item = self.list.iter().find(|(n, _)| *n == id as usize).map(|(_,s)| s.to_string());
        item
    }
}

impl MenuTrait for Menu {
    fn get_mut_list(&mut self) -> &mut ItemList {
        &mut self.list
    }

    fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    fn increase_id(&mut self) {
        self.id += 1;
    }

    fn id(&self) -> usize {
        self.id
    }
}

struct SubMenu {
    hmenu: HMENU,
    name: String,
    id: usize,
    list: ItemList
}
impl SubMenu {
    fn get_id(&self) -> usize {
        self.hmenu.0 as usize
    }
    fn get_name(&self) -> String {
        self.name.to_string()
    }
    fn new(name: String, id: usize) -> UWindowResult<Self> {
        let hmenu = unsafe {
            CreatePopupMenu()
                .map_err(|e| UWindowError::FailedToCreatePopupMenu(e.to_string()))?
        };
        Ok(Self { hmenu, name, id, list: vec![] })
    }
    fn push_list_to_parent(&mut self,list: &mut Vec<(usize, String)>) {
        list.append(&mut self.list);
    }
}

impl MenuTrait for SubMenu {
    fn get_mut_list(&mut self) -> &mut ItemList {
        &mut self.list
    }

    fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    fn increase_id(&mut self) {
        self.id += 1;
    }

    fn id(&self) -> usize {
        self.id
    }
}