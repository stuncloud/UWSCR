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

impl Token {
    pub fn len(&self) -> usize {
        match self {
            Token::Illegal(_) => 1,
            Token::Blank => 0,
            Token::Eof => 0,
            Token::Eol => 0,
            Token::Identifier(ident) => ident.len(),
            Token::Num(n) => n.to_string().len(),
            Token::Hex(h) => h.len() + 1,
            Token::String(s) |
            Token::ExpandableString(s) => s.len(),
            Token::Bool(b) => b.to_string().len(),
            Token::Null => 4,
            Token::Empty => 5,
            Token::Nothing => 7,
            Token::UObject(_) => 2,
            Token::UObjectNotClosing => 0,
            Token::NaN => 3,
            Token::Print => 5,
            Token::Dim => 3,
            Token::Public => 6,
            Token::Const => 5,
            Token::Thread => 5,
            Token::Async => 5,
            Token::Await => 5,
            Token::HashTable => 7,
            Token::Call => 4,
            Token::Uri(uri) => uri.len(),
            Token::Path(dir, file) => {
                let len = match dir {
                    Some(s) => s.len() + 1,
                    None => 0,
                } + file.len();
                len
            },
            Token::DefDll => 7,
            Token::Plus |
            Token::Minus |
            Token::Bang |
            Token::Asterisk |
            Token::Slash => 1,
            Token::And => 3,
            Token::Or => 2,
            Token::Xor => 3,
            Token::AndL => 4,
            Token::OrL => 3,
            Token::XorL => 4,
            Token::AndB => 4,
            Token::OrB => 3,
            Token::XorB => 4,
            Token::Mod => 3,
            Token::AddAssign |
            Token::SubtractAssign |
            Token::MultiplyAssign |
            Token::DivideAssign => 2,
            Token::Assign => 2,
            Token::EqualOrAssign => 1,
            Token::Equal => 2,
            Token::NotEqual => 2,
            Token::LessThan => 1,
            Token::LessThanEqual => 2,
            Token::GreaterThan => 1,
            Token::GreaterThanEqual => 2,
            Token::Question |
            Token::Comma |
            Token::Period |
            Token::Colon |
            Token::Semicolon |
            Token::Lparen |
            Token::Rparen |
            Token::Lbrace |
            Token::Rbrace |
            Token::Lbracket |
            Token::Rbracket |
            Token::LineContinue |
            Token::BackSlash => 1,
            Token::ColonBackSlash => 2,
            Token::If => 2,
            Token::IfB => 3,
            Token::Then => 4,
            Token::While => 5,
            Token::Repeat => 6,
            Token::For => 3,
            Token::To => 2,
            Token::In => 2,
            Token::Step => 4,
            Token::Select => 6,
            Token::Continue => 8,
            Token::Break => 5,
            Token::With => 4,
            Token::Try => 3,
            Token::TextBlock(_) => 9,
            Token::EndTextBlock => 12,
            Token::TextBlockBody(_) => 1,
            Token::Function => 8,
            Token::Procedure => 9,
            Token::Module => 6,
            Token::Class => 5,
            Token::Enum => 4,
            Token::Struct => 6,
            Token::Hash => 4,
            Token::BlockEnd(end) => end.len(),
            Token::Option(_) => 6,
            Token::ComErrIgn => 11,
            Token::ComErrRet => 11,
            Token::ComErrFlg => 11,
            Token::Exit => 4,
            Token::ExitExit => 8,
            Token::Comment => 2,
            Token::Ref => 3,
            Token::Variadic => 4,
            Token::Pipeline => 1,
            Token::Arrow => 2,
        }
    }
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
impl BlockEnd {
    fn len(&self) -> usize {
        match self {
            BlockEnd::Else => 4,
            BlockEnd::ElseIf => 6,
            BlockEnd::EndIf => 5,
            BlockEnd::Case => 4,
            BlockEnd::Default => 7,
            BlockEnd::Selend => 6,
            BlockEnd::Wend => 4,
            BlockEnd::Until => 5,
            BlockEnd::Next => 4,
            BlockEnd::EndFor => 6,
            BlockEnd::EndWith => 7,
            BlockEnd::Fend => 4,
            BlockEnd::EndModule => 9,
            BlockEnd::EndClass => 8,
            BlockEnd::Except => 6,
            BlockEnd::Finally => 7,
            BlockEnd::EndTry => 6,
            BlockEnd::EndEnum => 7,
            BlockEnd::EndStruct => 9,
            BlockEnd::EndHash => 7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, BlockEnd};

    #[test]
    fn hoge() {
        assert_eq!(Token::BlockEnd(BlockEnd::Else).to_string(), "Else".to_string());
    }
}