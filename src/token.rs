// use std::string::ToString;
use strum_macros::Display;

#[derive(Debug, Clone, PartialEq)]
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

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Illegal(char) => write!(f, "Illegal({char})"),
            Token::Blank => write!(f, "Blank"),
            Token::Eof => write!(f, "Eof"),
            Token::Eol => write!(f, "Eol"),
            Token::Identifier(_) => write!(f, "Identifier"),
            Token::Num(_) => write!(f, "Num"),
            Token::Hex(_) => write!(f, "Hex"),
            Token::String(_) => write!(f, "String"),
            Token::ExpandableString(_) => write!(f, "ExpandableString"),
            Token::Bool(_) => write!(f, "Bool"),
            Token::Null => write!(f, "Null"),
            Token::Empty => write!(f, "Empty"),
            Token::Nothing => write!(f, "Nothing"),
            Token::UObject(_) => write!(f, "UObject"),
            Token::UObjectNotClosing => write!(f, "UObjectNotClosing"),
            Token::NaN => write!(f, "NaN"),
            Token::Print => write!(f, "Print"),
            Token::Dim => write!(f, "Dim"),
            Token::Public => write!(f, "Public"),
            Token::Const => write!(f, "Const"),
            Token::Thread => write!(f, "Thread"),
            Token::Async => write!(f, "Async"),
            Token::Await => write!(f, "Await"),
            Token::HashTable => write!(f, "HashTable"),
            Token::Call => write!(f, "Call"),
            Token::Uri(_) => write!(f, "Uri"),
            Token::Path(_, _) => write!(f, "Path"),
            Token::DefDll => write!(f, "DefDll"),
            Token::Plus => write!(f, "Plus"),
            Token::Minus => write!(f, "Minus"),
            Token::Bang => write!(f, "Bang"),
            Token::Asterisk => write!(f, "Asterisk"),
            Token::Slash => write!(f, "Slash"),
            Token::And => write!(f, "And"),
            Token::Or => write!(f, "Or"),
            Token::Xor => write!(f, "Xor"),
            Token::AndL => write!(f, "AndL"),
            Token::OrL => write!(f, "OrL"),
            Token::XorL => write!(f, "XorL"),
            Token::AndB => write!(f, "AndB"),
            Token::OrB => write!(f, "OrB"),
            Token::XorB => write!(f, "XorB"),
            Token::Mod => write!(f, "Mod"),
            Token::AddAssign => write!(f, "AddAssign"),
            Token::SubtractAssign => write!(f, "SubtractAssign"),
            Token::MultiplyAssign => write!(f, "MultiplyAssign"),
            Token::DivideAssign => write!(f, "DivideAssign"),
            Token::Assign => write!(f, "Assign"),
            Token::EqualOrAssign => write!(f, "EqualOrAssign"),
            Token::Equal => write!(f, "Equal"),
            Token::NotEqual => write!(f, "NotEqual"),
            Token::LessThan => write!(f, "LessThan"),
            Token::LessThanEqual => write!(f, "LessThanEqual"),
            Token::GreaterThan => write!(f, "GreaterThan"),
            Token::GreaterThanEqual => write!(f, "GreaterThanEqual"),
            Token::Question => write!(f, "Question"),
            Token::Comma => write!(f, "Comma"),
            Token::Period => write!(f, "Period"),
            Token::Colon => write!(f, "Colon"),
            Token::Semicolon => write!(f, "Semicolon"),
            Token::Lparen => write!(f, "Lparen"),
            Token::Rparen => write!(f, "Rparen"),
            Token::Lbrace => write!(f, "Lbrace"),
            Token::Rbrace => write!(f, "Rbrace"),
            Token::Lbracket => write!(f, "Lbracket"),
            Token::Rbracket => write!(f, "Rbracket"),
            Token::LineContinue => write!(f, "LineContinue"),
            Token::BackSlash => write!(f, "BackSlash"),
            Token::ColonBackSlash => write!(f, "ColonBackSlash"),
            Token::If => write!(f, "If"),
            Token::IfB => write!(f, "IfB"),
            Token::Then => write!(f, "Then"),
            Token::While => write!(f, "While"),
            Token::Repeat => write!(f, "Repeat"),
            Token::For => write!(f, "For"),
            Token::To => write!(f, "To"),
            Token::In => write!(f, "In"),
            Token::Step => write!(f, "Step"),
            Token::Select => write!(f, "Select"),
            Token::Continue => write!(f, "Continue"),
            Token::Break => write!(f, "Break"),
            Token::With => write!(f, "With"),
            Token::Try => write!(f, "Try"),
            Token::TextBlock(_) => write!(f, "TextBlock"),
            Token::EndTextBlock => write!(f, "EndTextBlock"),
            Token::TextBlockBody(_) => write!(f, "TextBlockBody"),
            Token::Function => write!(f, "Function"),
            Token::Procedure => write!(f, "Procedure"),
            Token::Module => write!(f, "Module"),
            Token::Class => write!(f, "Class"),
            Token::Enum => write!(f, "Enum"),
            Token::Struct => write!(f, "Struct"),
            Token::Hash => write!(f, "Hash"),
            Token::BlockEnd(end) => write!(f, "{end}"),
            Token::Option(_) => write!(f, "Option"),
            Token::ComErrIgn => write!(f, "ComErrIgn"),
            Token::ComErrRet => write!(f, "ComErrRet"),
            Token::ComErrFlg => write!(f, "ComErrFlg"),
            Token::Exit => write!(f, "Exit"),
            Token::ExitExit => write!(f, "ExitExit"),
            Token::Comment => write!(f, "Comment"),
            Token::Ref => write!(f, "Ref"),
            Token::Variadic => write!(f, "Variadic"),
            Token::Pipeline => write!(f, "Pipeline"),
            Token::Arrow => write!(f, "Arrow"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display)]
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

#[cfg(test)]
mod tests {
    use super::{Token, BlockEnd};

    #[test]
    fn hoge() {
        assert_eq!(Token::BlockEnd(BlockEnd::Else).to_string(), "Else".to_string());
    }
}