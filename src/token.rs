
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Illegal(char),
    Blank, // 空行
    Eof,
    Eol, // 行末, コメント開始も行末として扱う

    // Identifiers + literals
    Identifier(String),
    // Int(i64),
    Num(f64),
    Hex(String),
    String(String),
    ExpandableString(String),
    Bool(bool),
    Null,
    Empty,
    Nothing,
    UObject(String),
    UObjectNotClosing,
    NaN,

    // Statements
    Print,
    Dim,
    Public,
    Const,
    Thread,
    Async,
    Await,
    HashTable,
    Call,
    Path(Option<String>, String), // directory, filename
    DefDll,

    // 演算子
    Plus, // +
    Minus, // -
    Bang, // !
    Asterisk, // *
    Slash, // /

    And, // and
    Or, // or
    Xor, // xor
    AndL, // logical and
    OrL, // logical or
    XorL, // logical xor
    AndB, // bit and
    OrB, // bit or
    XorB, // bit xor
    Mod, // mod

    AddAssign, // +=,
    SubtractAssign, // -=,
    MultiplyAssign, // *=,
    DivideAssign, // /=,

    Assign, // :=
    EqualOrAssign, // 代入または等式r
    Equal, // =, ==
    NotEqual, // <>, !=
    LessThan, // <
    LessThanEqual, // <=
    GreaterThan, // >
    GreaterThanEqual, // >=

    Question, // ? 三項演算子用

    // Delimiters
    Comma, // ,
    Period, // .
    Colon, // :
    Semicolon, // ;
    Lparen, // (
    Rparen, // )
    Lbrace, // {
    Rbrace, // }
    Lbracket, // [
    Rbracket, // ]
    LineContinue, // _
    BackSlash, // \ ファイルパス用
    ColonBackSlash, // :\ ファイルパス用

    // ブロック構文
    If,
    IfB,
    Then,
    Else,
    ElseIf,
    EndIf,

    While,
    Wend,

    Repeat,
    Until,

    For,
    To,
    In,
    Step,
    Next,

    Select,
    Case,
    Default,
    Selend,

    Continue,
    Break,

    With,
    EndWith,

    Try,
    Except,
    Finally,
    EndTry,

    TextBlock(bool),
    EndTextBlock,
    TextBlockBody(String),

    Function,
    Procedure,
    Fend,

    Exit,
    ExitExit,

    Module,
    EndModule,
    Class,
    EndClass,

    Enum,
    EndEnum,

    // Option
    Option,
    Explicit,
    SameStr,
    OptPublic,
    OptFinally,
    SpecialChar,
    ShortCircuit,
    NoStopHotkey,
    TopStopform,
    FixBalloon,
    Defaultfont,
    Position,
    Logpath,
    Loglines,
    Logfile,
    Dlgtitle,

    // その他
    Comment, // // ※文末扱い

    // 引数関連
    Ref,
    Variadic, // 可変長引数

    // 無名関数
    Pipeline,
    Arrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionSetting {
}
