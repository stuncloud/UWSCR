
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Illegal,
    Blank, // 空行
    Eof,
    Eol, // 行末、コメント開始も行末として扱う

    // Identifiers + literals
    Identifier(String),
    // Int(i64),
    Num(f64),
    Hex(String),
    String(String),
    Bool(bool),
    Null,
    Empty,
    Nothing,

    // Statements
    Print,
    Dim,
    Public,
    Const,
    Thread,
    HashTable,
    Call(String),
    DefDll(String),

    // 演算子
    Plus, // +
    Minus, // -
    Bang, // !
    Asterisk, // *
    Slash, // /

    And, // and
    Or, // or
    Xor, // xor
    Mod, // mod

    // Assign, // = (代入)
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
    LineBreak, // 改行
    BackSlash, // \ ファイルパス用

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

    With,
    EndWith,

    TextBlock,
    EndTextBlock,

    Function,
    Procedure,
    Fend,

    Module,
    EndModule,
    Class,
    EndClass,

    // キーワード
    Result,
    This,
    Global,

    // その他
    Option(String),
    Comment, // // ※文末扱い
}