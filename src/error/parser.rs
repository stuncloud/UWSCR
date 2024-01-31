use crate::write_locale;
use super::{CURRENT_LOCALE, Locale};
use crate::lexer::Position;
use crate::token::Token;
use crate::ast::{Identifier, Statement, Expression};

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum InternalError {
    /// moduleブロックの解析でStatementが返る異常
    ModuleBlockParsing,
}
impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::ModuleBlockParsing => write!(f, "モジュールブロック解析異常"),
        }
    }
}
impl From<InternalError> for ParseErrorKind {
    fn from(err: InternalError) -> Self {
        ParseErrorKind::InternalError(err)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    // /// 内部エラー (発生してはいけないやつ)
    InternalError(InternalError),
    SyntaxError,
    /// 次のトークンが期待されたものではない
    ///
    /// expected, next
    NextTokenIsUnexpected(Token, Token),
    /// 現トークンが期待されたものではない
    ///
    /// expected, next
    CurrentTokenIsUnexpected(Token, Token),
    /// ブロックの閉じトークンが期待されたものではない
    ///
    /// expected, next
    BlockClosingTokenIsUnexpected(Token, Token),
    /// トークンが識別子ではない
    CurrentTokenIsNotIdentifier,
    /// 現在のトークンが期待されたものではない
    CurrentTokenIsInvalid(Token),
    /// 次のトークンが期待されたものではない
    NextTokenIsInvalid(Token),
    /// 現在のトークンが期待されるトークンのいずれでもない
    ///
    /// expected
    TokenIsNotOneOfExpectedTokens(Vec<Token>),
    /// 式が必要な箇所に存在しない
    ExpressionIsExpected,
    /// Identifierに変換できないトークン
    TokenCanNotBeUsedAsIdentifier,
    /// トークンとして使えない不正な文字列
    IllegalCharacter(char),
    /// 識別子が必要
    IdentifierExpected,
    /// ブロック終端ではない
    BlockClosingTokenExpected,

    UnexpectedOption(String),
    InvalidExitCode,
    ValueMustBeDefined(Identifier),
    ParameterShouldBeDefault(String),
    ParameterCannotBeDefinedAfterVariadic(String),
    OutOfWith,
    OutOfLoop(Token),
    InvalidStatementInFinallyBlock(String),
    ClassHasNoConstructor(Identifier),
    InvalidDllType(String),
    DllPathNotFound,
    InvalidHexNumber(String),
    CanNotCallScript(String, String),
    /// uwslファイルの読み込みに失敗
    CanNotLoadUwsl(String, String),
    WhitespaceRequiredAfter(String),
    SizeRequired,
    EnumMemberShouldBeNumber(String, String),
    EnumMemberDuplicated(String, String),
    EnumValueShouldBeDefined(String, String),
    EnumValueIsInvalid(String, String, f64),
    InvalidThreadCall,
    TextBlockBodyIsMissing,
    InvalidUObjectEnd,
    MissingIdentifierAfterVar,
    ReservedKeyword(Token),
    FunctionRequiredAfterAsync,
    FunctionCallRequiredAfterAwait,
    InvalidMemberDefinition(Statement, bool),
    MissingIndex,
    /// 連想配列定義が不正
    InvalidHashMemberDefinition(Option<Expression>),
    InvalidCallUri(String),
    ExplicitError(String),
    DefinitionStatementNotAllowed,
    OptionStatementNotAllowed,
    IdentifierIsAlreadyDefined(String),
    AssigningToConstIsNotAllowed,
    CalledScriptHadError,
    InvalidAssignment,
    ParameterDuplicated(String),
    InvalidExpression,
    UndeclaredIdentifier(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub start: Position,
    pub end: Position,
    pub script_name: Option<String>
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseErrorKind::InternalError(internal) => write!(f,
                "Internal Error: {internal}"
            ),
            ParseErrorKind::SyntaxError => write!(f,
                "Syntax Error"
            ),
            ParseErrorKind::NextTokenIsUnexpected(expected, next) => write_locale!(f,
                "不正なトークン({next}); 期待されるトークンは{expected}",
                "Expected token was {expected}, but got {next}",
            ),
            ParseErrorKind::CurrentTokenIsUnexpected(expected, next) => write_locale!(f,
                "不正なトークン({next}); 期待されるトークンは{expected}",
                "Expected token was {expected}, but got {next}",
            ),
            ParseErrorKind::BlockClosingTokenIsUnexpected(expected, next) => write_locale!(f,
                "不正なブロック終端({next}); {expected}が必要です",
                "Expected token was {expected}, but got {next} for the end of block",
            ),
            ParseErrorKind::CurrentTokenIsNotIdentifier => write_locale!(f,
                "識別子ではありません",
                "This token is not an Identifier",
            ),
            ParseErrorKind::CurrentTokenIsInvalid(token) => write_locale!(f,
                "不正なトークンです: {token}",
                "Invalid token: {token}",
            ),
            ParseErrorKind::NextTokenIsInvalid(token) => write_locale!(f,
                "不正なトークンです: {token}",
                "Invalid token: {token}",
            ),
            ParseErrorKind::TokenIsNotOneOfExpectedTokens(expected) => write_locale!(f,
                "いずれかのトークンが必要です: {}",
                "One of these tokens is required: {}",
                expected.iter().map(|t| t.to_string()).reduce(|a, b| a + ", " + &b).unwrap_or_default()
            ),
            ParseErrorKind::ExpressionIsExpected => write_locale!(f,
                "式が必要です",
                "expression is required",
            ),
            ParseErrorKind::TokenCanNotBeUsedAsIdentifier => write_locale!(f,
                "識別子ではありません",
                "Token is not an Identifier",
            ),
            ParseErrorKind::IllegalCharacter(c) => write_locale!(f,
                "不正な文字: {}",
                "Invalid character: {}",
                c.escape_unicode()
            ),
            ParseErrorKind::IdentifierExpected => write_locale!(f,
                "識別子が必要です",
                "Identifier is Expected",
            ),
            ParseErrorKind::BlockClosingTokenExpected => write_locale!(f,
                "ブロック終端がありません",
                "Block is not closing correctly",
            ),

            ParseErrorKind::UnexpectedOption(name) => write_locale!(f,
                "不正なオプション名: {}",
                "Invalid option name: {}",
                name
            ),
            ParseErrorKind::InvalidExitCode => write_locale!(f,
                "終了コードが数値ではありません",
                "Exit code should be a number",
            ),
            ParseErrorKind::ValueMustBeDefined(name) => write_locale!(f,
                "値が必要です({})",
                "Value must be defined: {}",
                name.to_string()
            ),
            ParseErrorKind::ParameterShouldBeDefault(name) => write_locale!(f,
                "不正なパラメータ({}): デフォルト引数の後にデフォルト引数以外は定義できません",
                "Bad parameter ({}): Parameter should have default value",
                name
            ),
            ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(name) => write_locale!(f,
                "不正なパラメータ({}): 可変長引数の後に引数は定義できません",
                "Bad parameter ({}): You can not define parameter after variadic parameter",
                name
            ),
            ParseErrorKind::OutOfWith => write_locale!(f,
                "Withブロック外で . の左辺を省略できません",
                "You cannot omit the left side of a period outside a With block",
            ),
            ParseErrorKind::OutOfLoop(token) => write_locale!(f,
                "ループ外で{}は使用できません",
                "You can not use {} outside of the loop",
                token.to_string()
            ),
            ParseErrorKind::InvalidStatementInFinallyBlock(name) => write_locale!(f,
                "Finally部では{}を使用できません",
                "You can not use {} in finally block",
                name
            ),
            ParseErrorKind::ClassHasNoConstructor(name) => write_locale!(f,
                "コンストラクタ({}())が未定義です",
                "Constructor required: {}()",
                name
            ),
            // ParseErrorKind::InvalidJson => write_locale!(f,
            //     "",
            //     "Invalid json format"
            // ),
            // ParseErrorKind::InvalidFilePath => write_locale!(f,
            //     "",
            //     "Invalid file path"
            // ),
            ParseErrorKind::InvalidDllType(name) => write_locale!(f,
                "不正なDLL型({})",
                "Invalid dll type: {}",
                name
            ),
            ParseErrorKind::DllPathNotFound => write_locale!(f,
                "DLLのパスがありません",
                "Dll path not found"
            ),
            ParseErrorKind::InvalidHexNumber(s) => write_locale!(f,
                "${}は16進数ではありません",
                "${} is not a hex number",
                s
            ),
            ParseErrorKind::CanNotCallScript(path, err) => write_locale!(f,
                "callできません ({} [{}])",
                "Failed to load script: {} [{}]",
                path, err
            ),
            ParseErrorKind::CanNotLoadUwsl(path, err) => write_locale!(f,
                "uwsl読み込み失敗 ({} [{}])",
                "Failed to load uwsl file: {} [{}]",
                path, err
            ),
            ParseErrorKind::WhitespaceRequiredAfter(name) => write_locale!(f,
                "'{}'の後にはスペースが必要です",
                "Missing whitespace after '{}'",
                name
            ),
            ParseErrorKind::SizeRequired => write_locale!(f,
                "多次元配列は添字を指定する必要があります",
                "Size is required for multidimensional array",
            ),
            ParseErrorKind::EnumMemberShouldBeNumber(name, member) => write_locale!(f,
                "数値以外の値が指定されています ({}.{})",
                "Enum value should be a number literal: {}.{}",
                name, member
            ),
            ParseErrorKind::EnumMemberDuplicated(name, member) => write_locale!(f,
                "名前または値が重複しています ({}.{})",
                "Name or value for {}.{} is duplicated",
                name, member
            ),
            ParseErrorKind::EnumValueShouldBeDefined(name, member) => write_locale!(f,
                "値が未指定です ({}.{})",
                "Enum value is not defined: {}.{}",
                name, member
            ),
            ParseErrorKind::EnumValueIsInvalid(name, member, value) => write_locale!(f,
                "{}.{}の値は{}より大きくなくてはいけません",
                "Value for {}.{} must be greater then {}",
                name, member, value
            ),
            ParseErrorKind::InvalidThreadCall => write_locale!(f,
                "Threadで関数以外を呼び出すことは出来ません",
                "You must call a function to run a thread"
            ),
            ParseErrorKind::TextBlockBodyIsMissing => write_locale!(f,
                "Textblockが空です",
                "Thread syntax error"
            ),
            ParseErrorKind::InvalidUObjectEnd => write_locale!(f,
                "UObjectが@で閉じられていません",
                "Literal UObject should be closed by @"
            ),
            ParseErrorKind::MissingIdentifierAfterVar => write_locale!(f,
                "var/ref キーワードの後には変数が必要です",
                "Identifier is required after var or ref keyword"
            ),
            ParseErrorKind::ReservedKeyword(token) => write_locale!(f,
                "{}は予約されています",
                "{} is reserved",
                token.to_string()
            ),
            ParseErrorKind::FunctionRequiredAfterAsync => write_locale!(f,
                "asyncの後には関数定義(procedure/function)が必要です",
                "Function or procedure definition is required after async keyword",
            ),
            ParseErrorKind::FunctionCallRequiredAfterAwait => write_locale!(f,
                "awaitの後には関数呼び出しが必要です",
                "Function call is required after await keyword",
            ),
            ParseErrorKind::InvalidMemberDefinition(statement, is_class) => write_locale!(f,
                "不正な{}メンバ定義 ({statement})",
                "Invalid {} member definition: {statement}",
                if *is_class {"class"} else {"module"}
            ),
            ParseErrorKind::MissingIndex => write_locale!(f,
                "配列の添字がない",
                "Index is missing",
            ),
            ParseErrorKind::InvalidHashMemberDefinition(e) => match e {
                Some(e) => write_locale!(f,
                    "不正な連想配列メンバ定義: {e}",
                    "Invalid hashtbl member definition: {e}",
                ),
                None => write_locale!(f,
                    "不正な連想配列メンバ定義: 式が未指定",
                    "Invalid hashtbl member definition: expression is required",
                ),
            },
            ParseErrorKind::InvalidCallUri(uri) => write_locale!(f,
                "不正なスクリプト ({uri})",
                "Invalid script uri: {uri}",
            ),
            ParseErrorKind::ExplicitError(ident) => write_locale!(f,
                "未宣言の変数 {ident} への代入は禁止されています (OPTION EXPLICIT)",
                "Assigment to undeclared variable '{ident}' is prohibited by OPTION EXPLICIT",
            ),
            ParseErrorKind::DefinitionStatementNotAllowed => write_locale!(f,
                "ブロック構文内では定義できません",
                "Definition is not allowed in block statement",
            ),
            ParseErrorKind::OptionStatementNotAllowed => write_locale!(f,
                "ブロック構文内ではOPTION宣言できません",
                "OPTION declarement is not allowed in block statement",
            ),
            ParseErrorKind::IdentifierIsAlreadyDefined(name) => write_locale!(f,
                "{name}は定義済みです",
                "{name} is already defined",
            ),
            ParseErrorKind::AssigningToConstIsNotAllowed => write_locale!(f,
                "定数への代入はできません",
                "Assigning to constant is not allowed",
            ),
            ParseErrorKind::CalledScriptHadError => write_locale!(f,
                "call文で呼び出したスクリプトにエラーがありました",
                "Assigning to constant is not allowed",
            ),
            ParseErrorKind::InvalidAssignment => write_locale!(f,
                "代入式の左辺が不正です",
                "The left side of assignment expression is invalid",
            ),
            ParseErrorKind::ParameterDuplicated(name) => write_locale!(f,
                "同じ名前の引数があります ({name})",
                "Parameter name is duplicated: {name}",
            ),
            ParseErrorKind::InvalidExpression => write_locale!(f,
                "不正な式の呼び出しです",
                "Invalid expression",
            ),
            ParseErrorKind::UndeclaredIdentifier(name) => write_locale!(f,
                "変数または定数 {name} がありません",
                "There is no variable or constant named {name}",
            ),
        }
    }
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, start: Position, end: Position, script_name: Option<String>) -> Self {
        ParseError {kind, start, end, script_name}
    }
    pub fn new_explicit_error(ident: String, row: usize, script_name: Option<String>) -> Self {
        let len = ident.len();
        let kind = ParseErrorKind::ExplicitError(ident);
        let start = Position { row, column: 0 };
        let end = Position { row: row, column: len };
        Self { kind, start, end, script_name }
    }

    pub fn get_kind(self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.script_name {
            write!(f, "{}[{}] - {}", name, self.start, self.kind)
        } else {
            write!(f, "[{}] - {}", self.start, self.kind)
        }
    }
}
