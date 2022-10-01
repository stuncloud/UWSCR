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
                OBJID_CLIENT,
            },
            Accessibility::{
                ROLE_SYSTEM_ALERT, ROLE_SYSTEM_ANIMATION, ROLE_SYSTEM_APPLICATION, ROLE_SYSTEM_BORDER, ROLE_SYSTEM_BUTTONDROPDOWN, ROLE_SYSTEM_BUTTONDROPDOWNGRID, ROLE_SYSTEM_BUTTONMENU, ROLE_SYSTEM_CARET, ROLE_SYSTEM_CELL, ROLE_SYSTEM_CHARACTER, ROLE_SYSTEM_CHART, ROLE_SYSTEM_CHECKBUTTON, ROLE_SYSTEM_CLIENT, ROLE_SYSTEM_CLOCK, ROLE_SYSTEM_COLUMN, ROLE_SYSTEM_COLUMNHEADER, ROLE_SYSTEM_COMBOBOX, ROLE_SYSTEM_CURSOR, ROLE_SYSTEM_DIAGRAM, ROLE_SYSTEM_DIAL, ROLE_SYSTEM_DIALOG, ROLE_SYSTEM_DOCUMENT, ROLE_SYSTEM_DROPLIST, ROLE_SYSTEM_EQUATION, ROLE_SYSTEM_GRAPHIC, ROLE_SYSTEM_GRIP, ROLE_SYSTEM_GROUPING, ROLE_SYSTEM_HELPBALLOON, ROLE_SYSTEM_HOTKEYFIELD, ROLE_SYSTEM_INDICATOR, ROLE_SYSTEM_IPADDRESS, ROLE_SYSTEM_LINK, ROLE_SYSTEM_LIST, ROLE_SYSTEM_LISTITEM, ROLE_SYSTEM_MENUBAR, ROLE_SYSTEM_MENUITEM, ROLE_SYSTEM_MENUPOPUP, ROLE_SYSTEM_OUTLINE, ROLE_SYSTEM_OUTLINEBUTTON, ROLE_SYSTEM_OUTLINEITEM, ROLE_SYSTEM_PAGETAB, ROLE_SYSTEM_PAGETABLIST, ROLE_SYSTEM_PANE, ROLE_SYSTEM_PROGRESSBAR, ROLE_SYSTEM_PROPERTYPAGE, ROLE_SYSTEM_PUSHBUTTON, ROLE_SYSTEM_RADIOBUTTON, ROLE_SYSTEM_ROW, ROLE_SYSTEM_ROWHEADER, ROLE_SYSTEM_SCROLLBAR, ROLE_SYSTEM_SEPARATOR, ROLE_SYSTEM_SLIDER, ROLE_SYSTEM_SOUND, ROLE_SYSTEM_SPINBUTTON, ROLE_SYSTEM_SPLITBUTTON, ROLE_SYSTEM_STATICTEXT, ROLE_SYSTEM_STATUSBAR, ROLE_SYSTEM_TABLE, ROLE_SYSTEM_TEXT, ROLE_SYSTEM_TITLEBAR, ROLE_SYSTEM_TOOLBAR, ROLE_SYSTEM_TOOLTIP, ROLE_SYSTEM_WHITESPACE, ROLE_SYSTEM_WINDOW,
                IAccessible,
                AccessibleObjectFromWindow,
                AccessibleChildren,
                WindowFromAccessibleObject,
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
    id: Option<i32>
}

#[allow(unused)]
impl Acc {
    pub fn new(obj: IAccessible, id: Option<i32>) -> Self {
        Self { obj, id }
    }
    pub fn from_hwnd(hwnd: HWND) -> Option<Self> {
        if let HWND(0) = hwnd {
            None
        } else {
            unsafe {
                let mut ppvobject = null_mut::<IAccessible>() as *mut c_void;
                match AccessibleObjectFromWindow(hwnd, OBJID_CLIENT.0 as u32, &IAccessible::IID, &mut ppvobject) {
                    Ok(_) => {
                        let obj: IAccessible = transmute(ppvobject);
                        Some(Acc {obj, id: None})
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
    fn get_varchild(&self) -> VARIANT {
        self.id.unwrap_or(0).into_variant()
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
    pub fn invoke_default_action(&self, check: bool) -> bool {
        unsafe {
            if self.get_default_action().unwrap_or_default().is_empty() {
                // デフォルトアクションがない場合は失敗
                false
            } else {
                let varchild = self.get_varchild();
                match self.get_role() {
                    Some(role) => match role {
                        AccRole::Checkbutton => if check {
                            // チェック状態にする
                            if self.is_checked() {
                                // すでにチェック済みなのでなにもしない
                                self.has_valid_default_action()
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
                                self.has_valid_default_action()
                            }
                        }
                        _ => if check {
                            self.obj.accDoDefaultAction(&varchild).is_ok()
                        } else {
                            self.has_valid_default_action()
                        }
                    },
                    None => false,
                }
            }
        }
    }
    fn is_checked(&self) -> bool {
        if let Some(state) = self.get_state(None) {
            (state as u32 & STATE_SYSTEM_CHECKED) > 0
        } else {
            false
        }
    }
    fn has_valid_default_action(&self) -> bool {
        if let Some(action) = self.get_default_action() {
            action.len() > 0
        } else {
            false
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
                                    if let Some(acc) = Self::from_idispatch(disp, Some(id)) {
                                        let branch = acc.get_all_children();
                                        tree.push(branch);
                                    }
                                },
                                Err(e) => {
                                    if let HRESULT(0) = e.code() {
                                        let acc = Self::new(self.obj.to_owned(), Some(id));
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
                                    Self::from_idispatch(disp, Some(id))
                                },
                                Err(e) => {
                                    if let HRESULT(0) = e.code() {
                                        let acc = Self::new(self.obj.to_owned(), Some(id));
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
                role == AccRole::Statictext
            },
            AccType::Clickable(include_selectable_text) => match role {
                AccRole::Buttondropdown |
                AccRole::Buttondropdowngrid |
                AccRole::Buttonmenu |
                AccRole::Cell |
                AccRole::Checkbutton |
                AccRole::Columnheader |
                AccRole::Link |
                AccRole::Listitem |
                AccRole::Menuitem |
                AccRole::Outlinebutton |
                AccRole::Outlineitem |
                AccRole::Pagetab |
                AccRole::Pushbutton |
                AccRole::Radiobutton |
                AccRole::Rowheader |
                AccRole::Splitbutton
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

    pub fn search(&self, item: &SearchItem, order: &mut u32, backwards: bool) -> Option<Self> {
        unsafe {
            let cnt = self.get_child_count() as usize;
            if cnt == 0 {
                return None;
            }

            let mut rgvarchildren = vec![];
            rgvarchildren.resize(cnt, VARIANT::default());
            let mut pcobtained = 0;

            if AccessibleChildren(&self.obj, 0, &mut rgvarchildren, &mut pcobtained).is_err() {
                return None;
            }

            if backwards {
                rgvarchildren.reverse();
            }

            for variant in rgvarchildren {
                let variant00 = &variant.Anonymous.Anonymous;
                let vt = variant00.vt;
                let maybe_acc = match VARENUM(vt as i32) {
                    VT_I4 => {
                        let id = variant00.Anonymous.lVal;
                        match self.obj.get_accChild(&variant) {
                            Ok(disp) => {
                                Self::from_idispatch(disp, Some(id))
                                    .map(|acc| (acc, true))
                            },
                            Err(e) => {
                                if let HRESULT(0) = e.code() {
                                    if let Some(true) = self.is_visible(Some(&variant)) {
                                        let acc = Self::new(self.obj.clone(), Some(id));
                                        Some((acc, false))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            },
                        }
                    },
                    VT_DISPATCH => {
                        Self::from_pdispval(&variant00.Anonymous.pdispVal)
                            .map(|acc| (acc, true))
                    },
                    _ => None,
                };
                if let Some((acc, has_child)) = maybe_acc {
                    if let Some(role) = acc.get_role() {
                        // 指定ロールに含まれるか？
                        for target in &item.target {
                            if target.roles.contains(&role) {
                                if role == AccRole::Listitem && target.listview {
                                    // クラスがSysListView32であれば名前の比較を行う
                                    if let Some(class_name) = acc.get_classname() {
                                        if class_name.to_ascii_lowercase() == WC_LISTVIEW.to_ascii_lowercase() {
                                            if item.matches(&acc.get_name()) {
                                                *order -= 1;
                                                if *order < 1 {
                                                    return Some(acc);
                                                }
                                            }
                                        }
                                    }
                                    // 子のStatictextまたはTextに一致するものがあるか検索
                                    let child = item.generate_listview_searcher();
                                    if acc.search(&child, order, backwards).is_some() {
                                        return Some(acc);
                                    }
                                } else {
                                    let other = if role == AccRole::Text {
                                        acc.get_value()
                                    } else {
                                        acc.get_name()
                                    };
                                    // 名前の比較
                                    if item.matches(&other) {
                                        *order -= 1;
                                        if *order < 1 {
                                            return Some(acc);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // もともとがIDispatchの場合は子をサーチ
                    if has_child {
                        if let Some(acc) = acc.search(item, order, backwards) {
                            return Some(acc);
                        }
                    }
                }
            }
            None
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
            AccRole::Listitem => true,
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
                    .map(|obj| Self::new(obj, None))
            },
            None => None,
        }
    }
    fn from_idispatch(disp: IDispatch, id: Option<i32>) -> Option<Self> {
        disp.cast::<IAccessible>()
            .ok()
            .map(|obj| Self::new(obj, id))
    }
}

#[derive(Debug)]
pub struct SearchItem {
    name: String,
    short: bool,
    pub target: Vec<SearchTarget>,
    pub _listview: bool,
}

impl SearchItem {
    pub fn new(name: String, short: bool, target: Vec<SearchTarget>) -> Self {
        Self { name, short, target, _listview: false }
    }
    pub fn from_clkitem(item: &ClkItem) -> Self {
        let mut target = vec![];
        if item.target.button {
            target.push(SearchTarget::new(vec![
                AccRole::Pushbutton,
                AccRole::Checkbutton,
                AccRole::Radiobutton,
                AccRole::Buttondropdown,
                AccRole::Buttondropdowngrid,
                AccRole::Buttonmenu,
            ], false));
        }
        if item.target.list {
            target.push(SearchTarget::new(vec![AccRole::Listitem], false));
        }
        if item.target.tab {
            target.push(SearchTarget::new(vec![AccRole::Pagetab], false));
        }
        if item.target.menu {
            target.push(SearchTarget::new(vec![AccRole::Menuitem], false));
        }
        if item.target.treeview {
            target.push(SearchTarget::new(vec![
                AccRole::Outlinebutton,
                AccRole::Outlineitem,
            ], false));
        }
        if item.target.listview {
            target.push(SearchTarget::new(vec![
                AccRole::Listitem,
                AccRole::Rowheader,
                AccRole::Columnheader,
                AccRole::Cell,
            ], true));
        }
        if item.target.toolbar {
            target.push(SearchTarget::new(vec![AccRole::Splitbutton], false));
        }
        if item.target.link {
            target.push(SearchTarget::new(vec![AccRole::Link], false));
        }

        Self::new(item.name.to_string(), item.short, target)
    }
    pub fn generate_listview_searcher(&self) -> Self {
        Self {
            name: self.name.clone(),
            short: self.short,
            target: vec![SearchTarget::new(vec![
                AccRole::Statictext,
                AccRole::Text,
            ], false)],
            _listview: true,
        }
    }
    fn matches(&self, other: &Option<String>) -> bool {
        if let Some(title) = other {
            match_title(title, &self.name, self.short)
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct SearchTarget {
    pub roles: Vec<AccRole>,
    pub listview: bool,
}
impl SearchTarget {
    pub fn new(roles: Vec<AccRole>, listview: bool) -> Self {
        Self { roles, listview }
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
    Buttondropdown,
    Buttondropdowngrid,
    Buttonmenu,
    Caret,
    Cell,
    Character,
    Chart,
    Checkbutton,
    Client,
    Clock,
    Column,
    Columnheader,
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
    Helpballoon,
    Hotkeyfield,
    Indicator,
    Ipaddress,
    Link,
    List,
    Listitem,
    Menubar,
    Menuitem,
    Menupopup,
    Outline,
    Outlinebutton,
    Outlineitem,
    Pagetab,
    Pagetablist,
    Pane,
    Progressbar,
    Propertypage,
    Pushbutton,
    Radiobutton,
    Row,
    Rowheader,
    Scrollbar,
    Separator,
    Slider,
    Sound,
    Spinbutton,
    Splitbutton,
    Statictext,
    Statusbar,
    Table,
    Text,
    Titlebar,
    Toolbar,
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
            ROLE_SYSTEM_BUTTONDROPDOWN => Self::Buttondropdown,
            ROLE_SYSTEM_BUTTONDROPDOWNGRID => Self::Buttondropdowngrid,
            ROLE_SYSTEM_BUTTONMENU => Self::Buttonmenu,
            ROLE_SYSTEM_CARET => Self::Caret,
            ROLE_SYSTEM_CELL => Self::Cell,
            ROLE_SYSTEM_CHARACTER => Self::Character,
            ROLE_SYSTEM_CHART => Self::Chart,
            ROLE_SYSTEM_CHECKBUTTON => Self::Checkbutton,
            ROLE_SYSTEM_CLIENT => Self::Client,
            ROLE_SYSTEM_CLOCK => Self::Clock,
            ROLE_SYSTEM_COLUMN => Self::Column,
            ROLE_SYSTEM_COLUMNHEADER => Self::Columnheader,
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
            ROLE_SYSTEM_HELPBALLOON => Self::Helpballoon,
            ROLE_SYSTEM_HOTKEYFIELD => Self::Hotkeyfield,
            ROLE_SYSTEM_INDICATOR => Self::Indicator,
            ROLE_SYSTEM_IPADDRESS => Self::Ipaddress,
            ROLE_SYSTEM_LINK => Self::Link,
            ROLE_SYSTEM_LIST => Self::List,
            ROLE_SYSTEM_LISTITEM => Self::Listitem,
            ROLE_SYSTEM_MENUBAR => Self::Menubar,
            ROLE_SYSTEM_MENUITEM => Self::Menuitem,
            ROLE_SYSTEM_MENUPOPUP => Self::Menupopup,
            ROLE_SYSTEM_OUTLINE => Self::Outline,
            ROLE_SYSTEM_OUTLINEBUTTON => Self::Outlinebutton,
            ROLE_SYSTEM_OUTLINEITEM => Self::Outlineitem,
            ROLE_SYSTEM_PAGETAB => Self::Pagetab,
            ROLE_SYSTEM_PAGETABLIST => Self::Pagetablist,
            ROLE_SYSTEM_PANE => Self::Pane,
            ROLE_SYSTEM_PROGRESSBAR => Self::Progressbar,
            ROLE_SYSTEM_PROPERTYPAGE => Self::Propertypage,
            ROLE_SYSTEM_PUSHBUTTON => Self::Pushbutton,
            ROLE_SYSTEM_RADIOBUTTON => Self::Radiobutton,
            ROLE_SYSTEM_ROW => Self::Row,
            ROLE_SYSTEM_ROWHEADER => Self::Rowheader,
            ROLE_SYSTEM_SCROLLBAR => Self::Scrollbar,
            ROLE_SYSTEM_SEPARATOR => Self::Separator,
            ROLE_SYSTEM_SLIDER => Self::Slider,
            ROLE_SYSTEM_SOUND => Self::Sound,
            ROLE_SYSTEM_SPINBUTTON => Self::Spinbutton,
            ROLE_SYSTEM_SPLITBUTTON => Self::Splitbutton,
            ROLE_SYSTEM_STATICTEXT => Self::Statictext,
            ROLE_SYSTEM_STATUSBAR => Self::Statusbar,
            ROLE_SYSTEM_TABLE => Self::Table,
            ROLE_SYSTEM_TEXT => Self::Text,
            ROLE_SYSTEM_TITLEBAR => Self::Titlebar,
            ROLE_SYSTEM_TOOLBAR => Self::Toolbar,
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