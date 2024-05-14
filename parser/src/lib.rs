pub mod ast;
pub mod lexer;
pub mod token;
pub mod serializer;
pub mod error;

use ast::*;
use lexer::{Lexer, Position, TokenInfo};
use token::{Token, BlockEnd};
use error::{ParseError, ParseErrorKind};
use util::{
    get_script, get_utf8,
    settings::USETTINGS,
};

use std::path::PathBuf;
use std::env;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

pub type ParseErrors = Vec<ParseError>;
pub type ParserResult<T> = Result<T, ParseErrors>;

static CALLED_FILE_LOCATIONS: Lazy<Arc<Mutex<Vec<ScriptLocation>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

enum StatementType {
    /// - public
    /// - public hashtbl
    /// - public hash
    Public,
    /// - const
    /// - textblock
    /// - enum
    Const,
    /// - dim
    Dim,
    /// - option
    Option,
    /// - function
    /// - procedure
    /// - async function
    /// - async procedure
    /// - module
    /// - class
    Definition,
    /// - 単行if
    /// - print
    /// - continue
    /// - break
    /// - hashtable
    /// - hash
    /// - exit
    /// - exitexit
    /// - thread
    /// - comerrign
    /// - comerrret
    Script,
    /// - ifb
    /// - select
    /// - for
    /// - while
    /// - repeat
    /// - with
    /// - try
    Block,
    /// - call
    Call(ScriptLocation, BuilderScope),
    /// - def_dll
    DefDll,
    /// 式のみの文
    Expression,
}
enum ExpressionState {
    StartOfLine,
    Lambda,
    NotAccess,
    Default,
}
impl ExpressionState {
    fn is_start_of_line(&self) -> bool {
        match self {
            Self::StartOfLine => true,
            _ => false,
        }
    }
    fn is_lambda(&self) -> bool {
        match self {
            Self::Lambda => true,
            _ => false,
        }
    }
    fn is_sol_or_lambda(&self) -> bool {
        match self {
            Self::StartOfLine |
            Self::Lambda => true,
            _ => false,
        }
    }
    fn could_be_access(&self) -> bool {
        match self {
            Self::NotAccess => false,
            _ => true
        }
    }
}

/// 識別子の解析がどの文脈で行われているか
enum IdentifierType {
    /// 変数・定数宣言
    Declaration,
    /// 代入
    Assignment,
    /// 呼び出し
    Access,
    /// 関数パラメータ名
    Parameter,
    /// 定義した関数名など
    Definition,
    /// 登録しないもの
    Other,
    /// まだ定かではないもの
    NotSure,
}

pub struct Parser {
    lexer: Lexer,
    current_token: TokenInfo,
    next_token: TokenInfo,
    errors: ParseErrors,
    with: Option<Expression>,
    with_count: usize,
    builder: ProgramBuilder,
    /// trueで式の解析が厳しくなる
    /// - newでdir.is_some()であればtrueになる
    strict_mode: bool,
}

impl Parser {
    pub fn new(lexer: Lexer, dir: Option<PathBuf>, builtin_names: Option<Vec<String>>) -> Self {
        let strict_mode = builtin_names.is_some();
        let script_path = dir
            .filter(|path| path.capacity() > 0)
            .map(|path| {
                match env::var("GET_UWSC_NAME") {
                    Ok(name) => path.join(name),
                    Err(_) => path,
                }
            });
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            builder: ProgramBuilder::new(script_path, builtin_names),
            strict_mode,
        };
        parser.bump();
        parser.bump();

        parser
    }
    pub fn new_eval_parser(lexer: Lexer) -> Self {
        let builder = ProgramBuilder::new_eval_builder();
        let strict_mode = builder.is_strict_mode();
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            builder,
            strict_mode,
        };
        parser.bump();
        parser.bump();
        parser
    }
    pub fn new_diagnostics_parser(lexer: Lexer, script_path: PathBuf, builtin_names: Vec<String>) -> Self {
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            builder: ProgramBuilder::new(Some(script_path), Some(builtin_names)),
            strict_mode: true,
        };
        parser.bump();
        parser.bump();
        parser
    }

    pub fn call(lexer: Lexer, builder: ProgramBuilder, strict_mode: bool) -> Self {
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            builder,
            strict_mode,
        };
        parser.bump();
        parser.bump();

        parser
    }

    pub fn script_name(&self) -> String {
        self.builder.script_name()
    }

    fn token_to_precedence(token: &Token) -> Precedence {
        match token {
            Token::Period => Precedence::DotCall,
            Token::Lbracket => Precedence::Index,
            Token::Lparen => Precedence::FuncCall,
            Token::Slash | Token::Asterisk | Token::Mod => Precedence::Multiplicative,
            Token::Plus | Token::Minus => Precedence::Additive,
            Token::LessThan | Token::LessThanEqual => Precedence::Relational,
            Token::GreaterThan | Token::GreaterThanEqual => Precedence::Relational,
            Token::Equal | Token::EqualOrAssign | Token::NotEqual => Precedence::Equality,
            Token::And | Token::AndL | Token::AndB => Precedence::And,
            Token::Or | Token::OrL | Token::OrB |
            Token::Xor | Token::XorL | Token::XorB => Precedence::Or,
            Token::Question => Precedence::Ternary,
            Token::Assign => Precedence::Assign,
            _ => Precedence::Lowest,
        }
    }

    fn push_error(&mut self, kind: ParseErrorKind, start: Position, end: Position) {
        let script_name = self.script_name();
        let err = ParseError::new(kind, start, end, script_name);
        self.errors.push(err);
    }

    fn bump(&mut self) {
        self.current_token = self.next_token.clone();
        self.next_token = self.lexer.next_token();
    }
    fn bump_to_next_row(&mut self) {
        loop {
            match self.current_token.token {
                Token::Eol |
                Token::Eof => {
                    self.bump();
                    break;
                },
                _ => self.bump()
            }
        }
    }

    fn get_current_with(&self) -> Option<Expression> {
        self.with.clone()
    }

    fn set_with(&mut self, opt_exp: Option<Expression>) {
        self.with = opt_exp;
    }

    fn is_current_token(&mut self, token: &Token) -> bool {
        self.current_token.token == *token
    }

    fn is_current_token_in(&mut self, tokens: Vec<Token>) -> bool {
        tokens.contains(&self.current_token.token)
    }

    fn is_current_token_end_of_block(&mut self) -> bool {
        match &self.current_token.token {
            Token::BlockEnd(_) |
            Token::Rbrace => true,
            _ => false
        }
    }

    fn is_next_token(&mut self, token: &Token) -> bool {
        self.next_token.token == *token
    }

    /// 次のトークンが期待通りであればbumpする
    ///
    /// 異なる場合はエラーを積む
    fn bump_to_next_expected_token(&mut self, expected: Token) -> bool {
        if self.is_next_token(&expected) {
            self.bump();
            return true;
        } else {
            self.error_next_token_is_unexpected(expected);
            return false;
        }
    }

    /// 現在のトークンが期待されるものであるかどうか
    ///
    /// 異なる場合はエラーを積む
    fn is_current_token_expected(&mut self, expected: Token) -> bool {
        if self.is_current_token(&expected) {
            return true;
        } else {
            self.error_current_token_is_unexpected(expected);
            return false;
        }
    }

    /// 現在のブロック終了トークンが期待されるものであるかどうか
    ///
    /// 異なる場合はエラーを積む
    fn is_current_closing_token_expected(&mut self, expected: BlockEnd) -> bool {
        if let Token::BlockEnd(end) = &self.current_token.token {
            if end == &expected {
                true
            } else {
                self.error_current_block_closing_token_was_unexpected(expected);
                false
            }
        } else {
            self.error_current_block_closing_token_was_unexpected(expected);
            false
        }
    }

    /// 次のトークンが期待されたものではない
    fn error_next_token_is_unexpected(&mut self, expected: Token) {
        let next = &self.next_token;
        let end = next.get_end_pos();
        let kind = ParseErrorKind::NextTokenIsUnexpected(expected, next.token());
        self.push_error(kind, next.pos, end);
    }

    /// 現在のトークンが期待されたものではない
    fn error_current_token_is_unexpected(&mut self, expected: Token) {
        let current = &self.current_token;
        let end = current.get_end_pos();
        let kind = ParseErrorKind::CurrentTokenIsUnexpected(expected, current.token());
        self.push_error(kind, current.pos, end);
    }

    /// 現在位置の閉じトークンが期待されたものではない
    fn error_current_block_closing_token_was_unexpected(&mut self, blockend: BlockEnd) {
        let current = &self.current_token;
        let end = current.get_end_pos();
        let kind = ParseErrorKind::BlockClosingTokenIsUnexpected(Token::BlockEnd(blockend), current.token());
        self.push_error(kind, current.pos, end);
    }


    /// 現在のトークンが不正
    fn error_current_token_is_invalid(&mut self) {
        let current = &self.current_token;
        let end = current.get_end_pos();
        let kind = ParseErrorKind::CurrentTokenIsInvalid(current.token());
        self.push_error(kind, current.pos, end);
    }

    /// 次のトークンが不正
    fn error_next_token_is_invalid(&mut self) {
        let next = &self.next_token;
        let end = next.get_end_pos();
        let kind = ParseErrorKind::NextTokenIsInvalid(next.token());
        self.push_error(kind, next.pos, end);
    }

    /// 現在のトークンのエラー理由を指定
    fn error_on_current_token(&mut self, kind: ParseErrorKind) {
        let current = &self.current_token;
        let end = current.get_end_pos();
        self.push_error(kind, current.pos, end);
        self.bump();
    }

    /// 次のトークンのエラー理由を指定
    fn error_on_next_token(&mut self, kind: ParseErrorKind) {
        let next = &self.next_token;
        let end = next.get_end_pos();
        self.push_error(kind, next.pos, end);
    }

    fn current_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.current_token.token)
    }

    fn next_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.next_token.token)
    }

    pub fn as_errors(self) -> ParseErrors {
        self.errors
    }
    pub fn lines(&self) -> Vec<String> {
        self.lexer.lines.clone()
    }

    fn current_token_pos(&self) -> Position {
        self.current_token.pos
    }
    fn current_token_end_pos(&self) -> Position {
        self.current_token.get_end_pos()
    }
    fn current_line_end_pos(&self) -> Position {
        let row = self.current_token.pos.row;
        let line = &self.lexer.lines[row-1];
        Position { row, column: line.len() + 1}
    }

    pub fn parse(mut self) -> ParserResult<Program> {
        self.parse_to_builder();
        self.check_identifier();

        if self.errors.len() == 0 {
            let program = self.builder.build(self.lexer.lines);
            Ok(program)
        } else {
            Err(self.errors)
        }
    }
    pub fn parse_to_program_and_errors(mut self) -> (Program, ParseErrors) {
        self.parse_to_builder();
        self.check_identifier();
        let program = self.builder.build(self.lexer.lines);
        (program, self.errors)
    }
    pub fn parse_to_builder(&mut self) {
        while ! self.is_current_token(&Token::Eof) {
            match self.parse_statement(false) {
                Some((t, statement)) => {
                    match t {
                        StatementType::Public => {
                            self.builder.push_public(statement);
                        },
                        StatementType::Const => {
                            self.builder.push_const(statement);
                        },
                        StatementType::Dim => {
                            self.builder.push_script(statement);
                        },
                        StatementType::Option => {
                            self.builder.push_option(statement);
                        },
                        StatementType::Definition => {
                            self.builder.push_def(statement);
                        },
                        StatementType::Script => {
                            self.builder.push_script(statement);
                        },
                        StatementType::Block => {
                            self.builder.push_script(statement);
                        },
                        StatementType::Call(location, scope) => {
                            self.builder.push_call_scope(location, scope);
                            self.builder.push_script(statement);
                        },
                        StatementType::DefDll => {
                            self.builder.push_def(statement.clone());
                            self.builder.push_script(statement);
                        },
                        StatementType::Expression => {
                            self.builder.push_script(statement);
                        },
                    }
                },
                None => {
                    self.bump_to_next_row();
                    continue;
                },
            }
            self.bump();
        }
    }
    /// 以下をチェックする
    /// - OPTION EXPLICIT
    /// - 重複
    /// - アクセス
    fn check_identifier(&mut self) {
        if self.strict_mode {
            // OPTION EXPLICITチェック
            let (is_explicit, is_optpublic) = {
                let settings = USETTINGS.lock().unwrap();
                let is_explicit = settings.options.explicit || self.builder.is_explicit_option_enabled();
                let is_optpublic = settings.options.opt_public || self.builder.is_optpublic_option_enabled();
                (is_explicit, is_optpublic)
            };
            if is_explicit {
                self.builder.check_option_explicit()
                    .iter()
                    .for_each(|(location, names)| {
                        names.iter().for_each(|name| {
                            let err = ParseError::new(
                                ParseErrorKind::ExplicitError(name.name.to_owned()),
                                name.start, name.end, location.to_string()
                            );
                            self.errors.push(err)
                        })
                    });
            } else {
                self.builder.declare_implicitly();
            }
            // 重複チェック
            self.builder.check_duplicated()
                .iter()
                .for_each(|(location, names)| {
                    names.iter().for_each(|name| {
                        let err = ParseError::new(
                            ParseErrorKind::IdentifierIsAlreadyDefined(name.name.to_owned()),
                            name.start, name.end, location.to_string()
                        );
                        self.errors.push(err)
                    })
                });
            // OPTION OPTPUBLIC
            if is_optpublic {
                self.builder.check_public_duplicated()
                    .iter()
                    .for_each(|(location, names)| {
                        names.iter().for_each(|name| {
                            let err = ParseError::new(
                                ParseErrorKind::IdentifierIsAlreadyDefined(name.name.to_owned()),
                                name.start, name.end, location.to_string()
                            );
                            self.errors.push(err);
                        });
                    });
            }
            // 未定義チェック
            self.builder.check_access()
                .iter()
                .for_each(|(location, names)| {
                    names.iter().for_each(|name| {
                        let err = ParseError::new(
                            ParseErrorKind::UndeclaredIdentifier(name.name.to_owned()),
                            name.start, name.end, location.to_string()
                        );
                        self.errors.push(err)
                    })
                });
        }
    }

    fn parse_block_statement(&mut self) -> BlockStatement {
        self.bump();
        let mut block: BlockStatement  = vec![];

        while ! self.is_current_token_end_of_block() && ! self.is_current_token(&Token::Eof) {
            let start = self.current_token_pos();
            let end = self.current_token_end_pos();
            let line_end = self.current_line_end_pos();
            match self.parse_statement(false) {
                Some((t, statement)) => match t {
                    StatementType::Public => {
                        self.builder.push_public(statement);
                    },
                    StatementType::Const => {
                        self.builder.push_const(statement);
                    },
                    StatementType::Dim => {
                        if self.builder.is_in_module_member_definition() {
                            self.builder.push_dim_member(statement);
                        } else {
                            block.push(statement)
                        }
                    },
                    StatementType::Option => {
                        self.push_error(ParseErrorKind::OptionStatementNotAllowed, start, line_end);
                    },
                    StatementType::Definition => {
                        if self.builder.is_in_module_member_definition() {
                            match &statement.statement {
                                // 関数定義はメンバに加える
                                Statement::Function { name:_, params:_, body:_, is_proc:_, is_async:_ } => {
                                    self.builder.push_def(statement);
                                },
                                _ => {
                                    let end = self.current_token_pos();
                                    self.push_error(ParseErrorKind::InvalidMemberDefinition(statement.statement, self.builder.is_class_definition()), start, end);
                                }
                            }
                        } else {
                            self.push_error(ParseErrorKind::DefinitionStatementNotAllowed, start, end);
                        }
                    },
                    StatementType::Script => {
                        if self.builder.is_in_module_member_definition() {
                            match &statement.statement {
                                // 連想配列定義はdimメンバに加える
                                Statement::HashTbl(_, false) |
                                Statement::Hash(_) => {
                                    self.builder.push_dim_member(statement);
                                },
                                _ => {
                                    let end = self.current_token_pos();
                                    self.push_error(ParseErrorKind::InvalidMemberDefinition(statement.statement, self.builder.is_class_definition()), start, end);
                                }
                            }
                        } else {
                            block.push(statement);
                        }
                    },
                    StatementType::Block => {
                        if self.builder.is_in_module_member_definition() {
                            let end = self.current_token_pos();
                            self.push_error(ParseErrorKind::InvalidMemberDefinition(statement.statement, self.builder.is_class_definition()), start, end);
                        } else {
                            block.push(statement);
                        }
                    },
                    StatementType::Call(location, scope) => {
                        self.builder.push_call_scope(location, scope);
                        block.push(statement);
                    },
                    StatementType::DefDll => {
                        // def_dll定義はグローバルおよび定義した場所に置かれる
                        self.builder.push_def(statement.clone());
                        if ! self.builder.is_in_module_member_definition() {
                            // モジュールメンバ定義以外ならブロックにも追加する
                            block.push(statement);
                        }
                    },
                    StatementType::Expression => {
                        if self.builder.is_in_module_member_definition() {
                            let end = self.current_token_pos();
                            self.push_error(ParseErrorKind::InvalidMemberDefinition(statement.statement, self.builder.is_class_definition()), start, end);
                        } else {
                            block.push(statement);
                        }
                    },
                },
                None => {
                    self.bump_to_next_row();
                    continue;
                },
            }
            self.bump();
        }
        if self.builder.is_in_module_member_definition() {
            self.builder.take_module_members(&mut block);
        }
        block
    }

    fn parse_statement(&mut self, allow_continuation: bool) -> Option<(StatementType, StatementWithRow)> {
        let start = self.current_token_pos();
        let row = self.current_token.pos.row;
        let token = self.current_token.token.clone();
        let (stmttype, statement) = match token {
            Token::Dim => {
                self.builder.set_dim_scope();
                let stmt = self.parse_dim_statement();
                self.builder.reset_dim_scope();
                (StatementType::Dim, stmt?)
            },
            Token::Public => {
                self.builder.set_public_scope();
                let stmt = self.parse_public_statement();
                self.builder.reset_public_scope();
                (StatementType::Public, stmt?)
            },
            Token::Const => {
                self.builder.set_const_scope();
                let stmt = self.parse_const_statement();
                self.builder.reset_const_scope();
                (StatementType::Const, stmt?)
            },
            Token::If => {
                (StatementType::Script, self.parse_if_statement()?)
            },
            Token::IfB => {
                (StatementType::Block, self.parse_if_statement()?)
            },
            Token::Select => {
                (StatementType::Block, self.parse_select_statement()?)
            },
            Token::Print => {
                (StatementType::Script, self.parse_print_statement()?)
            },
            Token::For => {
                self.builder.increase_loop_count();
                let stmt = self.parse_for_statement();
                self.builder.decrease_loop_count();
                (StatementType::Block, stmt?)
            },
            Token::While => {
                self.builder.increase_loop_count();
                let stmt = self.parse_while_statement();
                self.builder.decrease_loop_count();
                (StatementType::Block, stmt?)
            },
            Token::Repeat => {
                self.builder.increase_loop_count();
                let stmt = self.parse_repeat_statement();
                self.builder.decrease_loop_count();
                (StatementType::Block, stmt?)
            },
            Token::Continue => {
                (StatementType::Script, self.parse_continue_statement()?)
            },
            Token::Break => {
                (StatementType::Script, self.parse_break_statement()?)
            },
            Token::Call => {
                let (stmt, location, scope) = self.parse_call_statement()?;
                (StatementType::Call(location, scope), stmt)
            },
            Token::DefDll => {
                (StatementType::DefDll, self.parse_def_dll_statement()?)
            },
            Token::Struct => {
                (StatementType::Definition, self.parse_struct_statement()?)
            },
            Token::HashTable => {
                self.builder.set_dim_scope();
                let stmt = self.parse_hashtable_statement(false);
                self.builder.reset_dim_scope();
                (StatementType::Script, stmt?)
            },
            Token::Hash => {
                let is_public = self.is_next_token(&Token::Public);
                if is_public {
                    self.builder.set_public_scope();
                } else {
                    self.builder.set_dim_scope();
                }
                let stmt = self.parse_hash_statement();
                if is_public {
                    self.builder.reset_public_scope();
                    (StatementType::Public, stmt?)
                } else {
                    self.builder.reset_dim_scope();
                    (StatementType::Script, stmt?)
                }
            },
            Token::Function => {
                self.builder.set_function_scope();
                let stmt = self.parse_function_statement(false, false);
                self.builder.reset_function_scope();
                (StatementType::Definition, stmt?)
            },
            Token::Procedure => {
                self.builder.set_function_scope();
                let stmt = self.parse_function_statement(true, false);
                self.builder.reset_function_scope();
                (StatementType::Definition, stmt?)
            },
            Token::Async => {
                self.builder.set_function_scope();
                let stmt = self.parse_async_function_statement();
                self.builder.reset_function_scope();
                (StatementType::Definition, stmt?)
            },
            Token::Exit => {
                (StatementType::Script, Statement::Exit)
            },
            Token::ExitExit => {
                (StatementType::Script, self.parse_exitexit_statement()?)
            },
            Token::Module => {
                self.builder.set_module_scope(false);
                let stmt = self.parse_module_statement(false);
                self.builder.reset_module_scope();
                (StatementType::Definition, stmt?)
            },
            Token::Class => {
                self.builder.set_module_scope(true);
                let stmt = self.parse_module_statement(true);
                self.builder.reset_module_scope();
                (StatementType::Definition, stmt?)
            },
            Token::TextBlock(is_ex) => {
                self.builder.set_const_scope();
                let stmt = self.parse_textblock_statement(is_ex);
                self.builder.reset_const_scope();
                (StatementType::Const, stmt?)
            },
            Token::With => {
                (StatementType::Block, self.parse_with_statement()?)
            },
            Token::Try => {
                (StatementType::Block, self.parse_try_statement()?)
            },
            Token::Option(ref name,_) => {
                (StatementType::Option, self.parse_option_statement(name)?)
            },
            Token::Enum => {
                self.builder.set_const_scope();
                let stmt = self.parse_enum_statement();
                self.builder.reset_const_scope();
                (StatementType::Const, stmt?)
            },
            Token::Thread => {
                (StatementType::Script, self.parse_thread_statement()?)
            },
            Token::ComErrIgn => {
                (StatementType::Script, Statement::ComErrIgn)
            },
            Token::ComErrRet => {
                (StatementType::Script, Statement::ComErrRet)
            },
            _ => {
                let expression = self.parse_expression_as_statement()?;
                match &expression {
                    Expression::FuncCall { func:_, args:_, is_await:_ } |
                    Expression::Assign(_, _) |
                    Expression::CompoundAssign(_, _, _) => {
                        (StatementType::Expression, Statement::Expression(expression))
                    },
                    _ => {
                        if self.strict_mode {
                            let end = self.current_token_pos();
                            self.push_error(ParseErrorKind::InvalidExpression, start, end);
                            return None;
                        } else {
                            (StatementType::Expression, Statement::Expression(expression))
                        }
                    }
                }
            },
        };
        if allow_continuation || self.is_next_token(&Token::Eol) || self.is_next_token(&Token::Eof) {
            Some((
                stmttype,
                StatementWithRow::new(statement, row, self.lexer.get_line(row), Some(self.script_name()))
            ))
        } else {
            let start = self.next_token.pos;
            let end = self.current_line_end_pos();
            self.push_error(ParseErrorKind::StatementContinuation, start, end);
            None
        }
    }

    fn parse_variable_definition(&mut self, value_required: bool) -> Option<Vec<(Identifier, Expression)>> {
        let mut expressions = vec![];

        loop {
            let var_name = self.parse_identifier(IdentifierType::Declaration)?;
            let expression = if self.is_next_token(&Token::Lbracket) {
                // 配列定義
                // 多次元配列定義の表記は
                // hoge[1][1][1]
                // hoge[][][1]   // 最後以外は省略可能
                // hoge[1, 1, 1] // カンマ区切り
                self.bump();
                let mut index_list = vec![];
                let mut is_multidimensional = false;
                let mut is_comma = false;
                loop {
                    if is_comma {
                        is_comma = false;
                        self.bump();
                        loop {
                            match self.next_token.token {
                                Token::Rbracket => {
                                    // 添字なしで閉じるのはダメ
                                    self.error_on_current_token(ParseErrorKind::SizeRequired);
                                    return None;
                                },
                                Token::Comma => {
                                    // 添字なし
                                    if self.is_current_token(&Token::Comma) {
                                        index_list.push(Expression::Literal(Literal::Empty));
                                    }
                                    self.bump();
                                },
                                _ => {
                                    self.bump();
                                    let e = self.parse_expression(Precedence::Lowest, ExpressionState::Default)?;
                                    index_list.push(e);
                                    match self.next_token.token {
                                        Token::Comma => continue,
                                        Token::Rbracket => {
                                            break;
                                        },
                                        _ => {
                                            self.error_next_token_is_invalid();
                                            return None;
                                        },
                                    }
                                }
                            }
                        }
                    }
                    match self.next_token.token {
                        Token::Rbracket => {
                            // ] の直前が [ なら空
                            let is_empty = self.is_current_token(&Token::Lbracket);
                            self.bump();
                            if ! self.is_next_token(&Token::Lbracket) && is_multidimensional && is_empty {
                                // 多次元で最後の[]が添字なしはダメ
                                self.error_on_current_token(ParseErrorKind::SizeRequired);
                                return None;
                            } else {
                                if is_empty {
                                    index_list.push(Expression::Literal(Literal::Empty));
                                }
                                // 次の [ があったら多次元
                                if self.is_next_token(&Token::Lbracket) {
                                    is_multidimensional = true;
                                    self.bump();
                                } else {
                                    // なければ終了
                                    break;
                                }
                            }
                        },
                        Token::Comma => {
                            // カンマの直前が [ なら空
                            if self.is_current_token(&Token::Lbracket) {
                                index_list.push(Expression::Literal(Literal::Empty));
                            }
                            is_comma = true;
                        },
                        _ => {
                            // 添字
                            self.bump();
                            let e = self.parse_expression(Precedence::Lowest, ExpressionState::Default)?;
                            index_list.push(e);
                            if self.is_next_token(&Token::Comma) {
                                // カンマ区切り形式
                                is_comma = true;
                                continue;
                            }
                        }
                    }
                }

                if ! self.is_next_token(&Token::EqualOrAssign) {
                    // 代入演算子がなければ配列宣言のみ
                    if value_required {
                        let kind = ParseErrorKind::ValueMustBeDefined(var_name);
                        self.error_on_next_token(kind);
                        return None;
                    } else {
                        Expression::Array(Vec::new(), index_list)
                    }
                } else {
                    self.bump();
                    let list = self.parse_expression_list(Token::Eol)?;
                    Expression::Array(list, index_list)
                }
            } else {
                // 変数定義
                // 代入演算子がなければ変数宣言のみ
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    if value_required {
                        let kind = ParseErrorKind::ValueMustBeDefined(var_name);
                        self.error_on_next_token(kind);
                        return None;
                    } else {
                        Expression::Literal(Literal::Empty)
                    }
                } else {
                    self.bump();
                    self.bump();
                    self.parse_expression(Precedence::Lowest, ExpressionState::Default)?
                }
            };
            expressions.push((var_name, expression));

            if self.is_next_token(&Token::Comma) {
                self.bump();
                self.bump();
            } else{
                break;
            }
        }

        Some(expressions)
    }

    fn parse_public_statement(&mut self) -> Option<Statement> {
        match &self.next_token.token {
            Token::HashTable => {
                self.bump();
                self.parse_hashtable_statement(true)
            },
            _ => {
                self.bump();
                self.parse_variable_definition(false)
                    .map(|v| Statement::Public(v))
            },
        }
    }

    fn parse_dim_statement(&mut self) -> Option<Statement> {
        self.bump();
        self.parse_variable_definition(false)
            .map(|v| Statement::Dim(v, self.builder.is_in_loop()))
    }

    fn parse_const_statement(&mut self) -> Option<Statement> {
        self.bump();
        self.parse_variable_definition(true)
            .map(|v| Statement::Const(v))
    }

    fn parse_hash_statement(&mut self) -> Option<Statement> {
        let is_public = if self.is_next_token(&Token::Public) {
            self.bump();
            true
        } else {
            false
        };
        self.bump();

        let name = self.parse_identifier(IdentifierType::Declaration)?;
        let option = if self.is_next_token(&Token::EqualOrAssign) {
            self.bump();
            self.bump();
            match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                Some(e) => Some(e),
                None => return None
            }
        } else {
            None
        };
        self.bump();
        self.bump();
        let mut members = vec![];
        while ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndHash)) {
            if self.is_current_token(&Token::Eol) {
                self.bump();
                continue;
            }
            let expression = self.parse_expression(Precedence::Lowest, ExpressionState::NotAccess);
            if let Some(Expression::Infix(infix, left, right)) = &expression {
                if *infix == Infix::Equal {
                    if let Some(e) = match *left.clone() {
                        Expression::Identifier(i) => Some(Expression::Identifier(i)),
                        Expression::Literal(l) => match l {
                            Literal::Num(_) |
                            Literal::String(_) |
                            Literal::ExpandableString(_) |
                            Literal::Bool(_) |
                            Literal::Empty |
                            Literal::Null |
                            Literal::Nothing => Some(Expression::Literal(l)),
                            _ => None
                        },
                        _ => None
                    } {
                        members.push((e, *right.clone()));
                        self.bump();
                        self.bump();
                        continue;
                    }
                }
            }
            let kind = ParseErrorKind::InvalidHashMemberDefinition(expression);
            self.error_on_current_token(kind);
            return None;
        }

        let hash = HashSugar::new(name, option, is_public, members);
        Some(Statement::Hash(hash))
    }

    fn parse_hashtable_statement(&mut self, is_public: bool) -> Option<Statement> {
        self.bump();
        let mut expressions = vec![];

        if is_public {
            self.builder.set_public_scope();
        } else {
            self.builder.set_dim_scope();
        }

        loop {
            let identifier = self.parse_identifier(IdentifierType::Declaration)?;
            let hash_option = if self.is_next_token(&Token::EqualOrAssign) {
                self.bump();
                self.bump();
                match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => Some(e),
                    None => return None
                }
            } else {
                None
            };

            expressions.push((identifier, hash_option));
            if self.is_next_token(&Token::Comma) {
                self.bump();
                self.bump();
            } else {
                break;
            }
        }
        Some(Statement::HashTbl(expressions, is_public))
    }

    fn parse_print_statement(&mut self) -> Option<Statement> {
        self.bump();
        let has_whitespace = self.current_token.skipped_whitespace;
        let expression = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => if has_whitespace {
                e
            } else {
                let kind = ParseErrorKind::WhitespaceRequiredAfter("print".into());
                self.error_on_current_token(kind);
                return None;
            },
            None => Expression::Literal(Literal::String("".to_string()))
        };

        Some(Statement::Print(expression))
    }


    fn parse_call_statement(&mut self) -> Option<(Statement, ScriptLocation, BuilderScope)> {
        // callしたスクリプトにエラーがあった際の位置情報
        let start = self.current_token.pos;
        let end = self.current_token.get_end_pos();

        let (script, builder, args) = match self.next_token.token.clone() {
            Token::Path(dir, name) => {
                // パス取得
                self.bump();
                // 引数の確認
                let args = if self.is_next_token(&Token::Lparen) {
                    self.bump();
                    // self.bump();
                    match self.parse_expression_list(Token::Rparen) {
                        Some(ve) => ve,
                        None => vec![],
                    }
                } else {
                    vec![]
                };

                let mut path = match dir {
                    Some(dir) => {
                        let path = PathBuf::from(dir);
                        if path.is_absolute() {
                            path
                        } else {
                            let mut parent = self.builder.script_dir();
                            parent.push(path);
                            parent
                        }
                    },
                    None => {
                        self.builder.script_dir()
                    },
                };
                path.push(&name);
                match path.extension() {
                    Some(os_str) => {
                        if let Some(ext) = os_str.to_str() {
                            // uwslファイルならデシリアライズして返す
                            if ext.to_ascii_lowercase().as_str() == "uwsl" {
                                match serializer::load(&path) {
                                    Ok(bin) => match serializer::deserialize(bin){
                                        Ok(program) => {
                                            return Some((Statement::Call(program, args), ScriptLocation::None, BuilderScope::default()));
                                        },
                                        Err(e) => {
                                            let kind = ParseErrorKind::CanNotLoadUwsl(
                                                path.to_string_lossy().to_string(),
                                                e.to_string()
                                            );
                                            self.error_on_current_token(kind);
                                        }
                                    },
                                    Err(e) => {
                                        let kind = ParseErrorKind::CanNotLoadUwsl(
                                            path.to_string_lossy().to_string(),
                                            e.to_string()
                                        );
                                        self.error_on_current_token(kind);
                                    }
                                }
                                return None;
                            }
                        }
                    },
                    _ => {
                        path.set_extension("uws");
                    },
                }
                let script = loop {
                    let script = match get_script(&path) {
                        Ok(s) => s,
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::NotFound {
                                let ext = path.extension();
                                // 拡張子がない場合は.uwsを付けて再挑戦
                                if ext.is_none() {
                                    path.set_extension("uws");
                                    continue;
                                }
                            }
                            let kind = ParseErrorKind::CanNotCallScript(path.to_string_lossy().to_string(), e.to_string());
                            self.error_on_current_token(kind);
                            return None;
                        },
                    };
                    break script;
                };
                let builder = self.builder.new_call_builder(Some(path));
                (script, builder, args)
            },
            Token::Uri(uri) => {
                let maybe_script = match reqwest::blocking::get(&uri) {
                    Ok(response) => if response.status().is_success() {
                        match response.text() {
                            Ok(s) => {
                                let bytes = s.as_bytes();
                                match get_utf8(bytes) {
                                    Ok(s) => {
                                        let re = regex::Regex::new("(\r\n|\r|\n)").unwrap();
                                        let script = re.replace_all(&s, "\r\n").to_string();
                                        Some(script)
                                    },
                                    Err(_) => None,
                                }
                            },
                            Err(_) => None,
                        }
                    } else {
                        None
                    },
                    Err(_) => {
                        None
                    },
                };
                let script = match maybe_script {
                    Some(s) => s,
                    None => {
                        let kind = ParseErrorKind::InvalidCallUri(uri);
                        self.error_on_next_token(kind);
                        return None;
                    },
                };
                self.bump();
                // 引数の確認
                let args = if self.is_next_token(&Token::Lparen) {
                    self.bump();
                    // self.bump();
                    match self.parse_expression_list(Token::Rparen) {
                        Some(ve) => ve,
                        None => vec![],
                    }
                } else {
                    vec![]
                };
                let builder = self.builder.new_uri(uri);
                (script, builder, args)
            },
            _ => {
                self.error_next_token_is_invalid();
                return None;
            }
        };

        let is_already_called = {
            let mut locations = CALLED_FILE_LOCATIONS.lock().unwrap();
            if locations.contains(builder.location_ref()) {
                true
            } else {
                locations.push(builder.location());
                false
            }
        };

        let mut call_parser = Parser::call(Lexer::new(&script), builder, self.strict_mode);
        call_parser.parse_to_builder();
        if is_already_called {
            // すでに呼び出されている場合はグローバル要素を除去
            call_parser.builder.remove_global();
        } else {
            // 初回呼び出し時のみエラー処理とグローバル定義さらいをやる

            if ! call_parser.errors.is_empty() {
                // エラーがあった場合は
                self.push_error(ParseErrorKind::CalledScriptHadError, start, end);
                self.errors.append(&mut call_parser.errors);
            }
            // callのbuilderからグローバル定義をさらう
            self.builder.append_global(&mut call_parser.builder);
        }
        // 実行部分のみでビルド
        let lines = call_parser.lines();
        let (program, location, scope) = call_parser.builder.build_call(lines);
        Some((Statement::Call(program, args), location, scope))
    }

    fn parse_def_dll_statement(&mut self) -> Option<Statement> {
        self.bump();
        let Identifier(name) = self.parse_identifier(IdentifierType::Definition)?;
        let (name, alias) = match &self.next_token.token {
            Token::Lparen => {
                self.bump();
                (name, None)
            },
            Token::Colon => {
                self.bump();
                self.bump();
                let alias = Some(name);
                let Identifier(name) = self.parse_identifier(IdentifierType::Other)?;
                if ! self.bump_to_next_expected_token(Token::Lparen) {
                    return None;
                }
                (name, alias)
            },
            _ => {
                let kind = ParseErrorKind::TokenIsNotOneOfExpectedTokens(vec![
                    Token::Lparen,
                    Token::Colon,
                ]);
                self.error_on_current_token(kind);
                return None;
            }
        };

        self.bump();
        let mut params = Vec::new();
        while ! self.is_current_token_in(vec![Token::Rparen, Token::Eof]) {
            match self.current_token.token {
                Token::Identifier(_) |
                Token::Struct => {
                    let def_dll_param = self.parse_dll_param(false)?;
                    params.push(def_dll_param);
                },
                Token::Ref => {
                    self.bump();
                    match self.current_token.token {
                        Token::Identifier(_) |
                        Token::Struct => {
                            let def_dll_param = self.parse_dll_param(true)?;
                            params.push(def_dll_param);
                        },
                        _ => {
                            self.error_current_token_is_invalid();
                            return None;
                        },
                    }
                },
                // 構造体
                Token::Lbrace => match self.parse_dll_struct() {
                    Some(s) => params.push(s),
                    None => return None,
                },
                // Token::Lbrace | Token::Rbrace => {},
                Token::Comma => {},
                Token::Eol=> {},
                _ => {
                    self.error_current_token_is_invalid();
                    return None;
                },
            }
            self.bump();
        }
        if ! self.is_current_token_expected(Token::Rparen) {
            return None;
        }
        if ! self.bump_to_next_expected_token(Token::Colon) {
            return None;
        }

        // 戻りの型, dllパス
        // 型省略時はVoid返す
        self.bump();
        let (ret_type, path) = match self.current_token.token() {
            // ::パス
            Token::Colon => {
                self.bump();
                if let Token::DllPath(p) = &self.current_token.token {
                    (DllType::Void, p.to_string())
                } else {
                    self.error_next_token_is_invalid();
                    return None;
                }
            },
            // :型:パス
            Token::Identifier(t) => {
                match t.parse() {
                    Ok(t) => {
                        if ! self.is_next_token(&Token::Colon) {
                            self.error_next_token_is_unexpected(Token::Colon);
                            return None;
                        }
                        self.bump();
                        self.bump();
                        if let Token::DllPath(p) = &self.current_token.token {
                            (t, p.to_string())
                        } else {
                            self.error_next_token_is_invalid();
                            return None;
                        }
                    },
                    Err(_) => {
                        let start = self.current_token_pos();
                        let end = self.current_token_end_pos();
                        self.push_error(ParseErrorKind::InvalidDllType(t), start, end);
                        return None;
                    },
                }
            },
            // :パス
            Token::DllPath(p) => {
                (DllType::Void, p)
            },
            _ => {
                self.error_next_token_is_invalid();
                return None;
            }
        };

        Some(Statement::DefDll { name, alias, params, ret_type, path })
    }

    fn parse_dll_struct(&mut self) -> Option<DefDllParam> {
        self.bump();
        let mut s = Vec::new();
        // let mut nested = 0;
        while ! self.is_current_token_in(vec![Token::Eol, Token::Eof]) {
            match self.current_token.token {
                Token::Identifier(_) |
                Token::Struct => {
                    let def_dll_param = self.parse_dll_param(false);
                    if def_dll_param.is_none() {
                        return None;
                    }
                    s.push(def_dll_param.unwrap());
                },
                Token::Ref => {
                    self.bump();
                    match self.current_token.token {
                        Token::Identifier(_) |
                        Token::Struct => {
                            let def_dll_param = self.parse_dll_param(true);
                            if def_dll_param.is_none() {
                                return None;
                            }
                            s.push(def_dll_param.unwrap());
                        },
                        _ => {
                            self.error_current_token_is_invalid();
                            return None;
                        },
                    }
                },
                Token::Lbrace => {
                    let struct_param = self.parse_dll_struct()?;
                    s.push(struct_param);
                },
                Token::Rbrace => break,
                Token::Comma => {},
                _ => {
                    self.error_current_token_is_invalid();
                    return None;
                },
            }
            self.bump();
        }
        if ! self.is_current_token_expected(Token::Rbrace) {
            return None;
        }
        Some(DefDllParam::Struct(s))
    }

    fn parse_dll_param(&mut self, is_ref: bool) -> Option<DefDllParam> {
        let dll_type = match &self.current_token.token {
            Token::Identifier(s) => match s.parse::<DllType>() {
                Ok(t) => t,
                Err(name) => {
                    self.error_on_current_token(ParseErrorKind::InvalidDllType(name));
                    return None;
                },
            },
            Token::Struct => DllType::UStruct,
            _ => {
                self.error_current_token_is_invalid();
                return None;
            },
        };
        if dll_type == DllType::CallBack {
            if ! self.bump_to_next_expected_token(Token::Lparen) {
                return None;
            }
            self.bump(); // ( の次のトークンに移動
            let mut argtypes = vec![];
            loop {
                let t = match &self.current_token.token {
                    Token::Identifier(i) => match DllType::from_str(&i) {
                        Ok(t) => t,
                        Err(name) => {
                            self.error_on_current_token(ParseErrorKind::InvalidDllType(name));
                            return None;
                        },
                    },
                    Token::Rparen => {
                        break;
                    }
                    _ => {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                };
                argtypes.push(t);
                match &self.next_token.token {
                    Token::Comma => {
                        self.bump(); // , に移動
                        self.bump(); // , の次のトークンに移動
                    },
                    Token::Rparen => {
                        self.bump(); // ) に移動
                        break;
                    },
                    _ => {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            }
            let rtype = if self.is_next_token(&Token::Colon) {
                self.bump();
                self.bump();
                match &self.current_token.token {
                    Token::Identifier(i) => match DllType::from_str(&i) {
                        Ok(t) => t,
                        Err(name) => {
                            self.error_on_current_token(ParseErrorKind::InvalidDllType(name));
                            return None;
                        },
                    },
                    _ => {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            } else {
                DllType::Void
            };
            Some(DefDllParam::Callback(argtypes, rtype))
        } else {
            if self.is_next_token(&Token::Lbracket) {
                self.bump(); // [ に移動
                self.bump(); // 数値または ] に移動
                match self.current_token.token() {
                    Token::Rbracket => Some(DefDllParam::Param{ dll_type, is_ref, size: DefDllParamSize::Size(0) }),
                    Token::Num(n) => {
                        self.bump_to_next_expected_token(Token::Rbracket).then_some(())?;
                        Some(DefDllParam::Param { dll_type, is_ref, size: DefDllParamSize::Size(n as usize) })
                    },
                    _ => {
                        // サイズが識別子の場合const文脈
                        self.builder.set_const_scope();
                        let Identifier(name) = self.parse_identifier(IdentifierType::Access)?;
                        self.builder.reset_const_scope();
                        self.bump_to_next_expected_token(Token::Rbracket).then_some(())?;
                        Some(DefDllParam::Param{ dll_type, is_ref, size: DefDllParamSize::Const(name) })
                    },
                }
            } else {
                Some(DefDllParam::Param{dll_type, is_ref, size: DefDllParamSize::None})
            }
        }
    }

    fn parse_struct_statement(&mut self) -> Option<Statement> {
        self.bump();
        let name = self.parse_identifier(IdentifierType::Definition)?;
        self.bump();
        self.bump();

        let mut struct_definition = vec![];
        while ! self.is_current_token_end_of_block() {
            // 空行及びコメント対策
            if self.current_token.token == Token::Eol {
                self.bump();
                continue;
            }
            let Identifier(member) = self.parse_identifier(IdentifierType::Other)?;

            if ! self.bump_to_next_expected_token(Token::Colon) {
                return None;
            }
            self.bump();
            let is_ref = if self.is_current_token(&Token::Ref) {
                self.bump();
                true
            } else {
                false
            };
            let Identifier(member_type) = self.parse_identifier(IdentifierType::Other)
                .map(|Identifier(ident)| Identifier(ident.to_ascii_lowercase()))?;

            let size = if let Token::Lbracket = self.next_token.token {
                self.bump();
                self.bump();
                match (&self.current_token.token, &self.next_token.token) {
                    (Token::Num(n), Token::Rbracket) => {
                        DefDllParamSize::Size(*n as usize)
                    },
                    (_, Token::Rbracket) => {
                        // 構造体メンバサイズはconst文脈
                        self.builder.set_const_scope();
                        let Identifier(name) = self.parse_identifier(IdentifierType::Access)?;
                        self.builder.reset_const_scope();
                        DefDllParamSize::Const(name)
                    },
                    (Token::Num(_), _) => {
                        self.error_next_token_is_invalid();
                        return None;
                    },
                    _ => {
                        self.error_current_token_is_invalid();
                        return None;
                    },
                }
            } else {
                DefDllParamSize::None
            };
            if size != DefDllParamSize::None {
                self.bump();
            }
            struct_definition.push((member, member_type, size, is_ref));
            self.bump();
            self.bump();
        }
        if ! self.is_current_closing_token_expected(BlockEnd::EndStruct) {
            return None;
        }
        Some(Statement::Struct(name, struct_definition))
    }

    fn parse_continue_statement(&mut self) -> Option<Statement> {
        if ! self.builder.is_in_loop() {
            self.error_on_current_token(ParseErrorKind::OutOfLoop(Token::Continue));
            return None;
        }
        match self.parse_continue_break_count() {
            Some(n) => Some(Statement::Continue(n)),
            None => Some(Statement::Continue(1)),
        }
    }

    fn parse_break_statement(&mut self) -> Option<Statement> {
        if ! self.builder.is_in_loop() {
            self.error_on_current_token(ParseErrorKind::OutOfLoop(Token::Break));
            return None;
        }
        match self.parse_continue_break_count() {
            Some(n) => Some(Statement::Break(n)),
            None => Some(Statement::Break(1)),
        }
    }

    fn parse_continue_break_count(&mut self) -> Option<u32> {
        match self.next_token.token {
            Token::Num(n) => {
                self.bump();
                Some(n as u32)
            },
            Token::Eol => None,
            // 単行ifのconsで呼んだ場合
            Token::BlockEnd(BlockEnd::Else) => None,
            _ => {
                self.bump();
                let start = self.current_token.pos;
                let end = self.current_token.get_end_pos();
                self.push_error(ParseErrorKind::LiteralNumberRequired, start, end);
                None
            }
        }
    }

    fn parse_loop_block_statement(&mut self) -> BlockStatement {
        self.builder.increase_loop_count();
        let block = self.parse_block_statement();
        self.builder.decrease_loop_count();
        block
    }

    fn parse_for_statement(&mut self) -> Option<Statement> {
        self.bump();

        let loopvar = self.parse_identifier(IdentifierType::Assignment)?;
        let index_var = if let Token::Comma = self.next_token.token {
            self.bump();
            if let Token::Comma = self.next_token.token {
                None
            } else {
                self.bump();
                let ident = self.parse_identifier(IdentifierType::Assignment)?;
                Some(ident)
            }
        } else {
            None
        };
        let islast_var = if let Token::Comma = self.next_token.token {
            self.bump();
            self.bump();
            let ident = self.parse_identifier(IdentifierType::Assignment)?;
            Some(ident)
        } else {
            None
        };
        match self.next_token.token {
            Token::EqualOrAssign => {
                // for文
                // for-inの特殊記法はNG
                if index_var.is_some() || islast_var.is_some() {
                    self.error_next_token_is_unexpected(Token::EqualOrAssign);
                    return None;
                }
                self.bump();
                self.bump();
                let from = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => e,
                    None => return None
                };
                if ! self.bump_to_next_expected_token(Token::To) {
                    return None;
                }
                self.bump();
                let to = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => e,
                    None => return None
                };
                let step = if self.is_next_token(&Token::Step) {
                    self.bump();
                    self.bump();
                    match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                        Some(e) => Some(e),
                        None => return None
                    }
                } else {
                    None
                };
                self.bump();
                let block = self.parse_loop_block_statement();

                let alt = match &self.current_token.token {
                    Token::BlockEnd(BlockEnd::Next) => None,
                    Token::BlockEnd(BlockEnd::Else) => {
                        self.bump();
                        let alt = self.parse_block_statement();
                        if ! self.is_current_closing_token_expected(BlockEnd::EndFor) {
                            return None;
                        }
                        Some(alt)
                    },
                    Token::BlockEnd(_) => {
                        self.error_current_block_closing_token_was_unexpected(BlockEnd::Next);
                        return None;
                    },
                    _ => {
                        self.error_on_current_token(ParseErrorKind::BlockClosingTokenExpected);
                        return None;
                    },
                };
                Some(Statement::For{loopvar, from, to, step, block, alt})
            },
            Token::In => {
                // for-in
                self.bump();
                self.bump();
                let collection = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => e,
                    None => {
                        self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                        return None;
                    },
                };
                self.bump();
                let block = self.parse_loop_block_statement();

                let alt = match &self.current_token.token {
                    Token::BlockEnd(BlockEnd::Next) => None,
                    Token::BlockEnd(BlockEnd::Else) => {
                        self.bump();
                        let alt = self.parse_block_statement();
                        if ! self.is_current_closing_token_expected(BlockEnd::EndFor) {
                            return None;
                        }
                        Some(alt)
                    },
                    Token::BlockEnd(_) => {
                        self.error_current_block_closing_token_was_unexpected(BlockEnd::Next);
                        return None;
                    },
                    _ => {
                        self.error_on_current_token(ParseErrorKind::BlockClosingTokenExpected);
                        return None;
                    },
                };
                Some(Statement::ForIn{loopvar, index_var, islast_var, collection, block, alt})
            },
            _ => {
                self.error_current_token_is_invalid();
                return None;
            }
        }
    }

    fn parse_while_statement(&mut self) -> Option<Statement> {
        self.bump();
        let expression = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => e,
            None => {
                self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                return None;
            }
        };
        let block = self.parse_loop_block_statement();
        if ! self.is_current_closing_token_expected(BlockEnd::Wend) {
            return None;
        }
        Some(Statement::While(expression, block))
    }

    fn parse_repeat_statement(&mut self) -> Option<Statement> {
        self.bump();
        let block = self.parse_loop_block_statement();
        if ! self.is_current_closing_token_expected(BlockEnd::Until) {
            return None;
        }
        self.bump();
        let row = self.current_token.pos.row;
        let line = self.lexer.get_line(row);
        let expression = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => e,
            None => {
                self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                return None;
            }
        };
        let stmt = StatementWithRow::new(Statement::Expression(expression), row, line, Some(self.script_name()));
        Some(Statement::Repeat(Box::new(stmt), block))
    }

    fn get_with_temp_name(&mut self) -> String {
        self.with_count += 1;
        format!("@with_tmp_{}", self.with_count)
    }

    fn parse_with_statement(&mut self) -> Option<Statement> {
        self.bump();
        let mut with_temp_assignment = None;
        let expression = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => match e {
                Expression::FuncCall{func:_, args:_,is_await:_} => {
                    let with_temp = Expression::Identifier(Identifier(self.get_with_temp_name()));
                    with_temp_assignment = Some(Statement::Expression(Expression::Assign(Box::new(with_temp.clone()), Box::new(e))));
                    with_temp
                },
                _ => e
            },
            None => {
                self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                return None
            },
        };
        let current_with = self.get_current_with();
        self.set_with(Some(expression.clone()));
        let mut block = self.parse_block_statement();
        if with_temp_assignment.is_some() {
            block.insert(0, StatementWithRow::new_non_existent_line(
                with_temp_assignment.unwrap()
            ));
        }
        if ! self.is_current_closing_token_expected(BlockEnd::EndWith) {
            return None;
        }
        self.set_with(current_with);
        Some(Statement::With(Some(expression), block))
    }

    fn parse_try_statement(&mut self) -> Option<Statement> {
        if self.is_next_token(&Token::Eol) {
            self.bump();
        } else {
            let start = self.next_token.pos;
            let end = self.current_line_end_pos();
            self.push_error(ParseErrorKind::InvalidSyntax, start, end);
        }
        let trys = self.parse_block_statement();
        let mut except = None;
        let mut finally = None;
        match self.current_token.token.clone() {
            Token::BlockEnd(BlockEnd::Except) => {
                if self.is_next_token(&Token::Eol) {
                    self.bump();
                } else {
                    let start = self.next_token.pos;
                    let end = self.current_line_end_pos();
                    self.push_error(ParseErrorKind::InvalidSyntax, start, end);
                }
                except = Some(self.parse_block_statement());
            },
            Token::BlockEnd(BlockEnd::Finally) => {},
            _ => {
                let kind = ParseErrorKind::TokenIsNotOneOfExpectedTokens(vec![
                    Token::BlockEnd(BlockEnd::Except),
                    Token::BlockEnd(BlockEnd::Finally)
                ]);
                self.error_on_current_token(kind);
                return None;
            },
        }
        match self.current_token.token.clone() {
            Token::BlockEnd(BlockEnd::Finally) => {
                if self.is_next_token(&Token::Eol) {
                    self.bump();
                } else {
                    let start = self.next_token.pos;
                    let end = self.current_line_end_pos();
                    self.push_error(ParseErrorKind::InvalidSyntax, start, end);
                }
                finally = match self.parse_finally_block_statement() {
                    Ok(b) => Some(b),
                    Err(s) => {
                        self.error_on_current_token(ParseErrorKind::InvalidStatementInFinallyBlock(s));
                        return None;
                    }
                };
            },
            Token::BlockEnd(BlockEnd::EndTry) => {},
            _ => {
                let kind = ParseErrorKind::TokenIsNotOneOfExpectedTokens(vec![
                    Token::BlockEnd(BlockEnd::Finally),
                    Token::BlockEnd(BlockEnd::EndTry),
                ]);
                self.error_on_current_token(kind);
                return None;
            },
        }
        if ! self.is_current_closing_token_expected(BlockEnd::EndTry) {
            return None;
        }

        Some(Statement::Try {trys, except, finally})
    }

    fn parse_finally_block_statement(&mut self) -> Result<BlockStatement, String> {
        self.bump();
        let mut block: BlockStatement  = vec![];

        while ! self.is_current_token_end_of_block() && ! self.is_current_token(&Token::Eof) {
            match self.parse_statement(false) {
                Some((_, s)) => match s.statement {
                    Statement::Exit => return Err("exit".into()),
                    Statement::Continue(_) => return Err("continue".into()),
                    Statement::Break(_) => return Err("break".into()),
                    _ => block.push(s)
                }
                None => {
                    self.bump_to_next_row();
                    continue;
                },
            }
            self.bump();
        }

        Ok(block)
    }

    fn parse_exitexit_statement(&mut self) -> Option<Statement> {
        let (code, bump) = match &self.next_token.token {
            Token::Num(n) => {
                (Some(*n as i32), true)
            },
            Token::Eol | Token::Eof => (Some(0), false),
            _ => {
                self.error_on_next_token(ParseErrorKind::InvalidExitCode);
                (None, false)
            }
        };
        if bump {
            self.bump();
        }
        code.map(|c| Statement::ExitExit(c))
    }

    fn parse_textblock_statement(&mut self, is_ex: bool) -> Option<Statement> {
        self.bump();
        let name = match &self.current_token.token {
            Token::Eol => None,
            _ => {
                let ident = self.parse_identifier(IdentifierType::Declaration);
                self.bump();
                ident
            },
        };
        self.bump();
        let body = if let Token::TextBlockBody(ref body, _) = self.current_token.token {
            body.clone()
        } else {
            self.error_on_current_token(ParseErrorKind::TextBlockBodyIsMissing);
            return None;
        };
        if self.is_next_token(&Token::EndTextBlock) {
            self.bump()
        } else {
            self.error_on_next_token(ParseErrorKind::BlockClosingTokenIsUnexpected(Token::EndTextBlock, self.next_token.token()));
            return None;
        }
        if name.is_some() {
            Some(Statement::TextBlock(name.unwrap(), Literal::TextBlock(body, is_ex)))
        } else {
            // コメントtextblock
            None
        }
    }

    fn parse_expression_as_statement(&mut self) -> Option<Expression> {
        match self.parse_expression(Precedence::Lowest, ExpressionState::StartOfLine) {
            Some(e) => {
                Some(e)
            }
            None => None
        }
    }

    fn parse_option_statement(&mut self, opt_name: &String) -> Option<Statement> {
        // self.bump();
        let name = opt_name.as_str();
        let statement = match name {
            "explicit" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    self.builder.set_option_explicit(true);
                    Statement::Option(OptionSetting::Explicit(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        self.builder.set_option_explicit(b);
                        Statement::Option(OptionSetting::Explicit(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "samestr" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::SameStr(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::SameStr(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "optpublic" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    self.builder.set_option_optpublic(true);
                    Statement::Option(OptionSetting::OptPublic(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        self.builder.set_option_optpublic(b);
                        Statement::Option(OptionSetting::OptPublic(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "optfinally" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::OptFinally(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::OptFinally(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "specialchar" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::SpecialChar(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::SpecialChar(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "shortcircuit" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::ShortCircuit(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::ShortCircuit(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "nostophotkey" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::NoStopHotkey(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::NoStopHotkey(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "topstopform" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::TopStopform(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::TopStopform(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "fixballoon" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::FixBalloon(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::FixBalloon(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            "defaultfont" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Defaultfont(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Defaultfont(s.clone()))
                } else {
                    self.error_current_token_is_invalid();
                    return None;
                }
            },
            "position" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n1) = self.current_token.token {
                    if ! self.bump_to_next_expected_token(Token::Comma) {
                        return None;
                    }
                    if let Token::Num(n2) = self.current_token.token {
                        return Some(Statement::Option(OptionSetting::Position(n1 as i32, n2 as i32)));
                    }
                }
                self.error_current_token_is_invalid();
                return None;
            },
            "logpath" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Logpath(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Logpath(s.clone()))
                } else {
                    self.error_current_token_is_invalid();
                    return None;
                }
            },
            "loglines" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n) = self.current_token.token {
                    Statement::Option(OptionSetting::Loglines(n as i32))
                } else {
                    self.error_current_token_is_invalid();
                    return None;
                }
            },
            "logfile" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n) = self.current_token.token {
                    Statement::Option(OptionSetting::Logfile(n as i32))
                } else {
                    self.error_current_token_is_invalid();
                    return None;
                }
            },
            "dlgtitle" => {
                if ! self.bump_to_next_expected_token(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Dlgtitle(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Dlgtitle(s.clone()))
                } else {
                    self.error_current_token_is_invalid();
                    return None;
                }
            },
            "guiprint" => {
                if self.is_next_token(&Token::EqualOrAssign) {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::GuiPrint(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                } else {
                    Statement::Option(OptionSetting::GuiPrint(true))
                }
            },
            "forcebool" => {
                if self.is_next_token(&Token::EqualOrAssign) {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::GuiPrint(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                } else {
                    Statement::Option(OptionSetting::ForceBool(true))
                }
            },
            "__allow_ie_object__" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::AllowIEObj(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::AllowIEObj(b))
                    } else {
                        self.error_current_token_is_invalid();
                        return None;
                    }
                }
            },
            name => {
                self.error_on_current_token(ParseErrorKind::UnexpectedOption(name.to_string()));
                return None;
            },
        };
        Some(statement)
    }

    fn parse_enum_statement(&mut self) -> Option<Statement> {
        self.bump();
        let Identifier(name) = self.parse_identifier(IdentifierType::Declaration)?;
        let mut u_enum = UEnum::new(&name);

        self.bump();
        self.bump();
        let mut next = 0.0;
        loop {
            if self.is_current_token(&Token::Eol) {
                self.bump();
                continue;
            }
            let Identifier(id) = self.parse_identifier(IdentifierType::Other)?;
            if self.is_next_token(&Token::EqualOrAssign) {
                self.bump();
                self.bump();
                let n = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => match e {
                        Expression::Literal(Literal::Num(n)) => n,
                        _ => {
                            self.error_on_current_token(ParseErrorKind::EnumMemberShouldBeNumber(name, id));
                            return None;
                        },
                    },
                    None => {
                        self.error_on_current_token(ParseErrorKind::EnumValueShouldBeDefined(name, id));
                        return None;
                    },
                };
                // next以下の数値が指定されたらエラー
                if n < next {
                    self.error_on_current_token(ParseErrorKind::EnumValueIsInvalid(name, id, next));
                    return None;
                }
                next = n;
            }
            if u_enum.add(&id, next).is_err() {
                self.error_on_current_token(ParseErrorKind::EnumMemberDuplicated(name, id));
                return None;
            }
            if ! self.bump_to_next_expected_token(Token::Eol) {
                return None;
            }
            self.bump();
            if self.is_current_token_end_of_block() {
                break;
            }
            next += 1.0;
        }
        if ! self.is_current_closing_token_expected(BlockEnd::EndEnum) {
            return None;
        }
        Some(Statement::Enum(name, u_enum))
    }

    fn parse_thread_statement(&mut self) -> Option<Statement> {
        self.bump();
        let expression = self.parse_expression(Precedence::Lowest, ExpressionState::Default);
        match expression {
            Some(Expression::FuncCall{func:_,args:_,is_await:_}) => Some(Statement::Thread(expression.unwrap())),
            _ => {
                self.error_on_current_token(ParseErrorKind::InvalidThreadCall);
                None
            }
        }
    }

    /// is_sol: 行の始めかどうか
    fn parse_expression(&mut self, precedence: Precedence, state: ExpressionState) -> Option<Expression> {
        let start = self.current_token_pos();
        let mut ident_pos: Option<(Identifier, Position, Position)> = None;
        // prefix
        let mut left = match self.current_token.token {
            Token::Empty => if state.is_start_of_line() && self.strict_mode {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::Literal(Literal::Empty)
            },
            Token::Null => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::Literal(Literal::Null)
            },
            Token::Nothing => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::Literal(Literal::Nothing)
            },
            Token::NaN => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::Literal(Literal::NaN)
            },
            Token::Num(n) => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::Literal(Literal::Num(n))
            },
            Token::Hex(_) => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                self.parse_hex_expression()?
            },
            Token::ExpandableString(_) |
            Token::String(_) => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                self.parse_string_expression()?
            },
            Token::Bool(_) => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                self.parse_bool_expression()?
            },
            Token::Lbracket => if state.is_start_of_line() && self.strict_mode {
                self.error_current_token_is_invalid();
                return None;
            } else {
                self.parse_array_expression()?
            },
            Token::Bang |
            Token::Minus |
            Token::Plus => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                self.parse_prefix_expression()?
            },
            Token::Lparen => self.parse_grouped_expression()?,
            Token::Function => {
                self.builder.set_anon_scope();
                let expr = self.parse_function_expression(false);
                self.builder.reset_anon_scope();
                expr?
            },
            Token::Procedure => {
                self.builder.set_anon_scope();
                let expr = self.parse_function_expression(true);
                self.builder.reset_anon_scope();
                expr?
            },
            Token::Pipeline => {
                self.builder.set_anon_scope();
                let expr = self.parse_lambda_function_expression();
                self.builder.reset_anon_scope();
                expr?
            },
            Token::Await => return self.parse_await_func_call_expression(),
            Token::Eol => {
                return None;
            },
            Token::Period => {
                let e = self.parse_with_dot_expression();
                if state.is_start_of_line() && e.is_some() {
                    if let Some(e) = self.parse_assignment(e.clone().unwrap(), start) {
                        return Some(e);
                    }
                }
                e?
            },
            Token::UObject(ref s) => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::UObject(s.clone())
            },
            Token::UObjectNotClosing => {
                self.error_on_current_token(ParseErrorKind::InvalidUObjectEnd);
                return None
            },
            Token::ComErrFlg => if state.is_start_of_line() && self.strict_mode  {
                self.error_current_token_is_invalid();
                return None;
            } else {
                Expression::ComErrFlg
            },
            Token::Ref => if state.is_start_of_line() {
                self.error_current_token_is_invalid();
                return None;
            } else {
                // COMメソッドの引数にvarが付く場合
                // var <Identifier> とならなければいけない
                self.bump();
                match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    Some(e) => return Some(Expression::RefArg(Box::new(e))),
                    None => {
                        self.error_on_current_token(ParseErrorKind::MissingIdentifierAfterVar);
                        return None;
                    }
                }
            },
            _ => {
                if state.is_sol_or_lambda() {
                    // 次のトークンを確認する
                    match &self.next_token.token {
                        // := による代入式
                        Token::Assign |
                        // ドット呼び出し
                        Token::Period |
                        // 関数呼び出し
                        Token::Lparen |
                        // インデックス呼び出し
                        Token::Lbracket => {
                            let identifier = self.parse_identifier(IdentifierType::NotSure)?;
                            ident_pos = Some((identifier.clone(), self.current_token_pos(), self.current_token_end_pos()));
                            Expression::Identifier(identifier)
                        },
                        // 代入文
                        Token::EqualOrAssign |
                        Token::AddAssign |
                        Token::SubtractAssign |
                        Token::MultiplyAssign |
                        Token::DivideAssign => {
                            let identifier = self.parse_identifier(IdentifierType::Assignment)?;
                            return self.parse_assignment(Expression::Identifier(identifier), start);
                        }
                        _ => {
                            if ! self.strict_mode || state.is_lambda() {
                                let identifier = self.parse_identifier(IdentifierType::NotSure)?;
                                ident_pos = Some((identifier.clone(), self.current_token_pos(), self.current_token_end_pos()));
                                Expression::Identifier(identifier)
                            } else {
                                self.error_current_token_is_invalid();
                                return None;
                            }
                        }
                    }
                } else {
                    let identifier = self.parse_identifier(IdentifierType::NotSure)?;
                    ident_pos = state.could_be_access().then_some((identifier.clone(), self.current_token_pos(), self.current_token_end_pos()));
                    Expression::Identifier(identifier)
                }
                // self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                // return None;
            },
        };

        // infix
        while (
            ! self.is_next_token(&Token::Semicolon) ||
            ! self.is_next_token(&Token::Eol)
        ) && precedence < self.next_token_precedence() {
            // if left.is_none() {
            //     return None;
            // }
            match self.next_token.token {
                Token::EqualOrAssign => {
                    left = if state.is_start_of_line() && self.strict_mode  {
                        if let Expression::FuncCall { func, args:_, is_await: false } = &left {
                            if let Expression::DotCall(_, _) = func.as_ref() {
                                return self.parse_assignment(left, start);
                            } else {
                                self.bump();
                                self.error_current_token_is_invalid();
                                return None;
                            }
                        } else {
                            self.bump();
                            self.error_current_token_is_invalid();
                            return None;
                        }
                    } else {
                        self.bump();
                        self.parse_infix_expression(left)?
                    };
                },
                Token::Plus |
                Token::Minus |
                Token::Slash |
                Token::Asterisk |
                Token::Equal |
                Token::NotEqual |
                Token::LessThan |
                Token::LessThanEqual |
                Token::GreaterThan |
                Token::GreaterThanEqual |
                Token::And |
                Token::Or |
                Token::Xor |
                Token::AndL |
                Token::OrL |
                Token::XorL |
                Token::AndB |
                Token::OrB |
                Token::XorB |
                Token::Mod => {
                    self.bump();
                    left = if state.is_start_of_line() && self.strict_mode  {
                        self.error_current_token_is_invalid();
                        return None;
                    } else {
                        self.parse_infix_expression(left)?
                    };
                },
                Token::To |
                Token::Step |
                Token::In => {
                    self.bump();
                    left = self.parse_infix_expression(left)?;
                },
                Token::Assign => {
                    left = self.parse_assign_expression(left, start)?;
                    if let Some((Identifier(name), start, end)) = ident_pos.to_owned() {
                        self.builder.set_assignee_name(&name, start, end);
                        ident_pos = None;
                    }
                },
                Token::Lbracket => {
                    self.bump();
                    left = {
                        let index = match self.parse_index_expression(left) {
                            Some(e) => e,
                            None => {
                                self.error_on_next_token(ParseErrorKind::MissingIndex);
                                return None;
                            },
                        };
                        if state.is_sol_or_lambda() {
                            if let Some(e) = self.parse_assignment(index.clone(), start) {
                                return Some(e);
                            }
                        }
                        index
                    }
                },
                Token::Lparen => {
                    self.bump();
                    left = self.parse_function_call_expression(left, false)?;
                },
                Token::Question => {
                    self.bump();
                    left = self.parse_ternary_operator_expression(left)?;
                },
                Token::Period => {
                    self.bump();
                    left = {
                        let dotcall = self.parse_dotcall_expression(left)?;
                        if state.is_sol_or_lambda() {
                            if let Some(e) = self.parse_assignment(dotcall.clone(), start) {
                                return Some(e);
                            }
                        }
                        dotcall
                    }
                },
                // _ => return left
                _ => break,
            }
        }
        if let Some((Identifier(name), start, end)) = ident_pos {
            self.builder.set_access_name(&name, start, end);
        }
        Some(left)
    }

    fn parse_identifier(&mut self, r#type: IdentifierType) -> Option<Identifier> {
        let start = self.current_token_pos();
        let end = self.current_token_end_pos();
        let identifier = match self.current_token.token() {
            Token::Identifier(ident) => Identifier(ident),
            token => self.token_to_identifier(&token)?
        };
        let name = &identifier.0;
        match r#type {
            IdentifierType::Declaration => {
                self.builder.set_declared_name(name, start, end);
            },
            IdentifierType::Assignment => {
                self.builder.set_assignee_name(name, start, end);
            },
            IdentifierType::Access => {
                self.builder.set_access_name(name, start, end);
            },
            IdentifierType::Parameter => {
                self.builder.set_param(name, start, end);
            },
            IdentifierType::Definition => {
                self.builder.set_definition_name(name, start, end);
            }
            IdentifierType::Other |
            IdentifierType::NotSure => {
                /* 登録しない */
            }
        }
        Some(identifier)
    }


    fn token_to_identifier(&mut self, token: &Token) -> Option<Identifier> {
        let identifier = match token {
            Token::Call |
            Token::Mod |
            Token::And | Token::AndL | Token::AndB |
            Token::Or | Token::OrL | Token::OrB |
            Token::Xor | Token::XorL | Token::XorB |
            Token::Bool(_) |
            Token::Null |
            Token::Empty |
            Token::Nothing |
            Token::Async |
            Token::Await |
            Token::ComErrFlg |
            Token::NaN => {
                self.error_on_current_token(ParseErrorKind::ReservedKeyword(token.clone()));
                return None;
            },
            Token::Blank |
            Token::Eof |
            Token::Eol => {
                self.error_on_current_token(ParseErrorKind::IdentifierExpected);
                return None;
            },
            Token::Num(_) |
            Token::Hex(_) |
            Token::String(_) |
            Token::ExpandableString(_) |
            Token::UObject(_) |
            Token::UObjectNotClosing |
            Token::Plus |
            Token::Minus |
            Token::Bang |
            Token::Asterisk |
            Token::Slash |
            Token::AddAssign |
            Token::SubtractAssign |
            Token::MultiplyAssign |
            Token::DivideAssign |
            Token::Assign |
            Token::EqualOrAssign |
            Token::Equal |
            Token::NotEqual |
            Token::LessThan |
            Token::LessThanEqual |
            Token::GreaterThan |
            Token::GreaterThanEqual |
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
            Token::BackSlash |
            Token::ColonBackSlash |
            Token::Option(_,_) |
            Token::Comment |
            Token::Ref |
            Token::Variadic |
            Token::Pipeline |
            Token::Uri(_) |
            Token::Path(_, _) |
            Token::DllPath(_) |
            Token::Arrow => {
                self.error_on_current_token(ParseErrorKind::TokenCanNotBeUsedAsIdentifier);
                return None
            },
            Token::Illegal(c) => {
                let kind = ParseErrorKind::IllegalCharacter(*c);
                self.error_on_current_token(kind);
                return None;
            },
            Token::Print |
            Token::Dim |
            Token::Public |
            Token::Const |
            Token::Thread |
            Token::HashTable |
            Token::DefDll |
            Token::If |
            Token::IfB |
            Token::Then |
            Token::While |
            Token::Repeat |
            Token::For |
            Token::To |
            Token::In |
            Token::Step |
            Token::Select |
            Token::Continue |
            Token::Break |
            Token::With |
            Token::Try |
            Token::TextBlock(_) |
            Token::EndTextBlock |
            Token::TextBlockBody(_,_) |
            Token::Function |
            Token::Procedure |
            Token::Module |
            Token::Class |
            Token::Enum |
            Token::Struct |
            Token::Hash |
            Token::BlockEnd(_) |
            Token::ComErrIgn |
            Token::ComErrRet |
            Token::Exit |
            Token::ExitExit => Identifier(token.to_string()),
            Token::Identifier(ref i) => Identifier(i.clone()),
        };
        Some(identifier)
    }

    fn parse_with_dot_expression(&mut self) -> Option<Expression> {
        match self.get_current_with() {
            Some(e) => self.parse_dotcall_expression(e),
            None => {
                self.error_on_current_token(ParseErrorKind::OutOfWith);
                return None;
            }
        }
    }

    fn parse_hex_expression(&mut self) -> Option<Expression> {
        if let Token::Hex(ref s) = self.current_token.token {
            match u64::from_str_radix(s, 16) {
                Ok(u) => Some(Expression::Literal(Literal::Num(u as i64 as f64))),
                Err(_) => {
                    self.error_on_current_token(ParseErrorKind::InvalidHexNumber(s.to_string()));
                    None
                }
            }
        } else {
            None
        }
    }

    fn parse_string_expression(&mut self) -> Option<Expression> {
        match self.current_token.token {
            Token::String(ref s) => Some(
                Expression::Literal(Literal::String(s.clone()))
            ),
            Token::ExpandableString(ref s) => Some(
                Expression::Literal(Literal::ExpandableString(s.clone()))
            ),
            _ => None
        }
    }

    fn parse_bool_expression(&mut self) -> Option<Expression> {
        match self.current_token.token {
            Token::Bool(v) => Some(
                Expression::Literal(Literal::Bool(v == true))
            ),
            _ => None
        }
    }

    fn parse_array_expression(&mut self) -> Option<Expression> {
        match self.parse_expression_list(Token::Rbracket) {
            Some(list) => Some(
                Expression::Literal(Literal::Array(list))
            ),
            None => None
        }
    }

    fn skip_next_eol(&mut self) {
        while self.is_next_token(&Token::Eol) {
            self.bump();
        }
    }

    fn parse_expression_list(&mut self, end: Token) -> Option<Vec<Expression>> {
        let mut list:Vec<Expression> = vec![];
        let skip_eol = end != Token::Eol;

        if self.is_next_token(&end) {
            self.bump();
            return Some(list);
        }

        if skip_eol {self.skip_next_eol();}
        self.bump();

        match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => list.push(e),
            None => return None
        }

        while self.is_next_token(&Token::Comma) {
            self.bump();
            if skip_eol {self.skip_next_eol();}
            self.bump();
            match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                Some(e) => list.push(e),
                None => return None
            }
            if skip_eol {self.skip_next_eol();}
        }

        if end == Token::Eol {
            if ! self.is_next_token(&end) && ! self.is_next_token(&Token::Eof) {
                self.error_next_token_is_invalid();
                return None;
            }
        } else {
            if ! self.bump_to_next_expected_token(end) {
                return None;
            }
        }

        Some(list)
    }

    fn parse_assignment(&mut self, left: Expression, start: Position) -> Option<Expression> {
        match &self.next_token.token {
            Token::EqualOrAssign => self.parse_assign_expression(left, start),
            Token::AddAssign => self.parse_compound_assign_expression(left, Token::AddAssign, start),
            Token::SubtractAssign => self.parse_compound_assign_expression(left, Token::SubtractAssign, start),
            Token::MultiplyAssign => self.parse_compound_assign_expression(left, Token::MultiplyAssign, start),
            Token::DivideAssign => self.parse_compound_assign_expression(left, Token::DivideAssign, start),
            _ => None
        }
    }

    fn parse_assign_expression(&mut self, left: Expression, start: Position) -> Option<Expression> {
        if left.is_not_assignable() {
            self.push_error(ParseErrorKind::InvalidAssignment, start, self.current_token_end_pos());
            return None;
        }

        self.bump();
        self.bump();

        let right = self.parse_expression(Precedence::Lowest, ExpressionState::Default)?;
        Some(Expression::Assign(Box::new(left), Box::new(right)))
    }

    fn parse_compound_assign_expression(&mut self, left: Expression, token: Token, start: Position) -> Option<Expression> {
        if left.is_not_assignable() {
            self.push_error(ParseErrorKind::InvalidAssignment, start, self.current_token_end_pos());
            return None;
        }

        self.bump();
        self.bump();

        let right = self.parse_expression(Precedence::Lowest, ExpressionState::Default)?;
        let infix = match token {
            Token::AddAssign => Infix::Plus,
            Token::SubtractAssign => Infix::Minus,
            Token::MultiplyAssign => Infix::Multiply,
            Token::DivideAssign => Infix::Divide,
            _ => unreachable!()
        };

        Some(Expression::CompoundAssign(Box::new(left), Box::new(right), infix))
    }

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let prefix = match self.current_token.token {
            Token::Bang => Prefix::Not,
            Token::Minus => Prefix::Minus,
            Token::Plus => Prefix::Plus,
            _ => return None,
        };
        self.bump();

        match self.parse_expression(Precedence::Prefix, ExpressionState::Default) {
            Some(e) => Some(Expression::Prefix(prefix, Box::new(e))),
            None => None
        }
    }

    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        let infix = match self.current_token.token {
            Token::Plus => Infix::Plus,
            Token::Minus => Infix::Minus,
            Token::Slash => Infix::Divide,
            Token::Asterisk => Infix::Multiply,
            Token::Equal => Infix::Equal,
            Token::EqualOrAssign => Infix::Equal,
            Token::NotEqual => Infix::NotEqual,
            Token::LessThan => Infix::LessThan,
            Token::LessThanEqual => Infix::LessThanEqual,
            Token::GreaterThan => Infix::GreaterThan,
            Token::GreaterThanEqual => Infix::GreaterThanEqual,
            Token::And => Infix::And,
            Token::Or => Infix::Or,
            Token::Xor => Infix::Xor,
            Token::AndL => Infix::AndL,
            Token::OrL => Infix::OrL,
            Token::XorL => Infix::XorL,
            Token::AndB => Infix::AndB,
            Token::OrB => Infix::OrB,
            Token::XorB => Infix::XorB,
            Token::Mod => Infix::Mod,
            Token::Assign => Infix::Assign,
            _ => return None
        };
        let precedence = self.current_token_precedence();
        self.bump();

        match self.parse_expression(precedence, ExpressionState::Default) {
            Some(e) => Some(Expression::Infix(infix, Box::new(left), Box::new(e))),
            None => None
        }
    }

    fn parse_index_expression(&mut self, left: Expression) -> Option<Expression> {
        self.bump();
        let index = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => e,
            None => return None
        };
        let hash_enum = if self.is_next_token(&Token::Comma) {
            self.bump();
            self.bump();
            match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                Some(e) => Some(e),
                None => return None
            }
        } else {
            None
        };
        if ! self.bump_to_next_expected_token(Token::Rbracket) {
            return None;
        }

        Some(Expression::Index(Box::new(left), Box::new(index), Box::new(hash_enum)))
    }

    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.bump();
        let expression = self.parse_expression(Precedence::Lowest, ExpressionState::Default);
        if ! self.bump_to_next_expected_token(Token::Rparen) {
            None
        } else {
            expression
        }
    }

    fn parse_dotcall_expression(&mut self, left: Expression) -> Option<Expression> {
        self.bump();
        let identifier = self.parse_identifier(IdentifierType::Other)?;
        let member = Expression::Identifier(identifier);
        Some(Expression::DotCall(Box::new(left), Box::new(member)))
    }

    fn parse_if_statement(&mut self) -> Option<Statement> {
        self.bump();
        let condition = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => e,
            None => return None
        };
        if self.is_next_token(&Token::Then) {
            self.bump();
        }
        if ! self.is_next_token(&Token::Eol) && ! self.is_next_token(&Token::Eof) {
            // eolじゃなかったら単行IF
            // if condition then consequence [else alternative]
            self.bump();
            let (_, consequence) = self.parse_statement(true)?;
            let alternative = if self.is_next_token(&Token::BlockEnd(BlockEnd::Else)) {
                self.bump();
                self.bump();
                let (_, s) = self.parse_statement(false)?;
                Some(s)
            } else {
                None
            };
            return Some(Statement::IfSingleLine {
                condition,
                consequence: Box::new(consequence),
                alternative: Box::new(alternative)
            });
        }

        // 複数行IF
        // if condition then
        //   consequence
        // else
        //   alternative
        // endif
        let consequence = self.parse_block_statement();

        if self.is_current_token(&Token::BlockEnd(BlockEnd::EndIf)) {
            return Some(Statement::If {
                condition,
                consequence,
                alternative: None
            });
        }

        if self.is_current_token(&Token::BlockEnd(BlockEnd::Else)) {
            let alternative:Option<BlockStatement> = Some(self.parse_block_statement());
            if ! self.is_current_closing_token_expected(BlockEnd::EndIf) {
                return None;
            }
            return Some(Statement::If {
                condition,
                consequence,
                alternative
            });
        }

        let mut alternatives = vec![];
        while self.is_current_token_in(vec![Token::BlockEnd(BlockEnd::Else), Token::BlockEnd(BlockEnd::ElseIf)]) {
            if self.is_current_token(&Token::BlockEnd(BlockEnd::Else)) {
                alternatives.push(
                    (None, self.parse_block_statement())
                );
                // break;
            } else {
                if self.is_current_token(&Token::BlockEnd(BlockEnd::ElseIf)) {
                    self.bump();
                    let row = self.current_token.pos.row;
                    let line = self.lexer.get_line(row);
                    let elseifcond = self.parse_expression(Precedence::Lowest, ExpressionState::Default)?;
                    if self.is_next_token(&Token::Then) {
                        self.bump();
                    }
                    let condstmt = StatementWithRow::new(Statement::Expression(elseifcond), row, line, Some(self.script_name()));
                    alternatives.push((Some(condstmt), self.parse_block_statement()));
                }
            }
        }
        if ! self.is_current_closing_token_expected(BlockEnd::EndIf) {
            return None;
        }
        Some(Statement::ElseIf {
            condition,
            consequence,
            alternatives
        })

    }

    fn parse_select_statement(&mut self) -> Option<Statement> {
        self.bump();
        let expression = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => e,
            None => {
                self.error_on_current_token(ParseErrorKind::ExpressionIsExpected);
                return None;
            }
        };
        let mut cases = vec![];
        let mut default = None;
        self.bump();
        self.bump();
        while self.is_current_token_in(vec![Token::Eol, Token::BlockEnd(BlockEnd::Case), Token::BlockEnd(BlockEnd::Default)]) {
            match self.current_token.token {
                Token::BlockEnd(BlockEnd::Case) => {
                    let case_values = match self.parse_expression_list(Token::Eol) {
                        Some(list) => list,
                        None => return None
                    };
                    cases.push((
                        case_values,
                        self.parse_block_statement()
                    ));
                },
                Token::BlockEnd(BlockEnd::Default) => {
                    self.bump();
                    default = Some(self.parse_block_statement());
                },
                Token::Eol => {
                    self.bump();
                }
                _ => break,
            }
        }
        if ! self.is_current_closing_token_expected(BlockEnd::Selend) {
            return None;
        }
        Some(Statement::Select {expression, cases, default})
    }

    fn parse_async_function_statement(&mut self) -> Option<Statement> {
        self.bump();
        match self.current_token.token {
            Token::Function => self.parse_function_statement(false, true),
            Token::Procedure => self.parse_function_statement(true, true),
            _ => {
                self.error_on_current_token(ParseErrorKind::FunctionRequiredAfterAsync);
                return None;
            },
        }
    }

    /// 関数定義の解析
    fn parse_function_statement(&mut self, is_proc: bool, is_async: bool) -> Option<Statement> {

        self.bump();
        let name = self.parse_identifier(IdentifierType::Definition)?;

        self.builder.set_result_as_param();

        let params = if self.is_next_token(&Token::Lparen) {
            self.bump();
            self.parse_function_parameters(Token::Rparen)?
        } else {
            vec![]
        };

        self.bump();
        let body = self.parse_block_statement();
        if ! self.is_current_closing_token_expected(BlockEnd::Fend) {
            return None;
        }

        Some(Statement::Function{name, params, body, is_proc, is_async})
    }


    fn parse_module_statement(&mut self, is_class: bool) -> Option<Statement> {
        let start = self.current_token.pos;
        let end = self.current_token.get_end_pos();
        let (end_token, blockend) = if is_class {
            (Token::BlockEnd(BlockEnd::EndClass), BlockEnd::EndClass)
        } else {
            (Token::BlockEnd(BlockEnd::EndModule), BlockEnd::EndModule)
        };
        self.bump();
        let identifier = self.parse_identifier(IdentifierType::Definition)?;
        self.bump();

        let members = self.parse_block_statement();

        let has_constructor = members.iter()
        .find(|s| {
            if let Statement::Function { name, params:_, body:_, is_proc: true, is_async:_ } = &s.statement {
                    name.0.eq_ignore_ascii_case(&identifier.0)
                } else {
                    false
                }
            })
            .is_some();

        if ! self.is_current_token(&end_token) {
            self.error_current_block_closing_token_was_unexpected(blockend);
            return None;
        }

        if is_class {
            if has_constructor {
                Some(Statement::Class(identifier, members))
            } else {
                self.push_error(ParseErrorKind::ClassHasNoConstructor(identifier), start, end);
                None
            }
        } else {
            Some(Statement::Module(identifier, members))
        }
    }

    fn parse_ternary_operator_expression(&mut self, left: Expression) -> Option<Expression> {

        self.bump();
        let consequence = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => Box::new(e),
            None => return None
        };

        if ! self.bump_to_next_expected_token(Token::Colon) {
            return None;
        }
        self.bump();
        let alternative = match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(e) => Box::new(e),
            None => return None
        };

        Some(Expression::Ternary{
            condition: Box::new(left),
            consequence,
            alternative
        })
    }

    fn parse_function_expression(&mut self, is_proc: bool) -> Option<Expression> {
        if ! self.bump_to_next_expected_token(Token::Lparen) {
            return None;
        }
        self.builder.set_result_as_param();
        let params = self.parse_function_parameters(Token::Rparen)?;
        let body = self.parse_block_statement();

        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Fend)) {
            self.error_current_block_closing_token_was_unexpected(BlockEnd::Fend);
            return None;
        }

        Some(Expression::AnonymusFunction {params, body, is_proc})
    }

    fn parse_lambda_function_expression(&mut self) -> Option<Expression> {
        self.builder.set_result_as_param();
        let params = if self.is_next_token(&Token::Arrow) {
            // 引数なし
            self.bump();
            vec![]
        } else {
            self.parse_function_parameters(Token::Arrow)?
        };
        self.bump(); // skip =>

        let mut body = vec![];
        loop {
            let optexpr = self.parse_expression(Precedence::Lowest, ExpressionState::Lambda);
            if optexpr.is_none() {
                return None;
            }

            let row = self.next_token.pos.row;
            if self.is_next_token(&Token::Pipeline) {
                let e = optexpr.unwrap();
                let assign = Expression::Assign(
                    Box::new(Expression::Identifier(Identifier("result".into()))),
                    Box::new(e)
                );
                body.push(StatementWithRow::new(
                    Statement::Expression(assign),
                    row,
                    self.lexer.get_line(row),
                    Some(self.script_name())
                ));
                break;
            } else if self.is_next_token(&Token::Eol) {
                body.push(StatementWithRow::new(
                    Statement::Expression(optexpr.unwrap()),
                    row,
                    self.lexer.get_line(row),
                    Some(self.script_name())
                ));
            } else {
                self.error_next_token_is_invalid();
                return None
            }
            self.bump();
            self.bump();
        }
        self.bump();

        Some(Expression::AnonymusFunction {params, body, is_proc: false})
    }

    fn parse_function_parameters(&mut self, end_token: Token) -> Option<Vec<FuncParam>> {
        let mut params = vec![];

        self.skip_next_eol();
        if self.is_next_token(&Token::Rparen) {
            self.bump();
            return Some(params);
        }
        let mut with_default_flg = false;
        let mut variadic_flg = false;
        self.skip_next_eol();
        self.bump();
        loop {
            match self.parse_param() {
                Some(param) => {
                    match &param.kind {
                        ParamKind::Identifier |
                        ParamKind::Reference => {
                            if with_default_flg {
                                self.error_on_current_token(ParseErrorKind::ParameterShouldBeDefault(param.name()));
                                return None;
                            } else if variadic_flg {
                                self.error_on_current_token(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(param.name()));
                                return None;
                            }
                        },
                        ParamKind::Default(_) => if variadic_flg {
                            self.error_on_current_token(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(param.name()));
                            return None;
                        } else {
                            with_default_flg = true;
                        },
                        ParamKind::Variadic => if with_default_flg {
                            self.error_on_current_token(ParseErrorKind::ParameterShouldBeDefault(param.name()));
                            return None;
                        } else if variadic_flg {
                            self.error_on_current_token(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(param.name()));
                            return None;
                        } else {
                            variadic_flg = true;
                        },
                        ParamKind::Dummy => continue,
                    }
                    params.push(param);
                },
                None => return None
            }
            self.skip_next_eol();
            if self.is_next_token(&Token::Comma) {
                self.bump();
                self.skip_next_eol();
                self.bump();
            } else {
                break;
            }
        }
        if ! self.bump_to_next_expected_token(end_token) {
            return None;
        }
        Some(params)
    }

    fn parse_param(&mut self) -> Option<FuncParam> {
        match self.current_token.token() {
            Token::Ref => {
                self.bump();
                let Identifier(name) = self.parse_identifier(IdentifierType::Parameter)?;
                let kind= if self.is_next_token(&Token::Lbracket) {
                    self.bump();
                    if self.bump_to_next_expected_token(Token::Rbracket) {
                        while self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if ! self.bump_to_next_expected_token(Token::Rbracket) {
                                return None;
                            }
                        }
                        ParamKind::Reference
                    } else {
                        return None;
                    }
                } else {
                    ParamKind::Reference
                };
                let param_type = if self.is_next_token(&Token::Colon) {
                    self.bump(); // : に移動
                    self.parse_param_type()?
                } else {
                    ParamType::Any
                };
                Some(FuncParam::new_with_type(Some(name), kind, param_type))
            },
            Token::Variadic => {
                self.bump();
                let Identifier(name) = self.parse_identifier(IdentifierType::Parameter)?;
                Some(FuncParam::new(Some(name), ParamKind::Variadic))
            },
            _ => {
                let Identifier(name) = self.parse_identifier(IdentifierType::Parameter)?;
                let (kind, param_type) = if self.is_next_token(&Token::Lbracket) {
                    // 配列引数定義
                    self.bump();
                    let k = if self.bump_to_next_expected_token(Token::Rbracket) {
                        while self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if !self.bump_to_next_expected_token(Token::Rbracket) {
                                return None;
                            }
                        }
                        ParamKind::Identifier
                    } else {
                        return None;
                    };
                    let t = if self.is_next_token(&Token::Colon) {
                        self.bump(); // : に移動
                        self.parse_param_type()?
                    } else {
                        ParamType::Any
                    };
                    (k, t)
                } else {
                    // 型指定の有無
                    let t = if self.is_next_token(&Token::Colon) {
                        self.bump(); // : に移動
                        self.parse_param_type()?
                    } else {
                        ParamType::Any
                    };
                    // デフォルト値の有無
                    let k = if self.is_next_token(&Token::EqualOrAssign) {
                        self.bump(); // = に移動
                        let e = if self.is_next_token(&Token::Comma) || self.is_next_token(&Token::Rparen) {
                            // 代入する値を省略した場合はEmptyが入る
                            Expression::Literal(Literal::Empty)
                        } else {
                            self.bump();
                            self.builder.set_default_param();
                            let expr = self.parse_expression(Precedence::Lowest, ExpressionState::Default);
                            self.builder.reset_default_param();
                            expr?
                        };
                        ParamKind::Default(e)
                    } else {
                        ParamKind::Identifier
                    };
                    (k, t)
                };
                Some(FuncParam::new_with_type(Some(name), kind, param_type))
            }
        }
    }

    fn parse_param_type(&mut self) -> Option<ParamType> {
        let next = self.next_token.token();
        match self.token_to_identifier(&next) {
            Some(Identifier(name)) => {
                self.bump();
                Some(ParamType::from(name))
            },
            None => {
                self.error_current_token_is_invalid();
                None
            }
        }
    }

    fn parse_await_func_call_expression(&mut self) -> Option<Expression> {
        self.bump();
        match self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
            Some(Expression::FuncCall{func,args,is_await:_}) => {
                Some(Expression::FuncCall{func,args,is_await:true})
            },
            _ => {
                self.error_on_current_token(ParseErrorKind::FunctionCallRequiredAfterAwait);
                None
            }
        }
    }

    fn parse_function_call_expression(&mut self, func: Expression, is_await: bool) -> Option<Expression> {
        let args = match self.parse_func_arguments() {
            Some(a) => a,
            None => return None
        };

        Some(Expression::FuncCall {
            func: Box::new(func),
            args,
            is_await,
        })
    }

    fn parse_func_arguments(&mut self) -> Option<Vec<Expression>> {
        let mut list:Vec<Expression> = vec![];
        let end = Token::Rparen;
        self.skip_next_eol();
        if self.is_next_token(&end) {
            self.bump();
            return Some(list);
        }
        loop {
            self.skip_next_eol();
            self.bump();
            if self.is_current_token(&Token::Comma) {
                // カンマが連続したので空引数
                list.push(Expression::EmptyArgument);
            } else if self.is_current_token(&end) {
                // カンマの後が ) なので空引数
                list.push(Expression::EmptyArgument);
                break;
            } else {
                // 引数の式をパース
                if let Some(e) = self.parse_expression(Precedence::Lowest, ExpressionState::Default) {
                    list.push(e);
                } else {
                    return None;
                }
                self.skip_next_eol();
                if self.is_next_token(&Token::Comma) {
                    self.bump();
                } else {
                    if ! self.bump_to_next_expected_token(end) {
                        return None;
                    }
                    break;
                }
            }
        }

        Some(list)
    }

}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::lexer::{Lexer, Position};
    use crate::{Parser, ParseError, ParseErrors, ParseErrorKind};

    impl StatementWithRow {
        fn new_expected(statement: Statement, row: usize) -> Self{
            Self { statement, row, line: "dummy".into(), script_name: None }
        }
    }

    fn print_errors(errors: ParseErrors, out: bool, input: &str, msg: &str) {
        println!("input: {input}");
        if out {
            println!("parser got {} errors", errors.len());
            for error in errors {
                println!("{:?}", error);
            }
        }
        panic!("{msg}");
    }

    /// - input: 入力
    /// - expected_script: スクリプト部分
    /// - expected_global: グローバル定義
    fn parser_test(input: &str, expected_script: Vec<StatementWithRow>, expected_global: Vec<StatementWithRow>) {
        let parser = Parser::new(Lexer::new(input), None, None);
        match parser.parse() {
            Ok(program) => {
                assert_eq!(program.script, expected_script);
                assert_eq!(program.global, expected_global);
            },
            Err(err) => {
                print_errors(err, true, input, "parser error");
            },
        };
    }

    fn parser_panic_test(input: &str, expected: Vec<StatementWithRow>, msg: &str) {
        let parser = Parser::new(Lexer::new(input), None, None);
        match parser.parse() {
            Ok(program) => {
                assert_eq!(program.script, expected);
                // assert_eq!(program.global, expected_global);
            },
            Err(err) => {
                print_errors(err, false, input, msg);
            },
        };
    }

    #[test]
    fn test_blank_row() {
        let input = r#"
print 1


print 2
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
                2,
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Num(2 as f64))),
                5,
            ),
        ], vec![])
    }

    #[test]
    fn test_dim_statement() {
        let testcases = vec![
            (
                "dim hoge = 1", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("hoge")),
                                Expression::Literal(Literal::Num(1 as f64))
                            ),
                        ],
                        false
                    ), 1)
                ]
            ),
            (
                "dim fuga", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("fuga")),
                                Expression::Literal(Literal::Empty)
                            )
                        ],
                        false
                    ), 1)
                ]
            ),
            (
                "dim piyo = EMPTY", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("piyo")),
                                Expression::Literal(Literal::Empty)
                            )
                        ],
                        false
                    ), 1)
                ]
            ),
            (
                "dim arr1[] = 1, 3, 5, 7, 9", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr1")),
                                Expression::Array(
                                    vec![
                                        Expression::Literal(Literal::Num(1 as f64)),
                                        Expression::Literal(Literal::Num(3 as f64)),
                                        Expression::Literal(Literal::Num(5 as f64)),
                                        Expression::Literal(Literal::Num(7 as f64)),
                                        Expression::Literal(Literal::Num(9 as f64)),
                                    ],
                                    vec![
                                        Expression::Literal(Literal::Empty),
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1)
                ]
            ),
            (
                "dim arr2[4]", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr2")),
                                Expression::Array(
                                    vec![],
                                    vec![
                                        Expression::Literal(Literal::Num(4.0)),
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1),
                ]
            ),
            (
                "dim arr2[1, 2]", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr2")),
                                Expression::Array(
                                    vec![],
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(2.0)),
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1),
                ]
            ),
            (
                "dim arr2[,, 1]", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr2")),
                                Expression::Array(
                                    vec![],
                                    vec![
                                        Expression::Literal(Literal::Empty),
                                        Expression::Literal(Literal::Empty),
                                        Expression::Literal(Literal::Num(1.0)),
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1),
                ]
            ),
            (
                "dim arr2[1,1,1]", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr2")),
                                Expression::Array(
                                    vec![],
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(1.0)),
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1)
                ]
            ),
            (
                "dim a = 1, b, c[1], d[] = 1,2", vec![
                    StatementWithRow::new_expected(Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("a")),
                                Expression::Literal(Literal::Num(1.0))
                            ),
                            (
                                Identifier(String::from("b")),
                                Expression::Literal(Literal::Empty)
                            ),
                            (
                                Identifier(String::from("c")),
                                Expression::Array(
                                    vec![],
                                    vec![
                                        Expression::Literal(Literal::Num(1.0))
                                    ],
                                )
                            ),
                            (
                                Identifier(String::from("d")),
                                Expression::Array(
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(2.0)),
                                    ],
                                    vec![
                                        Expression::Literal(Literal::Empty)
                                    ],
                                )
                            )
                        ],
                        false
                    ), 1),
                ]
            ),
        ];
        for (input, expected) in testcases {
            println!("{}", &input);
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_literarl() {
        let input = r#"
print 1
print 1.23
print $12AB
print true
print false
print "展開可能文字列リテラル"
print ['配', '列', 'リ', 'テ', 'ラ', 'ル']
print []
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
                2
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Num(1.23))),
                3
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(
                    Literal::Num(i64::from_str_radix("12AB", 16).unwrap() as f64)
                )),
                4
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Bool(true))),
                5
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(Literal::Bool(false))),
                6
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(
                    Literal::ExpandableString(String::from("展開可能文字列リテラル"))
                )),
                7
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(
                    Literal::Array(
                        vec![
                            Expression::Literal(Literal::String(String::from("配"))),
                            Expression::Literal(Literal::String(String::from("列"))),
                            Expression::Literal(Literal::String(String::from("リ"))),
                            Expression::Literal(Literal::String(String::from("テ"))),
                            Expression::Literal(Literal::String(String::from("ラ"))),
                            Expression::Literal(Literal::String(String::from("ル"))),
                        ]
                    )
                )),
                8
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Literal(
                    Literal::Array(vec![])
                )),
                9
            ),
        ], vec![]);
    }

    #[test]
    fn test_if() {
        let input = r#"
if a then
    print 1
    print 2
    print 3
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(1.0))),
                            3
                        ),
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(2.0))),
                            4
                        ),
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(3.0))),
                            5
                        ),
                    ],
                    alternative: None
                },
                2
            ),
        ], vec![]);
    }

    #[test]
    fn test_single_line_if() {
        let tests = vec![
            (
                "if a then print b",
                vec![
                    StatementWithRow::new_expected(
                        Statement::IfSingleLine {
                            condition: Expression::Identifier(Identifier(String::from("a"))),
                            consequence: Box::new(
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("b")))),
                                    1
                                )
                            ),
                            alternative: Box::new(None)
                        },
                        1
                    ),
                ]
            ),
            (
                "if a then print b else print c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::IfSingleLine {
                            condition: Expression::Identifier(Identifier(String::from("a"))),
                            consequence: Box::new(
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("b")))),
                                    1
                                )
                            ),
                            alternative: Box::new(Some(
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("c")))),
                                    1
                                )
                            )),
                        },
                        1
                    ),
                ]
            ),
            (
                "if a then print 1 else b = c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::IfSingleLine {
                            condition: Expression::Identifier(Identifier(String::from("a"))),
                            consequence: Box::new(
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
                                    1
                                )
                            ),
                            alternative: Box::new(Some(
                                StatementWithRow::new_expected(
                                    Statement::Expression(Expression::Assign(
                                        Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                        Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                    )),
                                    1
                                )
                            )),
                        },
                        1
                    ),
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_if_without_then() {
        let input = r#"
if b
    print 1
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If{
                    condition: Expression::Identifier(Identifier(String::from("b"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(1.0))),
                            3
                        )
                    ],
                    alternative: None
                },
                2
            )
        ], vec![]);
    }

    #[test]
    fn test_if_else() {
        let input = r#"
if a then
    print 1
else
    print 2.1
    print 2.2
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(1.0))),
                            3
                        ),
                    ],
                    alternative: Some(vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(2.1))),
                            5
                        ),
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(2.2))),
                            6
                        ),
                    ])
                },
                2
            )
        ], vec![]);

    }

    #[test]
    fn test_elseif() {
        let input = r#"
if a then
    print 1
elseif b then
    print 2
elseif c then
    print 3
elseif d
    print 4
else
    print 5
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::ElseIf {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(1.0))),
                            3
                        )
                    ],
                    alternatives: vec![
                        (
                            Some(StatementWithRow::new_expected(
                                Statement::Expression(Expression::Identifier(Identifier(String::from("b")))),
                                4
                            )),
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(2.0))),
                                    5
                                )
                            ],
                        ),
                        (
                            Some(StatementWithRow::new_expected(
                                Statement::Expression(Expression::Identifier(Identifier(String::from("c")))),
                                6
                            )),
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(3.0))),
                                    7
                                )
                            ],
                        ),
                        (
                            Some(StatementWithRow::new_expected(
                                Statement::Expression(Expression::Identifier(Identifier(String::from("d")))),
                                8
                            )),
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(4.0))),
                                    9
                                )
                            ],
                        ),
                        (
                            None,
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(5.0))),
                                    11
                                )
                            ],
                        ),
                    ]
                },
                2
            )
        ], vec![]);
    }
    #[test]
    fn test_elseif_without_else() {
        let input = r#"
if a then
    print 1
elseif b then
    print 2
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::ElseIf {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Literal(Literal::Num(1.0))),
                            3
                        )
                    ],
                    alternatives: vec![
                        (
                            Some(StatementWithRow::new_expected(
                                Statement::Expression(Expression::Identifier(Identifier(String::from("b")))),
                                4
                            )),
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(2.0))),
                                    5
                                )
                            ],
                        ),
                    ]
                },
                2
            )
        ], vec![]);
    }

    #[test]
    fn test_select() {
        let tests = vec![
            (
                r#"
select 1
    case 1,2
        print a
    case 3
        print b
    default
        print c
selend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Select {
                            expression: Expression::Literal(Literal::Num(1.0)),
                            cases: vec![
                                (
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(2.0))
                                    ],
                                    vec![
                                        StatementWithRow::new_expected(
                                            Statement::Print(Expression::Identifier(Identifier("a".to_string()))),
                                            4
                                        )
                                    ]
                                ),
                                (
                                    vec![
                                        Expression::Literal(Literal::Num(3.0))
                                    ],
                                    vec![
                                        StatementWithRow::new_expected(
                                            Statement::Print(Expression::Identifier(Identifier("b".to_string()))),
                                            6
                                        )
                                    ]
                                ),
                            ],
                            default: Some(vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier("c".to_string()))),
                                    8
                                )
                            ])
                        },
                        2
                    )
                ]
            ),
            (
                r#"
select 1
    default
        print c
selend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Select {
                            expression: Expression::Literal(Literal::Num(1.0)),
                            cases: vec![],
                            default: Some(vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier("c".to_string()))),
                                    4
                                )
                            ])
                        },
                        2
                    )
                ]
            ),
            (
                r#"
select 1
    case 1
        print a
selend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Select {
                            expression: Expression::Literal(Literal::Num(1.0)),
                            cases: vec![
                                (
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                    ],
                                    vec![
                                        StatementWithRow::new_expected(
                                            Statement::Print(Expression::Identifier(Identifier("a".to_string()))),
                                            4
                                        )
                                    ]
                                ),
                            ],
                            default: None
                        }, 2
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_prefix() {
        let input = r#"
print ! hoge
print -1
print +1
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Print(Expression::Prefix(
                    Prefix::Not,
                    Box::new(Expression::Identifier(Identifier(String::from("hoge"))))
                )),
                2
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Prefix(
                    Prefix::Minus,
                    Box::new(Expression::Literal(Literal::Num(1 as f64)))
                )),
                3
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Prefix(
                    Prefix::Plus,
                    Box::new(Expression::Literal(Literal::Num(1 as f64)))
                )),
                4
            )
        ], vec![]);
    }

    #[test]
    fn test_infix() {
        let input = r#"
print 3 + 5
print 3 - 5
print 3 * 5
print 3 / 5
print 3 > 5
print 3 < 5
print 3 = 5
print 3 == 5
print 3 != 5
print 3 <> 5
print 3 >= 5
print 3 <= 5
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Plus,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                2
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Minus,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                3
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Multiply,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                4
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Divide,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                5
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::GreaterThan,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                6
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::LessThan,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                7
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Equal,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                8
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::Equal,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                9
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::NotEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                10
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::NotEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                11
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::GreaterThanEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                12
            ),
            StatementWithRow::new_expected(
                Statement::Print(Expression::Infix(
                    Infix::LessThanEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                )),
                13
            ),
        ], vec![]);

    }

    #[test]
    fn test_precedence() {
        let tests = vec![
            (
                "print -a * b",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Multiply,
                            Box::new(Expression::Prefix(
                                Prefix::Minus,
                                Box::new(Expression::Identifier(Identifier(String::from("a"))))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("b"))))
                        )), 1
                    )
                ]
            ),
            (
                "print !-a",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Prefix(
                            Prefix::Not,
                            Box::new(Expression::Prefix(
                                Prefix::Minus,
                                Box::new(Expression::Identifier(Identifier(String::from("a"))))
                            ))
                        )), 1
                    )
                ]
            ),
            (
                "print a + b + c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Identifier(Identifier(String::from("b"))))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("c"))))
                        )), 1
                    )
                ]
            ),
            (
                "print a + b - c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Minus,
                            Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Identifier(Identifier(String::from("b"))))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("c"))))
                        )), 1
                    )
                ]
            ),
            (
                "print a * b * c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Multiply,
                            Box::new(Expression::Infix(
                                Infix::Multiply,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Identifier(Identifier(String::from("b"))))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("c"))))
                        )), 1
                    )
                ]
            ),
            (
                "print a * b / c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Divide,
                            Box::new(Expression::Infix(
                                Infix::Multiply,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Identifier(Identifier(String::from("b"))))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("c"))))
                        )), 1
                    )
                ]
            ),
            (
                "print a + b / c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Infix(
                                Infix::Divide,
                                Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                Box::new(Expression::Identifier(Identifier(String::from("c"))))
                            )),
                        )), 1
                    )
                ]
            ),
            (
                "print a + b * c + d / e - f",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Minus,
                            Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                    Box::new(Expression::Infix(
                                        Infix::Multiply,
                                        Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                        Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                    ))
                                )),
                                Box::new(Expression::Infix(
                                    Infix::Divide,
                                    Box::new(Expression::Identifier(Identifier(String::from("d")))),
                                    Box::new(Expression::Identifier(Identifier(String::from("e")))),
                                )),
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("f"))))
                        )), 1
                    )
                ]
            ),
            (
                "print 5 > 4 == 3 < 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::GreaterThan,
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                                Box::new(Expression::Infix(
                                    Infix::LessThan,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 5 < 4 != 3 > 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::NotEqual,
                                Box::new(Expression::Infix(
                                    Infix::LessThan,
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                                Box::new(Expression::Infix(
                                    Infix::GreaterThan,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 5 >= 4 = 3 <= 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::GreaterThanEqual,
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                                Box::new(Expression::Infix(
                                    Infix::LessThanEqual,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 3 + 4 * 5 == 3 * 1 + 4 * 5",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Infix(
                                        Infix::Multiply,
                                        Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                        Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    )),
                                )),
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Infix(
                                        Infix::Multiply,
                                        Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                        Box::new(Expression::Literal(Literal::Num(1 as f64))),
                                    )),
                                    Box::new(Expression::Infix(
                                        Infix::Multiply,
                                        Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                        Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    )),
                                )),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 3 > 5 == FALSE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::GreaterThan,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                )),
                                Box::new(Expression::Literal(Literal::Bool(false))),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 3 < 5 = TRUE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::LessThan,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                )),
                                Box::new(Expression::Literal(Literal::Bool(true))),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 1 + (2 + 3) + 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Literal(Literal::Num(1 as f64))),
                                    Box::new(Expression::Infix(
                                        Infix::Plus,
                                        Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                        Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    )),
                                )),
                                Box::new(Expression::Literal(Literal::Num(4 as f64))),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print (5 + 5) * 2",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Multiply,
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                )),
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print 2 / (5 + 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(
                            Expression::Infix(
                                Infix::Divide,
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                )),
                            )
                        ), 1
                    )
                ]
            ),
            (
                "print -(5 + 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Prefix(
                            Prefix::Minus,
                            Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                Box::new(Expression::Literal(Literal::Num(5 as f64))),
                            ))
                        )), 1
                    )
                ]
            ),
            (
                "print !(5 = 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Prefix(
                            Prefix::Not,
                            Box::new(Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                Box::new(Expression::Literal(Literal::Num(5 as f64))),
                            ))
                        )), 1
                    )
                ]
            ),
            (
                "print a + add(b * c) + d",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::FuncCall{
                                    func: Box::new(Expression::Identifier(Identifier(String::from("add")))),
                                    args: vec![
                                        Expression::Infix(
                                            Infix::Multiply,
                                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                            Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                        )
                                    ],
                                    is_await: false
                                })
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("d")))),
                        )), 1
                    )
                ]
            ),
            (
                "add(a, b, 1, 2 * 3, 4 + 5, add(6, 7 * 8))",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall{
                            func: Box::new(Expression::Identifier(Identifier(String::from("add")))),
                            args: vec![
                                Expression::Identifier(Identifier(String::from("a"))),
                                Expression::Identifier(Identifier(String::from("b"))),
                                Expression::Literal(Literal::Num(1 as f64)),
                                Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                ),
                                Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                ),
                                Expression::FuncCall{
                                    func: Box::new(Expression::Identifier(Identifier(String::from("add")))),
                                    args: vec![
                                        Expression::Literal(Literal::Num(6 as f64)),
                                        Expression::Infix(
                                            Infix::Multiply,
                                            Box::new(Expression::Literal(Literal::Num(7 as f64))),
                                            Box::new(Expression::Literal(Literal::Num(8 as f64))),
                                        )
                                    ],
                                    is_await: false,
                                }
                            ],
                            is_await: false,
                        }), 1
                    )
                ]
            ),
            (
                "print a * [1, 2, 3, 4][b * c] * d",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Multiply,
                            Box::new(Expression::Infix(
                                Infix::Multiply,
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Index(
                                    Box::new(Expression::Literal(Literal::Array(
                                        vec![
                                            Expression::Literal(Literal::Num(1 as f64)),
                                            Expression::Literal(Literal::Num(2 as f64)),
                                            Expression::Literal(Literal::Num(3 as f64)),
                                            Expression::Literal(Literal::Num(4 as f64)),
                                        ]
                                    ))),
                                    Box::new(Expression::Infix(
                                        Infix::Multiply,
                                        Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                        Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                    )),
                                    Box::new(None)
                                ))
                            )),
                            Box::new(Expression::Identifier(Identifier(String::from("d")))),
                        )), 1
                    )
                ]
            ),
            (
                "add(a * b[2], b[1], 2 * [1, 2][1])",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall{
                            func: Box::new(Expression::Identifier(Identifier(String::from("add")))),
                            args: vec![
                                Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                    Box::new(Expression::Index(
                                        Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                        Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                        Box::new(None)
                                    ))
                                ),
                                Expression::Index(
                                    Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                    Box::new(Expression::Literal(Literal::Num(1 as f64))),
                                    Box::new(None),
                                ),
                                Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                    Box::new(Expression::Index(
                                        Box::new(Expression::Literal(Literal::Array(
                                            vec![
                                                Expression::Literal(Literal::Num(1 as f64)),
                                                Expression::Literal(Literal::Num(2 as f64)),
                                            ]
                                        ))),
                                        Box::new(Expression::Literal(Literal::Num(1 as f64))),
                                        Box::new(None)
                                    ))
                                )
                            ],
                            is_await: false,
                        }), 1
                    )
                ]
            ),
            (
                "print a or b and c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Or,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Infix(
                                Infix::And,
                                Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            ))
                        )), 1
                    )
                ]
            ),
            (
                "print 1 + 5 mod 3",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Literal(Literal::Num(1 as f64))),
                            Box::new(Expression::Infix(
                                Infix::Mod,
                                Box::new(Expression::Literal(Literal::Num(5 as f64))),
                                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                            )),
                        )), 1
                    )
                ]
            ),
            (
                "print 3 * 2 and 2 xor (2 or 4)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Infix(
                            Infix::Xor,
                            Box::new(Expression::Infix(
                                Infix::And,
                                Box::new(Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                                    Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                )),
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                            )),
                            Box::new(Expression::Infix(
                                Infix::Or,
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                Box::new(Expression::Literal(Literal::Num(4 as f64))),
                            )),
                        )), 1
                    )
                ]
            ),
            (
                r#"
if a = b = c then
    print 1
endif
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::If {
                            condition: Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Infix(
                                    Infix::Equal,
                                    Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                    Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                )),
                                Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            ),
                            consequence: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
                                    3
                                )
                            ],
                            alternative: None
                        }, 2
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_assign() {
        let tests = vec![
            (
                "a = 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Literal(Literal::Num(1 as f64)))
                        )), 1
                    )
                ]
            ),
            (
                "a[0] = 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Index(
                                Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                Box::new(Expression::Literal(Literal::Num(0 as f64))),
                                Box::new(None)
                            )),
                            Box::new(Expression::Literal(Literal::Num(1 as f64)))
                        )), 1
                    )
                ]
            ),
            (
                "a[0][0] = 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Index(
                                Box::new(Expression::Index(
                                    Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                    Box::new(Expression::Literal(Literal::Num(0 as f64))),
                                    Box::new(None)
                                )),
                                Box::new(Expression::Literal(Literal::Num(0 as f64))),
                                Box::new(None)
                            )),
                            Box::new(Expression::Literal(Literal::Num(1 as f64)))
                        )), 1
                    )
                ]
            ),
            (
                "a = 1 = 2", // a に 1 = 2 を代入
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Infix(
                                Infix::Equal,
                                Box::new(Expression::Literal(Literal::Num(1 as f64))),
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                            ))
                        )), 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_for() {
        let tests = vec![
            (
                r#"
for i = 0 to 5
    print i
next
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::For {
                            loopvar: Identifier(String::from("i")),
                            from: Expression::Literal(Literal::Num(0 as f64)),
                            to: Expression::Literal(Literal::Num(5 as f64)),
                            step: None,
                            block: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("i")))),
                                    3
                                )
                            ],
                            alt: None
                        }, 2
                    )
                ]
            ),
            (
                r#"
for i = 5 to 0 step -1
    print i
next
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::For {
                            loopvar: Identifier(String::from("i")),
                            from: Expression::Literal(Literal::Num(5 as f64)),
                            to: Expression::Literal(Literal::Num(0 as f64)),
                            step: Some(Expression::Prefix(
                                Prefix::Minus,
                                Box::new(Expression::Literal(Literal::Num(1 as f64)))
                            )),
                            block: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("i")))),
                                    3
                                )
                            ],
                            alt: None
                        }, 2
                    )
                ]
            ),
            (
                r#"
for item in col
    print item
next
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::ForIn {
                            loopvar: Identifier(String::from("item")),
                            index_var: None,
                            islast_var: None,
                            collection: Expression::Identifier(Identifier(String::from("col"))),
                            block: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("item")))),
                                    3
                                )
                            ],
                            alt: None
                        }, 2
                    )
                ]
            ),
            (
                r#"
for item, i in col
next
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::ForIn {
                            loopvar: Identifier(String::from("item")),
                            index_var: Some(Identifier("i".into())),
                            islast_var: None,
                            collection: Expression::Identifier(Identifier(String::from("col"))),
                            block: vec![],
                            alt: None
                        }, 2
                    )
                ]
            ),
            (
                r#"
for item, i, last in col
next
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::ForIn {
                            loopvar: Identifier(String::from("item")),
                            index_var: Some(Identifier("i".into())),
                            islast_var: Some(Identifier("last".into())),
                            collection: Expression::Identifier(Identifier(String::from("col"))),
                            block: vec![],
                            alt: None
                        }, 2
                    )
                ]
            ),
            (
                r#"
for i = 0 to 5
    print i
else
    print not_found
endfor
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::For {
                            loopvar: Identifier(String::from("i")),
                            from: Expression::Literal(Literal::Num(0 as f64)),
                            to: Expression::Literal(Literal::Num(5 as f64)),
                            step: None,
                            block: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("i")))),
                                    3
                                )
                            ],
                            alt: Some(vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier("not_found".into()))),
                                    5
                                )
                            ])
                        }, 2
                    )
                ]
            ),
            (
                r#"
for item in col
    print item
else
    print not_found
endfor
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::ForIn {
                            loopvar: Identifier(String::from("item")),
                            index_var: None,
                            islast_var: None,
                            collection: Expression::Identifier(Identifier(String::from("col"))),
                            block: vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier(String::from("item")))),
                                    3
                                )
                            ],
                            alt: Some(vec![
                                StatementWithRow::new_expected(
                                    Statement::Print(Expression::Identifier(Identifier("not_found".into()))),
                                    5
                                )
                            ])
                        }, 2
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    #[should_panic(expected = "end of block should be NEXT")]
    fn test_error_for() {
        let input = r#"
for item in col
    print item
fend
        "#;
        let expected = vec![
            StatementWithRow::new_expected(
                Statement::ForIn {
                    loopvar: Identifier(String::from("item")),
                    index_var: None,
                    islast_var: None,
                    collection: Expression::Identifier(Identifier(String::from("col"))),
                    block: vec![
                        StatementWithRow::new_expected(
                            Statement::Print(Expression::Identifier(Identifier(String::from("item")))),
                            3
                        )
                    ],
                    alt: None
                }, 2
            )
        ];
        parser_panic_test(input, expected, "end of block should be NEXT");
    }

    #[test]
    fn test_while() {
        let input  = r#"
while (a == b) and (c >= d)
    dosomething()
wend
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::While(
                    Expression::Infix(
                        Infix::And,
                        Box::new(Expression::Infix(
                            Infix::Equal,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                        )),
                        Box::new(Expression::Infix(
                            Infix::GreaterThanEqual,
                            Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            Box::new(Expression::Identifier(Identifier(String::from("d")))),
                        )),
                    ),
                    vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::FuncCall {
                                func: Box::new(Expression::Identifier(Identifier(String::from("dosomething")))),
                                args: vec![],
                                is_await: false,
                            }), 3
                        )
                    ]
                ), 2
            )
        ], vec![]);
    }

    #[test]
    fn test_repeat() {
        let input  = r#"
repeat
    dosomething()
until (a == b) and (c >= d)
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Repeat(
                    Box::new(StatementWithRow::new_expected(Statement::Expression(Expression::Infix(
                        Infix::And,
                        Box::new(Expression::Infix(
                            Infix::Equal,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                        )),
                        Box::new(Expression::Infix(
                            Infix::GreaterThanEqual,
                            Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            Box::new(Expression::Identifier(Identifier(String::from("d")))),
                        )),
                    )), 4)),
                    vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::FuncCall {
                                func: Box::new(Expression::Identifier(Identifier(String::from("dosomething")))),
                                args: vec![],
                                is_await: false,
                            }), 3
                        )
                    ]
                ), 2
            )
        ], vec![]);
    }

    #[test]
    fn test_ternary_operator() {
        let tests = vec![
            (
                "print a ? b : c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Ternary{
                            condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                        }), 1
                    )
                ]
            ),
            (
                "x = a ? b : c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier(String::from("x")))),
                            Box::new(Expression::Ternary{
                                condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            })
                        )), 1
                    )
                ]
            ),
            (
                "print hoge[a?b:c]",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Index(
                            Box::new(Expression::Identifier(Identifier(String::from("hoge")))),
                            Box::new(Expression::Ternary{
                                condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                            }),
                            Box::new(None)
                        )), 1
                    )
                ]
            ),
            (
                "print x + y * a ? b + q : c / r",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Ternary{
                            condition: Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Identifier(Identifier(String::from("x")))),
                                Box::new(Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Identifier(Identifier(String::from("y")))),
                                    Box::new(Expression::Identifier(Identifier(String::from("a")))),
                                )),
                            )),
                            consequence: Box::new(Expression::Infix(
                                Infix::Plus,
                                Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                Box::new(Expression::Identifier(Identifier(String::from("q")))),
                            )),
                            alternative: Box::new(Expression::Infix(
                                Infix::Divide,
                                Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                Box::new(Expression::Identifier(Identifier(String::from("r")))),
                            )),
                        }), 1
                    )
                ]
            ),
            (
                "print a ? b: c ? d: e",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Ternary{
                            condition: Box::new(Expression::Identifier(Identifier("a".to_string()))),
                            consequence: Box::new(Expression::Identifier(Identifier("b".to_string()))),
                            alternative: Box::new(Expression::Ternary{
                                condition: Box::new(Expression::Identifier(Identifier("c".to_string()))),
                                consequence: Box::new(Expression::Identifier(Identifier("d".to_string()))),
                                alternative: Box::new(Expression::Identifier(Identifier("e".to_string())))
                            })
                        }), 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_hashtbl() {
        let tests = vec![
            (
                "hashtbl hoge",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                None
                            )
                        ], false), 1
                    )
                ],
                vec![]
            ),
            (
                "hashtbl hoge, fuga",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                None
                            ),
                            (
                                Identifier(String::from("fuga")),
                                None
                            )
                        ], false), 1
                    )
                ],
                vec![]
            ),
            (
                "hashtbl hoge = HASH_CASECARE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                Some(Expression::Identifier(Identifier("HASH_CASECARE".to_string()))),
                            )
                        ], false), 1
                    )
                ],
                vec![]
            ),
            (
                "hashtbl hoge = HASH_SORT",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                Some(Expression::Identifier(Identifier("HASH_SORT".to_string()))),
                            )
                        ], false), 1
                    )
                ],
                vec![]
            ),
            (
                "public hashtbl hoge = HASH_SORT, fuga",
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                Some(Expression::Identifier(Identifier("HASH_SORT".to_string()))),
                            ),
                            (
                                Identifier(String::from("fuga")),
                                None
                            )
                        ], true), 1
                    )
                ]
            ),
        ];
        for (input, expected, global) in tests {
            parser_test(input, expected, global);
        }
    }

    #[test]
    fn test_hash_sugar() {
        let tests = vec![
            (
                r#"
                hash hoge
                endhash
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Hash(HashSugar::new(
                            Identifier("hoge".into()),
                            None,
                            false,
                            vec![]
                        )),
                        2
                    )
                ],
                vec![]
            ),
            (
                r#"
                hash hoge = HASH_CASECARE
                endhash
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Hash(HashSugar::new(
                            Identifier("hoge".into()),
                            Some(Expression::Identifier(Identifier("HASH_CASECARE".to_string()))),
                            false,
                            vec![]
                        )),
                        2
                    )
                ],
                vec![]
            ),
            (
                r#"
                hash public hoge
                endhash
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Hash(HashSugar::new(
                            Identifier("hoge".into()),
                            None,
                            true,
                            vec![]
                        )),
                        2
                    )
                ]
            ),
            (
                r#"
                hash hoge
                    foo = 1
                    'bar' = 2
                    empty = 3
                    null = 5
                endhash
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Hash(HashSugar::new(
                            Identifier("hoge".into()),
                            None,
                            false,
                            vec![
                                (Expression::Identifier("foo".into()), Expression::Literal(Literal::Num(1.0))),
                                (Expression::Literal(Literal::String("bar".into())), Expression::Literal(Literal::Num(2.0))),
                                (Expression::Literal(Literal::Empty), Expression::Literal(Literal::Num(3.0))),
                                (Expression::Literal(Literal::Null), Expression::Literal(Literal::Num(5.0))),
                            ]
                        )),
                        2
                    )
                ],
                vec![]
            ),
        ];
        for (input, expected, global) in tests {
            parser_test(input, expected, global);
        }
    }

    #[test]
    fn test_function() {
        let tests = vec![
            (
                r#"
function hoge(foo, bar, baz)
    result = foo + bar + baz
fend
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: Identifier("hoge".to_string()),
                            params: vec![
                                FuncParam::new(Some("foo".into()), ParamKind::Identifier),
                                FuncParam::new(Some("bar".into()), ParamKind::Identifier),
                                FuncParam::new(Some("baz".into()), ParamKind::Identifier),
                            ],
                            body: vec![
                                StatementWithRow::new_expected(
                                    Statement::Expression(
                                        Expression::Assign(
                                            Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                            Box::new(Expression::Infix(
                                                Infix::Plus,
                                                Box::new(Expression::Infix(
                                                    Infix::Plus,
                                                    Box::new(Expression::Identifier(Identifier("foo".to_string()))),
                                                    Box::new(Expression::Identifier(Identifier("bar".to_string()))),
                                                )),
                                                Box::new(Expression::Identifier(Identifier("baz".to_string()))),
                                            )),
                                        )
                                    ), 3
                                )
                            ],
                            is_proc: false,
                            is_async: false
                        }, 2
                    )
                ]
            ),
            (
                r#"
procedure hoge(foo, var bar, baz[], qux = 1)
fend
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: Identifier("hoge".to_string()),
                            params: vec![
                                FuncParam::new(Some("foo".into()), ParamKind::Identifier),
                                FuncParam::new(Some("bar".into()), ParamKind::Reference),
                                FuncParam::new(Some("baz".into()), ParamKind::Identifier),
                                FuncParam::new(Some("qux".into()), ParamKind::Default(Expression::Literal(Literal::Num(1.0)))),
                            ],
                            body: vec![],
                            is_proc: true,
                            is_async: false,
                        }, 2
                    )
                ]
            ),
            (
                r#"
procedure hoge(foo: string, var bar: Hoge, qux: number = 1)
fend
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: Identifier("hoge".to_string()),
                            params: vec![
                                FuncParam::new_with_type(Some("foo".into()), ParamKind::Identifier, ParamType::String),
                                FuncParam::new_with_type(Some("bar".into()), ParamKind::Reference, ParamType::UserDefinition("Hoge".into())),
                                FuncParam::new_with_type(Some("qux".into()), ParamKind::Default(Expression::Literal(Literal::Num(1.0))), ParamType::Number),
                            ],
                            body: vec![],
                            is_proc: true,
                            is_async: false,
                        }, 2
                    )
                ]
            ),
            (
                r#"
procedure hoge(ref foo, args bar)
fend
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: Identifier("hoge".to_string()),
                            params: vec![
                                FuncParam::new(Some("foo".into()), ParamKind::Reference),
                                FuncParam::new(Some("bar".into()), ParamKind::Variadic),
                            ],
                            body: vec![],
                            is_proc: true,
                            is_async: false,
                        }, 2
                    )
                ]
            ),
            (
                r#"
print hoge(1)

function hoge(a)
    result = a
fend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::FuncCall{
                            func: Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(1.0)),
                            ],
                            is_await: false,
                        }), 2
                    )
                ],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: Identifier("hoge".to_string()),
                            params: vec![
                                FuncParam::new(Some("a".into()), ParamKind::Identifier),
                            ],
                            body: vec![
                                StatementWithRow::new_expected(
                                    Statement::Expression(
                                        Expression::Assign(
                                            Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                            Box::new(Expression::Identifier(Identifier("a".to_string()))),
                                        )
                                    ), 5
                                )
                            ],
                            is_proc: false,
                            is_async: false,
                        }, 4
                    ),
                ],
            ),
            (
                r#"
hoge = function(a)
    result = a
fend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                            Box::new(Expression::AnonymusFunction{
                                params: vec![
                                    FuncParam::new(Some("a".into()), ParamKind::Identifier),
                                ],
                                body: vec![
                                    StatementWithRow::new_expected(
                                        Statement::Expression(Expression::Assign(
                                            Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                            Box::new(Expression::Identifier(Identifier("a".to_string())))
                                        )), 3
                                    )
                                ],
                                is_proc: false
                            }),
                        )), 2
                    )
                ],
                vec![]
            ),
            (
                r#"
hoge = procedure(a)
    print a
fend
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                            Box::new(Expression::AnonymusFunction{
                                params: vec![
                                    FuncParam::new(Some("a".into()), ParamKind::Identifier),
                                ],
                                body: vec![
                                    StatementWithRow::new_expected(
                                        Statement::Print(
                                            Expression::Identifier(Identifier("a".to_string()))
                                        ), 3
                                    )
                                ],
                                is_proc: true,
                            }),
                        )), 2
                    )
                ],
                vec![]
            ),
        ];
        for (input, expected_script, expected_global) in tests {
            parser_test(input, expected_script, expected_global);
        }
    }

    #[test]
    fn test_func_call() {
        let tests = vec![
            (
                r#"
func(
    1,
    1,
    1
)
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(1.0)),
                                Expression::Literal(Literal::Num(1.0)),
                                Expression::Literal(Literal::Num(1.0)),
                            ],
                            is_await: false
                        })
                        , 2
                    ),
                ]
            ),
            (
                r#"
func(
    2
    ,2,
    2
)
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(2.0)),
                                Expression::Literal(Literal::Num(2.0)),
                                Expression::Literal(Literal::Num(2.0)),
                            ],
                            is_await: false
                        })
                        , 2
                    ),
                ]
            ),
            (
                r#"
func(3
    ,3,
    3)
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(3.0)),
                                Expression::Literal(Literal::Num(3.0)),
                                Expression::Literal(Literal::Num(3.0)),
                            ],
                            is_await: false
                        })
                        , 2
                    ),
                ]
            ),
            (
                "func( , , 4)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::EmptyArgument,
                                Expression::EmptyArgument,
                                Expression::Literal(Literal::Num(4.0)),
                            ],
                            is_await: false
                        })
                        , 1
                    ),
                ]
            ),
            (
                r#"
func(
    ,
    ,
    5
)
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::EmptyArgument,
                                Expression::EmptyArgument,
                                Expression::Literal(Literal::Num(5.0)),
                            ],
                            is_await: false
                        })
                        , 2
                    ),
                ]
            ),
            (
                "func( 5, , 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(5.0)),
                                Expression::EmptyArgument,
                                Expression::Literal(Literal::Num(5.0)),
                            ],
                            is_await: false
                        })
                        , 1
                    ),
                ]
            ),
            (
                "func( 5, , )",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::FuncCall {
                            func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                            args: vec![
                                Expression::Literal(Literal::Num(5.0)),
                                Expression::EmptyArgument,
                                Expression::EmptyArgument,
                            ],
                            is_await: false
                        })
                        , 1
                    ),
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_compound_assign() {
        let tests = vec![
            (
                "a += 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::CompoundAssign(
                            Box::new(Expression::Identifier(Identifier("a".to_string()))),
                            Box::new(Expression::Literal(Literal::Num(1.0))),
                            Infix::Plus,
                        )), 1
                    )
                ]
            ),
            (
                "a -= 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::CompoundAssign(
                            Box::new(Expression::Identifier(Identifier("a".to_string()))),
                            Box::new(Expression::Literal(Literal::Num(1.0))),
                            Infix::Minus,
                        )), 1
                    )
                ]
            ),
            (
                "a *= 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::CompoundAssign(
                            Box::new(Expression::Identifier(Identifier("a".to_string()))),
                            Box::new(Expression::Literal(Literal::Num(1.0))),
                            Infix::Multiply,
                        )), 1
                    )
                ]
            ),
            (
                "a /= 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::CompoundAssign(
                            Box::new(Expression::Identifier(Identifier("a".to_string()))),
                            Box::new(Expression::Literal(Literal::Num(1.0))),
                            Infix::Divide,
                        )), 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_dotcall() {
        let tests = vec![
            (
                "print hoge.a",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::DotCall(
                            Box::new(Expression::Identifier(Identifier("hoge".into()))),
                            Box::new(Expression::Identifier(Identifier("a".into()))),
                        )), 1
                    )
                ]
            ),
            (
                "print hoge.b()",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::FuncCall{
                            func: Box::new(Expression::DotCall(
                                Box::new(Expression::Identifier(Identifier("hoge".into()))),
                                Box::new(Expression::Identifier(Identifier("b".into()))),
                            )),
                            args: vec![],
                            is_await: false,
                        }), 1
                    )
                ]
            ),
            (
                "hoge.a = 1",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Assign(
                            Box::new(Expression::DotCall(
                                Box::new(Expression::Identifier(Identifier("hoge".into()))),
                                Box::new(Expression::Identifier(Identifier("a".into()))),
                            )),
                            Box::new(Expression::Literal(Literal::Num(1.0))),
                        )), 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
        }
    }

    #[test]
    fn test_def_dll() {
        let tests = vec![
            (
                "def_dll nest({long, long, {long, long}}):bool:nest.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "nest".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Struct(vec![
                                    DefDllParam::Param{dll_type: DllType::Long, is_ref: false, size: DefDllParamSize::None},
                                    DefDllParam::Param{dll_type: DllType::Long, is_ref: false, size: DefDllParamSize::None},
                                    DefDllParam::Struct(vec![
                                        DefDllParam::Param{dll_type: DllType::Long, is_ref: false, size: DefDllParamSize::None},
                                        DefDllParam::Param{dll_type: DllType::Long, is_ref: false, size: DefDllParamSize::None},
                                    ]),
                                ]),
                            ],
                            ret_type: DllType::Bool,
                            path: "nest.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll hoge()::hoge.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "hoge".into(),
                            alias: None,
                            params: vec![],
                            ret_type: DllType::Void,
                            path: "hoge.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll fuga():fuga.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "fuga".into(),
                            alias: None,
                            params: vec![],
                            ret_type: DllType::Void,
                            path: "fuga.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll size(int, dword[6], byte[BYTE_SIZE], var string, var long[2], {word,word}):bool:size.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "size".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Param{dll_type: DllType::Int, is_ref: false, size: DefDllParamSize::None},
                                DefDllParam::Param{dll_type: DllType::Dword, is_ref: false, size: DefDllParamSize::Size(6)},
                                DefDllParam::Param{dll_type: DllType::Byte, is_ref: false, size: DefDllParamSize::Const("BYTE_SIZE".into())},
                                DefDllParam::Param{dll_type: DllType::String, is_ref: true, size: DefDllParamSize::None},
                                DefDllParam::Param{dll_type: DllType::Long, is_ref: true, size: DefDllParamSize::Size(2)},
                                DefDllParam::Struct(vec![
                                    DefDllParam::Param{dll_type: DllType::Word, is_ref: false, size: DefDllParamSize::None},
                                    DefDllParam::Param{dll_type: DllType::Word, is_ref: false, size: DefDllParamSize::None},
                                ]),
                            ],
                            ret_type: DllType::Bool,
                            path: "size.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll cb1(callback(int, bool):long):cb1.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "cb1".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Callback(vec![DllType::Int, DllType::Bool], DllType::Long),
                            ],
                            ret_type: DllType::Void,
                            path: "cb1.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll cb2(callback(int)):cb2.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "cb2".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Callback(vec![DllType::Int], DllType::Void),
                            ],
                            ret_type: DllType::Void,
                            path: "cb2.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll cb3(callback():int):cb3.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "cb3".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Callback(vec![], DllType::Int),
                            ],
                            ret_type: DllType::Void,
                            path: "cb3.dll".into()
                        }, 1
                    )
                ]
            ),
            (
                "def_dll alias:real():alias.dll",
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "real".into(),
                            alias: Some("alias".into()),
                            params: vec![],
                            ret_type: DllType::Void,
                            path: "alias.dll".into()
                        }, 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected.clone(), expected);
        }
    }

    #[test]
    fn test_program_order() {
        // public/const, function/procedure, その他の順になる
        let input = r#"
dim d1 = 1
public p1 = 1
const c1 = 1
const c2 = 1
public p2 = 1
dim d2 = 1

function f1()
fend
procedure p1()
fend
function f2()
fend

public p3 = 1
        "#;
        let global = vec![
            StatementWithRow::new_expected(
                Statement::Const(vec![
                    (Identifier("c1".to_string()), Expression::Literal(Literal::Num(1.0)))
                ]), 4
            ),
            StatementWithRow::new_expected(
                Statement::Const(vec![
                    (Identifier("c2".to_string()), Expression::Literal(Literal::Num(1.0)))
                ]), 5
            ),
            StatementWithRow::new_expected(
                Statement::Public(vec![
                    (Identifier("p1".to_string()), Expression::Literal(Literal::Num(1.0)))
                ]), 3
            ),
            StatementWithRow::new_expected(
                Statement::Public(vec![
                    (Identifier("p2".to_string()), Expression::Literal(Literal::Num(1.0)))
                ]), 6
            ),
            StatementWithRow::new_expected(
                Statement::Public(
                    vec![(Identifier("p3".to_string()), Expression::Literal(Literal::Num(1.0)))]
                ), 16
            ),
            StatementWithRow::new_expected(
                Statement::Function {
                    name: Identifier("f1".to_string()),
                    params: vec![],
                    body: vec![],
                    is_proc: false,
                    is_async: false,
                }, 9
            ),
            StatementWithRow::new_expected(
                Statement::Function {
                    name: Identifier("p1".to_string()),
                    params: vec![],
                    body: vec![],
                    is_proc: true,
                    is_async: false,
                }, 11
            ),
            StatementWithRow::new_expected(
                Statement::Function {
                    name: Identifier("f2".to_string()),
                    params: vec![],
                    body: vec![],
                    is_proc: false,
                    is_async: false,
                }, 13
            ),
        ];
        let script = vec![
            StatementWithRow::new_expected(
                Statement::Dim(vec![
                    (Identifier("d1".to_string()), Expression::Literal(Literal::Num(1.0)))
                ], false), 2
            ),
            StatementWithRow::new_expected(
                Statement::Dim(vec![
                    (Identifier("d2".to_string()), Expression::Literal(Literal::Num(1.0)))
                ], false), 7
            ),
        ];
        parser_test(input, script, global);
    }

    #[test]
    fn test_module() {
        let input = r#"
module Hoge
    dim a = 1
    public b = 1
    const c = 1

    procedure Hoge()
        this.a = c
    fend

    function f(x, y)
        result = x + _f(y)
    fend

    dim _f = function(z)
        result = z + 1
    fend
endmodule
        "#;
        parser_test(input, vec![], vec![
            StatementWithRow::new_expected(
                Statement::Module(
                    Identifier("Hoge".to_string()),
                    vec![
                        StatementWithRow::new_expected(
                            Statement::Dim(vec![
                                (Identifier("a".to_string()), Expression::Literal(Literal::Num(1.0)))
                            ], false), 3
                        ),
                        StatementWithRow::new_expected(
                            Statement::Public(vec![
                                (Identifier("b".to_string()), Expression::Literal(Literal::Num(1.0)))
                            ]), 4
                        ),
                        StatementWithRow::new_expected(
                            Statement::Const(vec![
                                (Identifier("c".to_string()), Expression::Literal(Literal::Num(1.0)))
                            ]), 5
                        ),
                        StatementWithRow::new_expected(
                            Statement::Function {
                                name: Identifier("Hoge".to_string()),
                                params: vec![],
                                body: vec![
                                    StatementWithRow::new_expected(
                                        Statement::Expression(Expression::Assign(
                                            Box::new(Expression::DotCall(
                                                Box::new(Expression::Identifier(Identifier("this".to_string()))),
                                                Box::new(Expression::Identifier(Identifier("a".to_string()))),
                                            )),
                                            Box::new(Expression::Identifier(Identifier("c".to_string()))),
                                        )), 8
                                    ),
                                ],
                                is_proc: true,
                                is_async: false,
                            }, 7
                        ),
                        StatementWithRow::new_expected(
                            Statement::Function {
                                name: Identifier("f".to_string()),
                                params: vec![
                                    FuncParam::new(Some("x".into()), ParamKind::Identifier),
                                    FuncParam::new(Some("y".into()), ParamKind::Identifier),
                                ],
                                body: vec![
                                    StatementWithRow::new_expected(
                                        Statement::Expression(Expression::Assign(
                                            Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                            Box::new(Expression::Infix(
                                                Infix::Plus,
                                                Box::new(Expression::Identifier(Identifier("x".to_string()))),
                                                Box::new(Expression::FuncCall{
                                                    func: Box::new(Expression::Identifier(Identifier("_f".to_string()))),
                                                    args: vec![
                                                        Expression::Identifier(Identifier("y".to_string()))
                                                    ],
                                                    is_await: false,
                                                }),
                                            )),
                                        )), 12
                                    ),
                                ],
                                is_proc: false,
                                is_async: false,
                            }, 11
                        ),
                        StatementWithRow::new_expected(
                            Statement::Dim(vec![
                                (
                                    Identifier("_f".to_string()),
                                    Expression::AnonymusFunction {
                                        params: vec![
                                            FuncParam::new(Some("z".into()), ParamKind::Identifier),
                                        ],
                                        body: vec![
                                            StatementWithRow::new_expected(
                                                Statement::Expression(Expression::Assign(
                                                    Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                                    Box::new(Expression::Infix(
                                                        Infix::Plus,
                                                        Box::new(Expression::Identifier(Identifier("z".to_string()))),
                                                        Box::new(Expression::Literal(Literal::Num(1.0))),
                                                    )),
                                                )), 16
                                            ),
                                        ],
                                        is_proc: false
                                    }
                                )
                            ], false), 15
                        ),
                    ],
                ), 2
            )
        ]);
    }

    #[test]
    fn test_struct() {
        let input = r#"
struct Hoge
    x: Long
    y: long
    b: byte[100]
    c: byte[BYTE_SIZE]
    r: ref long
endstruct
        "#;
        parser_test(input, vec![], vec![
            StatementWithRow::new_expected(
                Statement::Struct(Identifier("Hoge".into()), vec![
                    ("x".into(), "long".into(), DefDllParamSize::None, false),
                    ("y".into(), "long".into(), DefDllParamSize::None, false),
                    ("b".into(), "byte".into(), DefDllParamSize::Size(100), false),
                    ("c".into(), "byte".into(), DefDllParamSize::Const("BYTE_SIZE".into()), false),
                    ("r".into(), "long".into(), DefDllParamSize::None, true),
                ]), 2
            ),
        ]);
    }

    #[test]
    fn test_param_type() {
        let input = r#"
function hoge(foo: string, bar: BarEnum, baz: number = 1)
fend
        "#;
        parser_test(input, vec![], vec![
            StatementWithRow::new_expected(
                Statement::Function {
                    name: Identifier("hoge".into()),
                    params: vec![
                        FuncParam::new_with_type(
                            Some("foo".into()), ParamKind::Identifier,
                            ParamType::String
                        ),
                        FuncParam::new_with_type(
                            Some("bar".into()), ParamKind::Identifier,
                            ParamType::UserDefinition("BarEnum".into())
                        ),
                        FuncParam::new_with_type(
                            Some("baz".into()),
                            ParamKind::Default(Expression::Literal(Literal::Num(1.0))),
                            ParamType::Number
                        ),
                    ],
                    body: vec![],
                    is_proc: false,
                    is_async: false
                },
                2
            )
        ]);
    }

    #[test]
    fn test_multi_row_list() {
        let testcases = vec![
            ( // 配列
                r#"
                print [
                    'foo',
                    'bar'
                    ,'baz'
                ]
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Print(Expression::Literal(Literal::Array(vec![
                            Expression::Literal(Literal::String("foo".into())),
                            Expression::Literal(Literal::String("bar".into())),
                            Expression::Literal(Literal::String("baz".into())),
                        ]))),
                        2
                    )
                ],
                vec![]
            ),
            ( // 関数呼び出し
                r#"
                dim ret = func(
                    foo,
                    ,
                    bar
                    ,baz
                )
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::Dim(vec![
                            (
                                Identifier("ret".into()),
                                Expression::FuncCall {
                                    func: Box::new(Expression::Identifier(Identifier("func".to_string()))),
                                    args: vec![
                                        Expression::Identifier("foo".into()),
                                        Expression::EmptyArgument,
                                        Expression::Identifier("bar".into()),
                                        Expression::Identifier("baz".into()),
                                    ],
                                    is_await: false,
                                }
                            )
                        ], false),
                        2
                    )
                ],
                vec![]
            ),
            ( // 関数定義
                r#"
                function func(
                    a, b,
                    c
                    ,d
                )
                fend
                "#,
                vec![],
                vec![
                    StatementWithRow::new_expected(
                        Statement::Function {
                            name: "func".into(),
                            params: vec![
                                FuncParam::new(Some("a".into()), ParamKind::Identifier),
                                FuncParam::new(Some("b".into()), ParamKind::Identifier),
                                FuncParam::new(Some("c".into()), ParamKind::Identifier),
                                FuncParam::new(Some("d".into()), ParamKind::Identifier),
                            ],
                            body: vec![],
                            is_proc: false,
                            is_async: false
                        },
                        2
                    )
                ]
            ),
            ( // def_dll
                r#"
                def_dll MessageBoxA(
                    hwnd,
                    string,
                    string,
                    uint
                ):int:user32
                "#,
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "MessageBoxA".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Param { dll_type: DllType::Hwnd, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::String, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::String, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::Uint, is_ref: false, size: DefDllParamSize::None },
                            ],
                            ret_type: DllType::Int,
                            path: "user32".into()
                        },
                        2
                    ),
                ],
                vec![
                    StatementWithRow::new_expected(
                        Statement::DefDll {
                            name: "MessageBoxA".into(),
                            alias: None,
                            params: vec![
                                DefDllParam::Param { dll_type: DllType::Hwnd, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::String, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::String, is_ref: false, size: DefDllParamSize::None },
                                DefDllParam::Param { dll_type: DllType::Uint, is_ref: false, size: DefDllParamSize::None },
                            ],
                            ret_type: DllType::Int,
                            path: "user32".into()
                        },
                        2
                    ),
                ]
            ),
        ];
        for (input, expected_script, expected_global) in testcases {
            parser_test(input, expected_script, expected_global);
        }
    }

    impl PartialOrd for ParseError {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            // match self.kind.partial_cmp(&other.kind) {
            //     Some(core::cmp::Ordering::Equal) => {}
            //     ord => return ord,
            // }
            match self.start.partial_cmp(&other.start) {
                Some(core::cmp::Ordering::Equal) => {}
                ord => return ord,
            }
            match self.end.partial_cmp(&other.end) {
                Some(core::cmp::Ordering::Equal) => {}
                ord => return ord,
            }
            self.script_name.partial_cmp(&other.script_name)
        }
    }
    impl Eq for ParseError {}
    impl Ord for ParseError {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.partial_cmp(other).unwrap()
        }
    }

    fn parser_error_test(input: &str, mut expected: ParseErrors) {
        let parser = Parser::new(Lexer::new(input), None, Some(vec![]));
        match parser.parse() {
            Ok(_) => {
                panic!("No error found on input: \r\n{input}");
            },
            Err(mut err) => {
                err.sort();
                expected.sort();
                assert_eq!(err, expected);
            }
        }
    }
    fn already_defined_error(ident: &str, start: (usize, usize)) -> ParseError {
        ParseError::new(
            ParseErrorKind::IdentifierIsAlreadyDefined(ident.to_string()),
            Position::new(start.0, start.1),
            Position::new(start.0, start.1 + ident.len()),
            String::new()
        )
    }

    // エラー系
    #[test]
    fn test_error_dups() {
        let test_case = vec![
            (
                r#"
const x = 1
const x = 2
public x = 3
dim x = 4
                "#,
                vec![
                    already_defined_error("X", (3, 7)),
                    already_defined_error("X", (4, 8)),
                    already_defined_error("X", (5, 5)),
                ]
            ),
            (
                r#"
dim x = 1
dim x = 2
                "#,
                vec![
                    already_defined_error("X", (3, 5)),
                ]
            ),
            (
                r#"
x = 1
dim x = 2
                "#,
                vec![
                    already_defined_error("X", (3, 5)),
                ]
            ),
            (
                r#"
const x = 1
const p = procedure()
    const x = 2
    public x = 2
    dim x = 2
fend
                "#,
                vec![
                    already_defined_error("X", (4, 11)),
                    already_defined_error("X", (5, 12)),
                    already_defined_error("X", (6, 9)),
                ]
            ),
            (
                r#"
dim x = 1
const p = procedure()
    const x = 2
    public x = 2
    dim x = 2
fend
                "#,
                vec![
                    already_defined_error("X", (5, 12)),
                    already_defined_error("X", (6, 9)),
                    already_defined_error("X", (2, 5)),
                ]
            ),
            (
                r#"
const x = 1
dim y = 1
const p = procedure()
    dim x = 2
    dim y = 2
fend
                "#,
                vec![
                    already_defined_error("X", (5, 9)),
                ]
            ),
            (
                r#"
dim x = 1
dim y = 1
dim p = procedure()
    dim x = 2
    dim y = 2
fend
                "#,
                vec![
                    already_defined_error("X", (5, 9)),
                    already_defined_error("Y", (6, 9)),
                ]
            ),
            (
                r#"
dim x = 1
dim p = procedure()
    dim x = 2
    dim y = 2
fend
dim y = 1
                "#,
                vec![
                    already_defined_error("X", (4, 9)),
                ]
            ),
            (
                r#"
const x = 1
procedure p()
    const x = 2
    public x = 2
    dim x = 2
fend
                "#,
                vec![
                    already_defined_error("X", (4, 11)),
                    already_defined_error("X", (5, 12)),
                    already_defined_error("X", (6, 9)),
                ]
            ),
            (
                r#"
const x = 1
module m
    const y = 1
    procedure m()
        const x = 2
        const y = 2
    fend
endmodule
                "#,
                vec![
                    already_defined_error("Y", (7, 15)),
                ]
            ),
            (
                r#"
const x = 1
module m
    const y = 1
    procedure m()
        public x = 2
        public y = 2
    fend
endmodule
                "#,
                vec![
                    already_defined_error("Y", (7, 16)),
                ]
            ),
            (
                r#"
const x = 1
module m
    const y = 1
    procedure m()
        dim x = 2
        dim y = 2
    fend
endmodule
                "#,
                vec![
                    already_defined_error("Y", (7, 13)),
                ]
            ),
        ];
        for (input, expected) in test_case {
            parser_error_test(input, expected);
        }
    }


    fn undeclared_error(ident: &str, start: (usize, usize)) -> ParseError {
        ParseError::new(
            ParseErrorKind::UndeclaredIdentifier(ident.to_string()),
            Position::new(start.0, start.1),
            Position::new(start.0, start.1 + ident.len()),
            String::new()
        )
    }

    #[test]
    fn test_error_access() {
        let test_case = vec![
            (
                r#"
print x
print y
                "#,
                vec![
                    undeclared_error("X", (2, 7)),
                    undeclared_error("Y", (3, 7)),
                ]
            ),
            (
                r#"
x = 1
print x
print y
                "#,
                vec![
                    undeclared_error("Y", (4, 7)),
                ]
            ),
            (
                r#"
x()
                "#,
                vec![
                    undeclared_error("X", (2, 1)),
                ]
            ),
            (
                r#"
print m.foo
                "#,
                vec![
                    undeclared_error("M", (2, 7)),
                ]
            ),
            (
                r#"
function f()
    result = a
fend
                "#,
                vec![
                    undeclared_error("A", (3, 14)),
                ]
            ),
            (
                r#"
module m
    procedure m
        result = a
    fend
    dim x = a
endmodule
                "#,
                vec![
                    undeclared_error("A", (4, 18)),
                    undeclared_error("A", (6, 13)),
                ]
            ),
        ];
        for (input, expected) in test_case {
            parser_error_test(input, expected);
        }
    }

    fn explicit_error(ident: &str, start: (usize, usize)) -> ParseError {
        ParseError::new(
            ParseErrorKind::ExplicitError(ident.to_string()),
            Position::new(start.0, start.1),
            Position::new(start.0, start.1 + ident.len()),
            String::new()
        )
    }

    #[test]
    fn test_error_option_explicit() {
        let test_case = vec![
            (
                r#"
OPTION EXPLICIT
x = 1
                "#,
                vec![
                    explicit_error("X", (3, 1)),
                ]
            ),
            (
                r#"
OPTION EXPLICIT
dim x = y := 1
                "#,
                vec![
                    explicit_error("Y", (3, 9)),
                ]
            ),
            (
                r#"
OPTION EXPLICIT
x = y := 1
                "#,
                vec![
                    explicit_error("X", (3, 1)),
                    explicit_error("Y", (3, 5)),
                ]
            ),
            (
                r#"
OPTION EXPLICIT
function f(a = w := 1)
    x = 1
    print y := 1
    result = z := 1
fend
                "#,
                vec![
                    explicit_error("W", (3, 16)),
                    explicit_error("X", (4, 5)),
                    explicit_error("Y", (5, 11)),
                    explicit_error("Z", (6, 14)),
                ]
            ),
            (
                r#"
OPTION EXPLICIT
module m
    procedure m
        v = 1
        print w := 1
    fend
    dim a = x := 1
    public b = y := 1
    const c = z := 1
endmodule
                "#,
                vec![
                    explicit_error("V", (5, 9)),
                    explicit_error("W", (6, 15)),
                    explicit_error("X", (8, 13)),
                    explicit_error("Y", (9, 16)),
                    explicit_error("Z", (10, 15)),
                ]
            ),
        ];
        for (input, expected) in test_case {
            parser_error_test(input, expected);
        }
    }
}