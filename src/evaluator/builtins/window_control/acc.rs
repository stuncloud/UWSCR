use std::ptr::null_mut;
use std::ffi::c_void;
use std::mem::{transmute, ManuallyDrop};

use windows::{
    core::{Interface, HRESULT,},
    Win32::{
        Foundation::{HWND, BSTR},
        UI::{
            WindowsAndMessaging::{
                STATE_SYSTEM_SELECTABLE, STATE_SYSTEM_CHECKED,
                OBJID_WINDOW,
            },
            Accessibility::{
                ROLE_SYSTEM_ALERT, ROLE_SYSTEM_ANIMATION, ROLE_SYSTEM_APPLICATION, ROLE_SYSTEM_BORDER, ROLE_SYSTEM_BUTTONDROPDOWN, ROLE_SYSTEM_BUTTONDROPDOWNGRID, ROLE_SYSTEM_BUTTONMENU, ROLE_SYSTEM_CARET, ROLE_SYSTEM_CELL, ROLE_SYSTEM_CHARACTER, ROLE_SYSTEM_CHART, ROLE_SYSTEM_CHECKBUTTON, ROLE_SYSTEM_CLIENT, ROLE_SYSTEM_CLOCK, ROLE_SYSTEM_COLUMN, ROLE_SYSTEM_COLUMNHEADER, ROLE_SYSTEM_COMBOBOX, ROLE_SYSTEM_CURSOR, ROLE_SYSTEM_DIAGRAM, ROLE_SYSTEM_DIAL, ROLE_SYSTEM_DIALOG, ROLE_SYSTEM_DOCUMENT, ROLE_SYSTEM_DROPLIST, ROLE_SYSTEM_EQUATION, ROLE_SYSTEM_GRAPHIC, ROLE_SYSTEM_GRIP, ROLE_SYSTEM_GROUPING, ROLE_SYSTEM_HELPBALLOON, ROLE_SYSTEM_HOTKEYFIELD, ROLE_SYSTEM_INDICATOR, ROLE_SYSTEM_IPADDRESS, ROLE_SYSTEM_LINK, ROLE_SYSTEM_LIST, ROLE_SYSTEM_LISTITEM, ROLE_SYSTEM_MENUBAR, ROLE_SYSTEM_MENUITEM, ROLE_SYSTEM_MENUPOPUP, ROLE_SYSTEM_OUTLINE, ROLE_SYSTEM_OUTLINEBUTTON, ROLE_SYSTEM_OUTLINEITEM, ROLE_SYSTEM_PAGETAB, ROLE_SYSTEM_PAGETABLIST, ROLE_SYSTEM_PANE, ROLE_SYSTEM_PROGRESSBAR, ROLE_SYSTEM_PROPERTYPAGE, ROLE_SYSTEM_PUSHBUTTON, ROLE_SYSTEM_RADIOBUTTON, ROLE_SYSTEM_ROW, ROLE_SYSTEM_ROWHEADER, ROLE_SYSTEM_SCROLLBAR, ROLE_SYSTEM_SEPARATOR, ROLE_SYSTEM_SLIDER, ROLE_SYSTEM_SOUND, ROLE_SYSTEM_SPINBUTTON, ROLE_SYSTEM_SPLITBUTTON, ROLE_SYSTEM_STATICTEXT, ROLE_SYSTEM_STATUSBAR, ROLE_SYSTEM_TABLE, ROLE_SYSTEM_TEXT, ROLE_SYSTEM_TITLEBAR, ROLE_SYSTEM_TOOLBAR, ROLE_SYSTEM_TOOLTIP, ROLE_SYSTEM_WHITESPACE, ROLE_SYSTEM_WINDOW,
                IAccessible,
                AccessibleObjectFromWindow,
                AccessibleChildren,
                WindowFromAccessibleObject,
                SELFLAG_TAKEFOCUS,SELFLAG_TAKESELECTION,SELFLAG_ADDSELECTION,
            },
            Controls::{
                STATE_SYSTEM_INVISIBLE,
                WC_LISTVIEW,
            },
        },
        System::{
            Com::{
                VARIANT, VARIANT_0_0, IDispatch,
            },
            Ole::{
                VARENUM,VT_I4,VT_DISPATCH,
                VariantInit,
            }
        }
    }
};

use crate::winapi::{WString, get_class_name};
use super::clkitem::{ClkItem, match_title};

#[derive(Debug, Clone)]
pub struct Acc {
    obj: IAccessible,
    id: Option<i32>,
    has_child: bool,
}

#[allow(unused)]
impl Acc {
    pub fn new(obj: IAccessible, id: i32) -> Self {
        Self { obj, id: Some(id), has_child: false }
    }
    pub fn from_hwnd(hwnd: HWND) -> Option<Self> {
        if let HWND(0) = hwnd {
            None
        } else {
            unsafe {
                let mut ppvobject = null_mut::<IAccessible>() as *mut c_void;
                match AccessibleObjectFromWindow(hwnd, OBJID_WINDOW.0 as u32, &IAccessible::IID, &mut ppvobject) {
                    Ok(_) => {
                        let obj: IAccessible = transmute(ppvobject);
                        Some(Acc {obj, id: None, has_child: true })
                    },
                    Err(_) => None,
                }
            }
        }
    }
    #[allow(unused)]
    pub fn get_hwnd(&self) -> Option<HWND> {
        unsafe {
            WindowFromAccessibleObject(&self.obj).ok()
        }
    }
    pub fn get_child_count(&self) -> i32 {
        unsafe {
            self.obj.accChildCount().unwrap_or(0)
        }
    }
    fn has_child(&self) -> bool {
        self.has_child && self.get_child_count() > 0
    }
    fn get_varchild(&self) -> VARIANT {
        self.id.unwrap_or(0).into_variant()
    }
    fn has_valid_name(&self) -> bool {
        if let Some(name) = self.get_name() {
            name.len() > 0
        } else {
            false
        }
    }
    fn get_item_name(&self) -> Option<String> {
        if let Some(AccRole::Text) = self.get_role() {
            self.get_value()
        } else {
            self.get_name()
        }
    }
    pub fn get_name(&self) -> Option<String> {
        unsafe {
            let varchild = self.get_varchild();
            self.obj.get_accName(&varchild)
                    .map(|bstr| bstr.to_string())
                    .ok()
        }
    }
    pub fn get_default_action(&self) -> Option<String> {
        unsafe {
            let varchild = self.get_varchild();
            self.obj.get_accDefaultAction(&varchild)
                    .map(|bstr| bstr.to_string())
                    .ok()
        }
    }
    pub fn click(&self, check: bool) -> AccClickResult {
        if let Some(AccRole::ListItem) = self.get_role() {
            // リスト項目の場合はまず選択
            self.select(false);
        }
        let result = if let Some(action) = self.get_default_action() {
            match self.invoke_default_action(check) {
                true => AccClickResult::new(true, AccClickReason::DefaultAction(action)),
                // デフォルトアクションが失敗したら選択する
                false => {
                    let result = self.select(false);
                    AccClickResult::new(result, AccClickReason::DefaultActionAndSelect(action))
                },
            }
        } else {
            let result = self.select(false);
            AccClickResult::new(result, AccClickReason::Select)
        };
        result
    }
    fn invoke_default_action(&self, check: bool) -> bool {
        unsafe {
            let varchild = self.get_varchild();
            match self.get_role() {
                Some(role) => match role {
                    AccRole::CheckButton |
                    AccRole::MenuItem => if check {
                        // チェック状態にする
                        if self.is_checked() {
                            // すでにチェック済みなのでなにもしない
                            true
                        } else {
                            // チェックする
                            self.obj.accDoDefaultAction(&varchild).is_ok()
                        }
                    } else {
                        // 未チェック状態にする
                        if self.is_checked() {
                            // チェックを外す
                            self.obj.accDoDefaultAction(&varchild).is_ok()
                        } else {
                            // すでに未チェックなのでなにもしない
                            true
                        }
                    }
                    _ => if check {
                        self.obj.accDoDefaultAction(&varchild).is_ok()
                    } else {
                        true
                    }
                },
                None => false,
            }
        }
    }
    fn select(&self, append: bool) -> bool {
        unsafe {
            let varchild = self.get_varchild();
            let flag = if append {
                SELFLAG_ADDSELECTION
            } else {
                SELFLAG_TAKEFOCUS|SELFLAG_TAKESELECTION
            } as i32;
            self.obj.accSelect(flag, &varchild).is_ok()
        }
    }
    fn is_checked(&self) -> bool {
        if let Some(state) = self.get_state(None) {
            (state as u32 & STATE_SYSTEM_CHECKED) > 0
        } else {
            false
        }
    }
    pub fn get_point(&self) -> Option<(i32, i32)>{
        unsafe {
            let varchild = self.get_varchild();
            let mut pxleft = 0;
            let mut pytop = 0;
            let mut pcxwidth = 0;
            let mut pcyheight = 0;
            self.obj.accLocation(&mut pxleft, &mut pytop, &mut pcxwidth, &mut pcyheight, &varchild).ok()?;
            let x = pxleft + pcxwidth / 2;
            let y = pytop + pcyheight / 2;
            Some((x, y))
        }
    }
    pub fn get_role(&self) -> Option<AccRole> {
        unsafe {
            let varchild = self.get_varchild();
            self.obj.get_accRole(&varchild)
                    .map(|variant| {
                        let role = i32::from_variant(variant);
                        role.into()
                    })
                    .ok()
        }
    }
    pub fn get_value(&self) -> Option<String> {
        unsafe {
            let varchild = self.get_varchild();
            self.obj.get_accValue(&varchild)
                    .map(|bstr| bstr.to_string())
                    .ok()
        }
    }
    pub fn set_value(&self, value: &str) {
        unsafe {
            let wide: Vec<u16> = value.to_wide_null_terminated();
            let szvalue = BSTR::from_wide(&wide);
            let varchild = self.get_varchild();
            let _ = self.obj.put_accValue(&varchild, &szvalue);
        }
    }
    // pub fn select(&self) -> AccResult<()> {
    //     unsafe {
    //         let varchild = self.get_varchild();
    //         self.obj.accSelect(SELFLAG_TAKEFOCUS as i32, &varchild)?;
    //     }
    //     Ok(())
    // }

    pub fn get_acc_static_text(&self) -> Vec<Self> {
        self.get_children(AccType::StaticText)
    }
    pub fn get_acc_click(&self) -> Vec<Self> {
        self.get_children(AccType::Clickable(false))
    }
    pub fn get_acc_click2(&self) -> Vec<Self> {
        self.get_children(AccType::Clickable(true))
    }
    pub fn get_acc_edit(&self) -> Vec<Self> {
        self.get_children(AccType::Editable)
    }
    pub fn get_acc_all(&self) -> Vec<Self> {
        self.get_children(AccType::Any)
    }

    fn get_classname(&self) -> Option<String> {
        let hwnd = self.get_hwnd()?;
        let class_name = get_class_name(hwnd);
        Some(class_name)
    }
    fn is_listview(&self) -> bool {
        if let Some(class_name) = self.get_classname() {
            class_name.eq(WC_LISTVIEW)
        } else {
            false
        }
    }

    pub fn get_children(&self, acc_type: AccType) -> Vec<Self> {
        let mut children = vec![];
        self.enum_children(&mut children, &acc_type);
        children
    }
    pub fn get_all_children(&self) -> AccTree {
        unsafe {
            let mut tree = AccTree::from_acc(self);
            let cnt = self.get_child_count() as usize;
            if cnt > 0 {
                let mut rgvarchildren: Vec<VARIANT> = Vec::with_capacity(cnt);
                rgvarchildren.resize(cnt, VARIANT::default());
                let mut pcobtained = 0;
                if AccessibleChildren(&self.obj, 0, &mut rgvarchildren, &mut pcobtained).is_err() {
                    return tree;
                }
                for variant in rgvarchildren {
                    let variant00 = &variant.Anonymous.Anonymous;
                    let vt = variant00.vt;
                    match VARENUM(vt as i32) {
                        VT_I4 => {
                            let id = variant00.Anonymous.lVal;
                            match self.obj.get_accChild(&variant) {
                                Ok(disp) => {
                                    if let Some(acc) = Self::from_idispatch(disp, id) {
                                        let branch = acc.get_all_children();
                                        tree.push(branch);
                                    }
                                },
                                Err(e) => {
                                    if let HRESULT(0) = e.code() {
                                        let acc = Self::new(self.obj.to_owned(), id);
                                        if acc.is_visible(Some(&variant)).unwrap_or(false) {
                                            let leaf = AccTree::from_acc(&acc);
                                            tree.push(leaf);
                                        }
                                    }
                                },
                            }
                        },
                        VT_DISPATCH => {
                            if let Some(acc) = Self::from_pdispval(&variant00.Anonymous.pdispVal) {
                                let branch = acc.get_all_children();
                                tree.push(branch);
                            }
                        },
                        _ => {},
                    }
                }
            }
            tree
        }
    }
    fn enum_children(&self, children: &mut Vec<Self>, acc_type: &AccType) {
        unsafe {
            let cnt = self.get_child_count() as usize;
            if cnt > 0 {
                println!("\u{001b}[31mparent: \u{001b}[36m{:?} \u{001b}[33m{:?}\u{001b}[0m", self.get_name(), self.get_role());
                let mut rgvarchildren: Vec<VARIANT> = Vec::with_capacity(cnt);
                rgvarchildren.resize(cnt, VARIANT::default());
                let mut pcobtained = 0;
                if AccessibleChildren(&self.obj, 0, &mut rgvarchildren, &mut pcobtained).is_err() {
                    return;
                }

                for variant in rgvarchildren {
                    let variant00 = &variant.Anonymous.Anonymous;
                    let vt = variant00.vt;
                    let maybe_acc = match VARENUM(vt as i32) {
                        VT_I4 => {
                            let id = variant00.Anonymous.lVal;
                            match self.obj.get_accChild(&variant) {
                                Ok(disp) => {
                                    Self::from_idispatch(disp, id)
                                },
                                Err(e) => {
                                    if let HRESULT(0) = e.code() {
                                        let acc = Self::new(self.obj.to_owned(), id);
                                        if acc.is_valid_type(acc_type).unwrap_or(false) {
                                            println!("child1: \u{001b}[36m{:?} \u{001b}[33m{:?}\u{001b}[0m", acc.get_name(), acc.get_role());
                                            children.push(acc);
                                        }
                                    }
                                    None
                                },
                            }
                        },
                        VT_DISPATCH => {
                            Self::from_pdispval(&variant00.Anonymous.pdispVal)
                        },
                        _ => None,
                    };

                    if let Some(acc) = maybe_acc {
                        println!("\u{001b}[32mchild2: \u{001b}[36m{:?} \u{001b}[33m{:?}\u{001b}[0m {:?}", acc.get_name(), acc.get_role(), acc.get_value());
                        if acc.is_valid_type(acc_type).unwrap_or(false) {
                            children.push(acc.clone());
                        }
                        acc.enum_children(children, acc_type);
                    }
                }
            }
        }
    }
    fn is_valid_type(&self, acc_type: &AccType) -> Option<bool> {
        let role = self.get_role()?;
        let is_valid = match acc_type {
            AccType::StaticText => {
                role == AccRole::StaticText
            },
            AccType::Clickable(include_selectable_text) => match role {
                AccRole::ButtonDropdown |
                AccRole::ButtonDropdownGrid |
                AccRole::ButtonMenu |
                AccRole::Cell |
                AccRole::CheckButton |
                AccRole::ColumnHeader |
                AccRole::Link |
                AccRole::ListItem |
                AccRole::MenuItem |
                AccRole::OutlineButton |
                AccRole::OutlineItem |
                AccRole::PageTab |
                AccRole::PushButton |
                AccRole::RadioButton |
                AccRole::RowHeader |
                AccRole::SplitButton
                => self.get_name().unwrap_or_default().len() > 0,
                _ => self.is_selectable()? && *include_selectable_text,
            },
            AccType::Editable => match role {
                AccRole::Text => true,
                _ => false,
            },
            AccType::Any => true,
        };
        let result = is_valid && self.is_visible(None).unwrap_or(false);
        Some(result)
    }

    pub fn search(&self, item: &SearchItem, order: &mut u32, backwards: bool) -> Option<SearchResult> {
        unsafe {
            let varchildren = self.get_varchildren(backwards);

            for varchild in varchildren {
                if let Some(acc) = self.get_acc_from_varchild(&varchild, true) {
                    if let Some(role) = acc.get_role() {
                        if item.target.is_valid_parent_role(&role, &acc) {
                            match role {
                                AccRole::List => if let Some(found) = acc.search_list(item, order, backwards) {
                                    return Some(found)
                                }
                                AccRole::MenuBar => if let Some(found) = acc.search_menu(item, order, backwards, None) {
                                    return Some(found)
                                },
                                _ => if let Some(found) = acc.search_child(&role, item, order, backwards, None) {
                                    return Some(found);
                                }
                            }
                        }
                    }
                    if acc.has_child() {
                        // 子があればサーチ
                        if let Some(found) = acc.search(item, order, backwards) {
                            // 見つかれば終了
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
    }
    fn search_child(&self, parent: &AccRole, item: &SearchItem, order: &mut u32, backwards: bool, path: Option<String>) -> Option<SearchResult> {
        unsafe {
            let varchildren = self.get_varchildren(backwards);
            let ignore_invisible = TargetRole::ignore_invisible(&parent);
            for varchild in varchildren {
                if let Some(acc) = self.get_acc_from_varchild(&varchild, ignore_invisible) {
                    let mut new_path = None;
                    if let Some(role) = acc.get_role() {
                        let name = match acc.get_item_name() {
                            Some(name) => name,
                            None => continue,
                        };
                        match parent {
                            AccRole::Window |
                            AccRole::Client |
                            AccRole::PageTablist |
                            AccRole::ToolBar => {
                                if item.target.contains(parent, &role) {
                                    if item.matches(&name, order) {
                                        return Some(SearchResult::Acc(acc));
                                    }
                                } else {
                                    continue;
                                }
                            },
                            // treeview, menu
                            AccRole::Outline => {
                                if item.target.contains(&parent, &role) {
                                    if item.is_path() {
                                        new_path = match &path {
                                            Some(p) => {
                                                let path = format!("{p}\\{name}");
                                                if item.matches(&path, order) {
                                                    return Some(SearchResult::Acc(acc));
                                                }
                                                Some(path)
                                            },
                                            None => {
                                                if item.matches(&name, order) {
                                                    return Some(SearchResult::Acc(acc));
                                                }
                                                Some(name)
                                            },
                                        };
                                    } else {
                                        if item.matches(&name, order) {
                                            return Some(SearchResult::Acc(acc));
                                        }
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }
                    if acc.has_child() {
                        if let Some(found) = self.search_child(parent, item, order, backwards, new_path) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }

    }

    fn search_list(&self, item: &SearchItem, order: &mut u32, backwards: bool) ->  Option<SearchResult> {
        let varchildren = self.get_varchildren(backwards);
        let list_items = varchildren.iter()
            .map(|varchild| self.get_acc_from_varchild(&varchild, false));
        let maybe_header = list_items.clone().find_map(|maybe_acc| {
                match maybe_acc {
                    Some(acc) => match acc.get_role() {
                        Some(role) => if role.eq(&AccRole::Window) {
                            Some(acc)
                        } else {
                            None
                        },
                        None => None,
                    },
                    None => None,
                }
            });
        if let Some(header) = maybe_header {
            // ListView
            if let Some(found) = header.search_listview_header(item, order, backwards) {
                return Some(found);
            }
            for maybe_acc in list_items {
                if let Some(acc) = maybe_acc {
                    if let Some(AccRole::ListItem) = acc.get_role() {
                        let name = match acc.get_name() {
                            Some(name) => name,
                            None => continue,
                        };
                        if item.matches(&name, order) {
                            return Some(SearchResult::Acc(acc));
                        } else {
                            if let Some(found) = acc.search_listview_items(item, order, backwards) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
            None
        } else {
            // List
            let mut group = vec![];
            for maybe_acc in list_items {
                if let Some(acc) = maybe_acc {
                    if let Some(AccRole::ListItem) = acc.get_role() {
                        let name = match acc.get_item_name() {
                            Some(name) => name,
                            None => continue,
                        };
                        if item.matches(&name, order) {
                            if item.is_group() {
                                // リストの複数選択
                                group.push(acc);
                                continue;
                            } else {
                                return Some(SearchResult::Acc(acc));
                            }
                        }
                    }
                }
            }
            if group.is_empty() {
                None
            } else {
                Some(SearchResult::Group(group))
            }
        }
    }
    fn search_listview_items(&self, item: &SearchItem, order: &mut u32, backwards: bool) ->  Option<SearchResult> {
        if self.has_child() {
            let varchildren = self.get_varchildren(backwards);
            for varchild in varchildren {
                if let Some(acc) = self.get_acc_from_varchild(&varchild, false) {
                    if let Some(role) = acc.get_role() {
                        match role {
                            AccRole::Text |
                            AccRole::StaticText => {
                                match acc.get_item_name() {
                                    Some(name) => {
                                        if item.matches(&name, order) {
                                            return Some(SearchResult::Acc(acc));
                                        }
                                    },
                                    None => {},
                                }
                            }
                            _ => if let Some(found) = acc.search_listview_items(item, order, backwards) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
        }
        None
    }
    fn search_listview_header(&self, item: &SearchItem, order: &mut u32, backwards: bool) ->  Option<SearchResult> {
        let varchildren = self.get_varchildren(backwards);
        for varchild in varchildren {
            if let Some(acc) = self.get_acc_from_varchild(&varchild, true) {
                if let Some(AccRole::ColumnHeader) = self.get_role() {
                    let name = match acc.get_name() {
                        Some(name) => name,
                        None => continue,
                    };
                    if item.matches(&name, order) {
                        return Some(SearchResult::Acc(acc));
                    }
                } else {
                    if acc.has_child() {
                        if let Some(found) = acc.search_listview_header(item, order, backwards) {
                            return Some(found);
                        }
                    }
                }
            }
        }
        None
    }
    fn search_menu(&self, item: &SearchItem, order: &mut u32, backwards: bool, path: Option<String>) ->  Option<SearchResult> {
        let varchildren = self.get_varchildren(backwards);
        for varchild in varchildren {
            if let Some(acc) = self.get_acc_from_varchild(&varchild, false) {
                if let Some(role) = acc.get_role() {
                    match role {
                        AccRole::MenuItem => {
                            if let Some(name) = acc.get_name() {
                                let name = if item.is_path() {
                                    if let Some(p) = &path {
                                        format!("{p}\\{name}")
                                    } else {
                                         name
                                    }
                                } else {
                                    name
                                };
                                if item.matches(&name, order) {
                                    return Some(SearchResult::Menu(acc));
                                }
                                if acc.has_child() {
                                    if let Some(found) = acc.search_menu(item, order, backwards, Some(name)) {
                                        return Some(found);
                                    }
                                }
                            }
                        },
                        AccRole::Window |
                        AccRole::MenuPopup => {
                            if acc.has_child() {
                                if let Some(found) = acc.search_menu(item, order, backwards, path.clone()) {
                                    return Some(found);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        None
    }

    fn get_varchildren(&self, backwards: bool) -> Vec<VARIANT> {
        let cnt = self.get_child_count() as usize;
        let mut rgvarchildren = vec![VARIANT::default(); cnt];
        let mut pcobtained = 0;
        if cnt > 0 {
            unsafe {
                AccessibleChildren(&self.obj, 0, &mut rgvarchildren, &mut pcobtained);
            }
            if backwards {
                rgvarchildren.reverse();
            }
        }
        rgvarchildren
    }
    fn get_acc_from_varchild(&self, varchild: &VARIANT, ignore_invisible: bool) -> Option<Self> {
        unsafe {
            let variant00 = &varchild.Anonymous.Anonymous;
            let vt = VARENUM(variant00.vt as i32);
            match vt {
                VT_I4 => {
                    let id = variant00.Anonymous.lVal;
                    let child = unsafe { self.obj.get_accChild(varchild) };
                    match child {
                        Ok(disp) => Self::from_idispatch(disp, id),//.map(|acc| (acc, true)),
                        Err(e) => if let HRESULT(0) = e.code() {
                            if ignore_invisible {
                                if self.is_visible(Some(varchild)).unwrap_or(false) {
                                    let acc = Self::new(self.obj.clone(), id);
                                    Some(acc)
                                } else {
                                    None
                                }
                            } else {
                                let acc = Self::new(self.obj.clone(), id);
                                Some(acc)
                            }
                        } else {
                            None
                        },
                    }
                },
                VT_DISPATCH => {
                    let disp = &variant00.Anonymous.pdispVal;
                    Self::from_pdispval(disp)//.map(|acc| (acc, true))
                },
                _ => None
            }
        }
    }
    pub fn get_state(&self, varchild: Option<&VARIANT>) -> Option<i32> {
        unsafe {
            let state = match varchild {
                Some(varchild) => self.obj.get_accState(varchild).ok(),
                None => {
                    let varchild = self.get_varchild();
                    self.obj.get_accState(&varchild).ok()
                },
            };
            if let Some(variant) = state {
                let variant00 = variant.Anonymous.Anonymous;
                if VARENUM(variant00.vt as i32) == VT_I4 {
                    Some(variant00.Anonymous.lVal)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
    fn is_visible(&self, varchild: Option<&VARIANT>) -> Option<bool> {
        let is_visible = match self.get_role()? {
            // 特定のロールは可視・不可視に関わらず許可
            AccRole::ListItem => true,
            _ => {
                // 可視なら許可
                match self.get_state(varchild) {
                    Some(state) => (state & STATE_SYSTEM_INVISIBLE.0 as i32) == 0,
                    None => false,
                }
            }
        };
        Some(is_visible)
    }
    fn is_selectable(&self) -> Option<bool> {
        let is_selectable = match self.get_state(None) {
            Some(state) => (state & STATE_SYSTEM_SELECTABLE as i32) > 0,
            None => false,
        };
        Some(is_selectable)
    }
    fn from_pdispval(pdispval: &Option<IDispatch>) -> Option<Self> {
        match pdispval {
            Some(disp) => {
                disp.cast::<IAccessible>()
                    .ok()
                    .map(|obj| Self { obj, id: None, has_child: true })
            },
            None => None,
        }
    }
    fn from_idispatch(disp: IDispatch, id: i32) -> Option<Self> {
        disp.cast::<IAccessible>()
            .ok()
            .map(|obj| Self { obj, id: Some(id), has_child: true })
    }
}


#[derive(Debug)]
pub struct AccClickResult(bool, AccClickReason);
#[derive(Debug)]
pub enum AccClickReason {
    /// デフォルトアクションを実行
    DefaultAction(String),
    /// 選択
    Select,
    /// デフォルトアクションに失敗して選択した
    DefaultActionAndSelect(String)
}
impl AccClickResult {
    fn new(result: bool, reason: AccClickReason) -> Self {
        Self(result, reason)
    }
    pub fn as_bool(&self) -> bool {
        self.0
    }
}

pub enum SearchResult {
    Acc(Acc),
    Group(Vec<Acc>),
    // Route(Vec<Acc>)
    Menu(Acc)
}
impl SearchResult {
    pub fn get_hwnd(&self) -> Option<HWND> {
        if let Self::Acc(acc) = self {
            acc.get_hwnd()
        } else {
            None
        }
    }
    pub fn get_point(&self) -> Option<(i32, i32)> {
        match self {
            SearchResult::Acc(acc) => acc.get_point(),
            SearchResult::Group(group) => {
                if let Some(last) = group.last() {
                    last.get_point()
                } else {
                    None
                }
            },
            SearchResult::Menu(_) => None,

        }
    }
    pub fn click(&self, check: bool) -> bool {
        match self {
            SearchResult::Acc(acc) => acc.click(check).as_bool(),
            SearchResult::Group(group) => {
                group.into_iter()
                    .map(|acc| acc.select(true))
                    .reduce(|a, b| a && b)
                    .unwrap_or(false)
            },
            SearchResult::Menu(acc) => {
                acc.click(check).as_bool()
            },
        }
    }
}


#[derive(Debug)]
pub struct SearchItem {
    name: String,
    short: bool,
    target: TargetRole,
    group: Vec<String>,
}

impl SearchItem {
    fn new(name: String, short: bool, target: TargetRole) -> Self {
        let (name, group) = if name.contains('\t') {
            (String::new(), name.split('\t').map(|s| s.to_string()).collect())
        } else {
            (name, vec![])
        };
        Self { name, short, target, group }
    }
    pub fn from_clkitem(item: &ClkItem) -> Self {
        let target = TargetRole::from_clkitem(item);
        Self::new(item.name.to_string(), item.short, target)
    }
    // pub fn generate_listview_searcher(&self) -> Self {
    //     Self {
    //         name: self.name.clone(),
    //         short: self.short,
    //         target: SearchTarget::ListViewItem.get_target_role()
    //     }
    // }
    fn matches(&self, other: &String, order: &mut u32) -> bool {
        if self.group.is_empty() {
            if match_title(other, &self.name, self.short) {
                *order -= 1;
                *order < 1
            } else {
                false
            }
        } else {
            self.group.iter()
                .find(|name| {
                    match_title(other, name, self.short)
                })
                .is_some()
        }
    }
    fn is_group(&self) -> bool {
        ! self.group.is_empty()
    }
    fn is_path(&self) -> bool {
        self.name.contains('\\')
    }

}

#[derive(Debug, Default)]
struct TargetRole {
    parent: Vec<AccRole>,
    // children: Vec<AccRole>
}
impl TargetRole {
    fn new(parent: Vec<AccRole>) -> Self {
        Self { parent }
    }
    fn from_clkitem(item: &ClkItem) -> Self {
        let mut parent = vec![];
        if item.target.button {
            parent.push(AccRole::Window);
        }
        if item.target.list {
            parent.push(AccRole::List);
        }
        if item.target.tab {
            parent.push(AccRole::PageTablist);
        }
        if item.target.menu {
            parent.push(AccRole::MenuBar);
        }
        if item.target.treeview {
            parent.push(AccRole::Outline);
        }
        if item.target.listview {
            parent.push(AccRole::List);
        }
        if item.target.toolbar {
            parent.push(AccRole::ToolBar);
        }
        if item.target.link {
            parent.push(AccRole::Client);
        }
        Self::new(parent)
    }
    fn match_parent(&self, role: &AccRole) -> bool {
        self.parent.iter()
            .find(|parent| parent.eq(&role))
            .is_some()
    }
    fn is_valid_parent_role(&self, role: &AccRole, acc: &Acc) -> bool {
        if self.match_parent(role) {
            let name_check = match role {
                // 該当親ロールで名前がないものは無視
                AccRole::Window |
                AccRole::Client => acc.has_valid_name(),
                // それ以外は名前をチェックしない
                _ => true,
            };
            name_check && acc.has_child()
        } else {
            false
        }

    }
    fn contains(&self, parent: &AccRole, role: &AccRole) -> bool {
        // self.children.contains(role)
        match parent {
            // ボタン類、リンク
            AccRole::Window => match role {
                AccRole::PushButton|
                AccRole::CheckButton|
                AccRole::RadioButton|
                AccRole::ButtonDropdown|
                AccRole::ButtonDropdownGrid|
                AccRole::ButtonMenu => true,
                _ => false
            },
            // リスト
            AccRole::List => AccRole::ListItem.eq(role),
            // タブ
            AccRole::PageTablist => AccRole::PageTab.eq(role),
            // メニュー
            AccRole::MenuBar => match role {
                AccRole::MenuItem|
                AccRole::MenuPopup => true,
                _ => false
            },
            // ツリービュー
            AccRole::Outline => match role {
                AccRole::OutlineItem|
                AccRole::OutlineButton => true,
                _ => false,
            },
            // リストビュー
            // ツールバー
            AccRole::ToolBar => match role {
                AccRole::PushButton => true,
                _ => false
            }
            // リンク
            AccRole::Client => AccRole::Link.eq(role),
            _ => false
        }
    }
    fn ignore_invisible(role: &AccRole) -> bool {
        match role {
            AccRole::Outline => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub enum AccType {
    StaticText,
    Clickable(bool),
    Editable,
    Any,
}

fn to_vt_i4(n: i32) -> VARIANT {
    unsafe {
        let mut variant = VARIANT::default();
        VariantInit(&mut variant);
        let mut variant00 = VARIANT_0_0::default();
        variant00.vt = VT_I4.0 as u16;
        variant00.Anonymous.lVal = n;
        variant.Anonymous.Anonymous = ManuallyDrop::new(variant00);
        variant
    }
}

trait I32Ext {
    fn into_variant(&self) -> VARIANT;
    fn from_variant(variant: VARIANT) -> Self;
}

impl I32Ext for i32 {
    fn into_variant(&self) -> VARIANT {
        to_vt_i4(*self)
    }

    fn from_variant(variant: VARIANT) -> Self {
        unsafe {
            let variant00 = &variant.Anonymous.Anonymous;
            let vt = variant00.vt;
            match VARENUM(vt as i32) {
                VT_I4 => variant00.Anonymous.lVal,
                _ => 0
            }
        }
    }
}



#[derive(Debug, PartialEq)]
pub enum AccRole {
    Alert,
    Animation,
    Application,
    Border,
    ButtonDropdown,
    ButtonDropdownGrid,
    ButtonMenu,
    Caret,
    Cell,
    Character,
    Chart,
    CheckButton,
    Client,
    Clock,
    Column,
    ColumnHeader,
    Combobox,
    Cursor,
    Diagram,
    Dial,
    Dialog,
    Document,
    Droplist,
    Equation,
    Graphic,
    Grip,
    Grouping,
    HelpBalloon,
    HotkeyField,
    Indicator,
    Ipaddress,
    Link,
    List,
    ListItem,
    MenuBar,
    MenuItem,
    MenuPopup,
    Outline,
    OutlineButton,
    OutlineItem,
    PageTab,
    PageTablist,
    Pane,
    ProgressBar,
    PropertyPage,
    PushButton,
    RadioButton,
    Row,
    RowHeader,
    ScrollBar,
    Separator,
    Slider,
    Sound,
    SpinButton,
    SplitButton,
    StaticText,
    StatusBar,
    Table,
    Text,
    TitleBar,
    ToolBar,
    Tooltip,
    Whitespace,
    Window,
    Unknown(i32),
}

impl From<i32> for AccRole {
    fn from(n: i32) -> Self {
        match n as u32 {
            ROLE_SYSTEM_ALERT => Self::Alert,
            ROLE_SYSTEM_ANIMATION => Self::Animation,
            ROLE_SYSTEM_APPLICATION => Self::Application,
            ROLE_SYSTEM_BORDER => Self::Border,
            ROLE_SYSTEM_BUTTONDROPDOWN => Self::ButtonDropdown,
            ROLE_SYSTEM_BUTTONDROPDOWNGRID => Self::ButtonDropdownGrid,
            ROLE_SYSTEM_BUTTONMENU => Self::ButtonMenu,
            ROLE_SYSTEM_CARET => Self::Caret,
            ROLE_SYSTEM_CELL => Self::Cell,
            ROLE_SYSTEM_CHARACTER => Self::Character,
            ROLE_SYSTEM_CHART => Self::Chart,
            ROLE_SYSTEM_CHECKBUTTON => Self::CheckButton,
            ROLE_SYSTEM_CLIENT => Self::Client,
            ROLE_SYSTEM_CLOCK => Self::Clock,
            ROLE_SYSTEM_COLUMN => Self::Column,
            ROLE_SYSTEM_COLUMNHEADER => Self::ColumnHeader,
            ROLE_SYSTEM_COMBOBOX => Self::Combobox,
            ROLE_SYSTEM_CURSOR => Self::Cursor,
            ROLE_SYSTEM_DIAGRAM => Self::Diagram,
            ROLE_SYSTEM_DIAL => Self::Dial,
            ROLE_SYSTEM_DIALOG => Self::Dialog,
            ROLE_SYSTEM_DOCUMENT => Self::Document,
            ROLE_SYSTEM_DROPLIST => Self::Droplist,
            ROLE_SYSTEM_EQUATION => Self::Equation,
            ROLE_SYSTEM_GRAPHIC => Self::Graphic,
            ROLE_SYSTEM_GRIP => Self::Grip,
            ROLE_SYSTEM_GROUPING => Self::Grouping,
            ROLE_SYSTEM_HELPBALLOON => Self::HelpBalloon,
            ROLE_SYSTEM_HOTKEYFIELD => Self::HotkeyField,
            ROLE_SYSTEM_INDICATOR => Self::Indicator,
            ROLE_SYSTEM_IPADDRESS => Self::Ipaddress,
            ROLE_SYSTEM_LINK => Self::Link,
            ROLE_SYSTEM_LIST => Self::List,
            ROLE_SYSTEM_LISTITEM => Self::ListItem,
            ROLE_SYSTEM_MENUBAR => Self::MenuBar,
            ROLE_SYSTEM_MENUITEM => Self::MenuItem,
            ROLE_SYSTEM_MENUPOPUP => Self::MenuPopup,
            ROLE_SYSTEM_OUTLINE => Self::Outline,
            ROLE_SYSTEM_OUTLINEBUTTON => Self::OutlineButton,
            ROLE_SYSTEM_OUTLINEITEM => Self::OutlineItem,
            ROLE_SYSTEM_PAGETAB => Self::PageTab,
            ROLE_SYSTEM_PAGETABLIST => Self::PageTablist,
            ROLE_SYSTEM_PANE => Self::Pane,
            ROLE_SYSTEM_PROGRESSBAR => Self::ProgressBar,
            ROLE_SYSTEM_PROPERTYPAGE => Self::PropertyPage,
            ROLE_SYSTEM_PUSHBUTTON => Self::PushButton,
            ROLE_SYSTEM_RADIOBUTTON => Self::RadioButton,
            ROLE_SYSTEM_ROW => Self::Row,
            ROLE_SYSTEM_ROWHEADER => Self::RowHeader,
            ROLE_SYSTEM_SCROLLBAR => Self::ScrollBar,
            ROLE_SYSTEM_SEPARATOR => Self::Separator,
            ROLE_SYSTEM_SLIDER => Self::Slider,
            ROLE_SYSTEM_SOUND => Self::Sound,
            ROLE_SYSTEM_SPINBUTTON => Self::SpinButton,
            ROLE_SYSTEM_SPLITBUTTON => Self::SplitButton,
            ROLE_SYSTEM_STATICTEXT => Self::StaticText,
            ROLE_SYSTEM_STATUSBAR => Self::StatusBar,
            ROLE_SYSTEM_TABLE => Self::Table,
            ROLE_SYSTEM_TEXT => Self::Text,
            ROLE_SYSTEM_TITLEBAR => Self::TitleBar,
            ROLE_SYSTEM_TOOLBAR => Self::ToolBar,
            ROLE_SYSTEM_TOOLTIP => Self::Tooltip,
            ROLE_SYSTEM_WHITESPACE => Self::Whitespace,
            ROLE_SYSTEM_WINDOW => Self::Window,
            _ => Self::Unknown(n)
        }
    }
}

pub struct AccTree(Acc, Vec<Self>);

impl AccTree {
    fn from_acc(acc: &Acc) -> Self {
        Self(acc.clone(), vec![])
    }
    fn push(&mut self, tree: Self) {
        self.1.push(tree);
    }
    // pub fn is_leaf(&self) -> bool {
    //     self.1.len() == 0
    // }
    // pub fn is_branch(&self) -> bool {
    //     self.1.len() > 0
    // }
}

impl std::fmt::Debug for AccTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role = self.0.get_role().unwrap_or(AccRole::Unknown(0));
        let class = self.0.get_classname().unwrap_or_default();
        let name = self.0.get_name().unwrap_or_default();
        let value = self.0.get_value().unwrap_or_default();
        let name = format!("{name} [{class}]");
        f.debug_struct(&name)
          .field("Role", &role)
          .field("Value", &value)
          .field("Child", &self.1)
          .finish()
    }
}