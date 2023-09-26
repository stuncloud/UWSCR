use std::fmt;
use std::str::FromStr;
use std::mem;

use serde::{Serialize, Deserialize};

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
    CompoundAssign(Box<Expression>, Box<Expression>, Infix), // += -= *= /=
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
    DotCall(Box<Expression>, Box<Expression>), // hoge.fuga hoge.piyo()
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
    Dim(Vec<(Identifier, Expression)>),
    Public(Vec<(Identifier, Expression)>),
    Const(Vec<(Identifier, Expression)>),
    HashTbl(Vec<(Identifier, Option<Expression>, bool)>),
    Hash(HashSugar),
    Print(Expression),
    /// スクリプトの実行部分, 引数(param_str)
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
    Repeat(Expression, BlockStatement),
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
        alternatives: Vec<(Option<Expression>, BlockStatement)>
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

pub struct ProgramBuilder {
    /// 定数
    consts: BlockStatement,
    /// パブリック変数
    publics: BlockStatement,
    /// OPTION定義
    options: BlockStatement,
    /// 関数等の定義
    /// - function/procedure
    /// - module/class
    /// - struct
    /// - def_dll
    definitions: BlockStatement,
    /// 実行される部分
    script: BlockStatement,
}
impl ProgramBuilder {
    pub fn new() -> Self {
        Self { consts: vec![], publics: vec![], options: vec![], definitions: vec![], script: vec![] }
    }
    pub fn build(mut self, lines: Vec<String>) -> Program {
        let mut global = vec![];
        global.append(&mut self.consts);
        global.append(&mut self.publics);
        global.append(&mut self.options);
        global.append(&mut self.definitions);
        let script = self.script;
        Program { global, script, lines }
    }
    pub fn push_const(&mut self, statement: StatementWithRow) {
        self.consts.push(statement)
    }
    pub fn push_public(&mut self, statement: StatementWithRow) {
        self.publics.push(statement)
    }
    pub fn push_option(&mut self, statement: StatementWithRow) {
        self.options.push(statement)
    }
    pub fn push_def(&mut self, statement: StatementWithRow) {
        self.definitions.push(statement)
    }
    pub fn push_script(&mut self, statement: StatementWithRow) {
        self.script.push(statement)
    }
    /// callのProgramのグローバル定義を加える
    pub fn set_call_program(&mut self, program: Program) -> Program {
        for s in program.global {
            match &s.statement {
                Statement::Option(_) => {
                    self.push_option(s);
                },
                Statement::Const(_) |
                Statement::TextBlock(_, _) => {
                    self.push_const(s);
                },
                Statement::Public(_) => {
                    self.push_public(s);
                },
                Statement::Function { name:_, params:_, body:_, is_proc:_, is_async:_ } |
                Statement::Module(_, _) |
                Statement::Class(_, _) |
                Statement::Struct(_, _) |
                Statement::DefDll { name:_, alias:_, params:_, ret_type:_, path:_ } => {
                    self.push_def(s);
                },
                _ => {}
            }
        }
        Program { global: vec![], script: program.script, lines: program.lines }
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
    Any, // どの型でも良い、オプション指定しなかった場合常にこれ
    String, // Object::String
    Number, // Object::Num
    Bool,
    Array, // Object::Array
    HashTbl, // Object::HashTbl
    Function, // Object::AnonFunc | Object::Function | Object::BuiltinFunction
    UObject, // Object::UObject | Object::UChild
    UserDefinition(String), // Object::Instance
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
    name: Option<String>,          // 名前がなければ可変長引数用のダミー
    pub kind: ParamKind,               // 種別
    pub param_type: ParamType, // 型指定
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
        self.name.clone().unwrap_or_default()
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

impl DllType {
    pub fn size(&self) -> usize {
        match self {
            DllType::Int |
            DllType::Long |
            DllType::Bool => mem::size_of::<i32>(),
            DllType::Uint |
            DllType::Dword => mem::size_of::<u32>(),
            DllType::Float => mem::size_of::<f32>(),
            DllType::Double => mem::size_of::<f64>(),
            DllType::Word |
            DllType::Wchar => mem::size_of::<u16>(),
            DllType::Byte |
            DllType::Boolean |
            DllType::Char => mem::size_of::<u8>(),
            DllType::Longlong => mem::size_of::<i64>(),
            DllType::Hwnd |
            DllType::Handle |
            DllType::String |
            DllType::Wstring |
            DllType::Pchar |
            DllType::PWchar |
            DllType::Pointer |
            DllType::Size |
            DllType::UStruct |
            DllType::CallBack |
            DllType::SafeArray => mem::size_of::<usize>(),
            DllType::Void => 0,
        }
    }
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