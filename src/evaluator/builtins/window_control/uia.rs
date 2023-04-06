#![allow(non_upper_case_globals)]

use windows::{
    core::{BSTR},
    Win32::{
        Foundation::{HWND, POINT},
        System::{
            Com::{
                CoCreateInstance, CLSCTX_ALL,
            }
        },
        UI::{
            Accessibility::{
                IUIAutomation, IUIAutomationElement, IUIAutomationCondition,
                CUIAutomation,
                UIA_InvokePatternId, IUIAutomationInvokePattern,
                UIA_SelectionItemPatternId, IUIAutomationSelectionItemPattern,
                UIA_TogglePatternId, IUIAutomationTogglePattern, ToggleState_On, ToggleState_Off, ToggleState_Indeterminate, ToggleState,
                UIA_ExpandCollapsePatternId, IUIAutomationExpandCollapsePattern,
                UIA_ValuePatternId, IUIAutomationValuePattern,
                // UIA_TextPatternId, IUIAutomationTextPattern,
                UIA_CONTROLTYPE_ID,
                UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_RadioButtonControlTypeId,
                UIA_ListControlTypeId, UIA_ComboBoxControlTypeId, UIA_ListItemControlTypeId,
                UIA_HeaderControlTypeId, UIA_HeaderItemControlTypeId,
                UIA_TextControlTypeId,
                UIA_TabControlTypeId, UIA_TabItemControlTypeId,
                // UIA_MenuControlTypeId, UIA_MenuBarControlTypeId,
                UIA_MenuItemControlTypeId,
                UIA_TreeControlTypeId, UIA_TreeItemControlTypeId,
                ExpandCollapseState, ExpandCollapseState_Collapsed, ExpandCollapseState_Expanded, ExpandCollapseState_PartiallyExpanded,
                UIA_DataGridControlTypeId, //UIA_DataItemControlTypeId,
                UIA_ToolBarControlTypeId,
                UIA_HyperlinkControlTypeId,
                UIA_EditControlTypeId,
                TreeScope, TreeScope_Children, TreeScope_Descendants,
            }
        }
    }
};

use super::clkitem::{ClkItem, match_title};
use crate::evaluator::builtins::ThreeState;
impl From<ToggleState> for ThreeState {
    fn from(state: ToggleState) -> Self {
        match state {
            ToggleState_On => Self::True,
            ToggleState_Off => Self::False,
            _ => Self::Other
        }
    }
}


pub struct UIA {
    automation: UIAutomation,
    element: UIAElement,
}
impl UIA {
    pub fn new(hwnd: HWND) -> Option<Self> {
        let automation = UIAutomation::new()?;
        let element = automation.element_from_hwnd(hwnd)?;
        let uia = UIA { automation, element };
        Some(uia)
    }
    fn find(&self, ci: &ClkItem) -> Option<UIAFound> {
        let condition = self.automation.create_true_condition()?;
        let mut target = UIATarget::new(ci, condition);
        self.element.search(&mut target)
    }
    pub fn click(&self, ci: &ClkItem, state: &ThreeState) -> Option<UIAClickPoint> {
        let found = self.find(ci)?;
        match found {
            UIAFound::Single(element, id) => {
                match id {
                    UIA_CheckBoxControlTypeId => element.check(state),
                    UIA_TabControlTypeId |
                    UIA_TreeControlTypeId => {
                        match element.get_expand_collapse_state() {
                            Some(ExpandCollapseState_Collapsed) => element.expand(),
                            Some(ExpandCollapseState_Expanded) |
                            Some(ExpandCollapseState_PartiallyExpanded) => element.collapse(),
                            _ => element.select()
                        }
                    },
                    _ => if state.as_bool() {
                        element.click()
                    } else {
                        let point = element.get_clickable_point();
                        Some(UIAClickPoint(point))
                    }
                }
            },
            UIAFound::Multi(elements) => {
                let point = elements.into_iter()
                    .map(|elem| elem.multi_select())
                    .last()
                    .unwrap_or_default();
                point
            },
            UIAFound::ListViewItem(list, text) => {
                list.select()?;
                text.get_clickable_point().map(|point| UIAClickPoint(Some(point)))
            }
        }
    }
    pub fn get_point(&self, ci: &ClkItem) -> Option<(i32, i32)> {
        match self.find(ci)? {
            UIAFound::Single(element, _) => element.get_clickable_point(),
            UIAFound::Multi(elements) => {
                let point = elements.into_iter()
                    .map(|elem| elem.get_clickable_point())
                    .last()
                    .unwrap_or_default();
                point
            },
            UIAFound::ListViewItem(_, text) => text.get_clickable_point()
        }
    }
    pub fn sendstr(hwnd: HWND, nth: u32, str: String) {
        let Some(uia) = Self::new(hwnd) else {return;};
        let Some(condition) = uia.automation.create_true_condition() else {return;};
        let Some(elements) = uia.element.find_all(TreeScope_Descendants, &condition) else {return;};
        let mut edit = elements.filter(|e| e.filter_by_type(UIA_EditControlTypeId));
        let found = if nth > 0 {
            edit.nth(nth as usize - 1)
        } else {
            edit.find(|e| e.is_focused())
        };
        if let Some(element) = found {
            element.write(str);
        }
    }
    pub fn chkbtn(hwnd: HWND, name: String, nth: u32) -> Option<i32> {
        let uia = Self::new(hwnd)?;
        let condition = uia.automation.create_true_condition()?;
        let found = uia.element.find_all(TreeScope_Descendants, &condition)?
            .filter(|e| e.filter_by_type(UIA_CheckBoxControlTypeId) || e.filter_by_type(UIA_MenuItemControlTypeId))
            .filter(|e| {
                if let Some(ename) = e.get_name() {
                    match_title(&ename, &name, true)
                } else {
                    false
                }
            })
            .nth(nth as usize - 1)?;
        found.get_check_state().map(|s| s.0)
    }
}

/// UIA系構造体につけると便利になる
trait UIATrait{}

struct UIAutomation {
    automation: IUIAutomation
}
impl UIAutomation {
    fn new() -> Option<Self> {
        unsafe {
            CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                .map(|automation| Self { automation })
                .ok()
        }
    }
    fn element_from_hwnd(&self, hwnd: HWND) -> Option<UIAElement> {
        unsafe {
            self.automation.ElementFromHandle(hwnd).into_option()
        }
    }
    fn _element_from_point(&self, x: i32, y: i32) -> Option<UIAElement> {
        unsafe {
            let pt = POINT { x, y };
            self.automation.ElementFromPoint(pt).into_option()
        }
    }
    fn create_true_condition(&self) -> Option<IUIAutomationCondition> {
        unsafe {
            self.automation.CreateTrueCondition().ok()
        }
    }
}

#[derive(Debug, Clone)]
struct UIAElement {
    element: IUIAutomationElement
}
impl UIATrait for UIAElement {}
impl From<IUIAutomationElement> for UIAElement {
    fn from(element: IUIAutomationElement) -> Self {
        Self { element }
    }
}
impl UIAElement {
    fn get_name(&self) -> Option<String> {
        unsafe {
            self.element.CurrentName().into_option()
        }
    }
    fn is_focused(&self) -> bool {
        unsafe {
            match self.element.CurrentHasKeyboardFocus() {
                Ok(b) => b.as_bool(),
                Err(_) => false,
            }
        }
    }
    fn search(&self, target: &mut UIATarget) -> Option<UIAFound> {
        unsafe {
            let array = self.element.FindAll(TreeScope_Children, &target.condition).ok()?;
            let len = array.Length().ok()?;
            for index in 0..len {
                if let Ok(elem) = array.GetElement(index) {
                    if let Ok(controltype_id) = elem.CurrentControlType() {
                        let element: UIAElement = elem.into();
                        if target.contains(&controltype_id) {
                            match controltype_id {
                                UIA_ListControlTypeId => {
                                    if let Some(header) = element.has_header(&target.condition) {
                                        // リストビュー
                                        if target.search_listview {
                                            if let Some(listitems) = element.get_list_items(&target.condition) {
                                                if target.is_multiple() {
                                                    let found = listitems
                                                        .filter(|item| {
                                                            match item.get_texts(&target.condition) {
                                                                Some(mut texts) => {
                                                                    texts.find(|e| target.includes(e))
                                                                        .is_some()
                                                                },
                                                                None => false,
                                                            }
                                                        })
                                                        .collect::<Vec<_>>();
                                                    if found.len() > 0 {
                                                        return Some(UIAFound::Multi(found));
                                                    }
                                                } else {
                                                    for item in listitems {
                                                        if let Some(mut texts) = item.get_texts(&target.condition) {
                                                            if let Some(text) = texts.find(|e| target.matches(e)) {
                                                                return Some(UIAFound::ListViewItem(item, text));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // ヘッダを検索
                                            if let Some(mut items) = header.get_header_items(&target.condition) {
                                                let found = items
                                                    .find(|e| target.matches(e))
                                                    .map(|e| UIAFound::Single(e, UIA_HeaderControlTypeId));
                                                if found.is_some() {
                                                    return found;
                                                }
                                            }
                                        }
                                    } else {
                                        // リスト
                                        if target.search_list {
                                            if let Some(mut listitems) = element.get_list_items(&target.condition) {
                                                // \tが含まれてたら該当する複数アイテムを返す
                                                if target.is_multiple() {
                                                    let found = listitems
                                                        .filter(|e| target.includes(&e))
                                                        .collect::<Vec<_>>();
                                                    if found.len() > 0 {
                                                        return Some(UIAFound::Multi(found));
                                                    }
                                                } else {
                                                    let found = listitems
                                                        .find(|e| target.matches(&e))
                                                        .map(|e| UIAFound::Single(e, UIA_ListControlTypeId));
                                                    if found.is_some() {
                                                        return found;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                UIA_ComboBoxControlTypeId => {
                                    // リストを展開する
                                    element.expand();
                                    if let Some(mut listitems) = element.get_list_items(&target.condition) {
                                        let found = listitems
                                            .find(|e| target.matches(&e))
                                            .map(|e| UIAFound::Single(e, UIA_ComboBoxControlTypeId));
                                        if found.is_some() {
                                            return found;
                                        }
                                    }
                                    element.collapse();
                                },
                                UIA_TabControlTypeId => {
                                    if let Some(mut items) = self.get_tab_items(&target.condition) {
                                        let found = items
                                            .find(|e| target.matches(e))
                                            .map(|e| UIAFound::Single(e, UIA_TabControlTypeId));
                                        if found.is_some() {
                                            return found;
                                        }
                                    }
                                },
                                // UIA_MenuControlTypeId |
                                // UIA_MenuBarControlTypeId => {
                                //     if let Some(menu) = element.search_menu(target, None) {
                                //         return Some(UIAFound::Single(menu, UIA_MenuBarControlTypeId));
                                //     }
                                // },
                                UIA_TreeControlTypeId => {
                                    let name = target.name.clone();
                                    let path = name.split("\\");
                                    if let Some(found) = element.search_treeview_item(target, None, path) {
                                        return Some(UIAFound::Single(found, UIA_TreeControlTypeId));
                                    }
                                },
                                UIA_DataGridControlTypeId => {
                                    todo!()
                                },
                                UIA_ToolBarControlTypeId => {
                                    if let Some(mut buttons) = self.get_toolbar_buttons(&target.condition) {
                                        let found = buttons
                                            .find(|e| target.matches(e))
                                            .map(|e| UIAFound::Single(e, UIA_ToolBarControlTypeId));
                                        if found.is_some() {
                                            return found;
                                        }
                                    }
                                },
                                UIA_HyperlinkControlTypeId |
                                UIA_ButtonControlTypeId |
                                UIA_CheckBoxControlTypeId |
                                UIA_RadioButtonControlTypeId => {
                                    if target.matches(&element) {
                                        return Some(UIAFound::Single(element, controltype_id));
                                    }
                                },
                                _ => {}
                            }
                        }
                        if let Some(found) = element.search(target) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
    }
    fn has_header(&self, condition: &IUIAutomationCondition) -> Option<UIAElement> {
        self.find_all(TreeScope_Children, condition)?
            .find(|e| e.filter_by_type(UIA_HeaderControlTypeId))
    }
    fn get_list_items(&self, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        self.filter(condition, UIA_ListItemControlTypeId)
    }
    fn get_tab_items(&self, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        self.filter(condition, UIA_TabItemControlTypeId)
    }
    fn get_toolbar_buttons(&self, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        self.filter(condition, UIA_ButtonControlTypeId)
    }
    fn get_header_items(&self, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        self.filter(condition, UIA_HeaderItemControlTypeId)
    }
    fn get_texts(&self, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        self.filter(condition, UIA_TextControlTypeId)
    }
    // fn search_menu(&self, target: &mut UIATarget, path: Option<UIATreePath>) -> Option<UIAElement> {
    //     let elements = self.find_all(TreeScope_Children, &target.condition)?;
    //     println!("\u{001b}[36m[debug] path: {:?}\u{001b}[0m", &path);
    //     for element in elements {
    //         println!("\u{001b}[33m[debug] name: {:?}\u{001b}[0m", &element.get_name());
    //         if let Some(ucid) = element.get_control_type_id() {
    //             match ucid {
    //                 UIA_MenuBarControlTypeId => {
    //                     if let Some(name) = element.get_name() {
    //                         let mut path = path.clone().unwrap_or_default();
    //                         path.add(name);
    //                         if element.expand().is_some() {
    //                             if let Some(found) = self.search_menu(target, Some(path)) {
    //                                 return Some(found);
    //                             }
    //                         }
    //                     }
    //                 },
    //                 UIA_MenuItemControlTypeId => {
    //                     if let Some(name) = element.get_name() {
    //                         if target.is_path() {
    //                             let mut path = path.clone().unwrap_or_default();
    //                             path.add(name);
    //                             if target.matches_by_name(&path.to_string()) {
    //                                 return Some(element);
    //                             }
    //                         } else {
    //                             if target.matches(&element) {
    //                                 return Some(element);
    //                             }
    //                         }
    //                     }
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    //     None
    // }
    fn search_treeview_item(&self, target: &mut UIATarget, name: Option<&str>, mut path: std::str::Split<&str>) -> Option<UIAElement> {
        let name = if let Some(name) = name {
            Some(name)
        } else {
            path.next()
        };
        let elements = self.find_all(TreeScope_Children, &target.condition)?
            .filter(|e| e.filter_by_type(UIA_TreeItemControlTypeId));
        for element in elements {
            if target.matches_with_name(&element, name) {
                let mut path2 = path.clone();
                if let Some(next) = path2.next() {
                    let state = element.get_expand_collapse_state();
                    element.expand();
                    // sleep(20);
                    if let Some(found) = element.search_treeview_item(target, Some(next), path2) {
                        return Some(found)
                    } else {
                        if let Some(ExpandCollapseState_Collapsed) = state {
                            element.collapse();
                        }
                    }
                } else {
                    if target.is_in_exact_order() {
                        return Some(element);
                    }
                }
            } else {
                let state = element.get_expand_collapse_state();
                element.expand();
                if let Some(found) = element.search_treeview_item(target, name, path.clone()) {
                    return Some(found);
                } else {
                    if let Some(ExpandCollapseState_Collapsed) = state {
                        element.collapse();
                    }
                }
            }
        }
        None
    }
    fn filter(&self, condition: &IUIAutomationCondition, ucid: UIA_CONTROLTYPE_ID) -> Option<impl Iterator<Item = UIAElement>> {
        let elements = self.find_all(TreeScope_Descendants, condition)?
            .filter(move |e| e.filter_by_type(ucid));
        Some(elements)
    }
    fn find_all(&self, scope: TreeScope, condition: &IUIAutomationCondition) -> Option<impl Iterator<Item = UIAElement>> {
        unsafe {
            if let Ok(array) = self.element.FindAll(scope, condition) {
                if let Ok(len) = array.Length() {
                    if len > 0 {
                        let elements = (0..len)
                            .map(move |index| array.GetElement(index).ok())
                            .map(|e| e.map(|element| Self {element}))
                            .flatten();
                        return Some(elements)
                    }
                }
            }
            None
        }
    }
    fn _get_control_type_id(&self) -> Option<UIA_CONTROLTYPE_ID> {
        unsafe {
            self.element.CurrentControlType().ok()
        }
    }
    fn _get_control_type(&self) -> Option<String> {
        unsafe {
            self.element.CurrentLocalizedControlType().ok()
                .map(|bstr| bstr.to_string())
        }
    }
    fn filter_by_type(&self, ucid: UIA_CONTROLTYPE_ID) -> bool {
        unsafe {
            if let Ok(id) = self.element.CurrentControlType() {
                id == ucid
            } else {
                false
            }
        }
    }
    fn click(&self) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId).ok()?;
            pattern.Invoke().ok()?;
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn expand(&self) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId).ok()?;
            pattern.Expand().ok()?;
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn collapse(&self) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId).ok()?;
            pattern.Collapse().ok()?;
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn get_expand_collapse_state(&self) -> Option<ExpandCollapseState> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId).ok()?;
            pattern.CurrentExpandCollapseState().ok()
        }
    }
    fn get_check_state(&self) -> Option<ToggleState> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId).ok()?;
            pattern.CurrentToggleState().ok()
        }
    }
    fn check(&self, state: &ThreeState) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId).ok()?;
            let toggle: ThreeState = pattern.CurrentToggleState().ok()?.into();
            match (toggle, state) {
                // 状態と指定値が一致した場合はなにもしない
                (ThreeState::True, ThreeState::True) |
                (ThreeState::False, ThreeState::False) |
                (ThreeState::Other, ThreeState::Other) => {},
                // ボタンがチェック済み
                (ThreeState::True, ThreeState::False) => {
                    pattern.Toggle().ok()?;
                    sleep(20);
                    if let Ok(ToggleState_Indeterminate) = pattern.CurrentToggleState() {
                        // 一度トグルして不定だったら再度トグル
                        pattern.Toggle().ok()?;
                    }
                },
                (ThreeState::True, ThreeState::Other) => {
                    pattern.Toggle().ok()?;
                    sleep(20);
                    if let Ok(ToggleState_Off) = pattern.CurrentToggleState() {
                        // 一度トグルして不定だったら再度トグル
                        pattern.Toggle().ok()?;
                    }
                },
                // ボタンのチェックなし
                (ThreeState::False, ThreeState::True) => {
                    pattern.Toggle().ok()?;
                },
                (ThreeState::False, ThreeState::Other) => {
                    pattern.Toggle().ok()?;
                    pattern.Toggle().ok()?;
                },
                // ボタンが不定
                (ThreeState::Other, ThreeState::True) => {
                    pattern.Toggle().ok()?;
                    pattern.Toggle().ok()?;
                },
                (ThreeState::Other, ThreeState::False) => {
                    pattern.Toggle().ok()?;
                },
            };
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn select(&self) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).ok()?;
            pattern.Select().ok()?;
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn multi_select(&self) -> Option<UIAClickPoint> {
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId).ok()?;
            pattern.AddToSelection().ok()?;
            let point = self.get_clickable_point();
            Some(UIAClickPoint(point))
        }
    }
    fn write(&self, str: String) -> Option<()>{
        unsafe {
            let pattern = self.element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId).ok()?;
            let val = BSTR::from(str);
            pattern.SetValue(&val).ok()
        }
    }
    fn get_clickable_point(&self) -> Option<(i32, i32)> {
        unsafe {
            let mut clickable = POINT::default();
            let mut gotclickable = false.into();
            self.element.GetClickablePoint(&mut clickable, &mut gotclickable).ok()?;
            if gotclickable.as_bool() {
                Some((clickable.x, clickable.y))
            } else {
                None
            }
        }
    }
}

pub struct UIAClickPoint(pub Option<(i32, i32)>);

struct UIATarget {
    name: String,
    nth: u32,
    types: Vec<UIA_CONTROLTYPE_ID>,
    condition: IUIAutomationCondition,
    partial: bool,
    /// リストを検索するかどうか
    search_list: bool,
    /// リストビューを検索するかどうか
    search_listview: bool,
}
impl UIATarget {
    fn new(ci: &ClkItem, condition: IUIAutomationCondition) -> Self {
        let name = ci.name.clone();
        let nth = ci.order;
        let partial = ci.short;
        let mut types = vec![];
        let mut search_list = false;
        let mut search_listview = false;
        if ci.target.button {
            types.push(UIA_ButtonControlTypeId);
            types.push(UIA_CheckBoxControlTypeId);
            types.push(UIA_RadioButtonControlTypeId);
        }
        if ci.target.list {
            types.push(UIA_ListControlTypeId);
            types.push(UIA_ComboBoxControlTypeId);
            search_list = true;
        }
        if ci.target.tab {
            types.push(UIA_TabControlTypeId);
        }
        // if ci.target.menu {
        //     types.push(UIA_MenuControlTypeId);
        //     types.push(UIA_MenuBarControlTypeId);
        // }
        if ci.target.treeview {
            types.push(UIA_TreeControlTypeId);
        }
        if ci.target.listview {
            types.push(UIA_ListControlTypeId);
            // types.push(UIA_DataGridControlTypeId);
            search_listview = true;
        }
        if ci.target.toolbar {
            types.push(UIA_ToolBarControlTypeId);
        }
        if ci.target.link {
            types.push(UIA_HyperlinkControlTypeId);
        }
        Self { name, nth, types, condition, partial, search_list, search_listview }
    }
    fn contains(&self, controltype_id: &UIA_CONTROLTYPE_ID) -> bool {
        self.types.contains(controltype_id)
    }
    fn matches(&mut self, element: &UIAElement) -> bool {
        if let Some(name) = element.get_name() {
            if match_title(&name, &self.name, self.partial) {
                self.is_in_exact_order()
            } else {
                false
            }
        } else {
            false
        }
    }
    fn matches_with_name(&mut self, element: &UIAElement, name: Option<&str>) -> bool {
        if let Some(name) = name {
            if let Some(ename) = element.get_name() {
                match_title(&ename, name, self.partial)
            } else {
                false
            }
        } else {
            false
        }
    }
    fn is_multiple(&self) -> bool {
        self.name.contains('\t')
    }
    // fn is_path(&self) -> bool {
    //     self.name.contains('\\')
    // }
    fn includes(&mut self, element: &UIAElement) -> bool {
        if let Some(name) = element.get_name() {
            self.name.split('\t')
                .find(|pat| match_title(&name, *pat, self.partial))
                .is_some()
        } else {
            false
        }
    }
    fn is_in_exact_order(&mut self) -> bool {
        if self.nth > 0 {
            self.nth -= 1;
            self.nth == 0
        } else {
            false
        }
    }
}

enum UIAFound {
    Single(UIAElement, UIA_CONTROLTYPE_ID),
    Multi(Vec<UIAElement>),
    /// リストビュー行、テキスト
    ListViewItem(UIAElement, UIAElement),
}
// impl UIAFound {
//     fn new(element: UIAElement, controltype_id: UIA_CONTROLTYPE_ID) -> Self {
//         Self { element, controltype_id }
//     }
// }

// #[derive(Debug, Clone)]
// struct UIATreePath(Vec<String>);
// impl std::fmt::Display for UIATreePath {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.0.join("\\"))
//     }
// }
// impl Default for UIATreePath {
//     fn default() -> Self {
//         Self(Default::default())
//     }
// }
// impl UIATreePath {
//     fn add(&mut self, name: String) {
//         self.0.push(name);
//     }
// }

trait WindowsResultExt<U> {
    fn into_option(self) -> Option<U>;
}

impl<U: UIATrait, T: Into<U>> WindowsResultExt<U> for windows::core::Result<T> {
    fn into_option(self) -> Option<U> {
        self.map(|obj| obj.into()).ok()
    }
}
impl WindowsResultExt<String> for windows::core::Result<BSTR> {
    fn into_option(self) -> Option<String> {
        self.map(|bstr| bstr.to_string()).ok()
    }
}

fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}