use std::error;
use std::fmt;

use crate::ast::{Expression, Infix, DllType};
use crate::evaluator::object::{Object, ObjectType};
pub use crate::write_locale;
pub use super::{CURRENT_LOCALE, Locale};
use crate::gui::UWindowError;
use crate::evaluator::object::fopen::FopenError;
use crate::evaluator::builtins::system_controls::POFF;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct UError {
    pub kind: UErrorKind,
    pub message: UErrorMessage,
    pub is_com_error: bool,
    pub line: UErrorLine,
}

impl UError {
    pub fn new(kind: UErrorKind, message: UErrorMessage) -> Self {
        Self {
            kind, message, is_com_error: false,
            line: UErrorLine::default()
        }
    }
    pub fn new_com_error(kind: UErrorKind, message: UErrorMessage) -> Self {
        Self {
            kind, message, is_com_error: true,
            line: UErrorLine::default()
        }
    }
    pub fn set_line(&mut self, row: usize, line: String, script_name: Option<String>) {
        let line = line.trim_start_matches([' ', '\t', '　']);
        self.line = UErrorLine::new(row, line.into(), script_name)
    }
    pub fn get_line(&self) -> UErrorLine {
        self.line.clone()
    }
    pub fn exitexit(n: i32) -> Self {
        Self {
            kind: UErrorKind::ExitExit(n),
            message: UErrorMessage::None,
            is_com_error: false,
            line: UErrorLine::None,
        }
    }
}

impl Default for UError {
    fn default() -> Self {
        Self {
            kind: UErrorKind::UnknownError,
            message: UErrorMessage::Unknown,
            is_com_error: false,
            line: UErrorLine::default()
        }
    }
}

impl error::Error for UError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind {
            _ => None
        }
    }
}

impl fmt::Display for UError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message {
            UErrorMessage::None => write!(f, "{}", self.kind),
            _ => write!(f,
                "[{}] {}",
                self.kind,
                self.message
            )
        }
    }
}

impl PartialEq for UError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind &&
        self.message == other.message
    }
}

#[derive(Debug, Clone)]
pub enum UErrorLine {
    None,
    Line {
        row: usize,
        line: String,
        script_name: Option<String>
    },
}

impl UErrorLine {
    pub fn new(row: usize, line: String, script_name: Option<String>) -> Self {
        Self::Line {row, line, script_name}
    }
    pub fn has_row(&self) -> bool {
        match self {
            UErrorLine::None => false,
            UErrorLine::Line { row, line:_, script_name:_ } => *row > 0,
        }
    }
}
impl Default for UErrorLine {
    fn default() -> Self {
        Self::None
    }
}

impl fmt::Display for UErrorLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UErrorLine::None => write_locale!(f,
                "※エラー行情報がありません※",
                "* Error row information not found *"
            ),
            UErrorLine::Line { row, line, script_name } => {
                if let Some(name) = &script_name {
                    write_locale!(f,
                        "{} {}行目: {}",
                        "{}, row {}: {}",
                        name, row, line
                    )
                } else {
                    write_locale!(f,
                        "{}行目: {}",
                        "Row {}: {}",
                        row, line
                    )
                }
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UErrorKind {
    Any(String),
    UnknownError,
    SyntaxError,
    UndefinedError,
    ArrayError,
    AssertEqError,
    AssignError,
    BitOperatorError,
    BuiltinFunctionError(String),
    CastError,
    ClassError,
    ComError(i32),
    ConversionError,
    DefinitionError(DefinitionType),
    DllFuncError,
    DlopenError,
    DotOperatorError,
    EnumError,
    EvalParseErrors(usize),
    EvaluatorError,
    FuncCallError,
    FuncDefError,
    HashtblError,
    ModuleError,
    OperatorError,
    ProgIdError,
    StructDefError,
    StructError,
    TaskError,
    UObjectError,
    UserDefinedError,
    UStructError,
    Win32Error(i32),
    WebSocketError,
    WebRequestError,
    FileIOError,
    DevtoolsProtocolError,
    BrowserControlError,
    WmiError,
    OpenCvError,
    ScreenShotError,
    ZipError,
    PrefixError(char),
    ExitExit(i32),
    InitializeError,
    ClipboardError,
    Poff(POFF, bool),
    HtmlNodeError,
    VariantError,
    ComArgError,
    ComCollectionError,
    ComEventError,
    ExcelError,
    SafeArrayError,
}

impl fmt::Display for UErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any(msg) => write!(f, "{}", msg),
            Self::UnknownError => write_locale!(f,
                "不明なエラー",
                "Unknown Error"
            ),
            Self::SyntaxError => write!(f, "Syntax Error"),
            Self::UndefinedError => write_locale!(f,
                "未定義エラー",
                "Undefined Error"
            ),
            Self::DlopenError => write_locale!(f,
                "DLL読み込みエラー",
                "Dll loading Error"
            ),
            Self::CastError => write_locale!(f,
                "型変換エラー",
                "Cast Error"
            ),
            Self::ComError(n) => write_locale!(f,
                "COMエラー (0x{:08X})",
                "COM Error (0x{:08X})",
                n
            ),
            Self::ConversionError => write_locale!(f,
                "変換エラー",
                "Conversion Error"
            ),
            Self::Win32Error(n) => write_locale!(f,
                "Win32 API エラー (0x{:08X})",
                "Win32 API Error (0x{:08X})",
                n
            ),
            Self::HashtblError => write_locale!(f,
                "Hashtbl定義エラー",
                "Hashtbl definition Error"
            ),
            Self::FuncDefError => write_locale!(f,
                "関数定義エラー",
                "Function definition Error"
            ),
            Self::StructDefError => write_locale!(f,
                "構造体定義エラー",
                "UStruct definition Error"
            ),
            Self::StructError => write_locale!(f,
                "構造体エラー",
                "UStruct Error"
            ),
            Self::ArrayError => write_locale!(f,
                "配列エラー",
                "Array Error"
            ),
            Self::EvaluatorError => write_locale!(f,
                "評価エラー",
                "Evaluator Error"
            ),
            Self::UObjectError => write_locale!(f,
                "UObjectエラー",
                "UObject Error"
            ),
            Self::AssignError => write_locale!(f,
                "代入エラー",
                "Assigning Error"
            ),
            Self::DotOperatorError => write_locale!(f,
                "ドット呼び出しエラー",
                "Dot operator Error"
            ),
            Self::UStructError => write_locale!(f,
                "構造体呼び出しエラー",
                "UStruct Error"
            ),
            Self::BitOperatorError => write_locale!(f,
                "ビット演算エラー",
                "Bit operator Error"
            ),
            Self::OperatorError => write_locale!(f,
                "演算子エラー",
                "Operator Error"
            ),
            Self::TaskError => write_locale!(f,
                "タスクエラー",
                "Task Error"
            ),
            Self::EvalParseErrors(n) => write_locale!(f,
                "eval関数の解析エラー ({}件)",
                "{} parser errors on eval function",
                n
            ),
            Self::BuiltinFunctionError(name) => write_locale!(f,
                "ビルトイン関数エラー({})",
                "Builtin function error: {}",
                name
            ),
            Self::ClassError => write_locale!(f,
                "クラスエラー",
                "Class error"
            ),
            Self::DllFuncError => write_locale!(f,
                "Dll関数エラー",
                "Dll function error",
            ),
            Self::FuncCallError => write_locale!(f,
                "関数呼び出しエラー",
                "Function call error",
            ),
            Self::EnumError => write_locale!(f,
                "Enumエラー",
                "Enum error",
            ),
            Self::DefinitionError(dt) => write_locale!(f,
                "{}定義エラー",
                "{} definition error",
                dt
            ),
            Self::ModuleError => write_locale!(f,
                "モジュールエラー",
                "Module error",
            ),
            Self::UserDefinedError => write_locale!(f,
                "ユーザー定義エラー",
                "User defined error",
            ),
            Self::AssertEqError => write_locale!(f,
                "assert_equalエラー",
                "assert_equal error",
            ),
            Self::ProgIdError => write_locale!(f,
                "不正なProgID",
                "Invalid ProgID",
            ),
            Self::DevtoolsProtocolError => write_locale!(f,
                "Devtools protocol エラー",
                "Devtools protocol error",
            ),
            Self::WebSocketError => write_locale!(f,
                "WebSocket エラー",
                "WebSocket error",
            ),
            Self::WebRequestError => write_locale!(f,
                "HTTPリクエスト エラー",
                "HTTP request error",
            ),
            Self::FileIOError => write_locale!(f,
                "IO エラー",
                "IO error",
            ),
            Self::BrowserControlError => write_locale!(f,
                "ブラウザ操作エラー",
                "Browser control error",
            ),
            Self::WmiError => write_locale!(f,
                "WMIエラー",
                "WMI error",
            ),
            Self::OpenCvError => write_locale!(f,
                "opencvエラー",
                "opencv error",
            ),
            Self::ScreenShotError => write_locale!(f,
                "スクリーンショットエラー",
                "Screenshot error",
            ),
            Self::ZipError => write_locale!(f,
                "Zipエラー",
                "Zip error",
            ),
            Self::PrefixError(c) => write_locale!(f,
                "接頭辞({c})エラー",
                "Prefix {c} error",
            ),
            Self::ExitExit(_) => write!(f, ""),
            Self::InitializeError => write_locale!(f,
                "初期化エラー",
                "Initializing error",
            ),
            Self::ClipboardError => write_locale!(f,
                "クリップボードエラー",
                "Clipboard error",
            ),
            Self::Poff(_, _) => write!(f, ""),
            Self::HtmlNodeError => write_locale!(f,
                "HtmlNodeエラー",
                "HtmlNode error",
            ),
            Self::VariantError => write_locale!(f,
                "VARIANT変換エラー",
                "VARIANT conversion error",
            ),
            Self::ComArgError => write_locale!(f,
                "COMメソッド呼び出しエラー",
                "COM method error",
            ),
            Self::ComCollectionError => write_locale!(f,
                "COMコレクションエラー",
                "COM collection error",
            ),
            Self::ComEventError => write_locale!(f,
                "COMイベントエラー",
                "COM event error",
            ),
            Self::ExcelError => write_locale!(f,
                "Excelエラー",
                "Excel error",
            ),
            Self::SafeArrayError => write_locale!(f,
                "SafeArrayエラー",
                "SafeArray error",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UErrorMessage {
    Unknown,
    SyntaxError,
    None,
    Any(String),
    AlreadyDefined(String),
    ArraySizeOmitted,
    AssertEqLeftAndRight(Object, Object),
    BadStringInfix(Infix),
    BuiltinArgCastError(Object, String),
    BuiltinArgInvalid(Object),
    BuiltinArgIsNotFunction,
    BuiltinArgRequiredAt(usize),
    CanNotConvertToNumber(serde_json::Number),
    CanNotConvertToUObject(Object),
    CastError(String),
    /// キャストしたい数値, 型名
    CastError2(f64, String),
    ComError(String, Option<String>),
    ConstantCantBeAssigned(String),
    ClassMemberCannotBeCalledDirectly(String),
    ConstructorIsNotValid(String),
    ConstructorNotDefined(String),
    VariantConvertionError(Object),
    DllArgNotAllowedInStruct,
    DllArgumentIsNotArray(DllType, usize),
    DllArgumentTypeUnexpected(DllType, usize, String),
    DllArrayHasInvalidType(DllType, usize),
    DllConversionError(DllType, usize, String),
    DllMissingArgument(DllType, usize),
    DllNestedStruct,
    DllUnknownType(String),
    DllResultTypeNotAllowed,
    DllArgCountMismatch,
    DllArgTypeMismatch(String, ObjectType),
    DllArrayArgTypeMismatch(String),
    DllArrayArgLengthMismatch,
    /// - 0: 指定サイズ
    /// - 1: 引数のサイズ
    DllStringArgToLarge(usize, usize),
    DlopenError(String),
    DotOperatorNotSupported(Object),
    ExplicitError(String),
    FailedToCreateNewInstance,
    FailedToCreateProcess,
    ForError(String),
    ForInError,
    FuncArgRequired(String),
    FuncBadParameter(Expression),
    FuncInvalidArgument(String),
    FunctionNotFound(String),
    FuncTooManyArguments(usize),
    GlobalVariableNotFound(Option<String>),
    IndexOutOfBounds(Object),
    InternetExplorerNotAllowed,
    InvalidArgument(Object),
    InvalidArraySize,
    InvalidExpression(Expression),
    InvalidHashIndexOption(Object),
    InvalidHashtblOption(Object),
    InvalidIndex(Object),
    InvalidKeyOrIndex(String),
    InvalidObject(Object),
    InvalidRegexPattern(String),
    StructConstructorArgumentError,
    IsNotStruct(String),
    IsPrivateMember(String, String),
    JsonParseError(String),
    LeftAndRightShouldBeNumber(Object, Infix, Object),
    MemberNotFound(String),
    MissingHashIndex(String),
    ModuleMemberNotFound(DefinitionType, String, String),
    InvalidModuleMember,
    NestedDefinition,
    NoIdentifierFound(String),
    NoSizeSpecified,
    NotAClass(String),
    NotAFunction(Object),
    NotAnArray(Object),
    NotANumber(Object),
    NotAVariable(Expression),
    NotFinite(f64),
    NotYetSupported(String),
    ParserErrors(String),
    Reserved(String),
    StructGotBadType(String, DllType, String),
    StructMemberNotFound(String, String),
    StructMemberNotFound2(String, usize),
    StructNotDefined(String),
    StructTypeNotValid(String, String),
    StructTypeUnsupported(DllType),
    TaskEndedIncorrectly(String),
    TooManyArguments(usize, usize),
    TypeMismatch(Object, Infix, Object),
    RightSideTypeInvalid(Infix),
    LeftSideTypeInvalid(Infix),
    DivZeroNotAllowed,
    UnableToGetCursorPosition,
    UnableToGetMonitorInfo,
    UnsupportedArchitecture,
    UnknownDllType(String),
    VariableNotFound(String),
    Win32Error(String),
    DTPElementNotFound(String),
    DTPError(i32, String),
    DTPInvalidElement(Value),
    DTPControlablePageNotFound,
    InvalidMember(String),
    WebResponseWasNotOk(String),
    InvalidErrorLine(usize),
    FailedToGetObject,
    GdiError(String),
    GivenNumberIsOutOfRange(f64, f64),
    WebSocketTimeout(Value),
    WebSocketConnectionError(String),
    InvalidParamType(String, ParamTypeDetail),
    UWindowError(UWindowError),
    EmptyArrayNotAllowed,
    InvalidMemberOrIndex(String),
    FopenError(FopenError),
    NotAnByte(Object),
    FailedToLoadImageFile(String),
    PrefixShouldBeNumber(Object),
    FailedToInitializeLogPrintWindow,
    UncontrollableBrowserDetected(u16, String),
    FailedToOpenPort(u16),
    InvalidBrowserType(i32),
    UnableToReference(String),
    InvalidReference,
    InvalidLeftExpression(Expression),
    InvalidRightExpression(Expression),
    FailedToOpenClipboard,
    NoOuterScopeFound,
    IsNotUserFunction(String),
    GetTimeParseError(String),
    FormatTimeError(String),
    BrowserRuntimeException(String),
    RemoteObjectIsNotObject(String, String),
    RemoteObjectIsNotFunction(String),
    RemoteObjectIsNotArray(String),
    InvalidTabPage(String),
    ArgumentIsNotNumber(usize, String),
    BrowserHasNoDebugPort(String, u16),
    BrowserDebuggingPortUnmatch(String, u16),
    RemoteObjectIsNotPromise,
    RemoteObjectDoesNotHaveValidLength,
    RemoteObjectIsNotPrimitiveValue,
    CanNotCallMethod(String),
    MemberShouldBeIdentifier,
    FromVariant(u16),
    ToVariant(String),
    InvalidComMethodArgOrder,
    NamedArgNotFound(String),
    FailedToConvertToCollection,
    VariantIsNotIDispatch,
    NamedArgNotAllowed,
    MissingArgument,
    FunctionRequired,
    EventInterfaceNotFound,
    ThirdPartyNotImplemented,
    GlobalCanNotBeAssigned,
    IsNotValidExcelObject,
    CanNotConvertToSafeArray,
    StructMemberSizeError(usize),
    StructMemberTypeError,
    StructMemberIsNotArray,
    StructMemberWasNullPointer,
    /// 構造体の文字列メンバへの代入する文字列のサイズが大きすぎる
    /// - .0: バッファサイズ
    /// - .1: 代入する文字列のサイズ
    UStructStringMemberSizeOverflow(usize, usize),
    InvalidCallbackReturnType(DllType),
    InvalidCallbackArgType(DllType),
    CallbackReturnValueCastError,
    DllArgConstSizeIsNotValid,
}

impl fmt::Display for UErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown Error"),
            Self::SyntaxError => write!(f, "Syntax Error"),
            Self::None => write!(f, ""),
            Self::Any(s) => write!(f, "{}", s),
            Self::DlopenError(msg) => write!(f, "{}", msg),
            Self::CastError(msg) => write!(f, "{}", msg),
            Self::CastError2(n, t) => write_locale!(f,
                "{n} を{t}型に変換できません",
                "Failed to convert {n} to type {t}",
            ),
            Self::ComError(msg, desc) => {
                match desc {
                    Some(desc) => write!(f, "{msg} ({desc})"),
                    None => write!(f, "{msg}")
                }
            }
            Self::VariantConvertionError(o) => write_locale!(f,
                "{} をVARIANT型に変換できません",
                "Failed to convert {} to VARIANT",
                o
            ),
            Self::Win32Error(msg) => write!(f, "{}", msg),
            Self::InvalidHashtblOption(o) => write_locale!(f,
                "不正なオプション ({})",
                "Invalid option ({})",
                o
            ),
            Self::ForError(msg) => write_locale!(f,
                "forの書式がおかしい ({})",
                "Invalid syntax on for statement ({})",
                msg
            ),
            Self::ForInError => write_locale!(f,
                "for-inには配列、連想配列、文字列、コレクションを渡してください",
                "for-in requires array, hashtable, string, or collection"
            ),
            Self::NestedDefinition => write_locale!(f,
                "関数定義はネストできません",
                "Nested definition of function/procedure is not allowed"
            ),
            Self::UnknownDllType(name) => write_locale!(f,
                "型が不明です ({})",
                "Type '{}' is unknown",
                name
            ),
            Self::StructNotDefined(name) => write_locale!(f,
                "構造体が未定義です ({})",
                "Ustruct '{}' is not defined",
                name
            ),
            Self::IsNotStruct(name) => write_locale!(f,
                "'{}' は構造体ではありません",
                "'{}' is not an UStruct",
                name
            ),
            Self::NoSizeSpecified => write_locale!(f,
                "サイズまたは次元が未指定です",
                "Size or dimension must be specified"
            ),
            Self::InvalidIndex(o) => write_locale!(f,
                "インデックスが不正です ({})",
                "'{}' is not a valid index",
                o
            ),
            Self::ArraySizeOmitted => write_locale!(f,
                "インデックスの省略は最大次元のみ可能です",
                "Only the size of largest dimension can be omitted"
            ),
            Self::InvalidArraySize => write_locale!(f,
                "配列サイズが不正です",
                "Size of array is invalid"
            ),
            Self::JsonParseError(s) => write_locale!(f,
                "Jsonのパースに失敗 ({})",
                "Failed to parse json: {}",
                s
            ),
            Self::NoIdentifierFound(id) => write_locale!(f,
                "識別子が見つかりません ({})",
                "Identifier not found: {}",
                id
            ),
            Self::NotANumber(o) => write_locale!(f,
                "数値ではありません ({})",
                "Not a number: {}",
                o
            ),
            Self::InvalidKeyOrIndex(s) => write_locale!(f,
                "インデックスの書式が不正です ({})",
                "Invalid index: {}",
                s
            ),
            Self::MissingHashIndex(s) => write_locale!(f,
                "連想配列の順列番号が指定されていません ({})",
                "Missing index: {}",
                s
            ),
            Self::InvalidHashIndexOption(o) => write_locale!(f,
                "連想配列の順列番号に付与する値が不正です ({})",
                "Invalid hash index option: {}",
                o
            ),
            Self::IndexOutOfBounds(o) => write_locale!(f,
                "インデックスが範囲外です ({})",
                "Index out of bounds: {}",
                o
            ),
            Self::NotAnArray(o) => write_locale!(f,
                "配列ではありません ({})",
                "Not an array nor hashtable: {}",
                o
            ),
            Self::MemberNotFound(s) => write_locale!(f,
                "メンバが存在しません ({})",
                "Member not found: {}",
                s
            ),
            Self::InvalidObject(o) => write_locale!(f,
                "モジュールまたはオブジェクトではありません ({})",
                "Not a valid object: {}",
                o
            ),
            Self::GlobalVariableNotFound(name) => match name {
                Some(name) => write_locale!(f,
                    "グローバル変数{}がありません",
                    "Global variable {} not found",
                    name
                ),
                None => write_locale!(f,
                    "グローバル変数がありません",
                    "Global variable not found"
                )
            },
            Self::NotAVariable(e) => write_locale!(f,
                "変数ではありません ({:?})",
                "Not a variable: {:?}",
                e
            ),
            Self::LeftAndRightShouldBeNumber(l, i, r) => write_locale!(f,
                "ビット演算子の両辺が数値ではありません ({} {} {})",
                "Both left and right of bit operator should be a number: {} {} {}",
                l, i, r
            ),
            Self::TypeMismatch(l, i, r) => write_locale!(f,
                "型が不正です ({} {} {})",
                "Mismatched type: {} {} {}",
                l, i, r
            ),
            Self::RightSideTypeInvalid(i) => write_locale!(f,
                "{i} の右辺の型が不正です",
                "The type of the right-hand side of the {i} operator is invalid.",
            ),
            Self::LeftSideTypeInvalid(i) => write_locale!(f,
                "{i} の左辺の型が不正です",
                "The type of the left-hand side of the {i} operator is invalid.",
            ),
            Self::DivZeroNotAllowed => write_locale!(f,
                "0除算はできません",
                "Dividing by zero is not allowed",
            ),
            Self::NotFinite(n) => write_locale!(f,
                "計算結果が無効です ({})",
                "Result value is not valid number: {}",
                n
            ),
            Self::BadStringInfix(i) => write_locale!(f,
                "文字列演算では使えない演算子です ({})",
                "Bad operator: {}",
                i
            ),
            Self::FunctionNotFound(n) => write_locale!(f,
                "関数が未定義です ({})",
                "Function not found: {}",
                n
            ),
            Self::TaskEndedIncorrectly(e) => write_locale!(f,
                "タスクが不正終了しました ({})",
                "Function not found: {}",
                e
            ),
            Self::ParserErrors(e) => write!(f, "{}", e),
            Self::TooManyArguments(given, should) => write_locale!(f,
                "引数が多すぎます({})、{}個またはそれ以下にしてください",
                "{} argument[s] where given, should be {} or less",
                given, should
            ),
            Self::ConstructorNotDefined(name) => write_locale!(f,
                "コンストラクタが未定義です: procedure {}()",
                "Constructor is not defined: procedure {}()",
                name
            ),
            Self::ConstructorIsNotValid(name) => write_locale!(f,
                "コンストラクタが不正です: {}()",
                "Constructor is not valid: {}()",
                name
            ),
            Self::NotAClass(name) => write_locale!(f,
                "クラスではありません: {}()",
                "{} is not a class",
                name
            ),
            Self::StructConstructorArgumentError => write_locale!(f,
                "構造体のアドレスを指定してください",
                "Argument must be the address of exsisting structure",
            ),
            Self::NotAFunction(o) => write_locale!(f,
                "関数ではありません ({})",
                "Not a function: {}",
                o.get_type()
            ),
            Self::DllMissingArgument(dlltype, pos) => write_locale!(f,
                "{1}番目の引数({0}型)がありません",
                "Missing argument of type {} at position {}",
                dlltype, pos
            ),
            Self::DllArrayHasInvalidType(dlltype, pos) => write_locale!(f,
                "{1}番目の配列型引数({0}[])に不正な型を含む配列変数が渡されました",
                "Given array contains invalid type value: {}[] at position {}",
                dlltype, pos
            ),
            Self::DllArgumentIsNotArray(dlltype, pos) => write_locale!(f,
                "{1}番目の配列型引数({0}[])に配列ではない変数が渡されました",
                "Argument is not an array: {}[] at position {}",
                dlltype, pos
            ),
            Self::DllConversionError(dlltype, pos, err) => write_locale!(f,
                "{1}番目の引数({0}型)の変換に失敗しました: {2}",
                "{2}: {0} at position {1}",
                dlltype, pos, err
            ),
            Self::DllArgumentTypeUnexpected(dlltype, pos, unexpected) => write_locale!(f,
                "{1}番目の引数({0}型)に不正な型が渡されました: {2}",
                "unexpected argument type {2} was given to {0} at position {1}",
                dlltype, pos, unexpected
            ),
            Self::DllNestedStruct => write_locale!(f,
                "構造体定義が入れ子になっています",
                "Nested struct"
            ),
            Self::DllArgNotAllowedInStruct => write_locale!(f,
                "構造体に含めることができない型がありました",
                "Invalid type for struct member",
            ),
            Self::DllUnknownType(name) => write_locale!(f,
                "不明な型です ({})",
                "Invalid parameter type: {}",
                name
            ),
            Self::DllResultTypeNotAllowed => write_locale!(f,
                "Dll関数の戻り値に指定できない型です",
                "Invalid return type for dll function",
            ),
            Self::DllArgCountMismatch => write_locale!(f,
                "Dll関数の引数の数が一致しません",
                "The number of arguments to the Dll function does not match",
            ),
            Self::DllArgTypeMismatch(dlltype, argtype) => write_locale!(f,
                "Dll関数の引数の型が一致しません、{dlltype}型に{argtype}が渡されました",
                "Dll function argument type mismatch, {argtype} passed to {dlltype} type",
            ),
            Self::DllArrayArgTypeMismatch(dlltype) => write_locale!(f,
                "Dll関数の引数の型が一致しません、配列引数に{dlltype}型以外の型が渡されました",
                "Dll function argument type mismatch, array argument passed type other than {dlltype}",
            ),
            Self::DllArrayArgLengthMismatch => write_locale!(f,
                "Dll関数の配列引数のサイズが一致しません",
                "Dll function array argument length mismatch",
            ),
            Self::DllStringArgToLarge(s1, s2) => write_locale!(f,
                "代入された文字列のサイズ({s2})が大きすぎます、{s1}以下にしてください",
                "The size of the assigned string ({s2}) is too large, should be less than {s1}.",
            ),
            Self::FuncBadParameter(e) => write_locale!(f,
                "不正なパラメータ ({:?})",
                "Invalid parmeter: {:?}",
                e
            ),
            Self::FuncArgRequired(name) => write_locale!(f,
                "引数が必要です ({})",
                "Argument required: {}",
                name
            ),
            Self::FuncInvalidArgument(name) => write_locale!(f,
                "不正な引数 ({})",
                "Invalid argument: {}",
                name
            ),
            Self::FuncTooManyArguments(n) => write_locale!(f,
                "引数が多すぎます、{}以下にしてください",
                "Number of arguments should be {} or less",
                n
            ),
            Self::FailedToCreateNewInstance => write_locale!(f,
                "インスタンスの作成に失敗しました",
                "Failed to create new instance",
            ),
            Self::InvalidExpression(e) => write_locale!(f,
                "不正な式 ({:?})",
                "Invalid expression: {:?}",
                e
            ),
            Self::ClassMemberCannotBeCalledDirectly(name) => write_locale!(f,
                "インスタンスを作らずクラスメンバを呼び出すことはできません ({0}.{0}())",
                "Calling {0}.{0}() is not allowed",
                name
            ),
            Self::IsPrivateMember(name, member) => write_locale!(f,
                "プライベートメンバの呼び出しは禁止です ({}.{})",
                "Calling private member is not allowed: {}.{}",
                name, member
            ),
            Self::DotOperatorNotSupported(o) => write_locale!(f,
                "モジュールまたはオブジェクトではありません ({})",
                ". operator is not supported: {}",
                o
            ),
            Self::CanNotConvertToNumber(n) => write_locale!(f,
                "{} を数値に変換できません",
                "Failed to convert {} to a number",
                n
            ),
            Self::CanNotConvertToUObject(o) => write_locale!(f,
                "{} をUObjectに変換できません",
                "Failed to convert {} to UObject",
                o
            ),
            Self::VariableNotFound(name) => write_locale!(f,
                "変数が見つかりません ({})",
                "Variable not found: {}",
                name
            ),
            Self::Reserved(name) => write_locale!(f,
                "{} は予約されています",
                "{} is reserved identifier",
                name
            ),
            Self::AlreadyDefined(name) => write_locale!(f,
                "{} は定義済みです",
                "{} is already defined",
                name
            ),
            Self::ConstantCantBeAssigned(name) => write_locale!(f,
                "定数には代入できません ({})",
                "{} is a constant and can not be assigned",
                name
            ),
            Self::ExplicitError(name) => write_locale!(f,
                "変数定義にはDimが必要 ({})",
                "Dim is required for defining {}",
                name
            ),
            Self::ModuleMemberNotFound(t, name, member) => write_locale!(f,
                "{}メンバが見つかりません ({}.{})",
                "{} member not found: {}.{}",
                t, name, member
            ),
            Self::InvalidModuleMember => write_locale!(f,
                "不正なモジュールメンバ宣言",
                "Invalid module member was defined",
            ),
            Self::StructGotBadType(name, dlltype, bad) => write_locale!(f,
                "{}は{}型ですが{}が与えられました",
                "Type of {} should be {} but got {}",
                name, dlltype, bad
            ),
            Self::StructMemberNotFound(name, member) => write_locale!(f,
                "メンバが見つかりません ({}.{})",
                "Member not found: {}.{}",
                name, member
            ),
            Self::StructMemberNotFound2(name, index) => write_locale!(f,
                "{name}に{index}番目のメンバが存在しません",
                "{name} has no member with index {index}",
            ),
            Self::StructTypeNotValid(name, t) => write_locale!(f,
                "{}は{}型です",
                "Type of {} should be {}",
                name, t
            ),
            Self::StructTypeUnsupported(t) => write_locale!(f,
                "{}型は未サポートです",
                "Type {} is not supported",
                t
            ),
            Self::AssertEqLeftAndRight(left, right) => write!(f,
                "left: {}; right: {}",
                left, right
            ),
            Self::BuiltinArgCastError(n, t) => write_locale!(f,
                "{}を{}型にキャストできません",
                "Unable to cast {} to {}",
                n, t
            ),
            Self::BuiltinArgRequiredAt(i) => write_locale!(f,
                "{}番目の引数は必須です",
                "Argument at position {} is required",
                i
            ),
            Self::BuiltinArgInvalid(o) => write_locale!(f,
                "引数の型が不正です: {}",
                "Invalid argument type: {}",
                o.get_type()
            ),
            Self::UnsupportedArchitecture => write_locale!(f,
                "サポート外OSアーキテクチャ",
                "OS architecture is not supported",
            ),
            Self::FailedToCreateProcess => write_locale!(f,
                "プロセスの作成に失敗",
                "Failed to create process",
            ),
            Self::BuiltinArgIsNotFunction => write_locale!(f,
                "引数がユーザー定義関数ではありません",
                "Argument should be user defined function",
            ),
            Self::InvalidArgument(o) => write_locale!(f,
                "不正な引数 ({})",
                "Invalid argument: {}",
                o
            ),
            Self::InvalidRegexPattern(pat) => write_locale!(f,
                "不正な正規表現 ({})",
                "Invalid regular expression: {}",
                pat
            ),
            Self::UnableToGetMonitorInfo => write_locale!(f,
                "モニタ情報が取得できませんでした",
                "Unable to get monitor info",
            ),
            Self::UnableToGetCursorPosition => write_locale!(f,
                "マウスカーソル座標が取得できませんでした",
                "Unable to get mouse cursor position",
            ),
            Self::NotYetSupported(name) => write_locale!(f,
                "{}は未実装です",
                "{} is not yet supported",
                name
            ),
            Self::InternetExplorerNotAllowed => write_locale!(f,
                "InternetExplorer.Applicationの実行は許可されていません",
                "InternetExplorer.Application is not allowed",
            ),
            Self::DTPElementNotFound(selector) =>write_locale!(f,
                "エレメントが見つかりません ({})",
                "Element not found: {}",
                selector
            ),
            Self::DTPError(n, msg) =>write!(f,
                "{}: {}",
                msg, n
            ),
            Self::DTPInvalidElement(v) =>write_locale!(f,
                "エレメントではありません ({})",
                "Not a valid element: {}",
                v.to_string()
            ),
            Self::DTPControlablePageNotFound =>write_locale!(f,
                "操作可能なページが見つかりません",
                "Target page not found",
            ),
            Self::InvalidMember(name) =>write_locale!(f,
                "{}がありません",
                "{} is not found",
                name
            ),
            Self::WebResponseWasNotOk(status) =>write_locale!(f,
                "不正なレスポンス ({status})",
                "Bad response: {status}",
            ),
            Self::InvalidErrorLine(row) =>write_locale!(f,
                "不正なエラー行指定 ({})",
                "Invalid error line: {}",
                row
            ),
            Self::FailedToGetObject =>write_locale!(f,
                "オブジェクトの取得に失敗",
                "Failed to get active object",
            ),
            Self::GdiError(msg) => write_locale!(f,
                "GDI関数の実行に失敗 ({})",
                "Failure on GDI function: {}",
                msg
            ),
            Self::GivenNumberIsOutOfRange(from, to) => write_locale!(f,
                "{}~{}を指定してください",
                "Specify a number between {} and {}",
                from, to
            ),
            Self::WebSocketTimeout(id) => write_locale!(f,
                "ID{}のレスポンスがありませんでした",
                "Got no response for request id {}",
                id
            ),
            Self::WebSocketConnectionError(status) => write_locale!(f,
                "接続に失敗しました ({status})",
                "Failed to connect: {status}",
            ),
            Self::InvalidParamType(n, t) => write_locale!(f,
                "不正な引数の型: {}は{}型のみ有効です",
                "Invalid argument type: {} should be type {}",
                n, t
            ),
            Self::UWindowError(e) => write!(f, "{e}"),
            Self::EmptyArrayNotAllowed => write_locale!(f,
                "空の配列は許可されていません",
                "Empty array is not allowed"
            ),
            Self::InvalidMemberOrIndex(o) => write_locale!(f,
                "メンバが存在しない、または不正なインデックス: {}",
                "Invalid index: {}",
                o
            ),
            Self::FopenError(e) => write!(f, "{}", e),
            Self::NotAnByte(o) => write_locale!(f,
                "不正な値({o}): バイト配列の要素には0-255しか代入できません",
                "You can not assign {o} as byte",
            ),
            Self::FailedToLoadImageFile(path) => write_locale!(f,
                "画像ファイルが正常に読み取れませんでした: {path}",
                "Failed to load image file: {path}",
            ),
            Self::PrefixShouldBeNumber(o) => write_locale!(f,
                "接頭辞が数値以外の値に付加されました: {o}",
                "Prefix can only be added to number but given value was '{o}'",
            ),
            Self::FailedToInitializeLogPrintWindow => write_locale!(f,
                "Printウィンドウの初期化に失敗",
                "Failed to initialize logprint window",
            ),
            Self::UncontrollableBrowserDetected(port, name) => write_locale!(f,
                "対象ポート({port})が開かれていない{name}が既に起動中のため自動操作が行えません、すべての{name}を終了して再実行してください",
                "Detected uncontrollable {name}, close all {name} and try again",
            ),
            Self::FailedToOpenPort(port) => write_locale!(f,
                "ポート{port}へ接続できませんでした",
                "Faild to create connection to port {port}",
            ),
            Self::InvalidBrowserType(n) => write_locale!(f,
                "不正なブラウザタイプ: {n}",
                "Invalid browser type: {n}",
            ),
            Self::UnableToReference(name) => write_locale!(f,
                "{name} を参照できません",
                "Unable to reference {name}",
            ),
            Self::InvalidReference => write_locale!(f,
                "有効な参照ではありません",
                "Invalid reference",
            ),
            Self::InvalidLeftExpression(e) => write_locale!(f,
                "式の左辺が不正な式です: {e}",
                "Expression on the left is invalid: {e}",
            ),
            Self::InvalidRightExpression(e) => write_locale!(f,
                "式の右辺が不正な式です: {e}",
                "Expression on the right is invalid: {e}",
            ),
            Self::FailedToOpenClipboard => write_locale!(f,
                "クリップボードが開けませんでした",
                "Failed to open clipboard",
            ),
            Self::NoOuterScopeFound => write_locale!(f,
                "参照渡しが行われましたが外部スコープがありません",
                "No outer scope found"
            ),
            Self::IsNotUserFunction(name) => write_locale!(f,
                "\"{name}\" はユーザー定義関数ではありません",
                "\"{name}\" is not a user define function"
            ),
            Self::GetTimeParseError(err) => write!(f, "{err}"),
            Self::FormatTimeError(err) => write!(f, "{err}"),
            Self::BrowserRuntimeException(err) => write!(f, "{err}"),
            Self::RemoteObjectIsNotObject(type_name, property) => write_locale!(f,
                "リモートオブジェクトに {property} がありません ({type_name})",
                "Remote object does not have {property} [{type_name}]",
            ),
            Self::RemoteObjectIsNotFunction(type_name) => write_locale!(f,
                "リモートオブジェクトが関数ではありません ({type_name})",
                "Remote object is not function [{type_name}]",
            ),
            Self::RemoteObjectIsNotArray(type_name) => write_locale!(f,
                "リモートオブジェクトが配列ではありません ({type_name})",
                "Remote object is not an array [{type_name}]",
            ),
            Self::InvalidTabPage(uri) => write_locale!(f,
                "操作可能なページではありません ({uri})",
                "Tab was not a valid page [{uri}]",
            ),
            Self::ArgumentIsNotNumber(i, value) => write_locale!(f,
                "{i}番目の引数が数値ではありません ({value})",
                "Argument at position {i} should be number",
            ),
            Self::BrowserHasNoDebugPort(exe, port) => write_locale!(f,
                "実行中の{exe}が見つかりましたがデバッグポート({port})が開かれていません",
                "Found running {exe}, but is not opening port {port}",
            ),
            Self::BrowserDebuggingPortUnmatch(exe, port) => write_locale!(f,
                "デバッグポート({port})が開かれていますがプロセスが{exe}ではありません",
                "Port {port} is not opened by {exe}",
            ),
            Self::RemoteObjectIsNotPromise => write_locale!(f,
                "RemoteObjectがPromiseではありません",
                "RemoteObject is not Promise",
            ),
            Self::RemoteObjectIsNotPrimitiveValue => write_locale!(f,
                "RemoteObjectを通常の値型に変換できません",
                "RemoteObject is not a primitive value",
            ),
            Self::RemoteObjectDoesNotHaveValidLength => write_locale!(f,
                "RemoteObjectが有効なlengthを返しませんでした",
                "RemoteObject did not return valid length value",
            ),
            Self::CanNotCallMethod(member) => write_locale!(f,
                "{member}というメソッドがありません",
                "Method named {member} does not exist",
            ),
            Self::MemberShouldBeIdentifier => write_locale!(f,
                "不正なメンバ指定",
                "Object member name should be Identifier",
            ),
            Self::FromVariant(vt) => write_locale!(f,
                "不正なVARIANT型({vt})",
                "Invalid VARIANT type: {vt}",
            ),
            Self::ToVariant(vt) => write_locale!(f,
                "VARIANT型に変換できません({vt})",
                "{vt} can not be converted to VARIANT",
            ),
            Self::InvalidComMethodArgOrder => write_locale!(f,
                "名前付き引数の後に名前なし引数は指定できません",
                "You can not pass unnamed argument after named argument",
            ),
            Self::NamedArgNotFound(name) => write_locale!(f,
                "{name}という引数が見つかりません",
                "No argument named {name} has found",
            ),
            Self::FailedToConvertToCollection => write_locale!(f,
                "COMオブジェクトがコレクションではありません",
                "COM object is not a collection",
            ),
            Self::VariantIsNotIDispatch => write_locale!(f,
                "VARIANTがIDispatchではありません",
                "VARIANT is not IDispatch",
            ),
            Self::NamedArgNotAllowed => write_locale!(f,
                "名前付き引数は許可されていません",
                "Named argument not allowed",
            ),
            Self::MissingArgument => write_locale!(f,
                "引数が不足しています",
                "1 or more arguments are missing",
            ),
            Self::FunctionRequired => write_locale!(f,
                "関数が指定されていません",
                "User defined function is required",
            ),
            Self::EventInterfaceNotFound => write_locale!(f,
                "インターフェースが見つかりません",
                "Interface not found",
            ),
            Self::ThirdPartyNotImplemented => write_locale!(f,
                "XL_OOOCはサポートされていません",
                "XL_OOOC is not supported",
            ),
            Self::GlobalCanNotBeAssigned => write_locale!(f,
                "GLOBALは代入できません",
                "Assigning GLOBAL to variable is not allowed",
            ),
            Self::IsNotValidExcelObject => write_locale!(f,
                "有効なExcelオブジェクトではありません",
                "Object is not valid Excel object",
            ),
            Self::CanNotConvertToSafeArray => write_locale!(f,
                "配列をSafeArrayに変換できません",
                "Can not convert array to SafeArray",
            ),
            Self::StructMemberSizeError(size) => write_locale!(f,
                "配列サイズが大きすぎます、{size}以下にしてください",
                "Array size is too large, should be less or equal to {size}"
            ),
            Self::StructMemberTypeError => write_locale!(f,
                "型が合いません、メンバの型に該当する値を入れてください",
                "Type missmatch",
            ),
            Self::StructMemberIsNotArray => write_locale!(f,
                "メンバが数値配列型ではありません",
                "Struct member is not an array of number",
            ),
            Self::StructMemberWasNullPointer => write_locale!(f,
                "メンバがNULLポインタです",
                "Struct member was null pointer",
            ),
            Self::UStructStringMemberSizeOverflow(bufsize, strsize) => write_locale!(f,
                "バッファサイズ({bufsize})より大きいサイズ({strsize})の文字列は代入できません",
                "You can not assign string value larger than buffersize",
            ),
            Self::InvalidCallbackReturnType(t) => write_locale!(f,
                "コールバック関数の戻り値の型が不正です: {t}",
                "Return type of callback function is invalid: {t}",
            ),
            Self::InvalidCallbackArgType(t) => write_locale!(f,
                "コールバック関数の引数の型が不正です: {t}",
                "Parameter type of callback function is invalid: {t}",
            ),
            Self::CallbackReturnValueCastError => write_locale!(f,
                "コールバック関数の戻り値のキャストに失敗",
                "Failed to cast return value of callback function",
            ),
            Self::DllArgConstSizeIsNotValid => write_locale!(f,
                "サイズを示す定数がない、または数値ではありません",
                "Constant indicating size is not valid",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionType {
    Variable,
    Const,
    Public,
    Function,
    Module,
    Class,
    Struct,
    DefDll,
    Any
}

impl fmt::Display for DefinitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Variable => write_locale!(f, "変数", "Variable"),
            Self::Const => write_locale!(f, "定数", "Const"),
            Self::Public => write_locale!(f, "パブリック", "Public"),
            Self::Function => write_locale!(f, "関数", "Function"),
            Self::Module => write_locale!(f, "モジュール", "Module"),
            Self::Class => write_locale!(f, "クラス", "Class"),
            Self::Struct => write_locale!(f, "構造体", "Struct"),
            Self::DefDll => write_locale!(f, "Dll関数", "Dll function"),
            Self::Any => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamTypeDetail {
    Any,
    String,
    Number,
    Bool,
    Array,
    HashTbl,
    Function,
    UObject,
    UserDefinition(String)
}

impl fmt::Display for ParamTypeDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamTypeDetail::Any => write!(f, ""),
            ParamTypeDetail::String => write_locale!(f, "文字列", "string"),
            ParamTypeDetail::Number => write_locale!(f, "数値", "number"),
            ParamTypeDetail::Bool => write_locale!(f, "真偽値", "bool"),
            ParamTypeDetail::Array => write_locale!(f, "配列", "array"),
            ParamTypeDetail::HashTbl => write_locale!(f, "連想配列", "hashtbl"),
            ParamTypeDetail::Function => write_locale!(f, "関数", "function"),
            ParamTypeDetail::UObject => write!(f, "UObject"),
            ParamTypeDetail::UserDefinition(s) => write!(f, "{}", s),
        }
    }
}

impl From<dlopen::Error> for UError {
    fn from(e: dlopen::Error) -> Self {
        Self::new(
            UErrorKind::DlopenError,
            UErrorMessage::Any(e.to_string())
        )
    }
}

impl From<cast::Error> for UError {
    fn from(e: cast::Error) -> Self {
        Self::new(
            UErrorKind::DlopenError,
            UErrorMessage::Any(e.to_string())
        )
    }
}

impl From<std::io::Error> for UError {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            UErrorKind::FileIOError,
            UErrorMessage::Any(e.to_string())
        )
    }
}

impl From<zip::result::ZipError> for UError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::new(UErrorKind::ZipError, UErrorMessage::Any(e.to_string()))
    }
}