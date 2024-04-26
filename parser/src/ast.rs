use std::fmt;
use std::str::FromStr;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Serialize, Deserialize};

use crate::lexer::Position;

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier(pub String);

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Identifier(name) = self;
        write!(f, "{}", name)
    }
}
impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Prefix {
    Plus,
    Minus,
    Not
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Prefix::Plus => write!(f, "+"),
            Prefix::Minus => write!(f, "-"),
            Prefix::Not => write!(f, "!"),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Infix {
    /// +
    Plus,
    /// -
    Minus,
    /// *
    Multiply,
    /// /
    Divide,
    /// =, ==
    Equal,
    /// <>, !=
    NotEqual,
    /// >=
    GreaterThanEqual,
    /// >
    GreaterThan,
    /// <=
    LessThanEqual,
    /// <
    LessThan,
    /// and
    And,
    /// or
    Or,
    /// xor
    Xor,
    /// andl
    AndL, // logical and
    /// orl
    OrL, // logical or
    /// xorl
    XorL, // logical xor
    /// andb
    AndB, // bit and
    /// orb
    OrB, // bit or
    /// xorb
    XorB, // bit xor
    /// mod
    Mod,
    /// =, :=
    Assign,
}

impl fmt::Display for Infix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Infix::Plus => write!(f, "+"),
            Infix::Minus => write!(f, "-"),
            Infix::Multiply => write!(f, "*"),
            Infix::Divide => write!(f, "/"),
            Infix::Equal => write!(f, "=="),
            Infix::NotEqual => write!(f, "<>"),
            Infix::GreaterThan => write!(f, ">"),
            Infix::GreaterThanEqual => write!(f, ">="),
            Infix::LessThan => write!(f, "<"),
            Infix::LessThanEqual => write!(f, "<="),
            Infix::And => write!(f, "and"),
            Infix::Or => write!(f, "or"),
            Infix::Xor => write!(f, "xor"),
            Infix::AndL => write!(f, "andL"),
            Infix::OrL => write!(f, "orL"),
            Infix::XorL => write!(f, "xorL"),
            Infix::AndB => write!(f, "andB"),
            Infix::OrB => write!(f, "orB"),
            Infix::XorB => write!(f, "xorB"),
            Infix::Mod => write!(f, "mod"),
            Infix::Assign => write!(f, ":="),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
    /// 識別子
    Identifier(Identifier),
    /// 配列宣言
    ///
    /// 0. 配列
    /// 1. 次元毎の添字
    Array(Vec<Expression>, Vec<Expression>),
    /// リテラル
    Literal(Literal),
    /// プリフィクス
    ///
    /// 0. + か -
    /// 1. 式
    Prefix(Prefix, Box<Expression>),
    /// 演算
    ///
    /// 1. 演算子
    /// 2. 左辺
    /// 3. 右辺
    Infix(Infix, Box<Expression>, Box<Expression>),
    /// 変数\[i] または 変数\[i, n] 表記
    ///
    /// 0. 変数を示す式
    /// 1. 添字を示す式
    /// 2. hashtblの2つ目の添字
    Index(Box<Expression>, Box<Expression>, Box<Option<Expression>>),
    /// 無名関数定義
    ///
    /// - params: 引数
    /// - body: 処理
    /// - is_proc: プロシージャかどうか
    AnonymusFunction {
        params: Vec<FuncParam>,
        body: BlockStatement,
        is_proc: bool,
    },
    /// 関数定義
    ///
    /// - params: 引数
    /// - body: 処理
    /// - is_proc: プロシージャかどうか
    FuncCall {
        func: Box<Expression>,
        args: Vec<Expression>,
        is_await: bool,
    },
    /// 代入式
    ///
    /// 1. 左辺
    /// 2. 右辺
    Assign(Box<Expression>, Box<Expression>),
    /// 複合代入
    ///
    /// 1. 左辺
    /// 2. 右辺
    /// 3. 演算子
    CompoundAssign(Box<Expression>, Box<Expression>, Infix),
    /// 三項演算子
    /// condition ? consequence : alternative
    Ternary {
        condition: Box<Expression>,
        consequence: Box<Expression>,
        alternative: Box<Expression>,
    },
    /// .呼び出し、左辺.右辺
    ///
    /// 1. 左辺
    /// 2. 右辺
    DotCall(Box<Expression>, Box<Expression>),
    /// UObject宣言
    UObject(String),
    /// COM_ERR_FLG
    ComErrFlg,
    /// COMメソッドの参照渡し
    /// 1. var 式 または ref 式
    RefArg(Box<Expression>),
    /// 省略された引数
    /// func( , )
    EmptyArgument,
    /// コールバック関数の引数であることを示す
    Callback,
}

impl Expression {
    pub fn get_identifier(&self) -> Option<String> {
        match self {
            Expression::Identifier(Identifier(ident)) => Some(ident.clone()),
            Expression::Index(e, _, _) => e.get_identifier(),
            _ => None,
        }
    }
    pub fn is_not_assignable(&self) -> bool {
        match self {
            Self::Identifier(_) |
            Self::Index(_, _, _) |
            Self::DotCall(_, _) => false,
            // COMのパラメータ付きプロパティかもしれない場合
            Self::FuncCall { func, args:_, is_await: false } => {
                if let Self::DotCall(_, _) = func.as_ref() {
                    false
                } else {
                    true
                }
            },
            _ => true
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Identifier(i) => write!(f, "{}", i),
            Expression::Array(arr, _) => {
                let list = arr.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "[{}]", list)
            },
            Expression::Literal(l) => write!(f, "{}", l),
            Expression::Prefix(p, e) => write!(f, "{}{}", p, e),
            Expression::Infix(i, l, r) => write!(f, "{} {} {}", l, i, r),
            Expression::Index(l, i, h) => {
                match &**h {
                    Some(e) => write!(f, "{}[{}, {}]", l, i, e),
                    None => write!(f, "{}[{}]", l, i)
                }
            },
            Expression::AnonymusFunction { params, body: _, is_proc } => {
                let t = if *is_proc {"procedure"} else {"function"};
                let p = params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}({})", t, p)
            },
            Expression::FuncCall { func, args, is_await } => {
                let w = if *is_await {"await "} else {""};
                let a = args.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}{}({})", w, func, a)
            },
            Expression::Assign(l, r) => write!(f, "{} := {}", l, r),
            Expression::CompoundAssign(l, r, i) => write!(f, "{} {}= {}", l, i , r),
            Expression::Ternary { condition, consequence, alternative } => {
                write!(f, "{} ? {} : {}", condition, consequence, alternative)
            },
            Expression::DotCall(l, r) => write!(f, "{}.{}", l, r),
            Expression::UObject(o) => write!(f, "{}", o),
            Expression::ComErrFlg => write!(f, ""),
            Expression::RefArg(a) => write!(f, "var {}", a),
            Expression::EmptyArgument => write!(f, ""),
            Expression::Callback => write!(f, "callback"),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Literal {
    // Int(i64),
    Num(f64),
    String(String),
    ExpandableString(String),
    TextBlock(String, bool),
    Bool(bool),
    Array(Vec<Expression>),
    // Path(String),
    Empty,
    Null,
    Nothing,
    NaN,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Num(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::ExpandableString(s) => write!(f, "\"{}\"", s),
            Literal::TextBlock(_, _) => write!(f, "textblock"),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Array(arr) => {
                let list = arr.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{}", list)
            },
            Literal::Empty => write!(f, "EMPTY"),
            Literal::Null => write!(f, "NULL"),
            Literal::Nothing => write!(f, "NOTHING"),
            Literal::NaN => write!(f, "NaN"),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
    /// dim宣言, boolはループ内かどうか
    Dim(Vec<(Identifier, Expression)>, bool),
    Public(Vec<(Identifier, Expression)>),
    Const(Vec<(Identifier, Expression)>),
    /// hashtbl定義: [(名前, オプション)], publicかどうか
    HashTbl(Vec<(Identifier, Option<Expression>)>, bool),
    Hash(HashSugar),
    Print(Expression),
    /// callで実行されるスクリプト, 引数(param_str)
    Call(Program, Vec<Expression>),
    DefDll {
        name: String,
        alias: Option<String>,
        params: Vec<DefDllParam>,
        ret_type: DllType,
        path: String,
    },
    Expression(Expression),
    For {
        loopvar: Identifier,
        from: Expression,
        to: Expression,
        step: Option<Expression>,
        block: BlockStatement,
        alt: Option<BlockStatement>, // else区
    },
    ForIn {
        loopvar: Identifier,
        index_var: Option<Identifier>,
        islast_var: Option<Identifier>,
        collection: Expression,
        block: BlockStatement,
        alt: Option<BlockStatement>, // else区
    },
    While(Expression, BlockStatement),
    Repeat(Box<StatementWithRow>, BlockStatement),
    Continue(u32),
    Break(u32),
    IfSingleLine {
        condition: Expression,
        consequence: Box<StatementWithRow>,
        alternative: Box<Option<StatementWithRow>>
    },
    If {
        condition: Expression,
        consequence: BlockStatement,
        alternative: Option<BlockStatement>
    },
    ElseIf {
        condition: Expression,
        consequence: BlockStatement,
        alternatives: Vec<(Option<StatementWithRow>, BlockStatement)>
    },
    Select {
        expression: Expression,
        cases: Vec<(Vec<Expression>, BlockStatement)>,
        default: Option<BlockStatement>
    },
    Function {
        name: Identifier,
        params: Vec<FuncParam>,
        body: BlockStatement,
        is_proc: bool,
        is_async: bool,
    },
    Exit,
    ExitExit(i32),
    Module(Identifier, BlockStatement),
    Class(Identifier, BlockStatement),
    /// (名前, 型, 配列サイズ, var/ref)
    Struct(Identifier, Vec<(String, String, DefDllParamSize, bool)>),
    TextBlock(Identifier, Literal),
    With(Option<Expression>, BlockStatement),
    Try {
        trys: BlockStatement,
        except: Option<BlockStatement>,
        finally: Option<BlockStatement>,
    },
    Option(OptionSetting),
    Enum(String, UEnum),
    Thread(Expression),
    ComErrIgn,
    ComErrRet,
}

impl std::fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let disp = match self {
            Statement::Dim(_, _) => "Dim",
            Statement::Public(_) => "Public",
            Statement::Const(_) => "Const",
            Statement::HashTbl(_,_) => "HashTbl",
            Statement::Hash(_) => "Hash",
            Statement::Print(_) => "Print",
            Statement::Call(_, _) => "Call",
            Statement::DefDll { name:_, alias:_, params:_, ret_type:_, path:_ } => "DefDll",
            Statement::Expression(_) => "Expression",
            Statement::For { loopvar:_, from:_, to:_, step:_, block:_, alt:_ } => "For",
            Statement::ForIn { loopvar:_, index_var:_, islast_var:_, collection:_, block:_, alt:_ } => "ForIn",
            Statement::While(_, _) => "While",
            Statement::Repeat(_, _) => "Repeat",
            Statement::Continue(_) => "Continue",
            Statement::Break(_) => "Break",
            Statement::IfSingleLine { condition:_, consequence:_, alternative:_ } => "IfSingleLine",
            Statement::If { condition:_, consequence:_, alternative:_ } => "If",
            Statement::ElseIf { condition:_, consequence:_, alternatives:_ } => "ElseIf",
            Statement::Select { expression:_, cases:_, default:_ } => "Select",
            Statement::Function { name:_, params:_, body:_, is_proc:_, is_async:_ } => "Function",
            Statement::Exit => "Exit",
            Statement::ExitExit(_) => "ExitExit",
            Statement::Module(_, _) => "Module",
            Statement::Class(_, _) => "Class",
            Statement::Struct(_, _) => "Struct",
            Statement::TextBlock(_, _) => "TextBlock",
            Statement::With(_, _) => "With",
            Statement::Try { trys:_, except:_, finally:_ } => "Try",
            Statement::Option(_) => "Option",
            Statement::Enum(_, _) => "Enum",
            Statement::Thread(_) => "Thread",
            Statement::ComErrIgn => "ComErrIgn",
            Statement::ComErrRet => "ComErrRet",
        };
        write!(f, "{disp}")
    }
}


#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct UEnum {
    pub name: String,
    members: Vec<UEnumMember>
}
pub type UEnumMember = (String, f64);
impl UEnum {
    pub fn new(name: &String) -> Self {
        UEnum {
            name: name.to_string(),
            members: Vec::new()
        }
    }
    pub fn get(&self, id: &String) -> Option<f64> {
        let value = self.members.iter().find(
            |m| &m.0 == id
        ).map(
            |m| m.1
        );
        value
    }
    pub fn add(&mut self, id: &String, value: f64) -> Result<(), ()> {
        if self.members.iter().find(|(m, n)| m == id || n == &value).is_some() {
            Err(())
        } else {
            self.members.push((id.to_string(), value));
            Ok(())
        }
    }
    pub fn include(&self, value: f64) -> bool {
        self.members
            .iter()
            .find(|(_, n)| *n == value)
            .is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementWithRow {
    pub statement: Statement,
    pub row: usize,
    pub line: String,
    pub script_name: Option<String>,
}

impl StatementWithRow {
    pub fn new(statement: Statement, row: usize, line: String, script_name: Option<String>) -> Self {
        Self {statement, row, line, script_name }
    }
    // 存在しない行
    pub fn new_non_existent_line(statement: Statement) -> Self {
        Self {
            statement,
            row: 0,
            line: "dummy".into(),
            script_name: None,
        }
    }

    pub fn get_identifier_names(&self) -> Option<HashMap<String, bool>> {
        let mut map = HashMap::new();
        let names = match &self.statement {
            Statement::Public(v) |
            Statement::Const(v) |
            Statement::Dim(v, _) => {
                Some(v.iter().map(|(Identifier(ident), _)| ident.to_ascii_uppercase()).collect())
            },
            Statement::HashTbl(v, _) => {
                Some(v.iter().map(|(Identifier(ident), _)| ident.to_ascii_uppercase()).collect())
            },
            Statement::Hash(hash) => {
                Some(vec![hash.name.0.to_ascii_uppercase()])
            },
            // 定数扱い
            Statement::Enum(ident, _) |
            Statement::TextBlock(Identifier(ident), _) => {
                Some(vec![ident.to_ascii_uppercase()])
            },
            _ => None,
        }?;
        for name in names {
            map.entry(name)
                .and_modify(|dup| *dup = true)
                .or_insert(false);
        }
        Some(map)
    }
}

impl PartialEq for StatementWithRow {
    fn eq(&self, other: &Self) -> bool {
        self.statement == other.statement && self.row == other.row
    }
}

pub type BlockStatement = Vec<StatementWithRow>;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Program {
    /// グローバル評価されるもの
    pub global: BlockStatement,
    /// 実行されるスクリプト
    pub script: BlockStatement,
    /// スクリプトの行情報
    pub lines: Vec<String>
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ScriptLocation {
    Path(PathBuf),
    Uri(String),
    #[default]
    None,
}
impl std::fmt::Display for ScriptLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptLocation::Path(path) => {
                match path.file_name() {
                    Some(name) => write!(f, "{}", name.to_str().unwrap_or_default()),
                    None => write!(f, ""),
                }
            },
            ScriptLocation::Uri(uri) => write!(f, "{uri}"),
            ScriptLocation::None => write!(f, ""),
        }
    }
}

#[derive(Clone, Default)]
pub struct ProgramBuilder {
    /// 定数
    consts: Vec<StatementWithRow>,
    /// パブリック変数
    publics: Vec<StatementWithRow>,
    /// OPTION定義
    options: Vec<StatementWithRow>,
    /// 関数等の定義
    /// - function/procedure
    /// - module/class
    /// - struct
    /// - def_dll
    definitions: Vec<StatementWithRow>,
    /// 実行される部分
    script: Vec<StatementWithRow>,
    /// スコープ情報
    scope: BuilderScope,
    /// スクリプトの位置
    location: ScriptLocation,
    /// callで呼び出されたスクリプトのスコープ
    call: Vec<(ScriptLocation, BuilderScope)>,
    /// callされた深さ
    depth: u32,
    builtin_names: Option<Vec<String>>
}
impl ProgramBuilder {
    pub fn new(path: Option<PathBuf>, builtin_names: Option<Vec<String>>) -> Self {
        // if let Some(names) = builtin_names {
        //     let _ = BUILTIN_NAMES.set(names);
        // }
        let location = path.map(|path| ScriptLocation::Path(path)).unwrap_or_default();
        Self {
            location,
            builtin_names,
            ..Default::default()
        }
    }
    pub fn new_uri(&self, uri: String) -> Self {
        Self {
            location: ScriptLocation::Uri(uri),
            builtin_names: self.builtin_names.clone(),
            ..Default::default()
        }
    }
    pub fn new_call_builder(&self, path: Option<PathBuf>) -> Self {
        let location = path.map(|path| ScriptLocation::Path(path)).unwrap_or_default();
        let depth = self.depth + 1;
        Self {
            location, depth,
            builtin_names: self.builtin_names.clone(),
            ..Default::default()
        }
    }
    pub fn new_eval_builder() -> Self {
        Self { ..Default::default() }
    }
    pub fn is_strict_mode(&self) -> bool {
        self.builtin_names.is_some()
    }
    pub fn location_ref(&self) -> &ScriptLocation {
        &self.location
    }
    pub fn location(&self) -> ScriptLocation {
        self.location.clone()
    }
    pub fn script_name(&self) -> String {
        self.location.to_string()
    }
    pub fn script_dir(&self) -> PathBuf {
        if let ScriptLocation::Path(path) = &self.location {
            path.parent().map(|p| p.to_path_buf())
        } else {
            None
        }.unwrap_or_default()
    }
    pub fn push_call_scope(&mut self, location: ScriptLocation, scope: BuilderScope) {
        self.call.push((location, scope));
    }
    pub fn build(mut self, lines: Vec<String>) -> Program {
        let mut global = vec![];
        global.append(&mut self.options);
        global.append(&mut self.consts);
        global.append(&mut self.publics);
        global.append(&mut self.definitions);
        let script = self.script;
        Program { global, script, lines }
    }
    pub fn build_call(self, lines: Vec<String>) -> (Program, ScriptLocation, BuilderScope) {
        let location = self.location.to_owned();
        let scope = self.scope.to_owned();
        let program = self.build(lines);
        (program, location, scope)
    }
    pub fn push_const(&mut self, statement: StatementWithRow) {
        if let Some(module) = self.scope.current_module_mut() {
            module.members.push(statement);
        } else {
            self.consts.push(statement);
        }
    }
    pub fn push_public(&mut self, statement: StatementWithRow) {
        if let Some(module) = self.scope.current_module_mut() {
            module.members.push(statement);
        } else {
            self.publics.push(statement);
        }
    }
    pub fn push_option(&mut self, statement: StatementWithRow) {
        self.options.push(statement)
    }
    pub fn push_def(&mut self, statement: StatementWithRow) {
        if let Some(module) = self.scope.current_module_mut() {
            module.members.push(statement);
        } else {
            self.definitions.push(statement)
        }
    }
    pub fn push_dim_member(&mut self, statement: StatementWithRow) {
        if let Some(module) = self.scope.current_module_mut() {
            module.members.push(statement);
        }
    }

    pub fn push_script(&mut self, statement: StatementWithRow) {
        self.script.push(statement)
    }
    /// 呼び出し元にグローバル定義及び識別子情報を移す
    pub fn append_global(&mut self, other: &mut Self) {
        self.consts.append(&mut other.consts);
        self.publics.append(&mut other.publics);
        self.options.append(&mut other.options);
        self.definitions.append(&mut other.definitions);
        self.call.append(&mut other.call);
    }
    /// 既に呼び出されているので不要なグローバル要素を除去
    pub fn remove_global(&mut self) {
        self.consts.clear();
        self.publics.clear();
        self.options.clear();
        self.definitions.clear();
        self.call.clear();
        self.scope = BuilderScope::default();
    }

    pub fn is_explicit_option_enabled(&self) -> bool {
        self.scope.option_explicit
    }
    pub fn is_optpublic_option_enabled(&self) -> bool {
        self.scope.option_optpublic
    }
    pub fn is_in_loop(&self) -> bool {
        self.scope.loop_count > 0
    }
    /// モジュール定義内かどうか (モジュール関数定義内も含む)
    pub fn is_in_module(&self) -> bool {
        self.scope.state.module
    }
    /// モジュールメンバ定義内かどうか
    pub fn is_in_module_member_definition(&self) -> bool {
        self.scope.state.module &&
        ! self.scope.state.function &&
        ! self.scope.state.anonymous
    }
    /// モジュール関数定義内かどうか
    pub fn is_in_module_member_function(&self) -> bool {
        self.scope.state.module && self.scope.state.function
    }
    /// class定義かどうか
    pub fn is_class_definition(&self) -> bool {
        self.scope.current_module.as_ref().map(|m| m.is_class).unwrap_or(false)
    }

    pub fn set_option_explicit(&mut self, b: bool) {
        self.scope.option_explicit = b;
    }

    pub fn set_option_optpublic(&mut self, b: bool) {
        self.scope.option_optpublic = b;
    }
    pub fn set_public_scope(&mut self) {
        self.scope.set_public();
    }
    pub fn reset_public_scope(&mut self) {
        self.scope.reset_public();
    }
    pub fn set_dim_scope(&mut self) {
        self.scope.set_dim();
    }
    pub fn reset_dim_scope(&mut self) {
        self.scope.reset_dim();
    }
    pub fn set_const_scope(&mut self) {
        self.scope.set_const();
    }
    pub fn reset_const_scope(&mut self) {
        self.scope.reset_const();
    }
    pub fn increase_loop_count(&mut self) {
        self.scope.increase_loop_count();
    }
    pub fn decrease_loop_count(&mut self) {
        self.scope.decrease_loop_count();
    }
    pub fn set_function_scope(&mut self) {
        self.scope.set_function();
    }
    pub fn reset_function_scope(&mut self) {
        self.scope.reset_function();
    }
    pub fn set_module_scope(&mut self, is_class: bool) {
        self.scope.set_module(is_class);
    }
    pub fn reset_module_scope(&mut self) {
        self.scope.reset_module();
    }
    pub fn set_anon_scope(&mut self) {
        self.scope.set_anon();
    }
    pub fn reset_anon_scope(&mut self) {
        self.scope.reset_anon();
    }
    /// デフォルトパラメータ評価中フラグをセット
    pub fn set_default_param(&mut self) {
        self.scope.set_default_param()
    }
    /// デフォルトパラメータ評価中フラグをリセット
    pub fn reset_default_param(&mut self) {
        self.scope.reset_default_param()
    }
    /// resultをセット
    pub fn set_result_as_param(&mut self) {
        self.set_param("result", Position::default(), Position::default());
    }
    /// パラメータ名をdim扱いでセット
    pub fn set_param(&mut self, name: &str, start: Position, end: Position) {
        let name = Name::new(name, start, end, self.depth);
        if self.scope.is_anonymous() {
            self.scope.push_anon_param(name);
        } else if self.scope.is_function() {
            self.scope.push_function_param(name);
        }
    }
    /// スコープ情報に名前をセット
    pub fn set_declared_name(&mut self, name: &str, start: Position, end: Position) {
        let name = Name::new(name, start, end, self.depth);
        let is_const = self.scope.is_const();
        let is_public = self.scope.is_public();
        let is_dim = self.scope.is_dim();
        let is_module = self.scope.is_module();
        let is_func = self.scope.is_function();
        if self.scope.is_anonymous() {
            // procedure()
            //     const x = 1
            //     public y = 1
            //     dim z = 1
            // fend
            // - const文脈
            // - public文脈
            // - dim文脈
            // - 即時関数
            if is_module {
                if is_const {
                    self.scope.push_module_const(name);
                } else if is_public {
                    self.scope.push_module_public(name);
                } else if is_dim {
                    self.scope.push_anon_dim(name);
                }
            } else {
                if is_const {
                    self.scope.push_const(name);
                } else if is_public {
                    self.scope.push_public(name);
                } else if is_dim {
                    self.scope.push_anon_dim(name);
                }
            }
        } else {
            if is_const {
                // 定数定義
                if is_module {
                    // module m
                    //     const x
                    //     function f()
                    //         const y
                    //     fend
                    self.scope.push_module_const(name);
                } else {
                    // const x
                    self.scope.push_const(name);
                }
            } else if is_public {
                // グローバル変数定義
                if is_module {
                    // モジュール内
                    self.scope.push_module_public(name);
                } else {
                    // モジュール外
                    self.scope.push_public(name);
                }
            } else if is_dim {
                // 変数定義
                if is_module {
                    // モジュール内
                    if is_func {
                        // 関数内
                        self.scope.push_function_dim(name);
                    } else {
                        // 関数外
                        self.scope.push_module_dim(name);
                    }
                } else {
                    // モジュール外
                    if is_func {
                        // 関数内
                        self.scope.push_function_dim(name);
                    } else {
                        // 関数外
                        self.scope.push_dim(name)
                    }
                }
            }
        }
    }
    pub fn set_assignee_name(&mut self, name: &str, start: Position, end: Position) {
        let is_const = self.scope.is_const();
        let is_public = self.scope.is_public();
        let is_dim = self.scope.is_dim();
        let name = Name::new(name, start, end, self.depth);
        if let Some(anon) = self.scope.current_anon_mut() {
            anon.assignee.push(name);
        } else if let Some(func) = self.scope.current_func_mut() {
            func.assignee.push(name);
        } else if let Some(module) = self.scope.current_module_mut() {
            if is_const {
                module.r#const.assignee.push(name);
            } else if is_public {
                module.public.assignee.push(name);
            } else if is_dim {
                module.dim.assignee.push(name);
            }
        } else {
            self.scope.assignee.push(name);
        }
    }
    fn is_in_builtins(&self, name: &str) -> bool {
        if self.scope.is_function() {
            // 関数内のみで以下が有効
            if "GET_FUNC_NAME".eq_ignore_ascii_case(name) {
                return true;
            }
            // module関数内では以下も有効
            if self.scope.is_module() {
                if "this".eq_ignore_ascii_case(name) {
                    return true;
                } else if "global".eq_ignore_ascii_case(name) {
                    return true;
                }
            }
        } else {
            // 関数外のみで以下が有効
            if "PARAM_STR".eq_ignore_ascii_case(name) {
                return true;
            }
        }
        if let Some(names) = &self.builtin_names {
            names.iter()
                .find(|builtin| builtin.eq_ignore_ascii_case(name))
                .is_some()
        } else {
            false
        }
    }
    /// 呼び出される識別子をセット、未定義変数かどうかチェックされる
    pub fn set_access_name(&mut self, name: &str, start: Position, end: Position) {
        if self.is_in_builtins(name) {
            return;
        }
        let name = Name::new(name, start, end, self.depth);
        let is_const = self.scope.is_const();
        let is_public = self.scope.is_public();
        let is_dim = self.scope.is_dim();
        let is_default_param = self.scope.is_default_param();
        let is_anon = self.scope.is_anonymous();
        let is_func = self.scope.is_function();
        let is_module = self.scope.is_module();
        if is_anon {
            // 無名関数
            if is_default_param {
                // 無名関数のデフォルトパラメータ
                if let Some(parent) = self.scope.anon_parent_mut() {
                    // 親となる無名関数がいればその文脈
                    parent.access.push(name);
                } else {
                    // 親無名関数がなければ、外のスコープの文脈
                    if let Some(func) = self.scope.current_func_mut() {
                        // 関数スコープ
                        // function x()
                        //     | p = access => p |()
                        func.access.push(name);
                    } else if let Some(module) = self.scope.current_module_mut() {
                        // モジュールスコープ
                        if is_const {
                            // module m
                            //     const x = | p = access => p |
                            module.r#const.access.push(name);
                        } else if is_public {
                            // module m
                            //     public x = | p = access => p |
                            module.public.access.push(name);
                        } else if is_dim {
                            // module m
                            //     dim x = | p = access => p |
                            module.dim.access.push(name);
                        }
                    } else {
                        // mainスコープ
                        if is_const {
                            // const x = function(p = access)
                            self.scope.r#const.access.push(name);
                        } else if is_public {
                            // public x = | p = access => p |
                            self.scope.public.access.push(name);
                        } else if is_dim {
                            // dim x = | p = access => p |
                            self.scope.dim.access.push(name);
                        } else {
                            // | p = access => p |()
                            self.scope.access.push(name);
                        }
                    }
                }
            } else {
                if let Some(anon) = self.scope.current_anon_mut() {
                    anon.access.push(name)
                }
            }
        } else if is_func {
            // 関数スコープ
            if is_default_param {
                // 関数のデフォルトパラメータはpublic文脈
                if let Some(module) = self.scope.current_module_mut() {
                    // module m
                    //     function x(p = access)
                    module.public.access.push(name);
                } else {
                    // function x(p = access)
                    self.scope.public.access.push(name);
                }
            } else {
                // 関数スコープ内
                if is_module {
                    // module m
                    //     function x()
                    //         const c = access
                    //         public p = access
                    //         dim p = access
                    //         result = access
                    if is_const {
                        // module const文脈
                        self.scope.push_module_const_access(name);
                    } else if is_public {
                        // module public文脈
                        self.scope.push_module_public_access(name);
                    } else if let Some(func) = self.scope.current_func_mut() {
                        // 関数スコープ文脈
                        func.access.push(name);
                    } else {
                        self.scope.push_module_dim_access(name);
                    }
                } else {
                    // function x()
                    //     const c = access
                    //     public p = access
                    //     dim p = access
                    //     result = access
                    if is_const {
                        // グローバル const文脈
                        self.scope.r#const.access.push(name);
                    } else if is_public {
                        // グローバル public文脈
                        self.scope.public.access.push(name);
                    } else {
                        // 関数スコープ文脈
                        if let Some(func) = self.scope.current_func_mut() {
                            func.access.push(name);
                        }
                    }
                }
            }
        } else if let Some(module) = self.scope.current_module_mut() {
            // module m
            //     const c = access
            //     public p = access
            //     dim p = access
            if is_const {
                module.r#const.access.push(name);
            } else if is_public {
                module.public.access.push(name);
            } else if is_dim {
                module.dim.access.push(name);
            }
        } else if is_const {
            // const x = access
            self.scope.r#const.access.push(name);
        } else if is_public {
            // public x = access
            self.scope.public.access.push(name);
        } else {
            // dim x = access
            // print access
            self.scope.access.push(name);
        }
    }
    pub fn set_definition_name(&mut self, name: &str, start: Position, end: Position) {
        let name = Name::new(name, start, end, self.depth);
        self.scope.definition.push(name);
    }
    pub fn take_module_members(&mut self, block: &mut BlockStatement) {
        if let Some(module) = self.scope.current_module.as_mut() {
            block.append(&mut module.members);
        }
    }

    /// 代入を暗黙の宣言とする
    pub fn declare_implicitly(&mut self) {
        self.scope.implicit_declaration();
        for (_, scope) in self.call.as_mut_slice() {
            scope.implicit_declaration();
        }
    }
    pub fn check_option_explicit(&self) -> Vec<(ScriptLocation, Names)> {
        let mut location_and_names = vec![];

        let mut call = self.get_call_public();
        let names = self.scope.check_option_explicit(&call);
        location_and_names.push((self.location.clone(), names));

        call.append(self.scope.public.names.clone());

        for (location, scope) in &self.call {
            let names = scope.check_option_explicit(&call);
            location_and_names.push((location.clone(), names));
        }

        location_and_names
    }
    pub fn check_duplicated(&self) -> Vec<(ScriptLocation, Names)> {
        let mut location_and_names = vec![];

        let mut call = self.get_call_const();

        let names = self.scope.check_duplicated(&call);
        location_and_names.push((self.location.clone(), names));

        call.append(self.scope.r#const.names.clone());
        for (location, scope) in &self.call {
            let names = scope.check_duplicated(&call);
            location_and_names.push((location.clone(), names));
        }

        location_and_names
    }
    pub fn check_public_duplicated(&self) -> Vec<(ScriptLocation, Names)> {
        let mut location_and_names = vec![];
        let mut call_public = self.get_call_public();

        let names = self.scope.check_public_duplicated(&call_public);
        location_and_names.push((self.location.clone(), names));

        call_public.append(self.scope.public.names.clone());
        for (location, scope) in &self.call {
            let names = scope.check_public_duplicated(&call_public);
            location_and_names.push((location.clone(), names));
        }
        location_and_names
    }
    pub fn check_access(&self) -> Vec<(ScriptLocation, Names)> {
        let mut location_and_names = vec![];

        let mut call = self.get_call_global();

        let names = self.scope.check_access(&call);
        location_and_names.push((self.location.clone(), names));

        call.append(&self.scope.r#const.names);
        call.append(&self.scope.public.names);
        call.append(&self.scope.definition);

        for (location, scope) in &self.call {
            let names = scope.check_access(&call);
            location_and_names.push((location.clone(), names));
        }

        location_and_names
    }
    fn get_call_public(&self) -> Names {
        let names = self.call.iter()
            .map(|(_, scope)| {
                scope.public.names.0.clone()
            })
            .flatten()
            .collect();
        Names(names)
    }
    fn get_call_const(&self) -> Names {
        let names = self.call.iter()
            .map(|(_, scope)| {
                scope.r#const.names.0.clone()
            })
            .flatten()
            .collect();
        Names(names)
    }
    fn get_call_global(&self) -> Names {
        let names = self.call.iter()
            .map(|(_, scope)| {
                let p = scope.public.names.0.clone();
                let c = scope.r#const.names.0.clone();
                let d = scope.definition.0.clone();
                [p, c, d].into_iter().flatten().collect::<Vec<_>>()
            })
            .flatten()
            .collect();
        Names(names)
    }
}

/// スコープ情報を管理する
#[derive(Clone, Default, Debug)]
pub struct BuilderScope {
    /// OPTION EXPLICITの状態を示す
    option_explicit: bool,
    /// OPTION OPTPUBLICの状態を示す
    option_optpublic: bool,
    /// ループの深さ
    loop_count: u32,
    /// const定義
    r#const: ConstScope,
    /// public定義
    public: PublicScope,
    /// dim定義
    dim: DimScope,
    /// 関数定義
    function: Functions,
    current_function: Option<FuncScope>,
    /// module定義
    module: Modules,
    /// 現在のModuleScope
    /// 1. module解析時にセット
    /// 2. 解析終了時にmodule定義に積んでNoneに戻す
    current_module: Option<ModuleScope>,
    /// 現在の無名関数
    current_anon: Option<AnonFuncScope>,
    /// スコープの状態
    state: ScopeState,
    /// 被代入変数名 OPTION EXPLICIT対象
    assignee: Names,
    /// 呼び出される変数名
    access: Names,
    /// 関数名, module名等
    definition: Names,
}
impl BuilderScope {
    /// 代入を暗黙の定義とみなす
    fn implicit_declaration(&mut self) {
        self.assignee.remove_dup();
        let mut declaration = self.assignee.get_undeclared_2(
            &vec![&self.dim.names],
            &vec![&self.r#const.names, &self.public.names]
        );
        self.dim.names.append_mut(&mut declaration);
        self.r#const.implicit_declaration();
        self.public.implicit_declaration();
        self.dim.implicit_declaration();
        for func in self.function.0.as_mut_slice() {
            func.implicit_declaration();
        }
        for module in self.module.0.as_mut_slice() {
            module.implicit_declaration();
        }
    }
    /// 重複定義だった名前を返す
    fn check_duplicated(&self, call: &Names) -> Names {
        let mut names = Names::default();

        // グローバル定数
        let dup = self.r#const.get_dups(call);
        names.append(dup);
        // グローバル変数
        let dup = self.public.get_dups(&self.r#const.names, call);
        names.append(dup);
        // ローカル変数
        let dup = self.dim.get_dups(&self.r#const.names, call);
        names.append(dup);
        // グローバル関数
        self.function.0.iter().for_each(|func| {
            let dup = func.get_dups(&self.r#const.names, call);
            names.append(dup);
        });
        // module
        self.module.0.iter().for_each(|module| {
            let dup = module.get_dups();
            names.append(dup);
        });

        names
    }
    /// OPTPUBLIC
    fn check_public_duplicated(&self, call_public: &Names) -> Names {
        let mut names = self.public.get_public_dups(call_public);
        self.module.0.iter().for_each(|module| {
            let dup = module.get_public_dups();
            names.append(dup);
        });
        names
    }
    /// OPTION EXPLICIT違反だった名前を返す
    /// - call: call文がある場合に全ファイルのpublicを渡す
    fn check_option_explicit(&self, call: &Names) -> Names {
        self.get_undeclared(UndeclaredNameType::Assign, call)
    }
    /// 呼び出しチェック
    fn check_access(&self, call: &Names) -> Names {
        self.get_undeclared(UndeclaredNameType::Access, call)
    }
    fn get_undeclared(&self, r#type: UndeclaredNameType, call: &Names) -> Names {
        let mut names = Names::default();
        let (mut undeclared, outer) = match &r#type {
            UndeclaredNameType::Access => (
                self.access.get_undeclared(&vec![&self.dim.names, &self.r#const.names, &self.public.names, &self.definition, &call]),
                vec![&self.r#const.names, &self.public.names, &self.definition, &call],
            ),
            UndeclaredNameType::Assign => (
                self.assignee.get_undeclared(&vec![&self.dim.names, &self.public.names, &call]),
                vec![&self.public.names, &call],
            )
        };
        names.append_mut(&mut undeclared);
        let mut undeclared = self.r#const.get_undeclared(&r#type, &match r#type {
            UndeclaredNameType::Access => vec![&self.r#const.names, call],
            UndeclaredNameType::Assign => vec![],
        });
        names.append_mut(&mut undeclared);
        let mut undeclared = self.public.get_undeclared(&r#type, &outer);
        names.append_mut(&mut undeclared);
        let mut undeclared = self.dim.get_undeclared(&r#type, &outer);
        names.append_mut(&mut undeclared);
        for func in &self.function.0 {
            let mut undeclared = func.get_undeclared(&r#type, &outer);
            names.append_mut(&mut undeclared);
        }
        for module in &self.module.0 {
            let mut undeclared = module.get_undeclared(&r#type, &outer);
            names.append_mut(&mut undeclared);
        }
        names
    }
    /* フラグ取得 */
    fn is_const(&self) -> bool {self.state.r#const}
    fn is_public(&self) -> bool {self.state.public}
    fn is_dim(&self) -> bool {self.state.dim}
    fn is_function(&self) -> bool {self.state.function}
    fn is_module(&self) -> bool {self.state.module}
    fn is_anonymous(&self) -> bool {self.state.anonymous}
    /* const文脈 */
    fn set_const(&mut self) {self.state.r#const = true;}
    fn reset_const(&mut self) {self.state.r#const = false;}
    /// グローバル定数を登録
    fn push_const(&mut self, name: Name) {self.r#const.names.push(name);}
    /* public文脈 */
    fn set_public(&mut self) {self.state.public = true;}
    fn reset_public(&mut self) {self.state.public = false;}
    /// グローバル変数を登録
    fn push_public(&mut self, name: Name) {self.public.names.push(name);}
    /* dim文脈 */
    fn set_dim(&mut self) {self.state.dim = true;}
    fn reset_dim(&mut self) {self.state.dim = false;}
    /// メインのローカル変数を登録
    fn push_dim(&mut self, name: Name) {self.dim.names.push(name);}
    /* 関数 */
    fn set_function(&mut self) {
        self.current_function = Some(FuncScope::default());
        self.state.function = true;
    }
    fn reset_function(&mut self) {
        if let Some(func) = self.current_function.to_owned() {
            if let Some(module) = self.current_module_mut() {
                module.function.0.push(func);
            } else {
                self.function.0.push(func);
            }
            self.current_function = None;
        }
        self.state.function = false;
    }
    fn current_func_mut(&mut self) -> Option<&mut FuncScope> {
        self.current_function.as_mut()
    }
    /// 関数内の変数定義
    fn push_function_dim(&mut self, name: Name) {
        if let Some(func) = self.current_function.as_mut() {
            func.dim.push(name);
        }
    }
    fn push_function_param(&mut self, name: Name) {
        if let Some(func) = self.current_function.as_mut() {
            func.param.push(name);
        }
    }
    /* ループ */
    fn increase_loop_count(&mut self) {self.loop_count += 1;}
    fn decrease_loop_count(&mut self) {self.loop_count -= 1;}
    /* module */
    fn set_module(&mut self, is_class: bool) {
        self.current_module = Some(ModuleScope::new(is_class));
        self.state.module = true;
    }
    fn reset_module(&mut self) {
        if let Some(module) = self.current_module.to_owned() {
            self.module.0.push(module);
            self.current_module = None;
        }
        self.state.module = false;
    }
    fn current_module_mut(&mut self) -> Option<&mut ModuleScope> {
        self.current_module.as_mut()
    }
    /// モジュールconstメンバを登録
    fn push_module_const(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.r#const.names.push(name);
        }
    }
    /// モジュールpublicメンバを登録
    fn push_module_public(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.public.names.push(name);
        }
    }
    /// モジュールdimメンバを登録
    fn push_module_dim(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.dim.names.push(name);
        }
    }
    /// モジュールconstのaccessを登録
    fn push_module_const_access(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.r#const.access.push(name);
        }
    }
    /// モジュールpublicのaccessを登録
    fn push_module_public_access(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.public.access.push(name);
        }
    }
    /// モジュールdimのaccessを登録
    fn push_module_dim_access(&mut self, name: Name) {
        if let Some(module) = self.current_module.as_mut() {
            module.dim.access.push(name);
        }
    }
    /* 無名関数 */
    fn anon_parent_mut(&mut self) -> Option<&mut AnonFuncScope> {
        if let Some(anon) = self.current_anon.as_mut() {
            anon.parent.as_deref_mut()
        } else {
            None
        }
    }
    fn current_anon_mut(&mut self) -> Option<&mut AnonFuncScope> {
        self.current_anon.as_mut()
    }
    fn push_anon_dim(&mut self, name: Name) {
        if let Some(anon) = self.current_anon.as_mut() {
            anon.dim.push(name);
        }
    }
    fn push_anon_param(&mut self, name: Name) {
        if let Some(anon) = self.current_anon.as_mut() {
            anon.param.push(name);
        }
    }
    fn set_anon(&mut self) {
        if let Some(parent) = self.current_anon.to_owned() {
            let current = AnonFuncScope::new_child(parent);
            self.current_anon = Some(current);
        } else {
            self.current_anon = Some(AnonFuncScope::new(&self.state));
            // 無名関数解析中はスコープ情報をリセット
            self.state.reset();
        }
        self.state.anonymous = true;
    }
    fn reset_anon(&mut self) {
        if let Some(mut current) = self.current_anon.to_owned() {
            if let Some(parent) = current.parent.to_owned() {
                // 親がいた場合は子として配置
                let mut parent = *parent;
                current.parent = None;
                parent.anon.0.push(current);
                // currentを親にする
                self.current_anon = Some(parent);
            } else {
                // 親がいない場合はスコープ情報を復元
                self.state.restore(&current.scope);
                // 自身を適切な場所に配置する
                if let Some(func) = self.current_function.as_mut() {
                    func.anon.0.push(current);
                } else if let Some(module) = self.current_module.as_mut() {
                    if self.state.r#const {
                        // const文脈
                        module.r#const.anon.0.push(current);
                    } else if self.state.public {
                        // public文脈
                        module.public.anon.0.push(current);
                    } else if self.state.dim {
                        // dim文脈
                        module.dim.anon.0.push(current);
                    }
                } else {
                    if self.state.r#const {
                        // const文脈
                        self.r#const.anon.0.push(current);
                    } else if self.state.public {
                        // public文脈
                        self.public.anon.0.push(current);
                    } else if self.state.dim {
                        // dim文脈
                        self.dim.anon.0.push(current);
                    }
                }
                // currentをNoneにする
                self.current_anon = None;
            }
        }
        self.state.anonymous = false;
    }
    /* デフォルトパラメータ */
    fn is_default_param(&self) -> bool {
        self.state.default_param
    }
    fn set_default_param(&mut self) {
        self.state.default_param = true;
    }
    fn reset_default_param(&mut self) {
        self.state.default_param = false;
    }
}
#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub struct Name {
    pub name: String,
    pub start: Position,
    pub end: Position,
    pub depth: u32,
}
impl Eq for Name {

}
impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl Name {
    fn new(name: &str, start: Position, end: Position, depth: u32) -> Self {
        Self { name: name.to_ascii_uppercase(), start, end, depth }
    }
    /// 重複判定
    /// - compare_name_only
    ///     - true : 名前が一致のみで重複、主に外部スコープとの比較
    ///     - false: 名前が一致をかつ後ろにあれば重複、主に内部スコープとの比較
    fn is_duplicated(&self, maybe_dup: &Self, flg: &DupFlg) -> bool {
        if self == maybe_dup {
            // 一致は無視
            false
        } else {
            match flg {
                DupFlg::ByName => {
                    self.name == maybe_dup.name
                },
                DupFlg::ByPos => {
                    // Positionが大きければ後にある
                    self.name == maybe_dup.name &&
                    (self.start < maybe_dup.start && self.end < maybe_dup.end)
                },
                DupFlg::ByDepth => {
                    self.name == maybe_dup.name &&
                    self.depth < maybe_dup.depth
                },
            }
        }
    }
}

/// 重複確認方法を示すフラグ
enum DupFlg {
    /// 名前を比較して一致なら重複
    ByName,
    /// 位置が対象より後ろなら重複
    ByPos,
    /// 対象より深い位置のファイルであれば重複
    ByDepth,
}

#[derive(Debug, Clone, Default)]
pub struct Names(Vec<Name>);
impl Names {
    fn push(&mut self, name: Name) {
        self.0.push(name);
    }
    fn append<V: Into<Vec<Name>>>(&mut self, names: V) {
        let mut names = names.into();
        self.0.append(&mut names);
    }
    fn append_mut(&mut self, names: &mut Self) {
        self.0.append(&mut names.0)
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Name> {
        self.0.iter()
    }
    fn if_duplicated(&self, other: &Name, flg: DupFlg) -> Option<Name> {
        self.0.iter()
            .find(|name| name.is_duplicated(other, &flg))
            .map(|_| other.clone())
    }
    /// 自身を宣言一覧と比較し、未宣言のものを返す
    fn get_undeclared(&self, declarations: &Vec<&Names>) -> Names {
        let names = self.0.iter()
            .filter_map(|name| {
                declarations.iter()
                    // 宣言リストに名前が含まれているものを探す
                    .find(|names| names.contains(name))
                    // どこにも含まれていなければNone
                    .is_none()
                    // 未宣言としNameを返す
                    .then_some(name.clone())
            })
            .collect();
        Names(names)
    }
    /// 宣言リストと比較し未宣言を返すが、ローカルは位置関係も考慮する
    fn get_undeclared_2(&self, local: &Vec<&Names>, global: &Vec<&Names>) -> Names {
        let names = self.0.iter()
            .filter_map(|name| {
                let in_global = global.iter()
                    // グローバルから探す
                    .find(|names| names.contains(name))
                    .is_some();
                if in_global {
                    // グローバルで一致があったので返さない
                    None
                } else {
                    // ローカルを探す
                    local.iter()
                        .find(|names| names.contains_bypos(name))
                        // 重複がない
                        .is_none()
                        // コピーを返す
                        .then_some(name.clone())
                }
            })
            .collect();
        Names(names)
    }
    fn contains(&self, other: &Name) -> bool {
        self.iter().find(|name| name.is_duplicated(other, &DupFlg::ByName)).is_some()
    }
    fn contains_bypos(&self, other: &Name) -> bool {
        self.iter().find(|name| name.is_duplicated(other, &DupFlg::ByPos)).is_some()
    }
    fn remove_dup(&mut self) {
        self.0.sort_by(|a, b| a.name.cmp(&b.name));
        self.0.dedup_by(|a, b| a.name == b.name);
    }
}
impl Into<Vec<Name>> for Names {
    fn into(self) -> Vec<Name> {
        self.0
    }
}
impl Into<Vec<Name>> for &Names {
    fn into(self) -> Vec<Name> {
        self.clone().0
    }
}


#[derive(Debug, Clone, Default)]
struct ConstScope {
    names: Names,
    anon: AnonFuncs,
    /// const x = access
    access: Names,
    /// const x = assignee := 1
    assignee: Names,
}
impl ConstScope {
    /// const文脈重複チェック
    /// 1. 自身の重複を確認
    /// 2. callのconstと比較
    /// 3. 無名関数内を確認
    fn get_dups(&self, call: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                self.names.if_duplicated(name, DupFlg::ByPos)
                    .or(call.if_duplicated(name, DupFlg::ByDepth))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let outer = vec![call, &self.names];
            let dup = anon.get_dups(outer, None);
            names.append(dup);
        }
        names
    }
    fn get_module_dups(&self) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                self.names.if_duplicated(name, DupFlg::ByPos)
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let dup = anon.get_dups(vec![&self.names], None);
            names.append(dup);
        }
        names
    }
    fn implicit_declaration(&mut self) {
        for anon in self.anon.0.as_mut_slice() {
            anon.implicit_declaration();
        }
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer: &Vec<&Names>) -> Names {
        let mut names = match r#type {
            UndeclaredNameType::Access => self.access.get_undeclared(outer),
            UndeclaredNameType::Assign => self.assignee.get_undeclared(outer),
        };
        let mut anon = self.anon.get_undeclared(r#type, outer);
        names.append_mut(&mut anon);
        names
    }
}
#[derive(Debug, Clone, Default)]
struct PublicScope {
    names: Names,
    anon: AnonFuncs,
    /// public x = access
    access: Names,
    /// public x = assignee := 1
    assignee: Names,
}
impl PublicScope {

    fn get_dups(&self, outer_const: &Names, call: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                outer_const.if_duplicated(name, DupFlg::ByName)
                    .or(call.if_duplicated(name, DupFlg::ByDepth))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let outer = vec![outer_const, call];
            let dup = anon.get_dups(outer, None);
            names.append(dup);
        }
        names
    }
    fn get_public_dups(&self, call_public: &Names) -> Names {
        let dup = self.names.iter()
            .filter_map(|name| {
                self.names.if_duplicated(name, DupFlg::ByPos)
                    .or(call_public.if_duplicated(name, DupFlg::ByDepth))

            })
            .collect();
        Names(dup)
    }
    fn get_module_dups(&self, module_const: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                module_const.if_duplicated(name, DupFlg::ByName)
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let outer = vec![module_const];
            let dup = anon.get_dups(outer, None);
            names.append(dup);
        }
        names
    }
    fn implicit_declaration(&mut self) {
        for anon in self.anon.0.as_mut_slice() {
            anon.implicit_declaration();
        }
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer: &Vec<&Names>) -> Names {
        let mut names = match r#type {
            UndeclaredNameType::Access => self.access.get_undeclared(outer),
            UndeclaredNameType::Assign => self.assignee.get_undeclared(outer),
        };
        let mut dec = outer.clone();
        dec.push(&self.names);
        let mut anon = self.anon.get_undeclared(r#type, &dec);
        names.append_mut(&mut anon);
        names
    }
}
#[derive(Debug, Clone, Default)]
struct DimScope {
    names: Names,
    anon: AnonFuncs,
    /// dim x = access
    access: Names,
    /// dim x = assignee := 1
    assignee: Names,
}
impl DimScope {
    fn get_dups(&self, outer_const: &Names, call: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                outer_const.if_duplicated(name, DupFlg::ByName)
                    .or(call.if_duplicated(name, DupFlg::ByDepth))
                    .or(self.names.if_duplicated(name, DupFlg::ByPos))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let outer = vec![outer_const, call];
            let dup = anon.get_dups(outer, Some(&self.names));
            names.append(dup);
        }
        names
    }
    fn get_module_dups(&self, module_const: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.names.iter()
            .filter_map(|name| {
                module_const.if_duplicated(name, DupFlg::ByName)
                    .or(self.names.if_duplicated(name, DupFlg::ByPos))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let outer = vec![module_const];
            let dup = anon.get_dups(outer, None);
            names.append(dup);
        }
        names
    }
    fn implicit_declaration(&mut self) {
        for anon in self.anon.0.as_mut_slice() {
            anon.implicit_declaration();
        }
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer: &Vec<&Names>) -> Names {
        let mut names = match r#type {
            UndeclaredNameType::Access => self.access.get_undeclared(outer),
            UndeclaredNameType::Assign => self.assignee.get_undeclared(outer),
        };
        let mut declarations = outer.clone();
        declarations.push(&self.names);
        let mut anon = self.anon.get_undeclared(r#type, &declarations);
        names.append_mut(&mut anon);
        names
    }
}
#[derive(Debug, Clone, Default)]
struct FuncScope {
    dim: Names,
    anon: AnonFuncs,
    /// OPTION EXPLICIT対象
    assignee: Names,
    /// 呼び出し
    access: Names,
    /// パラメータ名
    param: Names
}
impl FuncScope {
    fn get_dups(&self, outer_const: &Names, call: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.dim.iter()
            .filter_map(|name| {
                outer_const.if_duplicated(name, DupFlg::ByName)
                    .or(call.if_duplicated(name, DupFlg::ByDepth))
                    .or(self.dim.if_duplicated(name, DupFlg::ByPos))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let dup = anon.get_dups(vec![outer_const, call], Some(&self.dim));
            names.append(dup);
        }
        names
    }
    fn get_module_dups(&self, module_const: &Names) -> Names {
        let mut names = Names::default();
        let dup: Vec<Name> = self.dim.iter()
            .filter_map(|name| {
                module_const.if_duplicated(name, DupFlg::ByName)
                    .or(self.dim.if_duplicated(name, DupFlg::ByPos))
            })
            .collect();
        names.append(dup);
        for anon in &self.anon.0 {
            let dup = anon.get_dups(vec![module_const], Some(&self.dim));
            names.append(dup);
        }
        names
    }
    fn implicit_declaration(&mut self) {
        // パラメータ名を除く
        self.assignee.remove_dup();
        let mut declaration = self.assignee.get_undeclared_2(
            &vec![&self.dim, &self.param],
            &vec![]
        );
        self.dim.append_mut(&mut declaration);
        for anon in self.anon.0.as_mut_slice() {
            anon.implicit_declaration();
        }
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer: &Vec<&Names>) -> Names {
        let mut declarations = outer.clone();
        declarations.push(&self.dim);
        declarations.push(&self.param);
        let mut names = match r#type {
            UndeclaredNameType::Access => self.access.get_undeclared(&declarations),
            UndeclaredNameType::Assign => self.assignee.get_undeclared(&declarations),
        };
        let mut anon = self.anon.get_undeclared(r#type, &declarations);
        names.append_mut(&mut anon);
        names
    }
}
#[derive(Debug, Clone, Default)]
struct Functions(Vec<FuncScope>);
#[derive(Debug, Clone, Copy, Default)]
enum AnonDeclarationScope {
    Const,
    Public,
    Dim,
    #[default]
    None,
}
impl From<&ScopeState> for AnonDeclarationScope {
    fn from(state: &ScopeState) -> Self {
        if state.r#const {
            Self::Const
        } else if state.public {
            Self::Public
        } else if state.dim {
            Self::Dim
        } else {
            Self::None
        }
    }
}
// impl Into<AnonDeclarationScope> for &ScopeState {

// }
#[derive(Debug, Clone, Default)]
struct AnonFuncScope {
    scope: AnonDeclarationScope,
    /// 自身が無名関数内で定義されている場合、親を入れる
    parent: Option<Box<AnonFuncScope>>,
    /// 自身の内で定義された変数
    dim: Names,
    /// 自身の内で定義された無名関数
    anon: AnonFuncs,
    /// OPTION EXPLICIT対象
    assignee: Names,
    /// 呼び出し
    access: Names,
    /// パラメータ名
    param: Names,
}
impl AnonFuncScope {
    fn new<S>(scope: S) -> Self
        where S: Into<AnonDeclarationScope>
    {
        Self {
            scope: scope.into(),
            ..Default::default()
        }
    }
    fn new_child(parent: Self) -> Self {
        Self {
            scope: parent.scope,
            parent: Some(Box::new(parent)),
            ..Default::default()
        }
    }
    /// 重複チェック対象
    /// - global const
    /// - parent scope dim
    ///     - main, function
    ///     - anonymous
    /// - function local dim (自身)
    ///
    /// 引数
    /// - const: const定義
    /// - parent: 親スコープが無名関数ではない場合に指定
    fn get_dups(&self, r#const: Vec<&Names>, parent_dim: Option<&Names>) -> Names {
        let mut names = Names::default();
        let parent_anon_dims = self.get_parent_dims();
        let dups: Vec<Name> = self.dim.iter()
            .filter_map(|name| {
                r#const.iter().find_map(|names| names.if_duplicated(name, DupFlg::ByName))
                    .or( self.dim.if_duplicated(name, DupFlg::ByPos) )
                    .or( parent_anon_dims.as_ref().map(|names| names.if_duplicated(name, DupFlg::ByPos)).flatten() )
                    .or( parent_dim.as_ref().map(|names| names.if_duplicated(name, DupFlg::ByPos)).flatten() )
            })
            .collect();
        names.append(dups);
        names
    }
    fn get_parent_dims(&self) -> Option<Names> {
        let mut names = Names::default();
        if let Some(parent) = &self.parent {
            let dim = parent.dim.clone();
            names.append(dim);
            if let Some(dim) = parent.get_parent_dims() {
                names.append(dim);
            }
            Some(names)
        } else {
            None
        }
    }
    fn implicit_declaration(&mut self) {
        // パラメータ名を除く
        let mut assignee = self.assignee.get_undeclared(&vec![&self.param]);
        assignee.remove_dup();
        self.dim.append_mut(&mut assignee);
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer_declarations: &Vec<&Names>) -> Names {
        let mut declarations = outer_declarations.clone();
        declarations.push(&self.dim);
        declarations.push(&self.param);
        //vec![vec![&self.dim, &self.param], outer_declarations].into_iter().flatten().collect();
        let names = match r#type {
            UndeclaredNameType::Access => self.access.get_undeclared(&declarations),
            UndeclaredNameType::Assign => self.assignee.get_undeclared(&declarations),
        };
        names
    }
}
#[derive(Debug, Clone, Default)]
struct AnonFuncs(Vec<AnonFuncScope>);
impl AnonFuncs {
    fn get_undeclared(&self, r#type: &UndeclaredNameType, declarations: &Vec<&Names>) -> Names {
        let mut names = Names::default();
        for anon in &self.0 {
            let mut undeclared = anon.get_undeclared(r#type, declarations);
            names.append_mut(&mut undeclared);
        }
        names
    }
}

enum UndeclaredNameType {
    Access,
    Assign,
}
#[derive(Debug, Clone, Default)]
struct ModuleScope {
    r#const: ConstScope,
    public: PublicScope,
    dim: DimScope,
    function: Functions,
    is_class: bool,
    members: Vec<StatementWithRow>,
    // access: Names,
}
impl ModuleScope {
    fn new(is_class: bool) -> Self {
        Self {
            is_class,
            ..Default::default()
        }
    }
    /// 重複チェック対象
    /// - member const
    ///     - member const
    /// - member const anon
    ///     - global const
    ///     - member const
    /// - member public
    ///     - member const
    /// - member public anon
    ///     - global const
    ///     - member const
    /// - member dim
    ///     - member const
    ///     - member dim
    /// - member dim anon
    ///     - global const
    ///     - member const
    ///     - member dim
    /// - 関数
    fn get_dups(&self) -> Names {
        let mut names = Names::default();

        // member const
        let dup = self.r#const.get_module_dups();
        names.append(dup);
        // member public
        let dup = self.public.get_module_dups(&self.r#const.names);
        names.append(dup);
        // member dim
        let dup = self.dim.get_module_dups(&self.r#const.names);
        names.append(dup);
        // member function
        self.function.0.iter()
            .for_each(|func| {
                let dup = func.get_module_dups(&self.r#const.names);
                names.append(dup);
            });

        names
    }
    fn get_public_dups(&self) -> Names {
        let names = self.public.names.iter().filter_map(|name| {
            self.public.names.if_duplicated(name, DupFlg::ByPos)
        })
        .collect();
        Names(names)
    }
    fn implicit_declaration(&mut self) {
        self.r#const.implicit_declaration();
        self.public.implicit_declaration();
        self.dim.implicit_declaration();
        for func in self.function.0.as_mut_slice() {
            func.implicit_declaration();
        }
    }
    fn get_undeclared(&self, r#type: &UndeclaredNameType, outer: &Vec<&Names>) -> Names {
        let mut dec = outer.clone();
        match r#type {
            UndeclaredNameType::Access => {
                let mut inner = vec![&self.r#const.names, &self.public.names, &self.dim.names];
                dec.append(&mut inner);
            },
            UndeclaredNameType::Assign => {
                let mut inner = vec![&self.public.names, &self.dim.names];
                dec.append(&mut inner);
            },
        };
        let mut names = self.r#const.get_undeclared(r#type , &dec);
        let mut tmp = self.public.get_undeclared(r#type , &dec);
        names.append_mut(&mut tmp);
        let mut tmp = self.dim.get_undeclared(r#type , &dec);
        names.append_mut(&mut tmp);
        for func in &self.function.0 {
            let mut tmp = func.get_undeclared(r#type , &dec);
            names.append_mut(&mut tmp);
        }
        names
    }
}
#[derive(Debug, Clone, Default)]
struct Modules(Vec<ModuleScope>);

#[derive(Debug, Clone, Default, PartialEq)]
struct ScopeState {
    r#const: bool,
    public: bool,
    dim: bool,
    function: bool,
    module: bool,
    anonymous: bool,
    /// デフォルトパラメータのデフォルト値評価
    default_param: bool,
}
impl ScopeState {
    fn reset(&mut self) {
        self.r#const = false;
        self.public = false;
        self.dim = false;
    }
    fn restore(&mut self, ads: &AnonDeclarationScope) {
        match ads {
            AnonDeclarationScope::Const => {self.r#const = true;},
            AnonDeclarationScope::Public => {self.public = true;},
            AnonDeclarationScope::Dim => {self.dim = true;},
            AnonDeclarationScope::None => {},
        }
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Precedence {
    /// 優先度最低
    Lowest,
    /// :=
    Assign,
    /// ?:
    Ternary,
    /// or xor
    Or,
    /// and
    And,
    /// == != <>
    Equality,
    /// > < >= <=
    Relational,
    /// + -
    Additive,
    /// * / mod
    Multiplicative,
    /// X or !X
    Prefix,
    /// func(x)
    FuncCall,
    /// array[index]
    Index,
    /// foo.bar
    DotCall,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Params {
    Identifier(Identifier), // 通常の引数
    Reference(Identifier), // var引数
    Array(Identifier, bool), // 引数[] (変数強制), bool はrefかどうか
    WithDefault(Identifier, Box<Expression>), // デフォルト値
    Variadic(Identifier), // 可変長引数
    VariadicDummy, // 可変長引数用ダミーー
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Params::Identifier(ref i) => write!(f, "{}", i),
            Params::Reference(ref i) => write!(f, "ref {}", i),
            Params::Array(ref i, b) => if b {
                write!(f, "ref {}[]", i)
            } else {
                write!(f, "{}[]", i)
            },
            Params::WithDefault(ref i, _) => write!(f, "{} = [default]", i),
            Params::Variadic(ref i) => write!(f, "args {}", i),
            _ => write!(f, "")
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// どの型でも良い、オプション指定しなかった場合常にこれ
    Any,
    /// Object::String
    String,
    /// Object::Num
    Number,
    /// Object::Bool
    Bool,
    /// Object::Array
    Array,
    /// Object::HashTbl
    HashTbl,
    /// Object::AnonFunc | Object::Function | Object::BuiltinFunction
    Function,
    /// Object::UObject | Object::UChild
    UObject,
    /// Object::Instance
    UserDefinition(String),
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::Any => write!(f, ""),
            ParamType::String => write!(f, "string"),
            ParamType::Number => write!(f, "number"),
            ParamType::Bool => write!(f, "bool"),
            ParamType::Array => write!(f, "array"),
            ParamType::HashTbl => write!(f, "hash"),
            ParamType::Function => write!(f, "func"),
            ParamType::UObject => write!(f, "uobject"),
            ParamType::UserDefinition(ref name) => write!(f, "{}", name),
        }
    }
}

impl From<String> for ParamType {
    fn from(s: String) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "string" => ParamType::String,
            "number" => ParamType::Number,
            "bool" => ParamType::Bool,
            "array" => ParamType::Array,
            "hash" => ParamType::HashTbl,
            "func" => ParamType::Function,
            "uobject" => ParamType::UObject,
            _ => ParamType::UserDefinition(s.into())
        }
    }
}

impl ParamType {
    pub fn is_any(&self) -> bool {
        self == &Self::Any
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ParamKind {
    Identifier,          // 通常
    Reference,           // 参照渡し
    Variadic,            // 可変長引数
    Dummy,
    Default(Expression), // デフォルト引数
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct FuncParam {
    /// 名前がなければ可変長引数用のダミー
    name: Option<String>,
    /// 種別
    pub kind: ParamKind,
    /// 型指定
    pub param_type: ParamType,
}

impl FuncParam {
    pub fn new(name: Option<String>, kind: ParamKind) -> Self {
        Self { name, kind, param_type: ParamType::Any }
    }
    pub fn new_with_type(name: Option<String>, kind: ParamKind, param_type: ParamType) -> Self {
        Self { name, kind, param_type }
    }
    pub fn new_dummy() -> Self {
        Self { name: None, kind: ParamKind::Dummy, param_type: ParamType::Any }
    }
    pub fn name(&self) -> String {
        match &self.name {
            Some(name) => name,
            None => "###DUMY###",
        }.to_string()
    }
    pub fn has_type(&self) -> bool {
        ! self.param_type.is_any()
    }
}

impl fmt::Display for FuncParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = if self.name.is_some() {
            self.name.as_ref().unwrap()
        } else {
            return write!(f, "");
        };
        let sep = if self.param_type.is_any() {""} else {": "};
        match self.kind {
            ParamKind::Identifier => write!(f, "{}{}{}", name, sep, self.param_type),
            ParamKind::Reference => write!(f, "var {}{}{}", name, sep, self.param_type),
            ParamKind::Variadic => write!(f, "args {}", name),
            ParamKind::Default(ref d) => write!(f, "{}{}{} = {}", name, sep, self.param_type, d),
            ParamKind::Dummy => write!(f, ""),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DefDllParamSize {
    Const(String),
    Size(usize),
    None
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DefDllParam {
    Param {
        dll_type: DllType,
        is_ref: bool,
        size: DefDllParamSize,
    },
    /// `{}`定義された構造体
    Struct(Vec<DefDllParam>),
    /// コールバック関数
    Callback(Vec<DllType>, DllType),
}
impl std::fmt::Display for DefDllParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefDllParam::Param { dll_type, is_ref, size } => {
                let r = if *is_ref {"var "} else {""};
                let s = match size {
                    DefDllParamSize::Const(c) => format!("[{c}]"),
                    DefDllParamSize::Size(n) => format!("[{n}]"),
                    DefDllParamSize::None => format!(""),
                };
                write!(f, "{r}{dll_type}{s}")
            },
            DefDllParam::Struct(v) => {
                let s = v.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{{{s}}}")
            },
            DefDllParam::Callback(argtypes, rtype) => {
                let types = argtypes.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "callback({}):{}", types, rtype)
            },
        }
    }
}

impl FromStr for DllType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = match s.to_ascii_lowercase().as_str() {
            "int" => DllType::Int,
            "long" => DllType::Long,
            "bool" => DllType::Bool,
            "uint" => DllType::Uint,
            "hwnd" => DllType::Hwnd,
            "handle" => DllType::Handle,
            "string" => DllType::String,
            "wstring" => DllType::Wstring,
            "float" => DllType::Float,
            "double" => DllType::Double,
            "word" => DllType::Word,
            "dword" => DllType::Dword,
            "byte" => DllType::Byte,
            "char" => DllType::Char,
            "pchar" => DllType::Pchar,
            "wchar" => DllType::Wchar,
            "pwchar" => DllType::PWchar,
            "boolean" => DllType::Boolean,
            "longlong" => DllType::Longlong,
            "safearray" => DllType::SafeArray,
            "void" => DllType::Void,
            "pointer" => DllType::Pointer,
            "size" => DllType::Size,
            "struct" => DllType::UStruct,
            "callback" => DllType::CallBack,
            _ => {
                return Err(s.to_string());
            },
        };
        Ok(t)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DllType {
    Int,
    Long,
    Bool,
    Uint,
    Hwnd,
    Handle,
    String,
    Wstring,
    Float,
    Double,
    Word,
    Dword,
    Byte,
    Char,
    Pchar,
    Wchar,
    PWchar,
    Boolean,
    Longlong,
    SafeArray,
    Void,
    Pointer,
    Size,
    UStruct,
    CallBack,
}


impl fmt::Display for DllType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DllType::Int => write!(f, "int"),
            DllType::Long => write!(f, "long"),
            DllType::Bool => write!(f, "bool"),
            DllType::Uint => write!(f, "uint"),
            DllType::Hwnd => write!(f, "hwnd"),
            DllType::Handle => write!(f, "handle"),
            DllType::String => write!(f, "string"),
            DllType::Wstring => write!(f, "wstring"),
            DllType::Float => write!(f, "float"),
            DllType::Double => write!(f, "double"),
            DllType::Word => write!(f, "word"),
            DllType::Dword => write!(f, "dword"),
            DllType::Byte => write!(f, "byte"),
            DllType::Char => write!(f, "char"),
            DllType::Pchar => write!(f, "pchar"),
            DllType::Wchar => write!(f, "wchar"),
            DllType::PWchar => write!(f, "pwchar"),
            DllType::Boolean => write!(f, "boolean"),
            DllType::Longlong => write!(f, "longlong"),
            DllType::SafeArray => write!(f, "safearray"),
            DllType::Void => write!(f, "void"),
            DllType::Pointer => write!(f, "pointer"),
            DllType::Size => write!(f, "size"),
            DllType::UStruct => write!(f, "struct"),
            DllType::CallBack => write!(f, "callback"),
        }
    }
}

#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub enum OptionSetting {
    Explicit(bool),
    SameStr(bool),
    OptPublic(bool),
    OptFinally(bool),
    SpecialChar(bool),
    ShortCircuit(bool),
    NoStopHotkey(bool),
    TopStopform(bool),
    FixBalloon(bool),
    Defaultfont(String),
    Position(i32, i32),
    Logpath(String),
    Loglines(i32),
    Logfile(i32),
    Dlgtitle(String),
    GuiPrint(bool),
    ForceBool(bool),
    AllowIEObj(bool),
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct HashSugar {
    pub name: Identifier,
    pub option: Option<Expression>,
    pub is_public: bool,
    pub members: Vec<(Expression, Expression)>
}

impl HashSugar {
    pub fn new(
        name: Identifier,
        option: Option<Expression>,
        is_public: bool,
        members: Vec<(Expression, Expression)>
    ) -> Self {
        Self { name, option, is_public, members }
    }
}