use std::ffi::c_void;

use windows::{
    core::{self, Interface, ComInterface, BSTR},
    Win32::{
        Foundation::{HWND, POINT, E_INVALIDARG},
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
                STATE_SYSTEM_CHECKED,
            },
            Controls::STATE_SYSTEM_UNAVAILABLE,
        },
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
        System::Variant::{VARIANT, VT_I4, VT_DISPATCH},
        System::Com::IDispatch,
    }
};

use std::ops::ControlFlow;

use crate::{builtins::window_low::move_mouse_to, object::VariantExt, U32Ext};
use super::clkitem::{ClkItem, ClkTarget};

pub struct Acc {}

impl Acc {
    pub fn getitem(hwnd: HWND, target: u32, max_count: i32, ignore_disabled: bool) -> Option<Vec<String>> {
        let gi = GetItem::new(target, max_count, ignore_disabled)?;
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        let mut iter = window.into_iter();
        if gi.reverse {
            iter.reverse();
        }
        let result = Vec::with_capacity(gi.count);
        let flow = iter
            .filter(|child| {
                gi.role_matches_to(child.role)
            })
            .try_fold(result, |mut result, child| {
                if let Ok(name) = child.name() {
                    if !name.is_empty() {
                        result.push(name);
                    }
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
                child.role_is_one_of(&[ROLE_SYSTEM_CHECKBUTTON])
                && child.name_includes(name)
            })
            .and_then(|child| child.state().ok())
            .map(|state| ChkBtnResult::from(state).into())
    }
    pub fn get_edit_str(hwnd: HWND, nth: usize, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_TEXT], mouse)
    }
    pub fn get_static_str(hwnd: HWND, nth: usize, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_STATICTEXT], mouse)
    }
    pub fn get_cell_str(hwnd: HWND, nth: usize, mouse: bool) -> Option<String> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        window.find_nth_text(nth, &[ROLE_SYSTEM_CELL], mouse)
    }
    pub fn sendstr<R>(hwnd: HWND, nth: usize, str: &str, replace: R) where R: Into<bool> {
        if let Ok(window) = AccWindow::from_hwnd(hwnd) {
            let replace: bool = replace.into();
            if let Some(child) = window.find_nth(nth, |child| child.role_is_one_of(&[ROLE_SYSTEM_TEXT])) {
                if replace {
                    child.set_value(str);
                } else if let Ok(old) = child.value() {
                    let value = old + str;
                    child.set_value(&value);
                } else {
                    child.set_value(str);
                }
            }
        }
    }
    pub fn sendstr_cell<R>(hwnd: HWND, nth: usize, str: &str, replace: R) where R: Into<bool> {
        if let Ok(window) = AccWindow::from_hwnd(hwnd) {
            let replace: bool = replace.into();
            let maybe = window
                .find_nth(nth, |child| child.role_is_one_of(&[ROLE_SYSTEM_CELL]))
                .and_then(|cell| cell.into_iter().find(|child| child.role_is_one_of(&[ROLE_SYSTEM_TEXT])));
            if let Some(child) = maybe {
                if replace {
                    child.set_value(str);
                } else if let Ok(old) = child.value() {
                    let value = old + str;
                    child.set_value(&value);
                } else {
                    child.set_value(str);
                }
            }
        }
    }
    pub fn find_click_target(hwnd: HWND, item: &ClkItem) -> Option<AccChild> {
        let window = AccWindow::from_hwnd(hwnd).ok()?;
        let mut iter = window.into_iter();
        if item.backwards {
            iter.reverse();
        }
        let roles = u32::from(&item.target);
        let nth = item.order as usize;
        if let Some(path_iter) = item.name_as_path() {
            // 探す名前が path\to\item の場合
            let children = iter.filter(|child| child.role_is_one_of(&[ROLE_SYSTEM_OUTLINE, ROLE_SYSTEM_MENUBAR]));
            let found = path_iter.fold(None::<AccChild>, |found, name| {
                // let children = if let Some(child) = found {
                //     child.into_iter()
                // } else {
                //     children
                // };
                None
            });
            found
        } else {
            let filter = Self::find_click_target_filter(&item.name, item.short);
            iter.filter(|child| roles.includes(child.role))
                .filter_map(filter)
                .nth(nth)
        }
    }
    fn find_click_target_filter(name: &str, partial: bool) -> impl FnMut(AccChild) -> Option<AccChild>
    {
        let find_name_matched = move |id: AccIdChild<'_>| -> Option<AccChild> {
            id.valid_name()
                .is_some_and(|child_name| {
                    if partial {
                        child_name.partial_match(name)
                    } else {
                        child_name.exact_match(name)
                    }
                })
                .then_some(AccChild::from(id))
        };
        let f = move |child: AccChild| {
            match child.role {
                ROLE_SYSTEM_LIST => {
                    child.iter()
                        .filter(|id| id.role_is(ROLE_SYSTEM_LISTITEM))
                        .find_map(find_name_matched)
                },
                ROLE_SYSTEM_MENUBAR => {
                    child.iter()
                        .filter(|id| id.role_is(ROLE_SYSTEM_MENUITEM))
                        .find_map(find_name_matched)
                },
                ROLE_SYSTEM_PAGETABLIST => {
                    child.iter()
                        .filter(|id| id.role_is(ROLE_SYSTEM_PAGETAB))
                        .find_map(find_name_matched)
                },
                ROLE_SYSTEM_TOOLBAR => {
                    child.iter()
                        .find_map(find_name_matched)
                },
                ROLE_SYSTEM_OUTLINE => {
                    // treeviewはネストするのでどうする
                    todo!()
                }
                _ => Some(child)
            }
        };
        // Box::new(f)
        f
    }
    fn role_from_target(target: &ClkTarget) -> u32 {
        let mut role = 0;
        if target.button {
            role |= ROLE_SYSTEM_PUSHBUTTON|ROLE_SYSTEM_CHECKBUTTON|ROLE_SYSTEM_RADIOBUTTON;
        }
        if target.link {
            role |= ROLE_SYSTEM_LINK;
        }
        if target.list {
            role |= ROLE_SYSTEM_LIST;
        }
        if target.listview {
            role |= ROLE_SYSTEM_LIST;
        }
        if target.menu {
            role |= ROLE_SYSTEM_MENUBAR;
        }
        if target.tab {
            role |= ROLE_SYSTEM_PAGETABLIST;
        }
        if target.toolbar {
            role |= ROLE_SYSTEM_TOOLBAR;
        }
        if target.treeview {
            role |= ROLE_SYSTEM_OUTLINE;
        }
        role
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
    fn role_matches_to(&self, other: u32) -> bool {
        let other = GetItemRole::from(other);
        self.role.eq(&other)
    }
}

#[derive(Debug, PartialEq)]
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
            ROLE_SYSTEM_BUTTONDROPDOWN => Self::Clickable,
            // オブジェクトは、グリッドを展開するボタンを表します。
            ROLE_SYSTEM_BUTTONDROPDOWNGRID => Self::Clickable,
            // オブジェクトは、メニューを展開するボタンを表します。
            ROLE_SYSTEM_BUTTONMENU => Self::Clickable,
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
            ROLE_SYSTEM_MENUITEM => Self::Clickable,
            // オブジェクトはメニューを表します。各メニューは、特定のアクションを持つオプションの一覧です。 メニュー バーから選択すると表示されるドロップダウン メニューを含め、すべてのメニューの種類にロールが必要です。および ショートカット メニュー。マウスの右ボタンをクリックして表示されます。
            ROLE_SYSTEM_MENUPOPUP => Self::Other,
            // オブジェクトは、ツリー ビュー コントロールなどのアウトラインまたはツリー構造を表し、階層リストを表示し、ユーザーがブランチを展開および折りたたみできるようにします。
            ROLE_SYSTEM_OUTLINE => Self::Other,
            // オブジェクトは、アウトライン項目のように移動する項目を表します。 上方向キーと下方向キーは、アウトライン内を移動するために使用されます。 ただし、左方向キーと右方向キーを押したときに展開と折りたたみを行う代わりに、SPACE キーまたは Enter キーを押したときに項目にフォーカスがあるときに、これらのメニューが展開または折りたたみされます。
            ROLE_SYSTEM_OUTLINEBUTTON => Self::Other,
            // オブジェクトは、アウトライン構造またはツリー構造の項目を表します。
            ROLE_SYSTEM_OUTLINEITEM => Self::Clickable,
            // オブジェクトはページ タブを表します。ページ タブ コントロールの唯一の子は、関連付けられたページの内容を持つROLE_SYSTEM_GROUPING オブジェクトです。
            ROLE_SYSTEM_PAGETAB => Self::Other,
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
    Gray,
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
            ChkBtnResult::Gray => 2,
        }
    }
}

trait IAccessibleExt {
    fn as_iaccessible(&self) -> &IAccessible;

    /// ロールを得る
    fn role(&self, varchild: VARIANT) -> core::Result<u32> {
        unsafe {
            let role = self.as_iaccessible().get_accRole(varchild)?;
            let role = role.Anonymous.Anonymous.Anonymous.lVal as u32;
            Ok(role)
        }
    }
    /// ロール名を得る
    fn role_text(&self, varchild: VARIANT) -> core::Result<String> {
        unsafe {
            let role = self.role(varchild)?;
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
    fn role_is_one_of(&self, roles: &[u32], varchild: VARIANT) -> bool {
        match self.role(varchild) {
            Ok(role) => roles.contains(&role),
            Err(_) => false,
        }
    }
    fn role_is(&self, other: u32, varchild: VARIANT) -> bool {
        match self.role(varchild) {
            Ok(role) => role == other,
            Err(_) => false,
        }
    }
    /// 親オブジェクトを得る
    fn parent(&self) -> core::Result<Option<IAccessible>>
    where Self: Sized
    {
        unsafe {
            self.as_iaccessible().accParent().ok()
                .map(|disp| disp.cast::<IAccessible>())
                .transpose()
        }
    }
    /// 自身のHWNDを得る
    fn hwnd(&self) -> core::Result<HWND> {
        let mut hwnd = HWND::default();
        unsafe { WindowFromAccessibleObject(self.as_iaccessible(), Some(&mut hwnd))?; }
        Ok(hwnd)
    }
    /// スクリーン座標を得る\
    /// [left, top, width, height]
    fn location(&self, varchild: VARIANT) -> core::Result<[i32; 4]> {
        unsafe {
            let mut loc = [0i32; 4];
            self.as_iaccessible().accLocation(
                &mut loc[0], &mut loc[1],
                &mut loc[2], &mut loc[3],
                varchild
            )?;
            Ok(loc)
        }
    }
    /// HWNDに対する自身のクライアント座標を得る\
    /// [left, top, width, height]
    fn client_location(&self, hwnd: HWND, varchild: VARIANT) -> core::Result<[i32; 4]> {
        unsafe {
            let mut loc = self.location(varchild)?;
            let mut p = POINT { x: loc[0], y: loc[1] };
            ScreenToClient(hwnd, &mut p);
            loc[0] = p.x;
            loc[1] = p.y;
            Ok(loc)
        }
    }
    /// 自身の名前を返す
    fn name(&self, varchild: VARIANT) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accName(varchild).map(|bstr| bstr.to_string())
        }
    }
    /// 値を得る
    fn value(&self, varchild: VARIANT) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accValue(varchild).map(|bstr| bstr.to_string())
        }
    }
    fn set_value(&self, varchild: VARIANT, value: &str) -> core::Result<()> {
        unsafe {
            let bstr = BSTR::from(value);
            self.as_iaccessible().put_accValue(varchild, &bstr)
        }
    }
    /// 自身のデフォルトアクションを実行する
    fn default_action(&self, varchild: VARIANT) -> core::Result<()> {
        unsafe {
            self.as_iaccessible().accDoDefaultAction(varchild)
        }
    }
    /// 自身の状態を返す
    fn state(&self, varchild: VARIANT) -> core::Result<u32> {
        unsafe {
            let var_state = self.as_iaccessible().get_accState(varchild)?;
            let state = var_state.Anonymous.Anonymous.Anonymous.lVal as u32;
            Ok(state)
        }
    }
    /// 自身の状態に状態定数が含まれるかどうか
    fn includes(&self, state: u32, varchild: VARIANT) -> core::Result<bool> {
        let states = self.state(varchild)?;
        let includes = (states & state) == state;
        Ok(includes)
    }
    /// 自身の状態を文字列で返す
    fn state_text(&self, varchild: VARIANT) -> core::Result<Vec<String>> {
        let states = self.state(varchild)?;
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
    /// 説明を得る
    fn description(&self, varchild: VARIANT) -> core::Result<String> {
        unsafe {
            self.as_iaccessible().get_accDescription(varchild)
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
            let _ = self.as_iaccessible().accFocus()
                .inspect_err(|e| {dbg!(e);});
            let size = self.child_count();
            let mut rgvarchildren = vec![VARIANT::default(); size];
            let _ = AccessibleChildren(self.as_iaccessible(), 0, &mut rgvarchildren, &mut 0);
            rgvarchildren
        }
    }
    /// ディセーブルかどうか
    fn is_disabled(&self, varchild: VARIANT) -> bool {
        match self.state(varchild) {
            Ok(s) => {
                let disabled = STATE_SYSTEM_UNAVAILABLE.0;
                (s & disabled) == disabled
            },
            // エラーはdisabledと見なす
            Err(_) => true
        }
    }
    fn user_draw_text(&self) -> Option<String> {
        None
    }
    fn select(&self, flags: u32, varchild: VARIANT) -> core::Result<()> {
        unsafe {
            self.as_iaccessible().accSelect(flags as i32, varchild)
        }
    }
}
impl IAccessibleExt for IAccessible {
    fn as_iaccessible(&self) -> &IAccessible {
        &self
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
    fn from_iaccessible(acc: IAccessible) -> core::Result<Self> {
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
            AccChild::new(acc?, varchild)
        }
    }


    fn find_nth<P>(self, nth: usize, predicate: P) -> Option<AccChild>
    where P: FnMut(&AccChild) -> bool
    {
        self.into_iter()
            .filter(predicate)
            .nth(nth)
    }
    fn find_nth_text(self, nth: usize, roles: &[u32], mouse: bool) -> Option<String> {
        let found = self.find_nth(nth, |child| {
            child.role_is_one_of(roles)
        })?;
        let text = found.value().ok()?;
        if mouse {
            if let Ok([x, y, w, h]) = found.location() {
                let x = x + w / 2;
                let y = y + h / 2;
                move_mouse_to(x, y);
            }
        }
        Some(text)
    }
}

impl IntoIterator for AccWindow {
    type Item = AccChild;

    type IntoIter = AccIter;

    fn into_iter(self) -> Self::IntoIter {
        AccIter::new(self.inner, None)
    }
}

impl<'a> From<&'a AccChild> for &'a IAccessible {
    fn from(child: &'a AccChild) -> Self {
        &child.inner
    }
}

impl TryFrom<IAccessible> for AccChild {
    type Error = core::Error;

    fn try_from(acc: IAccessible) -> Result<Self, Self::Error> {
        Self::new(acc, VARIANT::default()).ok_or(E_INVALIDARG.into())
    }
}

#[derive()]
pub struct AccChild {
    inner: IAccessible,
    role: u32,
    varchild: VARIANT,
}
impl std::fmt::Debug for AccChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccChild")
            .field("inner", &self.inner)
            .field("role", &self.role)
            .field("varchild", &self.varchild.vt())
            .finish()
    }
}
impl IntoIterator for AccChild {
    type Item = AccChild;

    type IntoIter = AccIter;

    fn into_iter(self) -> Self::IntoIter {
        AccIter::new(self.inner, None)
    }
}
impl AccChild {
    fn iaccessible(&self) -> &IAccessible {
        self.into()
    }
    fn varchild(&self) -> VARIANT {
        self.varchild.clone()
    }
    fn new(acc: IAccessible, varchild: VARIANT) -> Option<Self> {
        let role = acc.role(varchild.clone()).ok()?;
        let child = Self { inner: acc, role, varchild };
        Some(child)
    }
    fn iter(&self) -> AccIdIter {
        AccIdIter::new(self.iaccessible())
    }
    fn from_idispatch(disp: &IDispatch) -> Option<Self> {
        disp.cast::<IAccessible>().ok()
            .and_then(|acc| Self::try_from(acc).ok())
    }
    fn maybe_parent(&self) -> Option<&IAccessible> {
        let parent = self.iaccessible();
        (parent.child_count() > 0).then_some(parent)
    }
    fn role_is_one_of(&self, roles: &[u32]) -> bool {
        self.iaccessible().role_is_one_of(roles, self.varchild())
    }
    fn role_is(&self, other: u32) -> bool {
        self.iaccessible().role_is(other, self.varchild())
    }
    fn role(&self) -> core::Result<u32> {
        self.iaccessible().role(self.varchild())
    }
    fn role_text(&self) -> core::Result<String> {
        self.iaccessible().role_text(self.varchild())
    }
    pub fn hwnd(&self) -> core::Result<HWND> {
        self.iaccessible().hwnd()
    }
    pub fn location(&self) -> core::Result<[i32; 4]> {
        self.iaccessible().location(self.varchild())
    }
    fn client_location(&self, hwnd: HWND) -> core::Result<[i32; 4]> {
        self.iaccessible().client_location(hwnd, self.varchild())
    }
    fn name(&self) -> core::Result<String> {
        self.iaccessible().name(self.varchild())
    }
    fn state(&self) -> core::Result<u32> {
        self.iaccessible().state(self.varchild())
    }
    fn state_text(&self) -> core::Result<Vec<String>> {
        self.iaccessible().state_text(self.varchild())
    }
    fn is_disabled(&self) -> bool {
        self.iaccessible().is_disabled(self.varchild())
    }
    fn value(&self) -> core::Result<String> {
        self.iaccessible().value(self.varchild())
    }
    fn set_value(&self, value: &str) {
        let _ = self.iaccessible().set_value(self.varchild(), value);
    }
    fn description(&self) -> core::Result<String> {
        self.iaccessible().description(self.varchild())
    }
    fn user_draw_text(&self) -> Option<String> {
        self.iaccessible().user_draw_text()
    }
    // fn get_combo_list(&self) -> Option<AccChild> {
    //     self.role_is_one_of(&[ROLE_SYSTEM_COMBOBOX]).then_some(())?;
    //     self.iter()
    //         .filter_map(|id| id.as_child())
    //         .find_map(|child| {
    //             child.into_iter()
    //                 .find(|id| id.role_is_one_of(&[ROLE_SYSTEM_LIST]))
    //         })
    // }

    // fn get_clickable_names(self, clickable: &[u32], ignore_disabled: bool) -> Vec<String> {
    //     let iter = self.iter();
    //     iter.flat_map(|id| {
    //         if /* id.role_is_one_of(clickable) {
    //             if ignore_disabled && id.is_disabled() {
    //                 Vec::new()
    //             } else if let Ok(name) = id.name() {
    //                 println!("\u{001b}[33m[debug] name: {name:?}\u{001b}[0m");
    //                 vec![name]
    //             } else {
    //                 Vec::new()
    //             }
    //         } else if */ let Some(child) = id.as_child() {
    //             child.get_clickable_names(clickable, ignore_disabled)
    //         } else {
    //             Vec::new()
    //         }
    //     })
    //     .collect()
    // }
    // fn get_names_by_role(&self, ignore_disabled: bool) -> Vec<String> {
    //     if let Ok(role) = self.role() {
    //         match role {
    //             // ツールバーは配下のボタンを返す
    //             ROLE_SYSTEM_TOOLBAR => {
    //                 self.iter()
    //                     .filter(|id| {
    //                         // ディセーブルフラグ
    //                         !(ignore_disabled && id.is_disabled()) &&
    //                         // ボタンのみ
    //                         id.role_is_one_of(&[ROLE_SYSTEM_PUSHBUTTON])
    //                     })
    //                     // 有効な名前であれば返す
    //                     .filter_map(|id| id.valid_name())
    //                     .collect()
    //             },
    //             ROLE_SYSTEM_PAGETABLIST => {
    //                 self.iter()
    //                     .filter(|id| !(ignore_disabled && id.is_disabled()))
    //                     .filter_map(|id| id.valid_name())
    //                     .collect()
    //             },
    //             // 以下はクリックされる機能があるためそのまま名前を返す
    //             ROLE_SYSTEM_LINK |
    //             ROLE_SYSTEM_PUSHBUTTON => {
    //                 if ignore_disabled && self.is_disabled() {
    //                     // ディセーブルフラグ
    //                     Default::default()
    //                 } else {
    //                     self.name()
    //                         .into_iter()
    //                         .filter_map(|name| {
    //                             (!name.is_empty())
    //                                 .then_some(name)
    //                         })
    //                         .collect()
    //                 }
    //             },
    //             _ => Default::default()
    //         }
    //     } else {
    //         // ロールが得られなかったら何も返さない
    //         Default::default()
    //     }
    // }

    fn default_action(&self) -> bool {
        self.iaccessible().default_action(self.varchild()).is_ok()
    }
    /// 他の選択に加えて選択
    pub fn add_select(&self) -> bool {
        self.iaccessible().select(SELFLAG_ADDSELECTION, self.varchild()).is_ok()
    }
    /// 単独で選択
    pub fn select(&self) -> bool {
        self.iaccessible().select(SELFLAG_TAKEFOCUS|SELFLAG_TAKESELECTION, self.varchild()).is_ok()
    }
    fn is_checked(&self) -> bool {
        if let Ok(state) = self.iaccessible().state(self.varchild()) {
            state.includes(STATE_SYSTEM_CHECKED)
        } else {
            false
        }
    }
    fn name_includes(&self, other: &str) -> bool {
        self.name().is_ok_and(|name| name.partial_match(other))
    }
    pub fn click(&self, check: bool) -> bool {
        match self.role {
            ROLE_SYSTEM_LISTITEM => self.select(),
            ROLE_SYSTEM_CHECKBUTTON |
            ROLE_SYSTEM_MENUITEM => if check {
                // チェック状態にする
                if self.is_checked() {
                    // 既にチェック済み
                    true
                } else {
                    // チェックする
                    self.default_action()
                }
            } else {
                // 未チェック状態にする
                if self.is_checked() {
                    // チェックを外す
                    self.default_action()
                } else {
                    // 既に未チェック
                    true
                }
            },
            _ => if check {
                // クリック
                self.default_action()
            } else {
                // クリックはしない
                true
            },
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
    fn new(acc: IAccessible, items: Option<Vec<VARIANT>>) -> Self {
        let items = match items {
            Some(items) => items,
            None => acc.children(),
        };
        Self {
            acc,
            items,
            reverse: false,
            parent: None,
            _depth: 0,
            index: 0,
        }
    }
    fn reverse(&mut self) {
        self.reverse = true;
        self.items.reverse();
    }
    fn new_branch(&mut self, acc: IAccessible, mut items: Vec<VARIANT>) {
        let _depth = self._depth + 1;
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
                self.index += 1;
                match varchild.vt() {
                    VT_I4 => {
                        AccChild::new(self.acc.clone(), varchild.clone())
                    },
                    VT_DISPATCH => {
                        let child = varchild.Anonymous.Anonymous.Anonymous.pdispVal
                            .as_ref()
                            .and_then(AccChild::from_idispatch)?;
                        let items = child.iaccessible().children();
                        let is_parent = items.iter().all(|v|v.vt() == VT_I4);
                        self.new_branch(child.inner.clone(), items);
                        if is_parent {
                            // varchildがインデックスなら子の次の子要素を返す
                            self.next()
                        } else {
                            // オブジェクトならひとまず子自身を返す
                            Some(child)
                        }
                    },
                    _ => None,
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
// /// 逆順サーチ用ACCイテレータ
// struct ReverseAccIter {
//     object: IAccessible,
//     current: AccBranch
// }
// impl ReverseAccIter {
//     fn new(object: IAccessible, branch: AccBranch) -> Self {
//         let mut current = branch;
//         current.children.reverse();
//         Self { object, current }
//     }
// }
// impl Iterator for ReverseAccIter {
//     type Item = AccItem;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.current.next(&self.object) {
//             Some(child) => {
//                 self.current.new_reverse_branch_if_parent(&child);
//                 Some(child)
//             },
//             None => {
//                 self.current.restore_branch()?;
//                 self.next()
//             },
//         }
//     }
// }

// #[derive(Clone, Default)]
// struct AccBranch {
//     parent: Option<Box<Self>>,
//     children: Vec<VARIANT>,
//     index: usize,
//     _depth: u32,
//     reverse: bool,
// }
// impl AccBranch {
//     fn new(children: Vec<VARIANT>) -> Self {
//         Self {
//             children,
//             ..Default::default()
//         }
//     }
//     fn new_reverse(mut children: Vec<VARIANT>) -> Self {
//         children.reverse();
//         Self { children, reverse: true, ..Default::default()}
//     }
//     fn new_branch_if_parent(&mut self, child: &AccChild) {
//         if let Some(parent) = child.maybe_parent() {
//             let children = parent.children();
//             let mut branch = Self::new(children);
//             branch._depth = self._depth + 1;
//             branch.parent = Some(Box::new(self.to_owned()));
//             *self = branch;
//         }
//     }
//     fn new_reverse_branch_if_parent(&mut self, child: &AccChild) {
//         if let Some(parent) = child.maybe_parent() {
//             let children = parent.children();
//             dbg!(children.len());
//             let mut branch = Self::new_reverse(children);
//             branch._depth = self._depth + 1;
//             branch.parent = Some(Box::new(self.to_owned()));
//             *self = branch;
//         }
//     }
//     fn restore_branch(&mut self) -> Option<()> {
//         let parent = self.parent.to_owned()?;
//         *self = *parent;
//         Some(())
//     }
//     fn _next(&mut self, parent: &IAccessible) -> Option<AccChild> {
//         unsafe {
//             while let Some(varchild) = self.children.get(self.index) {
//                 self.index += 1;

//                 let v00 = &varchild.Anonymous.Anonymous;
//                 let vt = v00.vt;
//                 let child = match vt {
//                     VT_I4 => {
//                         None
//                     },
//                     VT_DISPATCH => {
//                         v00.Anonymous.pdispVal
//                             .as_ref()
//                             .and_then(|disp| AccChild::from_idispatch(disp))
//                     },
//                     _ => None
//                 };
//                 if child.is_some() {
//                     return child.inspect(|c| {
//                         println!("\u{001b}[36m[debug] child: {c:?}\u{001b}[0m");
//                         println!("\u{001b}[33m[debug] depth: {}\u{001b}[0m", self._depth);
//                         println!(
//                             "\u{001b}[90m{:?} {:?} {:?}\u{001b}[0m",
//                             c.name(),
//                             c.role_text(),
//                             c.state_text(),
//                         );
//     ;                });
//                 }
//             }
//             None
//         }
//     }
// }


// impl<'a> IntoIterator for &'a AccChild {
//     type Item = AccIdChild<'a>;

//     type IntoIter = AccIdIter<'a>;

//     fn into_iter(self) -> Self::IntoIter {
//         unsafe {
//             let object = self.iaccessible();
//             AccIdIter::new(object)
//         }
//     }
// }

/// IAccessible + varchild (VT_I4) を探索するイテレータ
struct AccIdIter<'a> {
    object: &'a IAccessible,
    children: Vec<VARIANT>,
    index: usize,
}
impl<'a> AccIdIter<'a> {
    fn new(object: &'a IAccessible) -> Self {
        let children = object.children();
        Self {
            object,
            children,
            index: 0,
        }
    }
    fn reverse(self) -> ReverseAccIdIter<'a> {
        ReverseAccIdIter::new(self.object, self.children)
    }
}
impl<'a> Iterator for AccIdIter<'a> {
    type Item = AccIdChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.children.get(self.index)?;
        self.index += 1;
        Some(AccIdChild(self.object, id.clone()))
    }
}
struct ReverseAccIdIter<'a> {
    object: &'a IAccessible,
    children: Vec<VARIANT>,
    index: usize,
}
impl<'a> ReverseAccIdIter<'a> {
    fn new(object: &'a IAccessible, mut children: Vec<VARIANT>) -> Self {
        children.reverse();
        Self { object, children, index: 0 }
    }
}
impl<'a> Iterator for ReverseAccIdIter<'a> {
    type Item = AccIdChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.children.get(self.index)?;
        self.index += 1;
        Some(AccIdChild(self.object, id.clone()))
    }
}

impl From<AccIdChild<'_>> for AccChild {
    fn from(id: AccIdChild) -> Self {
        Self {
            inner: id.iaccessible().clone(),
            role: id.iaccessible().role(id.varchild()).unwrap_or_default(),
            varchild: id.varchild(),
        }
    }
}

struct AccIdChild<'a>(&'a IAccessible, VARIANT);
impl std::fmt::Debug for AccIdChild<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AccIdChild")
            .field(&self.0)
            .field(unsafe{&self.1.Anonymous.Anonymous.Anonymous.lVal})
            .finish()
    }
}
impl AccIdChild<'_> {
    fn iaccessible(&self) -> &IAccessible {
        self.0
    }
    fn varchild(&self) -> VARIANT {
        self.1.clone()
    }
    // fn as_child(&self) -> Option<AccChild> {
    //     unsafe {
    //         let v00 = &self.1.Anonymous.Anonymous;
    //         let pdispval = (v00.vt == VT_DISPATCH).then_some(&v00.Anonymous.pdispVal)?;
    //         let child = pdispval.as_ref().and_then(|disp| {
    //             AccChild::from_idispatch(disp)
    //         });
    //         match child {
    //             Some(child) => Some(child),
    //             None => self.0.get_accChild(self.1.clone()).ok()
    //                 .and_then(|disp| AccChild::from_idispatch(&disp))
    //         }
    //     }
    // }
    fn name(&self) -> core::Result<String> {
        self.iaccessible().name(self.varchild())
    }
    fn valid_name(&self) -> Option<String> {
        self.name().ok()
            .and_then(|name| {
                (!name.is_empty())
                    .then_some(name)
            })
    }
    fn client_location(&self, hwnd: HWND) -> core::Result<[i32; 4]> {
        self.iaccessible().client_location(hwnd, self.varchild())
    }
    fn role(&self) -> core::Result<u32> {
        self.iaccessible().role(self.varchild())
    }
    fn role_text(&self) -> core::Result<String> {
        self.iaccessible().role_text(self.varchild())
    }
    fn state_text(&self) -> core::Result<Vec<String>> {
        self.iaccessible().state_text(self.varchild())
    }
    fn is_disabled(&self) -> bool {
        self.iaccessible().is_disabled(self.varchild())
    }
    fn role_is_one_of(&self, roles: &[u32]) -> bool {
        self.iaccessible().role_is_one_of(roles, self.varchild())
    }
    fn role_is(&self, other: u32) -> bool {
        self.iaccessible().role_is(other, self.varchild())
    }
}

struct ScreenPoint(pub POINT);
impl From<POINT> for ScreenPoint {
    fn from(point: POINT) -> Self {
        Self(point)
    }
}
impl From<(i32, i32)> for ScreenPoint {
    fn from((x, y): (i32, i32)) -> Self {
        let point = POINT { x, y };
        point.into()
    }
}

impl From<&ClkTarget> for u32 {
    fn from(target: &ClkTarget) -> Self {
        let mut role = 0;
        if target.button {
            role |= ROLE_SYSTEM_PUSHBUTTON|ROLE_SYSTEM_CHECKBUTTON|ROLE_SYSTEM_RADIOBUTTON;
        }
        if target.link {
            role |= ROLE_SYSTEM_LINK;
        }
        if target.list {
            role |= ROLE_SYSTEM_LIST;
        }
        if target.listview {
            role |= ROLE_SYSTEM_LIST;
        }
        if target.menu {
            role |= ROLE_SYSTEM_MENUBAR;
        }
        if target.tab {
            role |= ROLE_SYSTEM_PAGETABLIST;
        }
        if target.toolbar {
            role |= ROLE_SYSTEM_TOOLBAR;
        }
        if target.treeview {
            role |= ROLE_SYSTEM_OUTLINE;
        }
        role
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
        if let Some(a) = self.find("(&") {
            if let Some(b) = self.find(")") {
                if b == a + 3 {
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
    }
    fn find_ignore_ascii_case(&self, other: &str) -> Option<usize> {
        let pat_bytes = other.as_bytes();
        self.as_bytes().windows(other.len()).enumerate()
            .find_map(|(i, w)| w.eq_ignore_ascii_case(pat_bytes).then_some(i) )
    }
}