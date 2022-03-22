use super::*;

use crate::evaluator::object::Object;
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
            None,
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
    pub fn new(list: Vec<Object>) -> Self {
        let menu = Self::create_menu(list);
        PopupMenu { menu }
    }
    pub fn show(&self, x: Option<i32>, y: Option<i32>) -> UWindowResult<Option<String>> {
        unsafe {
            let dummy = PoupupDummyWin::new()?;
            let mut cursor_pos = POINT::default();
            GetCursorPos(&mut cursor_pos);
            let x = x.unwrap_or(cursor_pos.x);
            let y = y.unwrap_or(cursor_pos.y);
            let b = TrackPopupMenu(
                self.menu.hmenu(),
                TPM_TOPALIGN|TPM_RETURNCMD|TPM_NONOTIFY,
                x, y, 0,
                dummy.hwnd(),
                std::ptr::null() as *const RECT
            );
            dummy.destroy();
            Ok(self.menu.get_item(b.0))
        }
    }
    fn create_menu(list: Vec<Object>) -> Menu {
        let mut menu = Menu::new();
        for i in 0..list.len() {
            match list.get(i) {
                Some(o) => match o {
                    Object::Array(_) => continue,
                    o => {
                        let item = o.to_string();
                        if let Some(Object::Array(vec)) = list.get(i+1) {
                            let mut sub = Self::create_submenu(item, vec, menu.get_submenu_id());
                            menu.set_submenu(&mut sub);
                        } else {
                            menu.append(item);
                        }
                    },
                },
                None => break,
            }
        }
        menu
    }
    fn create_submenu(name: String, list: &Vec<Object>, id: usize) -> SubMenu {
        let mut menu = SubMenu::new(name, id);
        for i in 0..list.len() {
            match list.get(i) {
                Some(o) => match o {
                    Object::Array(_) => continue,
                    o => {
                        let item = o.to_string();
                        if let Some(Object::Array(vec)) = list.get(i+1) {
                            let mut sub = Self::create_submenu(item, vec, menu.get_submenu_id());
                            menu.set_submenu(&mut sub);
                        } else {
                            menu.append(item);
                        }
                    },
                },
                None => break,
            }
        }
        menu
    }
}

type ItemList = Vec<(usize, String)>;

trait MenuTrait {
    fn get_submenu_id(&self) -> usize {
        self.id() * 100
    }
    fn set_submenu(&mut self, sub: &mut SubMenu) {
        unsafe {
            AppendMenuW(self.hmenu(), MF_POPUP, sub.get_id(), sub.get_name());
            let list = self.get_mut_list();
            sub.push_list_to_parent(list);
        }
    }
    fn append(&mut self, item: String) {
        unsafe {
            let id = self.id();
            self.set_list(id, &item);
            AppendMenuW(self.hmenu(), MF_ENABLED|MF_STRING, self.id(), item);
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
    fn new() -> Self {
        let hmenu = unsafe { CreatePopupMenu() };
        Self { hmenu, id: 1, list: vec![] }
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
    fn new(name: String, id: usize) -> Self {
        let hmenu = unsafe { CreatePopupMenu() };
        Self { hmenu, name, id, list: vec![] }
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