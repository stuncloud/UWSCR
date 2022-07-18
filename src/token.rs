use strum_macros::ToString;

#[derive(Debug, Clone, PartialEq, ToString)]
pub enum Token {
    /// 不正なトークン
    Illegal(char),
    /// 空行 (未使用)
    Blank,
    /// ファイルの末尾
    Eof,
    /// 行末, コメント開始も行末として扱う
    Eol,

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
    // callのuriサポート
    Uri(String),
    /// directory, filename
    Path(Option<String>, String),
    DefDll,

    // 演算子
    /// +
    Plus,
    /// -
    Minus,
    /// !
    Bang,
    /// *
    Asterisk,
    /// /
    Slash,

    /// and
    And,
    /// or
    Or,
    /// xor
    Xor,
    /// logical and
    AndL,
    /// logical or
    OrL,
    /// logical xor
    XorL,
    /// bit and
    AndB,
    /// bit or
    OrB,
    /// bit xor
    XorB,
    /// mod
    Mod,

    /// +=,
    AddAssign,
    /// -=,
    SubtractAssign,
    /// *=,
    MultiplyAssign,
    /// /=,
    DivideAssign,

    /// :=
    Assign,
    /// 代入または等式r
    EqualOrAssign,
    /// =, ==
    Equal,
    /// <>, !=
    NotEqual,
    /// <
    LessThan,
    /// <=
    LessThanEqual,
    /// >
    GreaterThan,
    /// >=
    GreaterThanEqual,

    /// ? 三項演算子用
    Question,

    // Delimiters
    /// ,
    Comma,
    /// .
    Period,
    /// :
    Colon,
    /// ;
    Semicolon,
    /// (
    Lparen,
    /// )
    Rparen,
    /// {
    Lbrace,
    /// }
    Rbrace,
    /// [
    Lbracket,
    /// ]
    Rbracket,
    /// _
    LineContinue,
    /// \ ファイルパス用
    BackSlash,
    /// :\ ファイルパス用
    ColonBackSlash,

    // ブロック構文
    If,
    IfB,
    Then,

    While,
    Repeat,

    For,
    To,
    In,
    Step,

    Select,

    Continue,
    Break,

    With,

    Try,

    TextBlock(bool),
    EndTextBlock,
    TextBlockBody(String),

    Function,
    Procedure,

    Module,
    Class,

    Enum,

    Struct,

    Hash, // hashtblシンタックスシュガー用

    BlockEnd(BlockEnd),

    // Option
    Option(String),

    // COM
    ComErrIgn,
    ComErrRet,
    ComErrFlg,

    // その他
    Exit,
    ExitExit,
    Comment, // // ※文末扱い

    // 引数関連
    Ref,
    /// 可変長引数
    Variadic,

    // 無名関数
    Pipeline,
    Arrow,
}

#[derive(Debug, Clone, PartialEq, ToString)]
pub enum BlockEnd {
    Else,
    ElseIf,
    EndIf,
    Case,
    Default,
    Selend,
    Wend,
    Until,
    Next,
    EndFor,
    EndWith,
    Fend,
    EndModule,
    EndClass,
    Except,
    Finally,
    EndTry,
    EndEnum,
    EndStruct,
    EndHash,
}