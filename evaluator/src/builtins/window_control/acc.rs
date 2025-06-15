use std::{ffi::c_void, mem::ManuallyDrop};

use windows::{
    core::{self, Interface, ComInterface, BSTR},
    Win32::{
        Foundation::{HWND, POINT},
        UI::{
            Accessibility::{
                IAccessible,
                AccessibleObjectFromWindow, WindowFromAccessibleObject,AccessibleObjectFromPoint,
                AccessibleChildren,
                GetStateTextW, GetRoleTextW,
                ROLE_SYSTEM_ALERT,ROLE_SYSTEM_ANIMATION,ROLE_SYSTEM_APPLICATION,ROLE_SYSTEM_BORDER,ROLE_SYSTEM_BUTTONDROPDOWN,ROLE_SYSTEM_BUTTONDROPDOWNGRID,ROLE_SYSTEM_BUTTONMENU,ROLE_SYSTEM_CARET,ROLE_SYSTEM_CELL,ROLE_SYSTEM_CHARACTER,ROLE_SYSTEM_CHART,ROLE_SYSTEM_CHECKBUTTON,ROLE_SYSTEM_CLIENT,ROLE_SYSTEM_CLOCK,ROLE_SYSTEM_COLUMN,ROLE_SYSTEM_COLUMNHEADER,ROLE_SYSTEM_COMBOBOX,ROLE_SYSTEM_CURSOR,ROLE_SYSTEM_DIAGRAM,ROLE_SYSTEM_DIAL,ROLE_SYSTEM_DIALOG,ROLE_SYSTEM_DOCUMENT,ROLE_SYSTEM_DROPLIST,ROLE_SYSTEM_EQUATION,ROLE_SYSTEM_GRAPHIC,ROLE_SYSTEM_GRIP,ROLE_SYSTEM_GROUPING,ROLE_SYSTEM_HELPBALLOON,ROLE_SYSTEM_HOTKEYFIELD,ROLE_SYSTEM_INDICATOR,ROLE_SYSTEM_IPADDRESS,ROLE_SYSTEM_LINK,ROLE_SYSTEM_LIST,ROLE_SYSTEM_LISTITEM,ROLE_SYSTEM_MENUBAR,ROLE_SYSTEM_MENUITEM,ROLE_SYSTEM_MENUPOPUP,ROLE_SYSTEM_OUTLINE,ROLE_SYSTEM_OUTLINEBUTTON,ROLE_SYSTEM_OUTLINEITEM,ROLE_SYSTEM_PAGETAB,ROLE_SYSTEM_PAGETABLIST,ROLE_SYSTEM_PANE,ROLE_SYSTEM_PROGRESSBAR,ROLE_SYSTEM_PROPERTYPAGE,ROLE_SYSTEM_PUSHBUTTON,ROLE_SYSTEM_RADIOBUTTON,ROLE_SYSTEM_ROW,ROLE_SYSTEM_ROWHEADER,ROLE_SYSTEM_SCROLLBAR,ROLE_SYSTEM_SEPARATOR,ROLE_SYSTEM_SLIDER,ROLE_SYSTEM_SOUND,ROLE_SYSTEM_SPINBUTTON,ROLE_SYSTEM_SPLITBUTTON,ROLE_SYSTEM_STATICTEXT,ROLE_SYSTEM_STATUSBAR,ROLE_SYSTEM_TABLE,ROLE_SYSTEM_TEXT,ROLE_SYSTEM_TITLEBAR,ROLE_SYSTEM_TOOLBAR,ROLE_SYSTEM_TOOLTIP,ROLE_SYSTEM_WHITESPACE,ROLE_SYSTEM_WINDOW,
                SELFLAG_ADDSELECTION, SELFLAG_TAKEFOCUS, SELFLAG_TAKESELECTION,
            },
            WindowsAndMessaging::{
                OBJID_WINDOW,
                STATE_SYSTEM_CHECKED, STATE_SYSTEM_FOCUSED, STATE_SYSTEM_LINKED, STATE_SYSTEM_SELECTABLE,
                SetForegroundWindow,
                CHILDID_SELF,
            },
            Controls::{
                STATE_SYSTEM_UNAVAILABLE, STATE_SYSTEM_INVISIBLE, STATE_SYSTEM_FOCUSABLE,STATE_SYSTEM_OFFSCREEN,
            },
        },
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
        System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4, VT_DISPATCH},
        System::Com::IDispatch,
    }
};
const ROLE_SYSTEM_LISTVIEW: u32 = ROLE_SYSTEM_LIST + 10000;
use std::ops::ControlFlow;

use crate::{builtins::window_low::move_mouse_to, object::VariantExt, U32Ext};
use super::clkitem::{ClkItem, ClkTarget};
use serde_json::{Value, json};
pub struct Acc {}

impl Acc {
    pub fn getitem(hwnd: HWND, target: u32, max_count: i32, ignore_disabled: bool) -> Option<Vec<String>> {
        let gi = GetItem::new(target, max_count, ignore_disabled)?;
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        if !gi.background {
            window.activate();
        }
        let mut iter = window.into_iter();
        if gi.reverse {
            iter.reverse();
        }
        let result = Vec::with_capacity(gi.count);
        let flow = iter
            .filter(|child| gi.validate(child))
            .try_fold(result, |mut result, child| {
                if let Some(item) = child.get_item_value() {
                    result.push(item);
                }
                if result.len() < result.capacity() {
                    ControlFlow::Continue(result)
                } else {
                    ControlFlow::Break(result)
                }
            });
        match flow {
            ControlFlow::Continue(result) |
            ControlFlow::Break(result) => Some(result),
        }
    }
    pub fn from_point(hwnd: HWND, clx: i32, cly: i32, pos_acc_type: u16) -> Option<PosAccResult> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        let child = window.child_from_client_point(clx, cly)?;
        match PosAccType::from(pos_acc_type) {
            PosAccType::DisplayOrApi => child.name().ok()
                .or(child.user_draw_text())
                .map(PosAccResult::String),
            PosAccType::Display => child.name().ok().map(PosAccResult::String),
            PosAccType::Api => child.user_draw_text().map(PosAccResult::String),
            PosAccType::Name => child.name().ok().map(PosAccResult::String),
            PosAccType::Value => child.value().ok().map(PosAccResult::String),
            PosAccType::Role => child.role_text().ok().map(PosAccResult::String),
            PosAccType::State => child.state_text().ok().map(PosAccResult::Vec),
            PosAccType::Description => child.description().ok().map(PosAccResult::String),
            PosAccType::Location => child.location().ok().map(PosAccResult::Location),
        }
    }
    pub fn get_check_state(hwnd: HWND, name: &str, nth: usize) -> Option<i32> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window
            .find_nth(nth, |child| {
                child.role_is_one_of(&[ROLE_SYSTEM_CHECKBUTTON, ROLE_SYSTEM_MENUITEM])
                && child.name_includes(name)
            })
            .and_then(|child| child.state().ok())
            .map(|state| ChkBtnResult::from(state).into())
    }
    pub fn get_edit_str(hwnd: HWND, nth: i32, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_TEXT], mouse)
    }
    pub fn get_static_str(hwnd: HWND, nth: i32, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_STATICTEXT], mouse)
    }
    pub fn get_cell_str(hwnd: HWND, nth: i32, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_CELL], mouse)
    }
    pub fn sendstr<R>(hwnd: HWND, nth: usize, str: &str, replace: R) where R: Into<bool> {
        if let Ok(window) = AccWindow::from_hwnd(hwnd) {
            let replace: bool = replace.into();
            let predicate = |child: &AccChild| {
                child.is_valid() && child.role_is(ROLE_SYSTEM_TEXT)
            };
            if let Some(child) = window.find_nth(nth, &predicate) {
                if replace {
                    let _ = child.set_value(str);
                } else if let Ok(old) = child.value() {
                    let value = old + str;
                    let _ = child.set_value(&value);
                } else {
                    let _ = child.set_value(str);
                }
            }
        }
    }
    pub fn sendstr_cell<R>(hwnd: HWND, nth: usize, str: &str, replace: R) where R: Into<bool> {
        if let Ok(window) = AccWindow::from_hwnd(hwnd) {
            let replace: bool = replace.into();
            let maybe = window
                .find_nth(nth, |child| child.role_is(ROLE_SYSTEM_CELL))
                .and_then(|cell| cell.into_iter().find(|child| child.role_is(ROLE_SYSTEM_TEXT)));
            if let Some(child) = maybe {
                if replace {
                    let _ = child.set_value(str);
                } else if let Ok(old) = child.value() {
                    let value = old + str;
                    let _ = child.set_value(&value);
                } else {
                    let _ = child.set_value(str);
                }
            }
        }
    }
    fn name_as_path<'a>(item: &'a ClkItem, roles: &[u32]) -> Option<std::str::Split<'a, &'a str>> {
        (roles.contains(&ROLE_SYSTEM_MENUBAR) || roles.contains(&ROLE_SYSTEM_OUTLINE)).then_some(())?;
        item.name_as_path()
    }
    pub fn find_click_target(hwnd: HWND, item: &ClkItem) -> Option<ClickTargetFound> {
        #[cfg(debug_assertions)]
        AccWindow::_out_all_item(hwnd, "D:\\temp");

        let window = AccWindow::from_hwnd(hwnd).ok()?;
        let mut iter = window.into_iter();
        if item.backwards {
            iter.reverse();
        }
        let roles: Vec<u32> = Self::roles_from_target(&item.target);
        let nth = (item.order as usize).saturating_sub(1);
        /* 探す名前が path\to\item の場合 */
        if let Some(path_iter) = Self::name_as_path(item, &roles) {
            let roles= Self::roles_from_path_target(&item.target);
            // メニューまたはツリービューを探す
            let parents = iter.filter(|child| child.role_is_one_of(&roles));
            let item_roles = [ROLE_SYSTEM_MENUITEM, ROLE_SYSTEM_OUTLINEITEM];
            let found = path_iter.fold(None::<Vec<AccChild>>, move |mut branches, name| {
                if let Some(_branches) = branches.take() {
                    let filtered = _branches.into_iter()
                        .flat_map(|branch| {
                            branch.dbg_detail();
                            branch.iter()
                                .filter(|child| child.role_is_one_of(&item_roles))
                                .filter(|c| c.name_matches_to(name, item.short))
                        })
                        .collect();
                    branches.replace(filtered);
                } else {
                    let filtered = parents.clone()
                        .flat_map(|p| {
                            p.into_iter()
                                .filter(|c| c.role_is_one_of(&item_roles))
                                .filter(|c| c.name_matches_to(name, item.short))
                        })
                        .collect();
                    branches.replace(filtered);
                }
                branches
            }).and_then(|found| found.into_iter().nth(nth));
            found.map(ClickTargetFound::Single)
        /* リスト複数選択の場合 */
        } else if (item.target.list||item.target.listview) && item.name.contains('\t') {
            let names = item.name.split('\t').collect::<Vec<_>>();
            let roles = Self::roles_multi_select(&item.target);
            let matches = iter.filter(|child| child.role_is_one_of(&roles))
                .flat_map(|child| {
                    child.into_iter()
                        .filter(|c| {c.role_is(ROLE_SYSTEM_LISTITEM)})
                        .filter(|c| names.iter().any(|name| c.name_matches_to(name, item.short)))
                })
                .collect::<Vec<_>>();
            (!matches.is_empty())
                .then_some(ClickTargetFound::Multi(matches))
        } else {
            let filter = Self::find_click_target_filter(&item.name, item.short);
            let found = iter.filter(|child| child.role_is_one_of(&roles))
                .flat_map(filter)
                .nth(nth)
                .inspect(AccChild::dbg_detail);
            found.map(ClickTargetFound::Single)
        }
    }
    fn find_click_target_filter(name: &str, partial: bool) -> impl FnMut(AccChild) -> Vec<AccChild>
    {
        let find_name_matched = move |child: AccChild| -> Option<AccChild> {
            child.name_matches_to(name, partial)
                .then_some(child)
        };
        move |child: AccChild| {
            match child.role().unwrap_or_default() {
                ROLE_SYSTEM_LIST => {
                    child.into_iter()
                        .filter(|c| c.role_is(ROLE_SYSTEM_LISTITEM))
                        .filter_map(find_name_matched)
                        .collect()
                },
                ROLE_SYSTEM_LISTVIEW => {
                    child.into_iter()
                        .filter(|c| c.role_is(ROLE_SYSTEM_LISTITEM))
                        .filter_map(find_name_matched)
                        .collect()
                }
                ROLE_SYSTEM_MENUBAR => {
                    child.into_iter()
                        .filter(|c| c.role_is(ROLE_SYSTEM_MENUITEM))
                        .filter_map(find_name_matched)
                        .collect()
                },
                ROLE_SYSTEM_PAGETABLIST => {
                    child.into_iter()
                        .filter(|c| c.role_is(ROLE_SYSTEM_PAGETAB))
                        .filter_map(find_name_matched)
                        .collect()
                },
                ROLE_SYSTEM_TOOLBAR => {
                    child.into_iter()
                        .filter_map(find_name_matched)
                        .collect()
                },
                ROLE_SYSTEM_OUTLINE => {
                    // treeviewはネストするのでどうする
                    child.into_iter()
                        .filter(|c| c.role_is(ROLE_SYSTEM_OUTLINEITEM))
                        .filter_map(find_name_matched)
                        .collect()
                }
                _ => find_name_matched(child).into_iter().collect()
            }
        }
    }
    fn roles_from_target(target: &ClkTarget) -> Vec<u32> {
        let mut roles = Vec::new();
        if target.button {
            roles.append(&mut vec![ROLE_SYSTEM_PUSHBUTTON,ROLE_SYSTEM_CHECKBUTTON,ROLE_SYSTEM_RADIOBUTTON]);
        }
        if target.link {
            roles.push(ROLE_SYSTEM_LINK);
        }
        if target.list {
            roles.push(ROLE_SYSTEM_LIST);
        }
        if target.listview {
            roles.push(ROLE_SYSTEM_LISTVIEW);
        }
        if target.menu {
            roles.push(ROLE_SYSTEM_MENUBAR);
        }
        if target.tab {
            roles.push(ROLE_SYSTEM_PAGETABLIST);
        }
        if target.toolbar {
            roles.push(ROLE_SYSTEM_TOOLBAR);
        }
        if target.treeview {
            roles.push(ROLE_SYSTEM_OUTLINE);
        }
        roles
    }
    /// アイテム名がパス形式 (path\to\item) の場合のターゲットを返す
    fn roles_from_path_target(target: &ClkTarget) -> Vec<u32> {
        let mut roles = Vec::new();
        if target.menu {
            roles.push(ROLE_SYSTEM_MENUBAR);
        }
        if target.treeview {
            roles.push(ROLE_SYSTEM_OUTLINE);
        }
        roles
    }
    /// アイテム名がタブ文字区切りの場合のターゲットを返す
    fn roles_multi_select(target: &ClkTarget) -> Vec<u32> {
        let mut roles = Vec::new();
        if target.list {
            roles.push(ROLE_SYSTEM_LIST);
        }
        if target.listview {
            roles.push(ROLE_SYSTEM_LISTVIEW);
        }
        roles
    }

    pub fn get_acc_tree(hwnd: HWND) -> Value {
        AccWindow::from_hwnd(hwnd)
            .map(AccWindow::into_value)
            .unwrap_or(Value::Null)
    }
}
pub enum ClickTargetFound {
    Single(AccChild),
    Multi(Vec<AccChild>),
}
impl ClickTargetFound {
    pub fn click(self, check: bool) -> bool {
        match self {
            ClickTargetFound::Single(child) => child.click(check),
            ClickTargetFound::Multi(children) => {
                let mut iter = children.into_iter();
                // 一つ目のアイテムを選択
                if iter.next().is_some_and(|c| c.select_one()) {
                    // 残りのアイテムも追加選択
                    iter.for_each(|c| {
                        c.add_select();
                    });
                    true
                } else {
                    false
                }
            },
        }
    }
    pub fn hwnd(&self) -> core::Result<HWND> {
        match self {
            ClickTargetFound::Single(child) => child.hwnd(),
            ClickTargetFound::Multi(children) => {
                children.first()
                    .map(|c| c.hwnd())
                    .transpose()
                    .map(|h| h.unwrap_or_default())
            },
        }
    }
    pub fn location(&self) -> core::Result<[i32; 4]> {
        match self {
            ClickTargetFound::Single(child) => child.location(),
            ClickTargetFound::Multi(children) => {
                children.last()
                    .map(|c| c.location())
                    .transpose()
                    .map(|loc| loc.unwrap_or_default())
            },
        }
    }
}

struct GetItem {
    role: GetItemRole,
    reverse: bool,
    background: bool,
    count: usize,
    ignore_disabled: bool,
}
impl GetItem {
    fn new(target: u32, max_count: i32, ignore_disabled: bool) -> Option<Self> {
        let role = match target {
            n if (n & 4194304) > 0 => GetItemRole::Clickable,
            n if (n & 272629760) > 0 => GetItemRole::ClickableOrSelectable,
            n if (n & 8388608) > 0 => GetItemRole::StaticText,
            n if (n & 16777216) > 0 => GetItemRole::Editable,
            _ => None?,
        };
        let (count, reverse) = if max_count.is_negative() {
            (max_count.unsigned_abs() as usize, true)
        } else {
            (max_count.unsigned_abs() as usize, (target & 65536) > 0)
        };
        let background = (target & 512) > 0;
        Some(Self { role, reverse, background, count, ignore_disabled })
    }
    fn validate(&self, child: &AccChild) -> bool {
        let child_role = child.role().unwrap_or_default();
        let child_state = child.state().unwrap_or_default();
        let is_valid_role = match self.role {
            GetItemRole::Clickable => {
                match child_role {
                    ROLE_SYSTEM_PUSHBUTTON |
                    ROLE_SYSTEM_CHECKBUTTON |
                    ROLE_SYSTEM_RADIOBUTTON |
                    ROLE_SYSTEM_LINK |
                    ROLE_SYSTEM_LISTITEM |
                    ROLE_SYSTEM_OUTLINEITEM |
                    ROLE_SYSTEM_PAGETAB |
                    ROLE_SYSTEM_TOOLBAR => true,
                    // 「リンクされています」であれば該当とする
                    _ => child_state.includes(STATE_SYSTEM_LINKED)
                }
            },
            GetItemRole::ClickableOrSelectable => {
                match child_role {
                    ROLE_SYSTEM_PUSHBUTTON |
                    ROLE_SYSTEM_CHECKBUTTON |
                    ROLE_SYSTEM_RADIOBUTTON |
                    ROLE_SYSTEM_LINK |
                    ROLE_SYSTEM_LISTITEM |
                    ROLE_SYSTEM_OUTLINEITEM |
                    ROLE_SYSTEM_PAGETAB |
                    ROLE_SYSTEM_TOOLBAR => true,
                    // テキストかつ選択可能であれば該当
                    ROLE_SYSTEM_TEXT |
                    ROLE_SYSTEM_STATICTEXT => child_state.includes(STATE_SYSTEM_SELECTABLE),
                    // 「リンクされています」であれば該当とする
                    _ => child_state.includes(STATE_SYSTEM_LINKED)
                }
            },
            GetItemRole::StaticText => child_role == ROLE_SYSTEM_STATICTEXT,
            GetItemRole::Editable => child_role == ROLE_SYSTEM_TEXT,
            GetItemRole::Other |
            GetItemRole::Invalid(_) => false
        };
        is_valid_role
            && child.is_valid()
            && !(self.ignore_disabled && child.is_disabled())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum GetItemRole {
    /// ITM_ACCCLK
    Clickable,
    /// ITM_ACCCLK2
    ClickableOrSelectable,
    /// ITM_ACCTXT
    StaticText,
    /// ITM_ACCEDIT
    Editable,
    /// 該当なし
    Other,
    /// 規定のロールですらない
    Invalid(u32),
}
impl From<u32> for GetItemRole {
    fn from(role: u32) -> Self {
        match role {
            // オブジェクトは、ユーザーに通知する必要があるアラートまたは条件を表します。 このロールは、アラートを具体化するが、メッセージ ボックス、グラフィック、テキスト、サウンドなどの別のユーザー インターフェイス要素に関連付けられていないオブジェクトにのみ使用されます。
            ROLE_SYSTEM_ALERT => Self::Other,
            // オブジェクトは、一連のビットマップ フレームを表示するコントロールなど、コンテンツが時間の経過と同時に変化するアニメーション コントロールを表します。 アニメーション コントロールは、ファイルがコピーされたとき、または他の時間のかかるタスクが実行されるときに表示されます。
            ROLE_SYSTEM_ANIMATION => Self::Other,
            // オブジェクトは、アプリケーションのメイン ウィンドウを表します。
            ROLE_SYSTEM_APPLICATION => Self::Other,
            // オブジェクトはウィンドウの境界線を表します。 境界線全体は、各辺の個別のオブジェクトではなく、1 つのオブジェクトで表されます。
            ROLE_SYSTEM_BORDER => Self::Other,
            // オブジェクトは、項目のリストを展開するボタンを表します。
            ROLE_SYSTEM_BUTTONDROPDOWN => Self::Other,
            // オブジェクトは、グリッドを展開するボタンを表します。
            ROLE_SYSTEM_BUTTONDROPDOWNGRID => Self::Other,
            // オブジェクトは、メニューを展開するボタンを表します。
            ROLE_SYSTEM_BUTTONMENU => Self::Other,
            // オブジェクトはシステム キャレットを表します。
            ROLE_SYSTEM_CARET => Self::Other,
            // オブジェクトは、テーブル内のセルを表します。
            ROLE_SYSTEM_CELL => Self::Other,
            // オブジェクトは、Microsoft Office Assistant などの漫画のようなグラフィック オブジェクトを表します。このオブジェクトは、アプリケーションのユーザーにヘルプを提供するために表示されます。
            ROLE_SYSTEM_CHARACTER => Self::Other,
            // オブジェクトは、データのグラフ作成に使用されるグラフィカル イメージを表します。
            ROLE_SYSTEM_CHART => Self::Other,
            // オブジェクトは、チェック ボックス コントロールを表します。これは、他のオプションとは別に選択またはクリアされるオプションです。
            ROLE_SYSTEM_CHECKBUTTON => Self::Clickable,
            // オブジェクトは、ウィンドウのクライアント領域を表します。 UI 要素のロールに関する質問がある場合、Microsoft Active Accessibility はこのロールを既定として使用します。
            ROLE_SYSTEM_CLIENT => Self::Other,
            // オブジェクトは、時間を表示するコントロールを表します。
            ROLE_SYSTEM_CLOCK => Self::Other,
            // オブジェクトは、テーブル内のセルの列を表します。
            ROLE_SYSTEM_COLUMN => Self::Other,
            // オブジェクトは列ヘッダーを表し、テーブル内の列の視覚的なラベルを提供します。
            ROLE_SYSTEM_COLUMNHEADER => Self::Other,
            // オブジェクトはコンボ ボックスを表します。定義済みの選択肢のセットを提供する、関連付けられたリスト ボックスを持つ編集コントロールです。
            ROLE_SYSTEM_COMBOBOX => Self::Other,
            // オブジェクトは、システムのマウス ポインターを表します。
            ROLE_SYSTEM_CURSOR => Self::Other,
            // オブジェクトは、データのダイアグラムに使用されるグラフィカル イメージを表します。
            ROLE_SYSTEM_DIAGRAM => Self::Other,
            // オブジェクトは、ダイヤルまたはノブを表します。
            ROLE_SYSTEM_DIAL => Self::Other,
            // オブジェクトは、ダイアログ ボックスまたはメッセージ ボックスを表します。
            ROLE_SYSTEM_DIALOG => Self::Other,
            // オブジェクトはドキュメント ウィンドウを表します。 ドキュメント ウィンドウは常にアプリケーション ウィンドウ内に含まれます。 このロールは MDI ウィンドウにのみ適用され、MDI タイトル バーを含むオブジェクトを参照します。
            ROLE_SYSTEM_DOCUMENT => Self::Other,
            // オブジェクトは、カレンダー コントロール SysDateTimePick32 を表します。 Microsoft Active Accessibility ランタイム コンポーネントは、このロールを使用して、日付またはカレンダー コントロールが見つかったことを示します。
            ROLE_SYSTEM_DROPLIST => Self::Other,
            // オブジェクトは数式を表します。
            ROLE_SYSTEM_EQUATION => Self::Other,
            // オブジェクトは図を表します。
            ROLE_SYSTEM_GRAPHIC => Self::Other,
            // オブジェクトは、ユーザーがウィンドウなどのユーザー インターフェイス要素を操作できるようにする特別なマウス ポインターを表します。 この例の 1 つは、右下隅をドラッグしてウィンドウのサイズを変更する場合です。
            ROLE_SYSTEM_GRIP => Self::Other,
            // オブジェクトは、他のオブジェクトを論理的にグループ化します。 グループ化オブジェクトとそこに含まれるオブジェクトの間には、常に親子関係があるとは限りません。
            ROLE_SYSTEM_GROUPING => Self::Other,
            // オブジェクトには、ヒントまたはヘルプ バルーンの形式でヘルプ トピックが表示されます。
            ROLE_SYSTEM_HELPBALLOON => Self::Other,
            // オブジェクトは、ユーザーがキーストロークの組み合わせまたはシーケンスを入力できるようにするキーボード ショートカット フィールドを表します。
            ROLE_SYSTEM_HOTKEYFIELD => Self::Other,
            // オブジェクトは、現在の項目を指すインジケーター (ポインター グラフィックなど) を表します。
            ROLE_SYSTEM_INDICATOR => Self::Other,
            // オブジェクトは、IP アドレス用に設計された編集コントロールを表します。 編集コントロールは、IP アドレスの特定の部分ごとにセクションに分割されます。
            ROLE_SYSTEM_IPADDRESS => Self::Editable,
            // オブジェクトは、他のリンクを表します。 このオブジェクトは、テキストやグラフィックのように見えることもありますが、ボタンに似た動作をします。
            ROLE_SYSTEM_LINK => Self::Clickable,
            // オブジェクトはリスト ボックスを表し、ユーザーは 1 つ以上の項目を選択できます。
            ROLE_SYSTEM_LIST => Self::Other,
            // オブジェクトは、リスト ボックスまたはコンボ ボックス、ドロップダウン リスト ボックス、またはドロップダウン コンボ ボックスのリスト部分の項目を表します。
            ROLE_SYSTEM_LISTITEM => Self::Clickable,
            // オブジェクトは、ユーザーがメニューを選択するメニュー バー (ウィンドウのタイトル バーの下に配置) を表します。
            ROLE_SYSTEM_MENUBAR => Self::Other,
            // オブジェクトはメニュー項目を表します。ユーザーがコマンドの実行、オプションの選択、または別のメニューの表示を選択できるメニュー エントリです。 機能的には、メニュー項目は、プッシュ ボタン、ラジオ ボタン、チェック ボックス、またはメニューと同じです。
            ROLE_SYSTEM_MENUITEM => Self::Other,
            // オブジェクトはメニューを表します。各メニューは、特定のアクションを持つオプションの一覧です。 メニュー バーから選択すると表示されるドロップダウン メニューを含め、すべてのメニューの種類にロールが必要です。および ショートカット メニュー。マウスの右ボタンをクリックして表示されます。
            ROLE_SYSTEM_MENUPOPUP => Self::Other,
            // オブジェクトは、ツリー ビュー コントロールなどのアウトラインまたはツリー構造を表し、階層リストを表示し、ユーザーがブランチを展開および折りたたみできるようにします。
            ROLE_SYSTEM_OUTLINE => Self::Other,
            // オブジェクトは、アウトライン項目のように移動する項目を表します。 上方向キーと下方向キーは、アウトライン内を移動するために使用されます。 ただし、左方向キーと右方向キーを押したときに展開と折りたたみを行う代わりに、SPACE キーまたは Enter キーを押したときに項目にフォーカスがあるときに、これらのメニューが展開または折りたたみされます。
            ROLE_SYSTEM_OUTLINEBUTTON => Self::Other,
            // オブジェクトは、アウトライン構造またはツリー構造の項目を表します。
            ROLE_SYSTEM_OUTLINEITEM => Self::Clickable,
            // オブジェクトはページ タブを表します。ページ タブ コントロールの唯一の子は、関連付けられたページの内容を持つROLE_SYSTEM_GROUPING オブジェクトです。
            ROLE_SYSTEM_PAGETAB => Self::Clickable,
            // オブジェクトは、ページ タブ コントロールのコンテナーを表します。
            ROLE_SYSTEM_PAGETABLIST => Self::Other,
            // オブジェクトは、フレームまたはドキュメント ウィンドウ内のペインを表します。 ユーザーは、ペイン間や現在のペインの内容の中は移動できますが、異なるペインの項目間は移動できません。 したがって、ペインは、フレームまたはドキュメント ウィンドウよりも低いが、個々のコントロールよりも高いグループ化レベルを表します。 ユーザーは、状況に応じて、TAB、F6、または CTRL + TAB キーを押すことによって、ペイン間を移動します。
            ROLE_SYSTEM_PANE => Self::Other,
            // オブジェクトは進行状況バーを表し、実行中の操作の完了量を動的に示します。 このコントロールは、ユーザー入力を受け取らなくなります。
            ROLE_SYSTEM_PROGRESSBAR => Self::Other,
            // オブジェクトはプロパティ シートを表します。
            ROLE_SYSTEM_PROPERTYPAGE => Self::Other,
            // オブジェクトは、プッシュ ボタン コントロールを表します。
            ROLE_SYSTEM_PUSHBUTTON => Self::Clickable,
            // オブジェクトは、オプション ボタン (以前はラジオ ボタン) を表します。 これは、相互に排他的なオプションのグループの 1 つです。 同じ親を共有し、この属性を持つすべてのオブジェクトは、相互に排他的な 1 つのグループの一部であると見なされます。 これらのオブジェクトを個別のグループに分割するには、ROLE_SYSTEM_GROUPING オブジェクトを使用します。
            ROLE_SYSTEM_RADIOBUTTON => Self::Clickable,
            // オブジェクトは、テーブル内のセルの行を表します。
            ROLE_SYSTEM_ROW => Self::Other,
            // オブジェクトは行ヘッダーを表し、テーブル行の視覚的なラベルを提供します。
            ROLE_SYSTEM_ROWHEADER => Self::Other,
            // オブジェクトは、垂直または水平のスクロール バーを表します。これは、クライアント領域の一部であるか、コントロールで使用されます。
            ROLE_SYSTEM_SCROLLBAR => Self::Other,
            // オブジェクトは、スペースを 2 つの領域に視覚的に分割するために使用されます。 区切り記号オブジェクトの例としては、区切り記号メニュー項目と、ウィンドウ内の分割ウィンドウを分割するバーがあります。
            ROLE_SYSTEM_SEPARATOR => Self::Other,
            // オブジェクトはスライダーを表します。これにより、ユーザーは、最小値と最大値の間で特定の増分で設定を調整できます。
            ROLE_SYSTEM_SLIDER => Self::Other,
            // オブジェクトは、さまざまなシステム イベントに関連付けられているシステム サウンドを表します。
            ROLE_SYSTEM_SOUND => Self::Other,
            // オブジェクトはスピン ボックスを表します。これは、ユーザーがスピン ボックスに関連付けられている別の "バディ" コントロールに表示される値をインクリメントまたはデクリメントできるようにするコントロールです。
            ROLE_SYSTEM_SPINBUTTON => Self::Other,
            // オブジェクトは、ボタンに直接隣接するドロップダウン リスト アイコンがあるツールバー上のボタンを表します。
            ROLE_SYSTEM_SPLITBUTTON => Self::Other,
            // オブジェクトは、他のコントロールのラベルやダイアログ ボックスの指示など、読み取り専用のテキストを表します。 静的テキストは変更または選択できません。
            ROLE_SYSTEM_STATICTEXT => Self::StaticText,
            // オブジェクトは、ウィンドウの下部にある領域であり、現在の操作、アプリケーションの状態、または選択したオブジェクトに関する情報を表示するステータス バーを表します。 ステータス バーには、さまざまな種類の情報を表示する複数のフィールドがあります。
            ROLE_SYSTEM_STATUSBAR => Self::Other,
            // オブジェクトは、セルの行と列、および必要に応じて行ヘッダーと列ヘッダーを含むテーブルを表します。
            ROLE_SYSTEM_TABLE => Self::Other,
            // オブジェクトは、編集を許可する選択可能なテキストを表すか、読み取り専用として指定されます。
            ROLE_SYSTEM_TEXT => Self::Editable,
            // オブジェクトは、ウィンドウのタイトルまたはキャプションバーを表します。
            ROLE_SYSTEM_TITLEBAR => Self::Other,
            // オブジェクトはツールバーを表します。これは、頻繁に使用される機能に簡単にアクセスできるコントロールのグループです。
            ROLE_SYSTEM_TOOLBAR => Self::Clickable,
            // オブジェクトは、役に立つヒントを提供するツールヒントを表します。
            ROLE_SYSTEM_TOOLTIP => Self::Other,
            // オブジェクトは、他のオブジェクト間の空白を表します。
            ROLE_SYSTEM_WHITESPACE => Self::Other,
            // オブジェクトはウィンドウ フレームを表します。このフレームには、タイトル バー、クライアント、ウィンドウのその他のオブジェクトなどの子オブジェクトが含まれます。
            ROLE_SYSTEM_WINDOW => Self::Other,
            r => Self::Invalid(r),
        }
    }
}
enum PosAccType {
    DisplayOrApi,
    Display,
    Api,
    Name,
    Value,
    Role,
    State,
    Description,
    Location,
}
impl From<u16> for PosAccType {
    fn from(value: u16) -> Self {
        match value {
            1 => Self::Display,
            2 => Self::Api,
            3 => Self::Name,
            4 => Self::Value,
            5 => Self::Role,
            6 => Self::State,
            7 => Self::Description,
            8 => Self::Location,
            _ => Self::DisplayOrApi,
        }
    }
}
pub enum PosAccResult {
    String(String),
    Vec(Vec<String>),
    Location([i32; 4])
}
enum ChkBtnResult {
    Unchecked,
    Checked,
    // Gray,
}
impl From<u32> for ChkBtnResult {
    fn from(state: u32) -> Self {
        match (state & STATE_SYSTEM_CHECKED) > 0 {
            true => Self::Checked,
            false => Self::Unchecked,
        }
    }
}
impl From<ChkBtnResult> for i32 {
    fn from(value: ChkBtnResult) -> Self {
        match value {
            ChkBtnResult::Unchecked => 0,
            ChkBtnResult::Checked => 1,
            // ChkBtnResult::Gray => 2,
        }
    }
}

pub trait IAccessibleExt {
    fn as_iaccessible(&self) -> &IAccessible;
    fn varchild(&self) -> VARIANT;
    fn childid_self() -> VARIANT {
        VARIANT {
            Anonymous: VARIANT_0 {
                Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                    vt: VT_I4,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: VARIANT_0_0_0 {
                        lVal: CHILDID_SELF as i32,
                    },
                })
            }
        }
    }
    /// ロールを得る
    fn role(&self) -> core::Result<u32> {
        unsafe {
            let role = self.as_iaccessible().get_accRole(self.varchild())?;
            let role = role.Anonymous.Anonymous.Anonymous.lVal as u32;
            Ok(role)
        }
    }
    /// ロール名を得る
    fn role_text(&self) -> core::Result<String> {
        unsafe {
            let role = self.role()?;
            let size = GetRoleTextW(role, None) as usize;
            let mut buf = vec![0; size+1];
            GetRoleTextW(role, Some(&mut buf));
            // remove \0
            let trimed = if let Some(right) = buf.iter().rposition(|n| *n != 0) {
                &buf[0..=right]
            } else {
                &buf
            };
            Ok(String::from_utf16_lossy(trimed))
        }
    }
    fn role_is_one_of(&self, roles: &[u32]) -> bool {
        self.role().is_ok_and(|r| roles.contains(&r))
    }
    fn role_is(&self, other: u32) -> bool {
        self.role().is_ok_and(|role| role==other)
    }
    /// 親オブジェクトを得る
    fn parent(&self) -> Option<Self> where Self: Sized;
    /// 自身のHWNDを得る
    fn hwnd(&self) -> core::Result<HWND> {
        let mut hwnd = HWND::default();
        unsafe { WindowFromAccessibleObject(self.as_iaccessible(), Some(&mut hwnd))?; }
        Ok(hwnd)
    }
    /// スクリーン座標を得る\
    /// [left, top, width, height]
    fn location(&self) -> core::Result<[i32; 4]> {
        unsafe {
            let mut loc = [0i32; 4];
            self.as_iaccessible().accLocation(
                &mut loc[0], &mut loc[1],
                &mut loc[2], &mut loc[3],
                self.varchild()
            )?;
            Ok(loc)
        }
    }
    /// HWNDに対する自身のクライアント座標を得る\
    /// [left, top, width, height]
    fn _client_location(&self, hwnd: HWND) -> core::Result<[i32; 4]> {
        unsafe {
            let mut loc = self.location()?;
            let mut p = POINT { x: loc[0], y: loc[1] };
            ScreenToClient(hwnd, &mut p);
            loc[0] = p.x;
            loc[1] = p.y;
            Ok(loc)
        }
    }
    /// 自身の名前を返す
    fn name(&self) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accName(self.varchild()).map(|bstr| bstr.to_string())
        }
    }
    /// 値を得る
    fn value(&self) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accValue(self.varchild()).map(|bstr| bstr.to_string())
        }
    }
    fn set_value(&self, value: &str) -> core::Result<()> {
        unsafe {
            let bstr = BSTR::from(value);
            self.as_iaccessible().put_accValue(self.varchild(), &bstr)
        }
    }
    fn default_action(&self) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accDefaultAction(self.varchild())
                .map(|bstr| bstr.to_string())
        }
    }
    /// 自身のデフォルトアクションを実行する
    fn do_default_action(&self) -> core::Result<()> {
        unsafe {
            self.as_iaccessible().accDoDefaultAction(self.varchild())
        }
    }
    /// 自身の状態を返す
    fn state(&self) -> core::Result<u32> {
        unsafe {
            let var_state = self.as_iaccessible().get_accState(self.varchild())?;
            let state = var_state.Anonymous.Anonymous.Anonymous.lVal as u32;
            Ok(state)
        }
    }
    // /// 自身の状態に状態定数が含まれるかどうか
    // fn state_includes(&self, state: u32) -> core::Result<bool> {
    //     let states = self.state(self.varchild())?;
    //     Ok(states.includes(state))
    // }
    /// 自身の状態を文字列で返す
    fn state_text(&self) -> core::Result<Vec<String>> {
        let states = self.state()?;
        let texts = (0..32).filter_map(|n| {
            let state = 2u32.pow(n);
            ((states & state) == state).then_some(Self::state_to_text(state))
        })
        .collect();
        Ok(texts)
    }
    /// 状態を表す定数を文字列にする
    fn state_to_text(state: u32) -> String {
        unsafe {
            let size = GetStateTextW(state, None) as usize;
            let mut buf = vec![0; size+1];
            GetStateTextW(state, Some(&mut buf));
            // remove \0
            let trimed = if let Some(right) = buf.iter().rposition(|n| *n != 0) {
                &buf[0..=right]
            } else {
                &buf
            };
            String::from_utf16_lossy(trimed)
        }
    }
    fn has_state(&self, state: u32) -> bool {
        self.state().is_ok_and(|s| s.includes(state))
    }
    /// 説明を得る
    fn description(&self) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accDescription(self.varchild())
                .map(|bstr| bstr.to_string())
        }
    }
    /// 子の数を得る
    fn child_count(&self) -> usize {
        unsafe {
            self.as_iaccessible().accChildCount().unwrap_or_default() as usize
        }
    }
    /// 子要素を得る
    fn children(&self) -> Vec<VARIANT> {
        unsafe {
            let size = self.child_count();
            let mut rgvarchildren = vec![VARIANT::default(); size];
            let _ = AccessibleChildren(self.as_iaccessible(), 0, &mut rgvarchildren, &mut 0);
            rgvarchildren
        }
    }
    /// ディセーブルかどうか
    fn is_disabled(&self) -> bool {
        match self.state() {
            Ok(s) => s.includes(STATE_SYSTEM_UNAVAILABLE.0),
            // エラーはdisabledと見なす
            Err(_) => true
        }
    }
    fn is_focused(&self) -> bool {
        match self.state() {
            Ok(s) => s.includes(STATE_SYSTEM_FOCUSED),
            Err(_) => false,
        }
    }
    fn is_focusable(&self) -> bool {
        match self.state() {
            Ok(s) => s.includes(STATE_SYSTEM_FOCUSABLE.0),
            Err(_) => false,
        }
    }
    fn is_visible(&self) -> bool {
        match self.state() {
            Ok(s) => !s.includes(STATE_SYSTEM_INVISIBLE.0),
            Err(_) => false,
        }
    }
    fn _is_offscreen(&self) -> bool {
        match self.state() {
            Ok(s) => !s.includes(STATE_SYSTEM_OFFSCREEN.0),
            Err(_) => false,
        }
    }
    fn user_draw_text(&self) -> Option<String> {
        None
    }
}
impl IAccessibleExt for IAccessible {
    fn as_iaccessible(&self) -> &IAccessible {
        self
    }
    fn varchild(&self) -> VARIANT {
        Self::childid_self()
    }
    fn parent(&self) -> Option<Self> where Self: Sized {
        unsafe {
            self.as_iaccessible().accParent()
                .and_then(|disp| disp.cast())
                .ok()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccWindow {
    /// IAccessibleオブジェクト
    inner: IAccessible,
    /// 自身のHWND
    hwnd: HWND,
}
impl AccWindow {
    /// IAccessibleからAccWindowを得る
    fn _from_iaccessible(acc: IAccessible) -> core::Result<Self> {
        unsafe {
            let mut hwnd = HWND::default();
            WindowFromAccessibleObject(&acc, Some(&mut hwnd))?;
            Ok(Self { inner: acc, hwnd})
        }
    }
    /// HWNDからAccWindowを得る
    fn from_hwnd(hwnd: HWND) -> core::Result<Self> {
        unsafe {
            let dwid = OBJID_WINDOW.0 as u32;
            let riid = &IAccessible::IID;
            let mut ppvobject = std::ptr::null_mut::<IAccessible>() as *mut c_void;
            AccessibleObjectFromWindow(hwnd, dwid, riid, &mut ppvobject)?;
            let inner = IAccessible::from_raw(ppvobject);
            Ok(Self { inner, hwnd})
        }
    }
    /// 自身のクライアント座標上にあるオブジェクトを得る (posacc用)
    fn child_from_client_point(&self, client_x: i32, client_y: i32) -> Option<AccChild> {
        unsafe {
            let mut p = POINT { x: client_x, y: client_y };
            ClientToScreen(self.hwnd, &mut p);
            let mut acc = None;
            let mut varchild = VARIANT::default();
            AccessibleObjectFromPoint(p, &mut acc, &mut varchild).ok()?;
            let child = AccChild::new(acc?, varchild, 0, 0);
            if child.role_is(ROLE_SYSTEM_STATICTEXT) {
                child.parent()
            } else {
                Some(child)
            }
        }
    }

    fn activate(&self) -> bool {
        unsafe {
            SetForegroundWindow(self.hwnd).as_bool()
        }
    }

    fn find_nth<P>(self, nth: usize, predicate: P) -> Option<AccChild>
    where P: FnMut(&AccChild) -> bool
    {
        let n = nth.saturating_sub(1);
        self.into_iter()
            .filter(predicate)
            .nth(n)
    }
    fn find_nth_text<T: Into<GetStrTarget>>(self, nth: T, roles: &[u32], mouse: bool) -> Option<String> {
        let found = match nth.into() {
            GetStrTarget::Focused => {
                self.activate();
                self.into_iter().find(|child| {
                    child.role_is_one_of(roles)
                    && child.is_focused()
                })?
            },
            GetStrTarget::OnlyEnabled(nth) => {
                self.find_nth(nth, |child| {
                    child.role_is_one_of(roles)
                    && !child.is_disabled()
                })?
            },
            GetStrTarget::IncludeDisabled(nth) => {
                self.find_nth(nth, |child| {
                    child.role_is_one_of(roles)
                })?
            },
        };
        let text = if found.role_is(ROLE_SYSTEM_TEXT) {
            found.value().ok()?
        } else {
            found.name().ok()?
        };
        if mouse {
            if let Ok([x, y, w, h]) = found.location() {
                let x = x + w / 2;
                let y = y + h / 2;
                move_mouse_to(x, y);
            }
        }
        Some(text)
    }

    fn into_value(self) -> Value {
        AccChild::from(self).into_value()
    }

    #[cfg(debug_assertions)]
    fn _out_all_item(hwnd: HWND, out_dir: &str) {
        if let Ok(window) = Self::from_hwnd(hwnd) {

            let all = window.into_iter()
                .map(|child| AccChildDetail::from(&child))
                .collect::<Vec<_>>();
            let contents = format!("{all:#?}");
            let mut path = std::path::PathBuf::from(out_dir);
            path.push(format!("{}.txt", hwnd.0));
            let _ = std::fs::write(&path, contents)
                .inspect_err(|e| {
                    dbg!(e, path);
                });
        }
    }
}

impl IntoIterator for AccWindow {
    type Item = AccChild;

    type IntoIter = AccIter;

    fn into_iter(self) -> Self::IntoIter {
        AccIter::new(self.inner, None, None)
    }
}

impl From<IAccessible> for AccChild {
    fn from(acc: IAccessible) -> AccChild {
        let varchild = AccChild::childid_self();
        Self::new(acc, varchild, 0, 0)
    }
}
pub struct AccChild {
    inner: IAccessible,
    // role: u32,
    varchild: VARIANT,
    index: usize,
    depth: u32,
}
impl std::fmt::Debug for AccChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccChild")
            .field("inner", &self.inner)
            .field("varchild", &self.varchild.vt())
            .field("index", &self.index)
            .field("depth", &self.depth)
            .finish()
    }
}
impl PartialEq for AccChild {
    fn eq(&self, other: &Self) -> bool {
        self.name().eq(&other.name())
        && self.id().eq(&other.id())
    }
}
impl IntoIterator for AccChild {
    type Item = AccChild;

    type IntoIter = AccIter;

    fn into_iter(self) -> Self::IntoIter {
        AccIter::new(self.inner, None, Some(self.depth+1))
    }
}
impl IAccessibleExt for AccChild {
    fn as_iaccessible(&self) -> &IAccessible {
        &self.inner
    }

    fn varchild(&self) -> VARIANT {
        self.varchild.clone()
    }
    fn parent(&self) -> Option<Self> where Self: Sized {
        let inner = if self.id().is_some_and(|id| id > 0) {
            self.inner.clone()
        } else {
            self.as_iaccessible().parent()?
        };
        Some(Self { inner, varchild: Self::childid_self(), index: 0, depth: self.depth.saturating_sub(1) })
    }
    fn role(&self) -> core::Result<u32> {
        let role = unsafe {
            self.as_iaccessible().get_accRole(self.varchild())?
                .Anonymous.Anonymous.Anonymous.lVal as u32
        };
        if role.eq(&ROLE_SYSTEM_LIST) {
            let is_listview = self.iter()
                .any(|c| !c.role_is(ROLE_SYSTEM_LISTITEM) && c.depth.eq(&(self.depth+1)));
            if is_listview {
                Ok(ROLE_SYSTEM_LISTVIEW)
            } else {
                Ok(ROLE_SYSTEM_LIST)
            }
        } else {
            Ok(role)
        }
    }
}
impl AccChild {
    fn id(&self) -> Option<i32> {
        unsafe {
            (self.varchild.vt() == VT_I4)
                .then_some(self.varchild.Anonymous.Anonymous.Anonymous.lVal)
        }
    }
    fn new(acc: IAccessible, varchild: VARIANT, index: usize, depth: u32) -> Self {
        Self { inner: acc, varchild, index, depth }
    }
    fn valid_name(&self) -> Option<String> {
        self.name().ok()
            .and_then(|name| {
                (!name.is_empty())
                    .then_some(name)
            })
    }
    fn name_matches_to(&self, other: &str, partial: bool) -> bool {
        self.valid_name()
            .is_some_and(|name| {
                if partial {
                    name.partial_match(other)
                } else {
                    name.exact_match(other)
                }
            })
    }
    fn iter(&self) -> AccIter {
        AccIter::new(self.inner.clone(), None, Some(self.depth+1))
    }
    fn from_idispatch(disp: &IDispatch, index: usize, depth: u32) -> Option<Self> {
        disp.cast::<IAccessible>().ok()
            .map(Self::from)
            .map(|mut child| {
                child.index = index;
                child.depth = depth;
                child
            })
    }
    fn select(&self, flags: u32) -> core::Result<()> {
        unsafe {
            self.as_iaccessible().accSelect(flags as i32, self.varchild())
        }
    }
    /// 他の選択に加えて選択
    pub fn add_select(&self) -> bool {
        self.select(SELFLAG_ADDSELECTION).is_ok()
    }
    /// 単独で選択
    pub fn select_one(&self) -> bool {
        self.select(SELFLAG_TAKEFOCUS|SELFLAG_TAKESELECTION).is_ok()
    }
    fn is_checked(&self) -> bool {
        if let Ok(state) = self.state() {
            state.includes(STATE_SYSTEM_CHECKED)
        } else {
            false
        }
    }
    fn name_includes(&self, other: &str) -> bool {
        self.name().is_ok_and(|name| name.partial_match(other))
    }
    pub fn click(self, check: bool) -> bool {
        match self.role().unwrap_or(0) {
            ROLE_SYSTEM_LISTITEM => self.list_click(check),
            ROLE_SYSTEM_OUTLINEITEM => self.treeview_click(check),
            ROLE_SYSTEM_CHECKBUTTON => self.check(check),
            ROLE_SYSTEM_MENUITEM => self.menu_click(check),
            n if n > 0 => if check {
                // クリック
                self.do_default_action().is_ok()
            } else {
                // クリックはしない
                true
            },
            _ => false,
        }
    }
    fn check(&self, check: bool) -> bool {
        match (check, self.is_checked()) {
            // 既にチェック済み
            (true, true) |
            // 既に未チェック
            (false, false) => true,
            // チェックする
            (true, false) => if self.do_default_action().is_ok() {
                self.is_checked()
            } else {
                false
            },
            // チェックを外す
            (false, true) => if self.do_default_action().is_ok() {
                ! self.is_checked()
            } else {
                false
            },
        }
    }
    /// 親を遡りトップからメニューリストを開いていく\
    /// 親がいればそれを展開し、その後再取得した自身を返す
    fn open_menu(self) -> Self {
        let parent = self.parent().filter(|p| p.role_is(ROLE_SYSTEM_MENUPOPUP))
            .and_then(|p| p.parent().filter(|p| p.role_is(ROLE_SYSTEM_MENUITEM)));
        if let Some(parent) = parent {
            // 親に親がいればそれを先に展開し、親オブジェクトを再取得
            let parent = parent.open_menu();
            // 親を展開
            let _ = parent.do_default_action();
            std::thread::sleep(std::time::Duration::from_millis(10));
            // 親から自身と同じオブジェクトを再取得して返す
            parent.iter()
                .find(|p| self.eq(p))
                // 同じオブジェクトがあるのは自明なのでunwrapしてもOK
                .unwrap()
        } else {
            // 親がいない場合は自身を返す
            self
        }
    }
    fn menu_click(self, check: bool) -> bool {
        match (check, self.is_checked()) {
            // 既にチェック済み
            (true, true) |
            // 既に未チェック
            (false, false) => true,
            // チェックする
            (true, false) |
            // チェックを外す
            (false, true) => {
                self.open_menu()
                    .do_default_action()
                    .is_ok()
            },
        }
    }
    fn list_click(&self, check: bool) -> bool {
        if self.has_state(STATE_SYSTEM_INVISIBLE.0) {
            // 不可視の場合コンボボックスかもしれない
            if let Some(combo) = self.find_ancestor(ROLE_SYSTEM_COMBOBOX) {
                if let Some(btn) = combo.iter()
                    // 開くボタンを探す
                    .find(|c| c.role_is(ROLE_SYSTEM_PUSHBUTTON) && c.depth == combo.depth)
                {
                    let _ = btn.do_default_action();
                    return if let Some(item) = combo.into_iter().find(|c| c.eq(self)) {
                        if check {
                            item.do_default_action().is_ok()
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                }
            }
        }
        if check {
            self.select_one()
        } else {
            true
        }
    }
    fn find_ancestor(&self, role: u32) -> Option<Self> {
        let parent = self.parent()?;
        if parent.role_is(role) {
            Some(parent)
        } else {
            parent.find_ancestor(role)
        }
    }
    fn treeview_click(&self, check: bool) -> bool {
        if ! check { return true; }
        self.select_one() && if self.default_action().is_ok() {
            self.do_default_action().is_ok()
        } else {
            true
        }
    }
    fn get_item_value(&self) -> Option<String> {
        if self.role_is(ROLE_SYSTEM_TEXT) && self.state().is_ok_and(|s| !s.includes(STATE_SYSTEM_LINKED)) {
            // エディットボックスかつリンクではないならvalueを返す
            // 空文字も返す
            self.value().ok()
        } else {
            // その他は名前
            self.name()
                .ok()
                .and_then(|name| {
                    let name = name.remove_mnemonic();
                    (!name.is_empty()).then_some(name.to_string())
                })
        }
    }
    fn dbg_detail(&self) {
        #[cfg(debug_assertions)]
        dbg!(AccChildDetail::from(self));
    }
    fn is_object(&self) -> bool {
        self.id().is_some_and(|n| n.eq(&0))
    }
    fn child_objects(&self) -> impl Iterator<Item = Self> {
        self.children().into_iter().enumerate()
            .filter_map(|(index, varchild)| {
                match varchild.vt() {
                    VT_I4 => {
                        Some(Self::new(self.inner.clone(), varchild, index, self.depth+1))
                    },
                    VT_DISPATCH => {
                        varchild.to_idispatch().ok()
                            .and_then(|disp| Self::from_idispatch(&disp, index, self.depth+1))
                    },
                    _ => None
                }
            })
    }
    fn into_value(self) -> Value {
        fn to_error_string(e: core::Error) -> String {
            format!("Error: {e}")
        }
        let children = if self.is_object() {
            let v = self.child_objects()
                .map(Self::into_value)
                .collect::<Vec<_>>();
            Some(v)
        } else {
            None
        };
        json!({
            "name": self.name().unwrap_or_else(to_error_string),
            "value": self.value().unwrap_or_else(to_error_string),
            "role": [
                self.role().map(|r|r.to_string()).unwrap_or_else(to_error_string),
                self.role_text().unwrap_or_else(to_error_string),
            ],
            "status": [
                self.state().map(|s|s.to_string()).unwrap_or_else(to_error_string),
                self.state_text().map(|v|v.join(", ")).unwrap_or_else(to_error_string),
            ],
            "default_action": self.default_action().unwrap_or_else(to_error_string),
            "description": self.description().unwrap_or_else(to_error_string),
            "location": self.location().unwrap_or_default(),
            "hwnd": self.hwnd().unwrap_or_default().0,
            "children": children,
        })
    }
    fn is_valid(&self) -> bool {
        (self.is_visible() || self.is_focusable())
        && self.state().is_ok_and(|s| s > 0)
    }
}
impl From<AccWindow> for AccChild {
    fn from(window: AccWindow) -> Self {
        Self::new(window.inner, Self::childid_self(), 0, 0)
    }
}

#[derive(Debug)]
#[cfg(debug_assertions)]
#[allow(unused)]
struct AccChildDetail {
    inner: String,
    name: String,
    role: String,
    role_text: String,
    status: String,
    status_text: String,
    value: String,
    description: String,
    location: String,
    id: Option<i32>,
    child_count: usize,
    default_action: String,
    hwnd: HWND,
    index: usize,
    depth: u32,
}
#[cfg(debug_assertions)]
impl From<&AccChild> for AccChildDetail {
    fn from(child: &AccChild) -> Self {
        Self {
            inner: format!("{:?}", &child.inner),
            name: child.name().unwrap_or_else(|e| e.to_string()),
            role: child.role().map(|n| n.to_string()).unwrap_or_else(|e| e.to_string()),
            role_text: child.role_text().unwrap_or_else(|e| e.to_string()),
            status: child.state().map(|s|s.to_string()).unwrap_or_else(|e|e.to_string()),
            status_text: child.state_text().map(|v| v.join(", ")).unwrap_or_else(|e| e.to_string()),
            value: child.value().unwrap_or_else(|e| e.to_string()),
            description: child.description().unwrap_or_else(|e| e.to_string()),
            location: child.location().map(|loc| format!("{loc:?}")).unwrap_or_else(|e|e.to_string()),
            id: child.id(),
            child_count: child.child_count(),
            default_action: child.default_action().unwrap_or_else(|e| e.to_string()),
            hwnd: child.hwnd().unwrap_or_default(),
            index: child.index,
            depth: child.depth,
        }
    }
}

#[derive(Clone)]
pub struct AccIter {
    acc: IAccessible,
    items: Vec<VARIANT>,
    reverse: bool,
    parent: Option<Box<Self>>,
    _depth: u32,
    index: usize,
}
impl AccIter {
    fn new(acc: IAccessible, items: Option<Vec<VARIANT>>, depth: Option<u32>) -> Self {
        let items = match items {
            Some(items) => items,
            None => acc.children(),
        };
        Self {
            acc,
            items,
            reverse: false,
            parent: None,
            _depth: depth.unwrap_or_default(),
            index: 0,
        }
    }
    fn reverse(&mut self) {
        self.reverse = true;
        self.items.reverse();
    }
    fn new_branch(&mut self, acc: IAccessible) {
        let _depth = self._depth + 1;
        let mut items = acc.children();
        if self.reverse {
            items.reverse();
        }
        // 下の階層のイテレータを作る
        let iter = Self {
            acc,
            items,
            reverse: self.reverse,
            parent: None,
            _depth,
            index: 0,
        };
        // selfに新たなイテレータを書き込み、自身を取り出す
        let current = std::mem::replace(self, iter);
        // 自身をparentに書き込む
        self.parent.replace(Box::new(current));
    }
}
impl Iterator for AccIter {
    type Item = AccChild;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if let Some(varchild) = self.items.get(self.index) {
                let index = self.index;
                self.index += 1;
                match varchild.vt() {
                    VT_I4 => {
                        Some(AccChild::new(self.acc.clone(), varchild.clone(), index, self._depth))
                    },
                    VT_DISPATCH => {
                        let child = varchild.Anonymous.Anonymous.Anonymous.pdispVal
                            .as_ref()
                            .and_then(|disp| AccChild::from_idispatch(disp, index, self._depth))?;
                        self.new_branch(child.inner.clone());
                        Some(child)
                    },
                    _vt => {
                        // vtが不正な場合はスキップ
                        self.next()
                    },
                }
            } else if let Some(parent) = self.parent.take() {
                *self = *parent;
                self.next()
            } else {
                None
            }
        }
    }
}

trait AccNameMatch {
    fn exact_match(&self, other: &str) -> bool;
    fn partial_match(&self, other: &str) -> bool;
    fn remove_mnemonic(&self) -> &str;
    fn find_ignore_ascii_case(&self, pat: &str) -> Option<usize>;
}

impl<T> AccNameMatch for T where T: std::ops::Deref<Target = str> {
    fn exact_match(&self, other: &str) -> bool {
        self.remove_mnemonic().eq_ignore_ascii_case(other)
    }

    fn partial_match(&self, other: &str) -> bool {
        self.find_ignore_ascii_case(other).is_some()
    }
    /// ニーモニックを除去した名前
    fn remove_mnemonic(&self) -> &str {
        const LEFT: &str = "(";
        const RIGHT: &str = ")";
        if let Some(a) = self.find(LEFT) {
            if let Some(b) = self.find(RIGHT) {
                // ) の位置が ( とアルファベットの後かどうか
                if b == a + LEFT.len() + 1 {
                    // ) の前の文字がアルファベットかどうか
                    if self.is_char_boundary(b-1) && self[b-1..b].chars().next().is_some_and(char::is_alphabetic) {
                        if a == 0 {
                            &self[b+1..]
                        } else {
                            &self[..a]
                        }
                    } else {
                        self
                    }
                } else {
                    self
                }
            } else {
                self
            }
        } else {
            self
        }
    }
    fn find_ignore_ascii_case(&self, other: &str) -> Option<usize> {
        let pat_bytes = other.as_bytes();
        self.as_bytes().windows(other.len()).enumerate()
            .find_map(|(i, w)| w.eq_ignore_ascii_case(pat_bytes).then_some(i) )
    }
}

enum GetStrTarget {
    OnlyEnabled(usize),
    Focused,
    IncludeDisabled(usize),
}
impl From<i32> for GetStrTarget {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Focused,
            1.. => Self::OnlyEnabled(value.unsigned_abs() as usize),
            _ => Self::IncludeDisabled(value.unsigned_abs() as usize),
        }
    }
}
