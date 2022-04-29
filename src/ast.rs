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
    Plus,
    Minus,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    GreaterThanEqual,
    GreaterThan,
    LessThanEqual,
    LessThan,
    And,
    Or,
    Xor,
    AndL, // logical and
    OrL, // logical or
    XorL, // logical xor
    AndB, // bit and
    OrB, // bit or
    XorB, // bit xor
    Mod,
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
    Identifier(Identifier),
    Array(Vec<Expression>, Vec<Expression>), // 配列、配列宣言時の添字リスト(多次元定義時のそれぞれの添字)
    Literal(Literal),
    Prefix(Prefix, Box<Expression>),
    Infix(Infix, Box<Expression>, Box<Expression>),
    Index(Box<Expression>, Box<Expression>, Box<Option<Expression>>), // optionはhashtblの2つ目の添字
    AnonymusFunction {
        params: Vec<FuncParam>,
        body: BlockStatement,
        is_proc: bool,
    },
    FuncCall {
        func: Box<Expression>,
        args: Vec<Expression>,
        is_await: bool,
    },
    Assign(Box<Expression>, Box<Expression>),
    CompoundAssign(Box<Expression>, Box<Expression>, Infix), // += -= *= /=
    Ternary { // ?: 三項演算子
        condition: Box<Expression>,
        consequence: Box<Expression>,
        alternative: Box<Expression>,
    },
    DotCall(Box<Expression>, Box<Expression>), // hoge.fuga hoge.piyo()
    UObject(String),
    ComErrFlg,
    VarArgument(Box<Expression>),
    EmptyArgument,
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
            // Expression::Params(p) => write!(f, "{}", p),
            Expression::UObject(o) => write!(f, "{}", o),
            Expression::ComErrFlg => write!(f, ""),
            Expression::VarArgument(a) => write!(f, "var {}", a),
            Expression::EmptyArgument => write!(f, ""),
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
    Call(Program, Vec<Expression>), // スクリプトの実行部分、引数(param_str)
    DefDll {
        name: String,
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
        block: BlockStatement
    },
    ForIn {
        loopvar: Identifier,
        collection: Expression,
        block: BlockStatement
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
    Struct(Identifier, Vec<(String, DllType)>),
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
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct StatementWithRow {
    pub statement: Statement,
    pub row: usize,
}

impl StatementWithRow {
    pub fn new(statement: Statement, row: usize) -> Self {
        Self {statement, row}
    }
    // 存在しない行
    pub fn new_non_existent_line(statement: Statement) -> Self {
        Self {
            statement,
            row: 0,
        }
    }
}

pub type BlockStatement = Vec<StatementWithRow>;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Program(pub BlockStatement, pub Vec<String>); // Vec<String>は行情報

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Precedence {
    Lowest,
    Assign,         // :=
    Ternary,        // ?:
    Or,             // or xor
    And,            // and
    Equality,       // == != <>
    Relational,     // > < >= <=
    Additive,       // + -
    Multiplicative, // * / mod
    Prefix,         // X or !X
    FuncCall,       // myfunc(x)
    Index,          // array[index]
    DotCall,        // hoge.fuga
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
    Array(bool),         // 配列引数, trueで参照渡し
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
            ParamKind::Array(b) => if b {
                write!(f, "var {}[]", name)
            } else {
                write!(f, "{}[]", name)
            },
            ParamKind::Dummy => write!(f, ""),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DefDllParam {
    Param {
        dll_type: DllType,
        is_var: bool,
        is_array: bool,
    },
    Struct(Vec<DefDllParam>),
}


impl FromStr for DllType {
    type Err = std::string::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = match s.to_ascii_lowercase().as_str() {
            "int" => DllType::Int,
            "long" => DllType::Long,
            "bool" => DllType::Bool,
            "uint" => DllType::Uint,
            "hwnd" => DllType::Hwnd,
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
            "struct" => DllType::Struct,
            "callback" => DllType::CallBack,
            unknown => DllType::Unknown(unknown.to_string()),
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
    Struct,
    CallBack,
    Unknown(String),
}

impl DllType {
    pub fn size(&self) -> usize {
        match self {
            DllType::Int |
            DllType::Long |
            DllType::Bool => mem::size_of::<i32>(),
            DllType::Uint |
            DllType::Dword => mem::size_of::<u32>(),
            DllType::Hwnd => mem::size_of::<isize>(),
            DllType::Float => mem::size_of::<f32>(),
            DllType::Double => mem::size_of::<f64>(),
            DllType::Word |
            DllType::Wchar => mem::size_of::<u16>(),
            DllType::Byte |
            DllType::Boolean |
            DllType::Char => mem::size_of::<u8>(),
            DllType::Longlong => mem::size_of::<i64>(),
            DllType::String |
            DllType::Wstring |
            DllType::Pchar |
            DllType::PWchar |
            DllType::Pointer |
            DllType::Struct |
            DllType::CallBack |
            DllType::Unknown(_) => mem::size_of::<usize>(),
            DllType::SafeArray |
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
            DllType::Struct => write!(f, "struct"),
            DllType::CallBack => write!(f, "callback"),
            DllType::Unknown(ref s) => write!(f, "Unknown({})", s),
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