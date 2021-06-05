use std::fmt;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier(pub String);

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Identifier(name) = self;
        write!(f, "{}", name)
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
        params: Vec<Expression>,
        body: BlockStatement,
        is_proc: bool,
    },
    FuncCall {
        func: Box<Expression>,
        args: Vec<Expression>,
    },
    Assign(Box<Expression>, Box<Expression>),
    CompoundAssign(Box<Expression>, Box<Expression>, Infix), // += -= *= /=
    Ternary { // ?: 三項演算子
        condition: Box<Expression>,
        consequence: Box<Expression>,
        alternative: Box<Expression>,
    },
    DotCall(Box<Expression>, Box<Expression>), // hoge.fuga hoge.piyo()
    Params(Params),
    UObject(String),
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
    Dim(Vec<(Identifier, Expression)>),
    Public(Vec<(Identifier, Expression)>),
    Const(Vec<(Identifier, Expression)>),
    HashTbl(Vec<(Identifier, Option<Expression>, bool)>),
    Print(Expression),
    Call(BlockStatement, Vec<Expression>), // スクリプトの実行部分、引数(param_str)
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
        consequence: Box<Statement>,
        alternative: Box<Option<Statement>>
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
        params: Vec<Expression>,
        body: BlockStatement,
        is_proc: bool,
    },
    Exit,
    ExitExit(i32),
    Module(Identifier, BlockStatement),
    Class(Identifier, BlockStatement),
    TextBlock(Identifier, Literal),
    With(Option<Expression>, BlockStatement),
    Try {
        trys: BlockStatement,
        except: Option<BlockStatement>,
        finally: Option<BlockStatement>,
    },
    Option(OptionSetting),
    Enum(String, UEnum),
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
        if self.members.iter().find(|m| &m.0 == id).is_some() {
            Err(())
        } else {
            self.members.push((id.to_string(), value));
            Ok(())
        }
    }
}

pub type BlockStatement = Vec<Statement>;
pub type Program = BlockStatement;

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
            Params::Variadic(ref i) => write!(f, "&{}", i),
            _ => write!(f, "")
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DefDllParam {
    Param(DllType),
    Var(DllType),
    Array(DllType, Option<usize>),
    VarArray(DllType, Option<usize>),
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
    Unknown(String),
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
}