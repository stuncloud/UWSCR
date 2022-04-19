use std::error;
use std::fmt;

use crate::ast::{Expression, Infix, DllType};
use crate::evaluator::object::Object;
pub use crate::write_locale;
pub use super::{CURRENT_LOCALE, Locale};
use crate::gui::UWindowError;
use crate::evaluator::object::fopen::FopenError;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct UError {
    kind: UErrorKind,
    message: UErrorMessage,
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
    pub fn set_line(&mut self, row: usize, line: Option<String>) {
        self.line = UErrorLine::new(row, line)
    }
    pub fn get_line(&self) -> UErrorLine {
        self.line.clone()
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
pub struct UErrorLine {
    pub row: usize,
    pub line: Option<String>
}

impl UErrorLine {
    pub fn new(row: usize, line: Option<String>) -> Self {
        Self {row, line}
    }
    pub fn has_row(&self) -> bool {
        self.row > 0
    }
    pub fn has_line(&self) -> bool {
        self.line.is_some()
    }
    pub fn set_line_if_none(&mut self, line: String) {
        if self.line.is_none() && self.row > 0 {
            self.line = Some(line);
        }
    }
}

impl fmt::Display for UErrorLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_locale!(f,
            "{}行目: {}",
            "Row {}: {}",
            self.row,
            match &self.line {
                Some(line) => line.clone(),
                None => "スクリプト外または不明なエラー".into()
            }
        )
    }
}

impl Default for UErrorLine {
    fn default() -> Self {
        Self {row: 0, line: None}
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
                ". 呼び出しエラー",
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
    InvalidStructArgument(String),
    IsNotStruct(String),
    IsPrivateMember(String, String),
    JsonParseError(String),
    LeftAndRightShouldBeNumber(Object, Infix, Object),
    MemberNotFound(String),
    MissingHashIndex(String),
    ModuleMemberNotFound(DefinitionType, String, String),
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
    StructNotDefined(String),
    StructTypeNotValid(String, String),
    StructTypeUnsupported(DllType),
    TaskEndedIncorrectly(String),
    TooManyArguments(usize, usize),
    TypeMismatch(Object, Infix, Object),
    UnableToGetCursorPosition,
    UnableToGetMonitorInfo,
    UnknownArchitecture(String),
    UnknownDllType(String),
    VariableNotFound(String),
    Win32Error(String),
    DTPElementNotFound(String),
    DTPError(i32, String),
    DTPInvalidElement(Value),
    DTPControlablePageNotFound,
    InvalidMember(String),
    WebResponseWasNotOk(u16, String),
    InvalidErrorLine(usize),
    FailedToGetObject,
    GdiError(String),
    GivenNumberIsOutOfRange(f64, f64),
    WebSocketTimeout(Value),
    WebSocketConnectionError(u16, String),
    InvalidParamType(String, ParamTypeDetail),
    UWindowError(UWindowError),
    EmptyArrayNotAllowed,
    InvalidMemberOrIndex(String),
    FopenError(FopenError),
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
            Self::ComError(msg, desc) => write!(f,
                "{}{}",
                msg, match desc {
                    Some(s) => format!(" ({})", s),
                    None => String::new()
                }
            ),
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
                "引数が多すぎます ({}/ {}またはそれ以下にしてください)",
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
            Self::InvalidStructArgument(name) => write_locale!(f,
                "不正な引数 ({}): 構造体のアドレスを指定してください",
                "{} is not a valid argument; should be the address of structure",
                name
            ),
            Self::NotAFunction(o) => write_locale!(f,
                "関数ではありません ({})",
                "Not a function: {}",
                o
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
                "{1}番目の引数({0}型)のに不正な型が渡されました: {2}",
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
                "不正な引数: {}",
                "Invalid argument: {}",
                o
            ),
            Self::UnknownArchitecture(arch) => write_locale!(f,
                "不明なアーキテクチャ: {}",
                "Unknown architecture: {}",
                arch
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
            Self::WebResponseWasNotOk(st, msg) =>write_locale!(f,
                "不正なレスポンス ({} {})",
                "Bad response: {} {}",
                st, msg
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
            Self::WebSocketConnectionError(status, status_text) => write_locale!(f,
                "接続に失敗しました ({} {})",
                "Failed to connect: {} {}",
                status, status_text
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