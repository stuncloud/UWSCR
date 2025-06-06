
use crate::{
        Evaluator,
        builtins::*,
        object::{
            browser::{BrowserBuilder, BrowserType},
            WebRequest,
            HtmlNode,
        },
    };

use strum_macros::{EnumString, VariantNames};
use num_derive::{ToPrimitive, FromPrimitive};
use num_traits::FromPrimitive;

use std::sync::{Arc, Mutex};

pub fn builtin_func_sets() -> BuiltinFunctionSets {
    let mut sets = BuiltinFunctionSets::new();
    sets.add("browsercontrol", browser_control, get_desc!(browser_control));
    sets.add("browserbuilder", browser_builder, get_desc!(browser_builder));
    sets.add("remoteobjecttype", remote_object_type, get_desc!(remote_object_type));
    sets.add("webrequest", webrequest, get_desc!(webrequest));
    sets.add("webrequestbuilder", webrequest_builder, get_desc!(webrequest_builder));
    sets.add("parsehtml", parse_html, get_desc!(parse_html));
    sets.add("brgetdata", browser_getdata, get_desc!(browser_getdata));
    sets.add("brsetdata", browser_setdata, get_desc!(browser_setdata));
    sets.add("brgetsrc", browser_getsource, get_desc!(browser_getsource));
    sets.add("brlink", browser_link, get_desc!(browser_link));
    sets
}

const DEFAULT_PORT: u16 = 9222;

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, VariantNames, ToPrimitive, FromPrimitive)]
pub enum BcEnum {
    #[strum[props(desc="Google Chrome")]]
    BC_CHROME  = 1,
    #[strum[props(desc="Microsoft Edge")]]
    BC_MSEDGE  = 2,
    #[strum[props(desc="Vivaldi", hidden="true")]]
    BC_VIVALDI = 11,
}

#[builtin_func_desc(
    desc="ブラウザを起動しBrowserオブジェクトを返す",
    args=[
        {n="対象ブラウザ",t="定数",d=r#"以下のいずれかを指定
- BC_CHROME: Google Chrome
- BC_MSEDGE: Microsoft Edge
"#},
        {n="ポート",t="数値",d="ブラウザのデバッグポート番号を指定、省略時は9222",o}
    ],
    rtype={desc="対象ブラウザのオブジェクト",types="Browser"}
)]
pub fn browser_control(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = args.get_as_int(0, None)?;
    let Some(browser_type) = FromPrimitive::from_i32(t) else {
        return Err(builtin_func_error(UErrorMessage::InvalidBrowserType(t)));
    };
    let r#type = match browser_type {
        BcEnum::BC_CHROME => BrowserType::Chrome,
        BcEnum::BC_MSEDGE => BrowserType::MSEdge,
        BcEnum::BC_VIVALDI => BrowserType::Vivaldi,
    };
    let port = args.get_as_int(1, Some(DEFAULT_PORT))?;
    let mut builder = BrowserBuilder::new(r#type, DEFAULT_PORT);
    builder.port(port);
    let browser = builder.start()?;
    Ok(Object::Browser(browser))
}

#[builtin_func_desc(
    desc="BrowserBuilderオブジェクトを返す",
    args=[
        {n="対象ブラウザ",t="定数",d=r#"以下のいずれかを指定
- BC_CHROME: Google Chrome
- BC_MSEDGE: Microsoft Edge
"#},
    ],
    rtype={desc="BrowserBuilderオブジェクト",types="BrowserBuilder"}
)]
pub fn browser_builder(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let t = args.get_as_int(0, None)?;
    let Some(browser_type) = FromPrimitive::from_i32(t) else {
        return Err(builtin_func_error(UErrorMessage::InvalidBrowserType(t)));
    };
    let r#type = match browser_type {
        BcEnum::BC_CHROME => BrowserType::Chrome,
        BcEnum::BC_MSEDGE => BrowserType::MSEdge,
        BcEnum::BC_VIVALDI => BrowserType::Vivaldi,
    };
    let builder = BrowserBuilder::new(r#type, DEFAULT_PORT);
    Ok(Object::BrowserBuilder(Arc::new(Mutex::new(builder))))
}

#[builtin_func_desc(
    desc="RemoteObjectの型を返す",
    args=[
        {n="remote",t="RemoteObject",d="型を調べたいRemoteObject"},
    ],
    rtype={desc="型情報",types="文字列"}
)]
pub fn remote_object_type(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let remote = args.get_as_remoteobject(0)?;
    #[cfg(debug_assertions)]
    println!("\u{001b}[90m{:?}\u{001b}[0m", remote);
    let t = remote.get_type();
    Ok(t.into())
}

#[builtin_func_desc(
    desc="指定URLにGETリクエストを送る",
    args=[
        {n="url",t="文字列",d="リクエストを送るURL"},
    ],
    rtype={desc="レスポンスオブジェクト",types="WebResponse"}
)]
pub fn webrequest(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let uri = args.get_as_string(0, None)?;
    let req = WebRequest::new();
    let res = req.get(&uri)?;
    Ok(Object::WebResponse(res))
}

#[builtin_func_desc(
    desc="WebRequestオブジェクトを返す",
    args=[],
    rtype={desc="リクエストオブジェクト",types="WebRequest"}
)]
pub fn webrequest_builder(_: &mut Evaluator, _: BuiltinFuncArgs) -> BuiltinFuncResult {
    let req = WebRequest::new();
    Ok(Object::WebRequest(Arc::new(Mutex::new(req))))
}

#[builtin_func_desc(
    desc="HTMLをパースします"
    args=[
        {n="html",t="文字列",d="パースするHTML文字列"},
    ],
    rtype={desc="HtmlNodeオブジェクト",types="HtmlNode"}
)]
pub fn parse_html(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let html = args.get_as_string(0, None)?;
    let node = HtmlNode::new(&html);
    Ok(Object::HtmlNode(node))
}

#[builtin_func_desc(
    desc="タブ上のエレメントから値を得る"
    sets=[
        "name-value",
        [
            {n="タブ",t="TabWindow",d="取得したい値のあるタブ"},
            {n="name",t="文字列",d="対象エレメントのname値"},
            {n="value",t="文字列",d="同一nameがある場合に指定するvalue値",o},
            {n="n番目",t="数値",d="該当エレメントが複数ある場合に順番を指定",o},
        ],
        "タグ+プロパティ",
        [
            {n="タブ",t="TabWindow",d="取得したい値のあるタブ"},
            {n="'TAG=name'",t="文字列",d="対象のタグ名を'TAG=タグ名'という書式で指定"},
            {n="'prop=val'",t="文字列",d="'プロパティ=値'という書式で任意のプロパティとその値を持つタグを探す",o},
            {n="n番目",t="数値",d="該当タグが複数ある場合に順番を指定",o},
        ],
        "タグ指定",
        [
            {n="タブ",t="TabWindow",d="取得したい値のあるタブ"},
            {n="'TAG=name'",t="文字列",d="対象のタグ名を'TAG=タグ名'という書式で指定"},
            {n="n番目",t="数値",d="該当タグが複数ある場合に順番を指定",o},
        ],
        "テーブル",
        [
            {n="タブ",t="TabWindow",d="取得したい値のあるタブ"},
            {n="'TAG=TABLE'",t="文字列",d="TABLEタグを対象にする"},
            {n="n番目",t="数値",d="TABLEが複数ある場合に順番を指定",o},
            {n="行",t="数値",d="TABLEの行番号",o},
            {n="列",t="数値",d="TABLEの列番号",o},
        ],
    ]
    rtype={desc="該当エレメントの値",types=""}
)]
pub fn browser_getdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let name = args.get_as_string(1, None)?;
    let obj = if let Some((left, right)) = name.split_once('=') {
        if left.eq_ignore_ascii_case("tag") {
            // タグ指定
            if right.eq_ignore_ascii_case("table") {
                // テーブル
                let nth = args.get_as_nth(2)? as usize;
                let row = args.get_as_nth(3)? as usize;
                let col = args.get_as_nth(4)? as usize;
                tab.get_data_by_table_point(nth, row, col)?
            } else {
                // テーブル以外のタグ
                match args.get_as_num_or_string(2).unwrap_or(TwoTypeArg::U(1_usize)) {
                    TwoTypeArg::T(prop) => {
                        // プロパティ指定
                        if let Some((prop_name, prop_value)) = prop.split_once('=') {
                            let nth = args.get_as_nth(3)?;
                            tab.get_data_by_tagname_and_property(right.into(), prop_name, prop_value, nth as usize)?
                        } else {
                            // プロパティ指定がない
                            Object::Empty
                        }
                    },
                    TwoTypeArg::U(nth) => {
                        // 順番指定
                        tab.get_data_by_tagname(right.into(), nth)?
                    },
                }
            }
        } else {
            // タグ指定じゃない場合はnameとみなす
            let value = args.get_as_string_or_empty(2)?;
            let nth = args.get_as_nth(3)?;
            tab.get_data_by_name_value(name, value, nth as usize)?
        }
    } else {
        // name-value指定
        let value = args.get_as_string_or_empty(2)?;
        let nth = args.get_as_nth(3)?;
        tab.get_data_by_name_value(name, value, nth as usize)?
    };
    Ok(obj)
}

#[builtin_func_desc(
    desc="文字入力やクリックを行う"
    sets=[
        "name-value",
        [
            {n="タブ",t="TabWindow",d="入力を対象のあるタブ"},
            {n="値",t="文字列または配列",d="入力する値、対象がinput[type=file]なら配列で複数指定可"},
            {n="name",t="文字列",d="入力対象エレメントのname値"},
            {n="value",t="文字列",d="同一nameがある場合に対象エレメントのvalue値を指定",o},
            {n="n番目",t="数値",d="同一name/valueがある場合に順番を指定",o},
            {n="直接入力",t="真偽値",d="TRUEならvalue値を直接変更、FALSE(デフォルト)ならキー入力をエミュレート",o},
        ],
        "RemoteObject",
        [
            {n="エレメント",t="RemoteObject",d="対象エレメントのオブジェクト"},
            {n="値",t="文字列または配列",d="入力する値、対象がinput[type=file]なら配列で複数指定可"},
        ],
        "クリック name-value"
        [
            {n="タブ",t="TabWindow",d="クリック対象のあるタブ"},
            {n="TRUE",t="真偽値",d="クリックする場合TRUEを指定"},
            {n="name",t="文字列",d="入力対象エレメントのname値"},
            {n="value",t="文字列",d="同一nameがある場合に対象エレメントのvalue値を指定",o},
            {n="n番目",t="数値",d="同一タグがある場合に順番を指定",o},
        ],
        "クリック タグ+プロパティ",
        [
            {n="タブ",t="TabWindow",d="クリック対象のあるタブ"},
            {n="TRUE",t="真偽値",d="クリックする場合TRUEを指定"},
            {n="'TAG=name'",t="文字列",d="'TAG=タグ名'の書式で指定タグを探す"},
            {n="'prop=val'",t="文字列",d="'プロパティ=値'という書式で任意のプロパティとその値を持つタグを探す",o},
            {n="n番目",t="数値",d="同一タグがある場合に順番を指定",o},
        ],
        "クリック タグ指定",
        [
            {n="タブ",t="TabWindow",d="クリック対象のあるタブ"},
            {n="TRUE",t="真偽値",d="クリックする場合TRUEを指定"},
            {n="'TAG=name'",t="文字列",d="'TAG=タグ名'の書式で指定タグを探す"},
            {n="n番目",t="数値",d="同一タグがある場合に順番を指定",o},
        ],
        "クリック IMG",
        [
            {n="タブ",t="TabWindow",d="クリック対象のあるタブ"},
            {n="TRUE",t="真偽値",d="クリックする場合TRUEを指定"},
            {n="'TAG=IMG'",t="文字列",d="IMGタグをクリックする場合に指定"},
            {n="src",t="文字列",d="IMGタグのsrc値を指定",o},
            {n="n番目",t="数値",d="同一srcがある場合に順番を指定",o},
        ],
    ],
    rtype={desc="成功時TRUE",types="真偽値"}
)]
pub fn browser_setdata(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    if let Ok(tab) = args.get_as_tabwindow(0) {
        match args.get_as_string_array_or_bool(1, None)? {
            // value書き換え
            TwoTypeArg::T(new_value) => {
                let name = args.get_as_string(2, None)?;
                let value = args.get_as_string_or_empty(3)?;
                let nth = args.get_as_nth(4)? as usize;
                let direct = args.get_as_bool(5, Some(false))?;
                let obj = tab.set_data_by_name_value(new_value, name, value, nth, direct)?;
                Ok(obj)
            },
            // クリック
            TwoTypeArg::U(click) => {
                let obj = if click {
                    let name = args.get_as_string(2, None)?;
                    if let Some((left, tag)) = name.split_once('=') {
                        if left.eq_ignore_ascii_case("tag") {
                            if tag.eq_ignore_ascii_case("img") {
                                // imgタグ
                                let src = args.get_as_string_or_empty(3)?;
                                let nth = args.get_as_nth(4)? as usize;
                                tab.click_img(src, nth)?
                            } else {
                                // img以外
                                match args.get_as_num_or_string(3).unwrap_or(TwoTypeArg::U(1_usize)) {
                                    TwoTypeArg::T(prop) => {
                                        if let Some((prop_name, prop_value)) = prop.split_once('=') {
                                            let nth = args.get_as_nth(4)? as usize;
                                            tab.click_by_tag_and_property(tag.into(), prop_name, prop_value, nth)?
                                        } else {
                                            Object::Bool(false)
                                        }
                                    },
                                    TwoTypeArg::U(nth) => {
                                        tab.click_by_nth_tag(tag.into(), nth)?
                                    },
                                }
                            }
                        } else {
                            // タグ指定じゃないのでname-valueとしてクリック
                            let value = args.get_as_string_or_empty(3)?;
                            let nth = args.get_as_nth(4)? as usize;
                            tab.click_by_name_value(name, value, nth)?
                        }
                    } else {
                        // name-valueでクリック
                        let value = args.get_as_string_or_empty(3)?;
                        let nth = args.get_as_nth(4)? as usize;
                        tab.click_by_name_value(name, value, nth)?
                    }
                } else {
                    Object::Bool(false)
                };
                Ok(obj)
            },
        }
    } else {
        let remote = args.get_as_remoteobject(0)?;
        let new_value = args.get_as_string_array(1)?;
        let result = remote.emulate_key_input(new_value)?;
        Ok(result.into())
    }
}

#[builtin_func_desc(
    desc="指定タグのHTMLソースを返す"
    args=[
        {n="タブ",t="TabWindow",d="取得対象タグのあるタブ"},
        {n="タグ名",t="文字列",d="取得対象タグ名"},
        {n="n番目",t="数値",d="該当タグが複数ある場合に順番を指定",o},
    ],
    rtype={desc="該当タグがあればそのHTML",types="文字列"}
)]
pub fn browser_getsource(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let tag = args.get_as_string(1, None)?;
    let nth = args.get_as_nth(2)? as usize;
    let obj = tab.get_source(tag, nth)?;
    Ok(obj)
}

#[builtin_func_desc(
    desc="リンクをクリック"
    args=[
        {n="タブ",t="TabWindow",d="クリック対象リンクのあるタブ"},
        {n="リンク文字",t="文字列",d="リンクに表示されている文字列"},
        {n="n番目",t="数値",d="該当リンクが複数ある場合に順番を指定",o},
        {n="完全一致",t="真偽値",d="TRUEの場合リンク文字が完全一致するリンクを探す",o},
    ],
    rtype={desc="該当リンククリックに成功した場合TRUE",types="真偽値"}
)]
pub fn browser_link(_: &mut Evaluator, args: BuiltinFuncArgs) -> BuiltinFuncResult {
    let tab = args.get_as_tabwindow(0)?;
    let text = args.get_as_string(1, None)?;
    let nth = args.get_as_nth(2)? as usize;
    let exact_match = args.get_as_bool(3, Some(false))?;
    let obj = tab.click_link(text, nth, exact_match)?;
    Ok(obj)
}