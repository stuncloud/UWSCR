use crate::ast::*;
use crate::lexer::{Lexer, Position, TokenInfo};
use crate::token::{Token, BlockEnd};
use crate::{get_script, get_utf8};
use crate::serializer;
use crate::error::parser::{ParseError, ParseErrorKind};

use std::path::PathBuf;
use std::env;
use std::str::FromStr;

pub type PareseErrors = Vec<ParseError>;
pub type ParserResult<T> = Result<T, PareseErrors>;

pub struct Parser {
    lexer: Lexer,
    current_token: TokenInfo,
    next_token: TokenInfo,
    errors: PareseErrors,
    with: Option<Expression>,
    with_count: usize,
    in_loop: bool,
    script_name: String, // callしたスクリプトの名前,
    script_dir: Option<PathBuf>,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        let script_name = env::var("GET_UWSC_NAME").unwrap_or(String::new());
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            in_loop: false,
            script_name,
            script_dir: None,
        };
        parser.bump();
        parser.bump();

        parser
    }
    pub fn set_script_dir(&mut self, script_dir: PathBuf) {
        self.script_dir = Some(script_dir);
    }

    pub fn call(lexer: Lexer, script_name: String, script_dir: Option<PathBuf>) -> Self {
        let mut parser = Parser {
            lexer,
            current_token: TokenInfo::new(Token::Eof),
            next_token: TokenInfo::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            in_loop: false,
            script_name,
            script_dir,
        };
        parser.bump();
        parser.bump();

        parser
    }

    pub fn script_name(&self) -> String {
        self.script_name.clone()
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

    fn bump(&mut self) {
        self.current_token = self.next_token.clone();
        self.next_token = self.lexer.next_token();
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
    fn is_next_token_expected(&mut self, token: Token) -> bool {
        if self.is_next_token(&token) {
            self.bump();
            return true;
        } else {
            self.error_got_invalid_next_token(token);
            return false;
        }
    }

    fn is_current_token_expected(&mut self, token: Token) -> bool {
        if self.is_current_token(&token) {
            return true;
        } else {
            self.error_got_invalid_token(token);
            return false;
        }
    }

    fn is_expected_close_token(&mut self, current_token: Token) -> bool {
        if self.is_current_token(&current_token) {
            return true;
        } else {
            self.error_got_invalid_close_token(current_token);
            return false;
        }
    }

    fn error_got_invalid_next_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken(token, self.next_token.token()),
            self.next_token.pos,
            self.script_name()
        ))
    }

    fn error_got_invalid_close_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::InvalidBlockEnd(token, self.current_token.token()),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_got_invalid_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken(token, self.current_token.token()),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_token_is_not_identifier(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::IdentifierExpected(self.current_token.token()),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_got_unexpected_token(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken2(self.current_token.token()),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_got_unexpected_next_token(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken2(self.next_token.token()),
            self.next_token.pos,
            self.script_name(),
        ))
    }

    fn error_got_bad_parameter(&mut self, kind: ParseErrorKind) {
        self.errors.push(ParseError::new(
            kind,
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_got_invalid_dlltype(&mut self, name: String) {
        self.errors.push(ParseError::new(
            ParseErrorKind::InvalidDllType(name),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn error_got_invalid_dllpath(&mut self, pos: Position) {
        self.errors.push(ParseError::new(
            ParseErrorKind::DllPathNotFound,
            pos,
            self.script_name()
        ))
    }

    fn error_invalid_hash_member_definition(&mut self, e: Option<Expression>, pos: Position) {
        self.errors.push(ParseError::new(
            ParseErrorKind::InvalidHashMemberDefinition(e),
            pos,
            self.script_name()
        ))
    }

    fn error_missing_array_size(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::SizeRequired,
            self.current_token.pos,
            self.script_name()
        ));
    }

    fn error_no_prefix_parser(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::NoPrefixParserFound(self.current_token.token()),
            self.current_token.pos,
            self.script_name()
        ))
    }

    fn current_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.current_token.token)
    }

    fn next_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.next_token.token)
    }

    pub fn as_errors(self) -> PareseErrors {
        self.errors
    }

    pub fn parse(mut self) -> ParserResult<Program> {
        let builder = self.parse_to_builder();

        if self.errors.len() == 0 {
            let program = builder.build(self.lexer.lines);
            Ok(program)
        } else {
            Err(self.errors)
        }
    }
    pub fn parse_to_builder(&mut self) -> ProgramBuilder {

        let mut builder = ProgramBuilder::new();

        while ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(s) => match s.statement {
                    Statement::Option(_) => {
                        builder.push_option(s);
                    },
                    Statement::Const(_) |
                    Statement::TextBlock(_, _) => {
                        builder.push_const(s);
                    },
                    Statement::Public(_) => {
                        builder.push_public(s);
                    },
                    Statement::Function{name, params, body, is_proc, is_async} => {
                        let mut new_body = Vec::new();
                        for row in body {
                            match row.statement {
                                Statement::Const(_) |
                                Statement::TextBlock(_, _) => {
                                    builder.push_const(row);
                                },
                                Statement::Public(_) => {
                                    builder.push_public(row);
                                }
                                Statement::DefDll { name:_, alias:_, params:_, ret_type:_, path:_ } => {
                                    // グローバルに登録
                                    builder.push_def(row.clone());
                                    // スクリプトにも残る
                                    new_body.push(row);
                                },
                                _ => new_body.push(row)
                            }
                        }
                        let func = StatementWithRow::new(
                            Statement::Function {
                                name, params, body: new_body, is_proc, is_async
                            },
                            s.row,
                            s.line,
                            s.script_name
                        );
                        builder.push_def(func);
                    },
                    Statement::DefDll { name:_, alias:_, params:_, ret_type:_, path:_ } => {
                        // グローバルに登録
                        builder.push_def(s.clone());
                        // スクリプトにも残る
                        builder.push_script(s);
                    },
                    Statement::Module(_, _) |
                    Statement::Class(_, _) |
                    Statement::Struct(_, _) => {
                        builder.push_def(s);
                    },
                    Statement::Call(program, params) => {
                        let program = builder.set_call_program(program);
                        let call_stmt = StatementWithRow::new(Statement::Call(program, params), s.row, s.line, s.script_name);
                        builder.push_script(call_stmt);
                    },
                    _ => {
                        // program.push(s)
                        builder.push_script(s);
                    }
                },
                None => {}
            }
            self.bump();
        }

        let mut explicit_errors = builder.check_explicit();
        self.errors.append(&mut explicit_errors);

        builder
    }

    fn parse_block_statement(&mut self) -> BlockStatement {
        self.bump();
        let mut block: BlockStatement  = vec![];

        while ! self.is_current_token_end_of_block() && ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(s) => block.push(s),
                None => ()
            }
            self.bump();
        }

        block
    }

    fn parse_statement(&mut self) -> Option<StatementWithRow> {
        let row = self.current_token.pos.row;
        let token = self.current_token.token.clone();
        let statement = match token {
            Token::Dim => self.parse_dim_statement(),
            Token::Public => self.parse_public_statement(),
            Token::Const => self.parse_const_statement(),
            Token::If |
            Token::IfB => self.parse_if_statement(),
            Token::Select => self.parse_select_statement(),
            Token::Print => self.parse_print_statement(),
            Token::For => self.parse_for_statement(),
            Token::While => self.parse_while_statement(),
            Token::Repeat => self.parse_repeat_statement(),
            Token::Continue => self.parse_continue_statement(),
            Token::Break => self.parse_break_statement(),
            Token::Call => self.parse_call_statement(),
            Token::DefDll => self.parse_def_dll_statement(),
            Token::Struct => self.parse_struct_statement(),
            Token::HashTable => self.parse_hashtable_statement(false),
            Token::Hash => self.parse_hash_statement(),
            Token::Function => self.parse_function_statement(false, false),
            Token::Procedure => self.parse_function_statement(true, false),
            Token::Async => self.parse_async_function_statement(),
            Token::Exit => Some(Statement::Exit),
            Token::ExitExit => self.parse_exitexit_statement(),
            Token::Module => self.parse_module_statement(),
            Token::Class => self.parse_class_statement(),
            Token::TextBlock(is_ex) => self.parse_textblock_statement(is_ex),
            Token::With => self.parse_with_statement(),
            Token::Try => self.parse_try_statement(),
            Token::Option(ref name) => self.parse_option_statement(name),
            Token::Enum => self.parse_enum_statement(),
            Token::Thread => self.parse_thread_statement(),
            Token::ComErrIgn => Some(Statement::ComErrIgn),
            Token::ComErrRet => Some(Statement::ComErrRet),
            _ => self.parse_expression_statement(),
        };
        match statement {
            Some(s) => Some(StatementWithRow::new(
                s, row, self.lexer.get_line(row), Some(self.script_name())
            )),
            None => None
        }
    }

    fn parse_variable_definition(&mut self, value_required: bool) -> Option<Vec<(Identifier, Expression)>> {
        let mut expressions = vec![];

        loop {
            let var_name = match self.parse_identifier() {
                Some(i) => i,
                None => return None
            };
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
                                    self.error_missing_array_size();
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
                                    match self.parse_expression(Precedence::Lowest, false) {
                                        Some(e) => index_list.push(e),
                                        None => return None,
                                    }
                                    match self.next_token.token {
                                        Token::Comma => continue,
                                        Token::Rbracket => {
                                            break;
                                        },
                                        _ => {
                                            self.error_got_unexpected_next_token();
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
                                self.error_missing_array_size();
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
                            match self.parse_expression(Precedence::Lowest, false) {
                                Some(e) => index_list.push(e),
                                None => return None,
                            }
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
                        self.errors.push(ParseError::new(
                            ParseErrorKind::ValueMustBeDefined(var_name),
                            self.next_token.pos,
                            self.script_name()
                        ));
                        return None;
                    } else {
                        Expression::Array(Vec::new(), index_list)
                    }
                } else {
                    self.bump();
                    let list = match self.parse_expression_list(Token::Eol) {
                        Some(vec_e) => vec_e,
                        None => return None
                    };
                    Expression::Array(list, index_list)
                }
            } else {
                // 変数定義
                // 代入演算子がなければ変数宣言のみ
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    if value_required {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::ValueMustBeDefined(var_name),
                            self.next_token.pos,
                            self.script_name()
                        ));
                        return None;
                    } else {
                        Expression::Literal(Literal::Empty)
                    }
                } else {
                    self.bump();
                    self.bump();
                    match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => e,
                        None => return None
                    }
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
                return self.parse_hashtable_statement(true);
            },
            _ => self.bump(),
        }
        match self.parse_variable_definition(false) {
            Some(v) => Some(Statement::Public(v)),
            None => None
        }
    }

    fn parse_dim_statement(&mut self) -> Option<Statement> {
        self.bump();
        match self.parse_variable_definition(false) {
            Some(v) => Some(Statement::Dim(v)),
            None => None
        }
    }

    fn parse_const_statement(&mut self) -> Option<Statement> {
        // match &self.next_token.token {
        //     Token::Identifier(_) => self.bump(),
        //     _ => return None,
        // }
        self.bump();
        match self.parse_variable_definition(true) {
            Some(v) => Some(Statement::Const(v)),
            None => None
        }
    }

    fn parse_hash_statement(&mut self) -> Option<Statement> {
        let is_public = if self.is_next_token(&Token::Public) {
            self.bump();
            true
        } else {
            false
        };
        self.bump();
        let name = if let Some(i) = self.parse_identifier() {
            i
        } else {
            self.error_token_is_not_identifier();
            return None;
        };
        let option = if self.is_next_token(&Token::EqualOrAssign) {
            self.bump();
            self.bump();
            match self.parse_expression(Precedence::Lowest, false) {
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
            let expression = self.parse_expression(Precedence::Lowest, false);
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
            self.error_invalid_hash_member_definition(expression, self.current_token.pos);
            return None;
        }
        let hash = HashSugar::new(name, option, is_public, members);
        Some(Statement::Hash(hash))
    }

    fn parse_hashtable_statement(&mut self, is_public: bool) -> Option<Statement> {
        self.bump();
        let mut expressions = vec![];

        loop {
            let identifier = match self.parse_identifier() {
                Some(i) => i,
                None => return None
            };
            let hash_option = if self.is_next_token(&Token::EqualOrAssign) {
                self.bump();
                self.bump();
                match self.parse_expression(Precedence::Lowest, false) {
                    Some(e) => Some(e),
                    None => return None
                }
            } else {
                None
            };
            expressions.push((identifier, hash_option, is_public));
            if self.is_next_token(&Token::Comma) {
                self.bump();
                self.bump();
            } else {
                break;
            }
        }
        Some(Statement::HashTbl(expressions))
    }

    fn parse_print_statement(&mut self) -> Option<Statement> {
        self.bump();
        let has_whitespace = self.current_token.skipped_whitespace;
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => if has_whitespace {
                e
            } else {
                self.errors.push(ParseError::new(
                    ParseErrorKind::WhitespaceRequiredAfter("print".into()),
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            },
            None => Expression::Literal(Literal::String("".to_string()))
        };

        Some(Statement::Print(expression))
    }


    fn parse_call_statement(&mut self) -> Option<Statement> {
        let (script, name, dir, args) = match self.next_token.token.clone() {
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

                let mut path = match &self.script_dir {
                    Some(p) => p.clone(),
                    None => PathBuf::new(),
                };
                if let Some(dir) = dir {
                    let new_path = PathBuf::from(dir);
                    if new_path.is_absolute() {
                        path = new_path;
                    } else {
                        path.push(new_path);
                    }
                }
                let dir = path.clone();
                path.push(&name);
                match path.extension() {
                    Some(os_str) => {
                        if let Some(ext) = os_str.to_str() {
                            // uwslファイルならデシリアライズして返す
                            if ext.to_ascii_lowercase().as_str() == "uwsl" {
                                match serializer::load(&path) {
                                    Ok(bin) => match serializer::deserialize(bin){
                                        Ok(program) => {
                                            return Some(Statement::Call(program, args));
                                        },
                                        Err(e) => {
                                            self.errors.push(ParseError::new(
                                                ParseErrorKind::CanNotLoadUwsl(
                                                    path.to_string_lossy().to_string(),
                                                    format!("{}", *e)
                                                ),
                                                self.current_token.pos,
                                                self.script_name()
                                            ));
                                        }
                                    },
                                    Err(e) => {
                                        self.errors.push(ParseError::new(
                                            ParseErrorKind::CanNotLoadUwsl(
                                                path.to_string_lossy().to_string(),
                                                e.to_string()
                                            ),
                                            self.current_token.pos,
                                            self.script_name()
                                        ));
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
                            self.errors.push(ParseError::new(
                                ParseErrorKind::CanNotCallScript(path.to_string_lossy().to_string(), e.to_string()),
                                self.current_token.pos,
                                self.script_name()
                            ));
                            return None;
                        },
                    };
                    break script;
                };
                (script, name, Some(dir), args)
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
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidCallUri(uri),
                            self.next_token.pos,
                            self.script_name()
                        ));
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
                (script, uri, None, args)
            },
            _ => {
                self.error_got_unexpected_next_token();
                return None;
            }
        };

        let call_parser = Parser::call(
            Lexer::new(&script),
            name,
            dir,
        );
        match call_parser.parse() {
            Ok(program) => Some(Statement::Call(program, args)),
            Err(mut e) => {
                self.errors.append(&mut e);
                None
            },
        }
    }

    fn parse_def_dll_statement(&mut self) -> Option<Statement> {
        self.bump();
        let name = match self.current_token.token {
            Token::Identifier(ref s) => s.clone(),
            _ => {
                self.error_token_is_not_identifier();
                return None;
            }
        };
        let (name, alias) = match &self.next_token.token {
            Token::Lparen => {
                (name, None)
            },
            Token::Colon => {
                self.bump();
                self.bump();
                let alias = Some(name);
                let name = if let Token::Identifier(ident) = &self.current_token.token() {
                    ident.to_string()
                } else {
                    self.error_token_is_not_identifier();
                    return None;
                };
                (name, alias)
            },
            _ => {
                self.error_got_unexpected_next_token();
                return None;
            }
        };
        if ! self.is_next_token_expected(Token::Lparen) {
            return None;
        }

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
                            self.error_got_unexpected_token();
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
                    self.error_got_unexpected_token();
                    return None;
                },
            }
            self.bump();
        }
        if ! self.is_current_token_expected(Token::Rparen) {
            return None;
        }
        if ! self.is_next_token_expected(Token::Colon) {
            return None;
        }
        // 戻りの型, dllパス
        // :型:パス
        // ::パス
        // :パス
        // 型省略時はVoid返す
        let (ret_type, path) = match self.next_token.token {
            Token::Colon => {
                // ::パス
                self.bump();
                match self.parse_dll_path() {
                    Some(p) => (DllType::Void, p),
                    None => return None,
                }
            },
            Token::Identifier(ref s) => {
                match s.parse() {
                    Ok(t) => {
                        // :型:パス
                        self.bump();
                        if self.is_next_token(&Token::Colon) {
                            self.bump();
                            match self.parse_dll_path() {
                                Some(p) => (t, p),
                                None => return None,
                            }
                        } else {
                            self.error_got_unexpected_token();
                            return None;
                        }
                    },
                    Err(_) => {
                        // :パス
                        match self.parse_dll_path() {
                            Some(p) => (DllType::Void, p),
                            None => return None,
                        }
                    },
                }
            },
            _ => {
                self.error_got_unexpected_token();
                return None;
            },
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
                            self.error_got_unexpected_token();
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
                    self.error_got_unexpected_token();
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

    fn parse_dll_path(&mut self) -> Option<String> {
        self.bump();
        let pos = self.current_token.pos;
        let mut path = String::new();
        while ! self.is_current_token_in(vec![Token::Eol, Token::Eof]) {
            match self.current_token.token {
                Token::Identifier(ref s) => path = format!("{}{}", path, s),
                Token::ColonBackSlash => path = format!("{}:\\", path),
                Token::BackSlash => path = format!("{}\\", path),
                Token::Period => path = format!("{}.", path),
                _ => {
                    self.error_got_invalid_dllpath(pos);
                    return None;
                },
            }
            self.bump();
        }
        Some(path)
    }

    fn parse_dll_param(&mut self, is_ref: bool) -> Option<DefDllParam> {
        let dll_type = match &self.current_token.token {
            Token::Identifier(s) => match s.parse::<DllType>() {
                Ok(t) => t,
                Err(name) => {
                    self.error_got_invalid_dlltype(name);
                    return None;
                },
            },
            Token::Struct => DllType::UStruct,
            _ => {
                self.error_got_unexpected_token();
                return None;
            },
        };
        if dll_type == DllType::CallBack {
            if ! self.is_next_token_expected(Token::Lparen) {
                return None;
            }
            self.bump(); // ( の次のトークンに移動
            let mut argtypes = vec![];
            loop {
                let t = match &self.current_token.token {
                    Token::Identifier(i) => match DllType::from_str(&i) {
                        Ok(t) => t,
                        Err(name) => {
                            self.error_got_invalid_dlltype(name);
                            return None;
                        },
                    },
                    Token::Rparen => {
                        break;
                    }
                    _ => {
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                            self.error_got_invalid_dlltype(name);
                            return None;
                        },
                    },
                    _ => {
                        self.error_got_unexpected_token();
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
                    Token::Identifier(i) => {
                        if ! self.is_next_token_expected(Token::Rbracket) {
                            return None;
                        }
                        Some(DefDllParam::Param{ dll_type, is_ref, size: DefDllParamSize::Const(i) })
                    },
                    Token::Num(n) => {
                        if ! self.is_next_token_expected(Token::Rbracket) {
                            return None;
                        }
                        Some(DefDllParam::Param { dll_type, is_ref, size: DefDllParamSize::Size(n as usize) })
                    },
                    _ => {
                        self.error_got_unexpected_token();
                        return None;
                    },
                }
            } else {
                Some(DefDllParam::Param{dll_type, is_ref, size: DefDllParamSize::None})
            }
        }
    }

    fn parse_struct_statement(&mut self) -> Option<Statement> {
        self.bump();
        let name = match self.parse_identifier_expression() {
            Some(Expression::Identifier(i)) => i,
            _ => {
                self.error_token_is_not_identifier();
                return None;
            },
        };
        self.bump();
        self.bump();

        let mut struct_definition = vec![];
        while ! self.is_current_token_end_of_block() {
            // 空行及びコメント対策
            if self.current_token.token == Token::Eol {
                self.bump();
                continue;
            }
            let member = match self.parse_identifier_expression() {
                Some(Expression::Identifier(Identifier(i))) => i,
                _ => {
                    self.error_token_is_not_identifier();
                    return None;
                },
            };

            if ! self.is_next_token_expected(Token::Colon) {
                return None;
            }
            self.bump();
            let is_ref = if self.is_current_token(&Token::Ref) {
                self.bump();
                true
            } else {
                false
            };
            let member_type = match self.parse_identifier_expression() {
                Some(Expression::Identifier(Identifier(s))) => s.to_ascii_lowercase(),
                _ => {
                    self.error_token_is_not_identifier();
                    return None;
                },
            };
            let size = if let Token::Lbracket = self.next_token.token {
                self.bump();
                self.bump();
                match (&self.current_token.token, &self.next_token.token) {
                    (Token::Num(n), Token::Rbracket) => {
                        DefDllParamSize::Size(*n as usize)
                    },
                    (Token::Identifier(i), Token::Rbracket) => {
                        DefDllParamSize::Const(i.to_string())
                    },
                    (Token::Num(_), _) => {
                        self.error_got_unexpected_next_token();
                        return None;
                    },
                    _ => {
                        self.error_got_unexpected_token();
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
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndStruct)) {
            self.errors.push(ParseError::new(
                ParseErrorKind::InvalidBlockEnd(Token::BlockEnd(BlockEnd::EndStruct), self.current_token.token()),
                self.current_token.pos,
                self.script_name()
            ));
            return None;
        }
        Some(Statement::Struct(name, struct_definition))
    }

    fn parse_continue_statement(&mut self) -> Option<Statement> {
        if ! self.in_loop {
            self.errors.push(ParseError::new(
                ParseErrorKind::OutOfLoop(Token::Continue),
                self.current_token.pos,
                self.script_name()
            ));
            return None;
        }
        self.bump();
        match self.parse_number_expression() {
            Some(Expression::Literal(Literal::Num(n))) => Some(Statement::Continue(n as u32)),
            Some(_) => None,
            None => Some(Statement::Continue(1)),
        }
    }

    fn parse_break_statement(&mut self) -> Option<Statement> {
        if ! self.in_loop {
            self.errors.push(ParseError::new(
                ParseErrorKind::OutOfLoop(Token::Break),
                self.current_token.pos,
                self.script_name()
            ));
            return None;
        }
        self.bump();
        match self.parse_number_expression() {
            Some(Expression::Literal(Literal::Num(n))) => Some(Statement::Break(n as u32)),
            Some(_) => None,
            None => Some(Statement::Break(1)),
        }
    }

    fn parse_loop_block_statement(&mut self) -> BlockStatement {
        let is_in_loop = self.in_loop;
        self.in_loop = true;
        let block = self.parse_block_statement();
        if ! is_in_loop {
            self.in_loop = false;
        }
        block
    }

    fn parse_for_statement(&mut self) -> Option<Statement> {
        self.bump();
        let loopvar = match self.parse_identifier() {
            Some(i) => i,
            None => {
                self.error_token_is_not_identifier();
                return None;
            }
        };
        let comma_pos = self.next_token.pos;
        let index_var = if let Token::Comma = self.next_token.token {
            self.bump();
            if let Token::Comma = self.next_token.token {
                None
            } else {
                self.bump();
                match self.parse_identifier() {
                    Some(i) => Some(i),
                    None => {
                        self.error_token_is_not_identifier();
                        return None;
                    },
                }
            }
        } else {
            None
        };
        let islast_var = if let Token::Comma = self.next_token.token {
            self.bump();
            self.bump();
            match self.parse_identifier() {
                Some(i) => Some(i),
                None => {
                    self.error_token_is_not_identifier();
                    return None;
                },
            }
        } else {
            None
        };
        match self.next_token.token {
            Token::EqualOrAssign => {
                // for文
                // for-inの特殊記法はNG
                if index_var.is_some() || islast_var.is_some() {
                    self.errors.push(ParseError::new(
                        ParseErrorKind::UnexpectedToken(Token::EqualOrAssign, Token::Comma),
                        comma_pos,
                        self.script_name()
                    ));
                    return None;
                }
                self.bump();
                self.bump();
                let from = match self.parse_expression(Precedence::Lowest, false) {
                    Some(e) => e,
                    None => return None
                };
                if ! self.is_next_token_expected(Token::To) {
                    return None;
                }
                self.bump();
                let to = match self.parse_expression(Precedence::Lowest, false) {
                    Some(e) => e,
                    None => return None
                };
                let step = if self.is_next_token(&Token::Step) {
                    self.bump();
                    self.bump();
                    match self.parse_expression(Precedence::Lowest, false) {
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
                        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndFor)) {
                            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndFor));
                            return None;
                        }
                        Some(alt)
                    },
                    _ => {
                        self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Next));
                        return None;
                    },
                };
                Some(Statement::For{loopvar, from, to, step, block, alt})
            },
            Token::In => {
                // for-in
                self.bump();
                self.bump();
                let collection = match self.parse_expression(Precedence::Lowest, false) {
                    Some(e) => e,
                    None => return None
                };
                self.bump();
                let block = self.parse_loop_block_statement();

                let alt = match &self.current_token.token {
                    Token::BlockEnd(BlockEnd::Next) => None,
                    Token::BlockEnd(BlockEnd::Else) => {
                        self.bump();
                        let alt = self.parse_block_statement();
                        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndFor)) {
                            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndFor));
                            return None;
                        }
                        Some(alt)
                    },
                    _ => {
                        self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Next));
                        return None;
                    },
                };
                Some(Statement::ForIn{loopvar, index_var, islast_var, collection, block, alt})
            },
            _ => {
                self.error_got_unexpected_token();
                return None;
            }
        }
    }

    fn parse_while_statement(&mut self) -> Option<Statement> {
        self.bump();
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };
        let block = self.parse_loop_block_statement();
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Wend)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Wend));
            return None;
        }
        Some(Statement::While(expression, block))
    }

    fn parse_repeat_statement(&mut self) -> Option<Statement> {
        self.bump();
        let block = self.parse_loop_block_statement();
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Until)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Until));
            return None;
        }
        self.bump();
        let row = self.current_token.pos.row;
        let line = self.lexer.get_line(row);
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
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
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => match e {
                Expression::FuncCall{func:_, args:_,is_await:_} => {
                    let with_temp = Expression::Identifier(Identifier(self.get_with_temp_name()));
                    with_temp_assignment = Some(Statement::Expression(Expression::Assign(Box::new(with_temp.clone()), Box::new(e))));
                    with_temp
                },
                _ => e
            },
            None => return None,
        };
        let current_with = self.get_current_with();
        self.set_with(Some(expression.clone()));
        let mut block = self.parse_block_statement();
        if with_temp_assignment.is_some() {
            block.insert(0, StatementWithRow::new_non_existent_line(
                with_temp_assignment.unwrap()
            ));
        }
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndWith)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndWith));
            return None;
        }
        self.set_with(current_with);
        Some(Statement::With(Some(expression), block))
    }

    fn parse_try_statement(&mut self) -> Option<Statement> {
        self.bump();
        let trys = self.parse_block_statement();
        let mut except = None;
        let mut finally = None;
        match self.current_token.token.clone() {
            Token::BlockEnd(BlockEnd::Except) => {
                self.bump();
                except = Some(self.parse_block_statement());
            },
            Token::BlockEnd(BlockEnd::Finally) => {},
            t => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::UnexpectedToken3(vec![
                        Token::BlockEnd(BlockEnd::Except),
                        Token::BlockEnd(BlockEnd::Finally)
                    ], t),
                    self.current_token.pos,
                    self.script_name()
            ));
                return None;
            },
        }
        match self.current_token.token.clone() {
            Token::BlockEnd(BlockEnd::Finally) => {
                self.bump();
                finally = match self.parse_finally_block_statement() {
                    Ok(b) => Some(b),
                    Err(s) => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidStatementInFinallyBlock(s),
                            self.current_token.pos,
                            self.script_name()
                        ));
                        return None;
                    }
                };
            },
            Token::BlockEnd(BlockEnd::EndTry) => {},
            t => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::UnexpectedToken3(vec![
                        Token::BlockEnd(BlockEnd::Finally),
                        Token::BlockEnd(BlockEnd::EndTry)
                    ], t),
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            },
        }
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndTry)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndTry));
            return None;
        }

        Some(Statement::Try {trys, except, finally})
    }

    fn parse_finally_block_statement(&mut self) -> Result<BlockStatement, String> {
        self.bump();
        let mut block: BlockStatement  = vec![];

        while ! self.is_current_token_end_of_block() && ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(s) => match s.statement {
                    Statement::Exit => return Err("exit".into()),
                    Statement::Continue(_) => return Err("continue".into()),
                    Statement::Break(_) => return Err("break".into()),
                    _ => block.push(s)
                }
                None => ()
            }
            self.bump();
        }

        Ok(block)
    }

    fn parse_exitexit_statement(&mut self) -> Option<Statement> {
        self.bump();
        if let Token::Num(n) = self.current_token.token {
            Some(Statement::ExitExit(n as i32))
        } else if self.is_current_token_in(vec![Token::Eol, Token::Eof]) {
            Some(Statement::ExitExit(0))
        } else {
            self.errors.push(ParseError::new(
                ParseErrorKind::InvalidExitCode,
                self.current_token.pos,
                self.script_name()
            ));
            None
        }
    }

    fn parse_textblock_statement(&mut self, is_ex: bool) -> Option<Statement> {
        self.bump();
        let name = match self.current_token.token {
            Token::Identifier(ref name) => {
                Some(Identifier(name.clone()))
            },
            Token::Eol => None,
            _ => {
                self.error_got_unexpected_token();
                return None;
            },
        };
        if name.is_some() {
            self.bump();
        }
        self.bump();
        let body = if let Token::TextBlockBody(ref body) = self.current_token.token {
            body.clone()
        } else {
            self.errors.push(ParseError::new(
                ParseErrorKind::TextBlockBodyIsMissing,
                self.current_token.pos,
                self.script_name()
            ));
            return None;
        };
        if self.is_next_token(&Token::EndTextBlock) {
            self.bump()
        } else {
            self.errors.push(ParseError::new(
                ParseErrorKind::InvalidBlockEnd(Token::EndTextBlock, self.next_token.token()),
                self.current_token.pos,
                self.script_name()
            ));
            return None;
        }
        self.bump();
        if name.is_some() {
            Some(Statement::TextBlock(name.unwrap(), Literal::TextBlock(body, is_ex)))
        } else {
            // コメントtextblock
            None
        }
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        match self.parse_expression(Precedence::Lowest, true) {
            Some(e) => {
                if self.is_next_token(&Token::Semicolon) || self.is_next_token(&Token::Eol) {
                    self.bump();
                }
                Some(Statement::Expression(e))
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
                    Statement::Option(OptionSetting::Explicit(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::Explicit(b))
                    } else {
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
                        return None;
                    }
                }
            },
            "optpublic" => {
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    Statement::Option(OptionSetting::OptPublic(true))
                } else {
                    self.bump();
                    self.bump();
                    if let Token::Bool(b) = self.current_token.token {
                        Statement::Option(OptionSetting::OptPublic(b))
                    } else {
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
                        return None;
                    }
                }
            },
            "defaultfont" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Defaultfont(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Defaultfont(s.clone()))
                } else {
                    self.error_got_unexpected_token();
                    return None;
                }
            },
            "position" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n1) = self.current_token.token {
                    if ! self.is_next_token_expected(Token::Comma) {
                        return None;
                    }
                    if let Token::Num(n2) = self.current_token.token {
                        return Some(Statement::Option(OptionSetting::Position(n1 as i32, n2 as i32)));
                    }
                }
                self.error_got_unexpected_token();
                return None;
            },
            "logpath" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Logpath(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Logpath(s.clone()))
                } else {
                    self.error_got_unexpected_token();
                    return None;
                }
            },
            "loglines" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n) = self.current_token.token {
                    Statement::Option(OptionSetting::Loglines(n as i32))
                } else {
                    self.error_got_unexpected_token();
                    return None;
                }
            },
            "logfile" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::Num(n) = self.current_token.token {
                    Statement::Option(OptionSetting::Logfile(n as i32))
                } else {
                    self.error_got_unexpected_token();
                    return None;
                }
            },
            "dlgtitle" => {
                if ! self.is_next_token_expected(Token::EqualOrAssign) {
                    return None;
                }
                self.bump();
                if let Token::String(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Dlgtitle(s.clone()))
                } else if let Token::ExpandableString(ref s) = self.current_token.token {
                    Statement::Option(OptionSetting::Dlgtitle(s.clone()))
                } else {
                    self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
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
                        self.error_got_unexpected_token();
                        return None;
                    }
                }
            },
            name => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::UnexpectedOption(name.to_string()),
                    self.current_token.pos, self.script_name()
                ));
                return None;
            },
        };
        Some(statement)
    }

    fn parse_enum_statement(&mut self) -> Option<Statement> {
        self.bump();
        let name = if let Some(Identifier(name)) = self.parse_identifier() {
            name
        } else {
            self.error_token_is_not_identifier();
            return None;
        };
        self.bump();
        self.bump();
        let mut u_enum = UEnum::new(&name);
        let mut next = 0.0;
        loop {
            if let Some(Identifier(id)) = self.parse_identifier() {
                if self.is_next_token(&Token::EqualOrAssign) {
                    self.bump();
                    self.bump();
                    let n = match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => match e {
                            Expression::Literal(Literal::Num(n)) => n,
                            _ => {
                                self.errors.push(ParseError::new(
                                    ParseErrorKind::EnumMemberShouldBeNumber(name, id),
                                    self.current_token.pos,
                                    self.script_name()
                                ));
                                return None;
                            },
                        },
                        None => {
                            self.errors.push(ParseError::new(
                                ParseErrorKind::EnumValueShouldBeDefined(name, id),
                                self.current_token.pos,
                                self.script_name()
                            ));
                            return None;
                        },
                    };
                    // next以下の数値が指定されたらエラー
                    if n < next {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::EnumValueIsInvalid(name, id, next),
                            self.current_token.pos,
                            self.script_name()
                        ));
                        return None;
                    }
                    next = n;
                }
                if u_enum.add(&id, next).is_err() {
                    self.errors.push(ParseError::new(
                        ParseErrorKind::EnumMemberDuplicated(name, id),
                        self.current_token.pos,
                        self.script_name()
                    ));
                    return None;
                }
                if ! self.is_next_token_expected(Token::Eol) {
                    return None;
                }
                self.bump();
                if self.is_current_token_end_of_block() {
                    break;
                }
                next += 1.0;
            } else {
                self.error_token_is_not_identifier();
                return None;
            }
        }
        if ! self.is_expected_close_token(Token::BlockEnd(BlockEnd::EndEnum)) {
            return None;
        }
        Some(Statement::Enum(name, u_enum))
    }

    fn parse_thread_statement(&mut self) -> Option<Statement> {
        self.bump();
        let expression = self.parse_expression(Precedence::Lowest, false);
        match expression {
            Some(Expression::FuncCall{func:_,args:_,is_await:_}) => Some(Statement::Thread(expression.unwrap())),
            _ => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::InvalidThreadCall,
                    self.current_token.pos,
                    self.script_name()
                ));
                None
            }
        }
    }

    fn parse_assignment(&mut self, token: Token, expression: Expression) -> Option<Expression> {
        match token {
            Token::EqualOrAssign => return self.parse_assign_expression(expression),
            Token::AddAssign => return self.parse_compound_assign_expression(expression, Token::AddAssign),
            Token::SubtractAssign => return self.parse_compound_assign_expression(expression, Token::SubtractAssign),
            Token::MultiplyAssign => return self.parse_compound_assign_expression(expression, Token::MultiplyAssign),
            Token::DivideAssign => return self.parse_compound_assign_expression(expression, Token::DivideAssign),
            _ => None
        }
    }

    fn parse_expression(&mut self, precedence: Precedence, is_sol: bool) -> Option<Expression> {
        // prefix
        let mut left = match self.current_token.token {
            Token::Identifier(_) => {
                let identifier = self.parse_identifier_expression();
                if is_sol {
                    if let Some(e) = self.parse_assignment(self.next_token.token.clone(), identifier.clone().unwrap()) {
                        return Some(e);
                    }
                }
                identifier
            },
            Token::Empty => Some(Expression::Literal(Literal::Empty)),
            Token::Null => Some(Expression::Literal(Literal::Null)),
            Token::Nothing => Some(Expression::Literal(Literal::Nothing)),
            Token::NaN => Some(Expression::Literal(Literal::NaN)),
            Token::Num(_) => self.parse_number_expression(),
            Token::Hex(_) => self.parse_hex_expression(),
            Token::ExpandableString(_) |
            Token::String(_) => self.parse_string_expression(),
            Token::Bool(_) => self.parse_bool_expression(),
            Token::Lbracket => self.parse_array_expression(),
            Token::Bang | Token::Minus | Token::Plus => self.parse_prefix_expression(),
            Token::Lparen => self.parse_grouped_expression(),
            Token::Function => self.parse_function_expression(false),
            Token::Procedure => self.parse_function_expression(true),
            Token::Await => return self.parse_await_func_call_expression(),
            Token::Then | Token::Eol => return None,
            Token::Period => {
                let e = self.parse_with_dot_expression();
                if is_sol && e.is_some() {
                    if let Some(e) = self.parse_assignment(self.next_token.token.clone(), e.clone().unwrap()) {
                        return Some(e);
                    }
                }
                e
            },
            Token::UObject(ref s) => {
                Some(Expression::UObject(s.clone()))
            },
            Token::UObjectNotClosing => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::InvalidUObjectEnd,
                    self.current_token.pos,
                    self.script_name()
                ));
                return None
            },
            Token::Pipeline => self.parse_lambda_function_expression(),
            Token::ComErrFlg => Some(Expression::ComErrFlg),
            Token::Ref => {
                // COMメソッドの引数にvarが付く場合
                // var <Identifier> とならなければいけない
                self.bump();
                match self.parse_expression(Precedence::Lowest, false) {
                    Some(e) => return Some(Expression::RefArg(Box::new(e))),
                    None => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::MissingIdentifierAfterVar,
                            self.current_token.pos,
                            self.script_name()
                        ));
                        return None
                    }
                }
            },
            _ => match self.parse_identifier_expression() {
                Some(e) => {
                    if is_sol {
                        if let Some(e) = self.parse_assignment(self.next_token.token.clone(), e.clone()) {
                            return Some(e);
                        }
                    }
                    Some(e)
                },
                None => return None
            },
        };


        // infix
        while (
            ! self.is_next_token(&Token::Semicolon)
            || ! self.is_next_token(&Token::Eol)
        ) && precedence < self.next_token_precedence() {
            if left.is_none() {
                return None;
            }
            match self.next_token.token {
                Token::Plus |
                Token::Minus |
                Token::Slash |
                Token::Asterisk |
                Token::Equal |
                Token::EqualOrAssign |
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
                Token::Mod |
                Token::To |
                Token::Step |
                Token::In => {
                    self.bump();
                    left = self.parse_infix_expression(left.unwrap());
                },
                Token::Assign => left = self.parse_assign_expression(left.unwrap()),
                Token::Lbracket => {
                    self.bump();
                    left = {
                        let index = match self.parse_index_expression(left.unwrap()) {
                            Some(e) => e,
                            None => {
                                self.errors.push(ParseError::new(
                                    ParseErrorKind::MissingIndex,
                                    self.next_token.pos, self.script_name()
                                ));
                                return None;
                            },
                        };
                        if is_sol {
                            if let Some(e) = self.parse_assignment(self.next_token.token.clone(), index.clone()) {
                                return Some(e);
                            }
                        }
                        Some(index)
                    }
                },
                Token::Lparen => {
                    self.bump();
                    left = self.parse_function_call_expression(left.unwrap(), false);
                },
                Token::Question => {
                    self.bump();
                    left = self.parse_ternary_operator_expression(left.unwrap());
                },
                Token::Period => {
                    self.bump();
                    left = {
                        let dotcall = self.parse_dotcall_expression(left.unwrap());
                        if is_sol {
                            if let Some(e) = self.parse_assignment(self.next_token.token.clone(), dotcall.clone().unwrap()) {
                                return Some(e);
                            }
                        }
                        dotcall
                    }
                },
                _ => return left
            }
        }

        left
    }

    fn parse_identifier(&mut self) -> Option<Identifier> {
        let token = self.current_token.token.clone();
        match &token {
            Token::Identifier(ref i) => Some(Identifier(i.clone())),
            t => self.token_to_identifier(&t),
        }
    }

    fn parse_identifier_expression(&mut self) -> Option<Expression> {
        match self.parse_identifier() {
            Some(i) => Some(Expression::Identifier(i)),
            None => None
        }
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
            Token::NaN => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::ReservedKeyword(token.clone()),
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            },
            Token::Blank |
            Token::Eof |
            Token::Eol |
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
            Token::Option(_) |
            Token::Comment |
            Token::Ref |
            Token::Variadic |
            Token::Pipeline |
            Token::Arrow |
            Token::Illegal(_) => {
                self.error_no_prefix_parser();
                return None;
            },
            Token::Print |
            Token::Dim |
            Token::Public |
            Token::Const |
            Token::Thread |
            Token::HashTable |
            Token::Uri(_) |
            Token::Path(_, _) |
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
            Token::TextBlockBody(_) |
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
            Token::ComErrFlg |
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
                self.errors.push(ParseError::new(
                    ParseErrorKind::OutOfWith,
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            }
        }
    }

    fn parse_number_expression(&mut self) -> Option<Expression> {
        match self.current_token.token {
            Token::Num(ref mut num) => Some(
                Expression::Literal(Literal::Num(num.clone()))
            ),
            _ => None
        }
    }

    fn parse_hex_expression(&mut self) -> Option<Expression> {
        if let Token::Hex(ref s) = self.current_token.token {
            match u64::from_str_radix(s, 16) {
                Ok(u) => Some(Expression::Literal(Literal::Num(u as i64 as f64))),
                Err(_) => {
                    self.errors.push(ParseError::new(
                        ParseErrorKind::InvalidHexNumber(s.to_string()),
                        self.current_token.pos,
                        self.script_name()
                    ));
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

        match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => list.push(e),
            None => return None
        }

        while self.is_next_token(&Token::Comma) {
            self.bump();
            if skip_eol {self.skip_next_eol();}
            self.bump();
            match self.parse_expression(Precedence::Lowest, false) {
                Some(e) => list.push(e),
                None => return None
            }
            if skip_eol {self.skip_next_eol();}
        }

        if end == Token::Eol {
            if ! self.is_next_token(&end) && ! self.is_next_token(&Token::Eof) {
                self.error_got_unexpected_next_token();
                return None;
            }
        } else {
            if ! self.is_next_token_expected(end) {
                return None;
            }
        }

        Some(list)
    }

    fn parse_assign_expression(&mut self, left: Expression) -> Option<Expression> {
        self.bump();
        self.bump();

        match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => Some(Expression::Assign(Box::new(left), Box::new(e))),
            None => None
        }
    }

    fn parse_compound_assign_expression(&mut self, left: Expression, token: Token) -> Option<Expression> {
        self.bump();
        self.bump();

        let right = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };

        let infix = match token {
            Token::AddAssign => Infix::Plus,
            Token::SubtractAssign => Infix::Minus,
            Token::MultiplyAssign => Infix::Multiply,
            Token::DivideAssign => Infix::Divide,
            _ => return None
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

        match self.parse_expression(Precedence::Prefix, false) {
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

        match self.parse_expression(precedence, false) {
            Some(e) => Some(Expression::Infix(infix, Box::new(left), Box::new(e))),
            None => None
        }
    }

    fn parse_index_expression(&mut self, left: Expression) -> Option<Expression> {
        self.bump();
        let index = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };
        let hash_enum = if self.is_next_token(&Token::Comma) {
            self.bump();
            self.bump();
            match self.parse_expression(Precedence::Lowest, false) {
                Some(e) => Some(e),
                None => return None
            }
        } else {
            None
        };
        if ! self.is_next_token_expected(Token::Rbracket) {
            return None;
        }

        Some(Expression::Index(Box::new(left), Box::new(index), Box::new(hash_enum)))
    }

    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.bump();
        let expression = self.parse_expression(Precedence::Lowest, false);
        if ! self.is_next_token_expected(Token::Rparen) {
            None
        } else {
            expression
        }
    }

    fn parse_dotcall_expression(&mut self, left: Expression) -> Option<Expression> {
        self.bump();
        let member = match self.parse_identifier_expression() {
            Some(e) => e,
            None => {
                self.error_token_is_not_identifier();
                return None;
            }
        };
        Some(Expression::DotCall(Box::new(left), Box::new(member)))
    }

    fn parse_if_statement(&mut self) -> Option<Statement> {
        self.bump();
        let condition = match self.parse_expression(Precedence::Lowest, false) {
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
            let consequence = match self.parse_statement() {
                Some(s) => s,
                None => return None
            };
            let alternative = if self.is_next_token(&Token::BlockEnd(BlockEnd::Else)) {
                self.bump();
                self.bump();
                match self.parse_statement() {
                    Some(s) => Some(s),
                    None => return None
                }
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
            if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndIf)) {
                self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndIf));
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
                    let elseifcond = match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => e,
                        None => return None
                    };
                    let condstmt = StatementWithRow::new(Statement::Expression(elseifcond), row, line, Some(self.script_name()));
                    alternatives.push((Some(condstmt), self.parse_block_statement()));
                }
            }
        }
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndIf)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::EndIf));
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
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };
        let mut cases = vec![];
        let mut default = None;
        self.bump();
        self.bump();
        while self.is_current_token_in(vec![Token::BlockEnd(BlockEnd::Case), Token::BlockEnd(BlockEnd::Default)]) {
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
                _ => return None
            }
        }
        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Selend)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Selend));
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
                self.errors.push(ParseError::new(
                    ParseErrorKind::FunctionRequiredAfterAsync,
                    self.current_token.pos,
                    self.script_name(),
                ));
                return None;
            },
        }
    }

    fn parse_function_statement(&mut self, is_proc: bool, is_async: bool) -> Option<Statement> {
        self.bump();
        let name = match self.parse_identifier() {
            Some(i) => i,
            None => {
                self.error_token_is_not_identifier();
                return None;
            },
        };

        let params = if self.is_next_token(&Token::Lparen) {
            self.bump();
            match self.parse_function_parameters(Token::Rparen) {
                Some(p) => p,
                None => return None
            }
        } else {
            vec![]
        };

        self.bump();
        let body = self.parse_block_statement();

        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Fend)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Fend));
            return None;
        }
        Some(Statement::Function{name, params, body, is_proc, is_async})
    }

    fn parse_module_statement(&mut self) -> Option<Statement> {
        self.bump();
        let identifier = match self.parse_identifier() {
            Some(i) => i,
            None => {
                self.error_token_is_not_identifier();
                return None;
            },
        };
        self.bump();
        let mut block = vec![];
        while ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndModule)) {
            if self.is_current_token(&Token::Eof) {
                self.errors.push(ParseError::new(
                    ParseErrorKind::InvalidBlockEnd(Token::BlockEnd(BlockEnd::EndModule), self.current_token.token.clone()),
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            }
            match self.parse_statement() {
                Some(s) => block.push(s),
                None => ()
            }
            self.bump();
        }
        Some(Statement::Module(identifier, block))
    }

    fn parse_class_statement(&mut self) -> Option<Statement> {
        let class_statement_pos = self.current_token.pos;
        self.bump();
        let identifier = match self.parse_identifier() {
            Some(i) => i,
            None => {
                self.error_token_is_not_identifier();
                return None;
            },
        };
        self.bump();
        let mut block = vec![];
        let mut has_constructor = false;
        while ! self.is_current_token(&Token::BlockEnd(BlockEnd::EndClass)) {
            if self.is_current_token(&Token::Eof) {
                self.errors.push(ParseError::new(
                    ParseErrorKind::InvalidBlockEnd(Token::BlockEnd(BlockEnd::EndClass), self.current_token.token.clone()),
                    self.current_token.pos,
                    self.script_name()
                ));
                return None;
            }
            let cur_pos = self.current_token.pos;
            match self.parse_statement() {
                Some(s) => match s.statement {
                    Statement::Dim(_) |
                    Statement::Public(_) |
                    Statement::Const(_) |
                    Statement::TextBlock(_, _) |
                    Statement::DefDll { name: _, alias:_, params: _, ret_type: _, path: _ } |
                    Statement::HashTbl(_) => block.push(s),
                    Statement::Function{ref name, params: _, body: _, is_proc: _, is_async:_} => {
                        if name == &identifier {
                            has_constructor = true;
                        }
                        block.push(s);
                    },
                    _ => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidClassMemberDefinition(s.statement),
                            cur_pos,
                            self.script_name()
                        ));
                        return None;
                    },
                },
                None => ()
            }
            self.bump();
        }
        if ! has_constructor {
            self.errors.push(ParseError::new(
                ParseErrorKind::ClassHasNoConstructor(identifier),
                class_statement_pos,
                self.script_name()
            ));
            return None;
        }
        Some(Statement::Class(identifier, block))
    }

    fn parse_ternary_operator_expression(&mut self, left: Expression) -> Option<Expression> {

        self.bump();
        let consequence = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => Box::new(e),
            None => return None
        };

        if ! self.is_next_token_expected(Token::Colon) {
            return None;
        }
        self.bump();
        let alternative = match self.parse_expression(Precedence::Lowest, false) {
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
        if ! self.is_next_token_expected(Token::Lparen) {
            return None;
        }

        let params = match self.parse_function_parameters(Token::Rparen) {
            Some(p) => p,
            None => return None
        };

        let body = self.parse_block_statement();

        if ! self.is_current_token(&Token::BlockEnd(BlockEnd::Fend)) {
            self.error_got_invalid_close_token(Token::BlockEnd(BlockEnd::Fend));
            return None;
        }

        Some(Expression::AnonymusFunction {params, body, is_proc})
    }

    fn parse_lambda_function_expression(&mut self) -> Option<Expression> {
        let params = if self.is_next_token(&Token::Arrow) {
            // 引数なし
            self.bump();
            vec![]
        } else {
            match self.parse_function_parameters(Token::Arrow) {
                Some(p) => p,
                None => return None,
            }
        };
        self.bump(); // skip =>

        let mut body = vec![];
        loop {
            let optexpr = self.parse_expression(Precedence::Lowest, true);
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
                self.error_got_unexpected_next_token();
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
                    let ident = Identifier(param.name());
                    match &param.kind {
                        ParamKind::Identifier |
                        ParamKind::Reference => {
                            if with_default_flg {
                                self.error_got_bad_parameter(ParseErrorKind::ParameterShouldBeDefault(ident));
                                return None;
                            } else if variadic_flg {
                                self.error_got_bad_parameter(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(ident));
                                return None;
                            }
                        },
                        ParamKind::Default(_) => if variadic_flg {
                            self.error_got_bad_parameter(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(ident));
                            return None;
                        } else {
                            with_default_flg = true;
                        },
                        ParamKind::Variadic => if with_default_flg {
                            self.error_got_bad_parameter(ParseErrorKind::ParameterShouldBeDefault(ident));
                            return None;
                        } else if variadic_flg {
                            self.error_got_bad_parameter(ParseErrorKind::ParameterCannotBeDefinedAfterVariadic(ident));
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
        if ! self.is_next_token_expected(end_token) {
            // self.error_got_invalid_close_token(Token::Rparen);
            return None;
        }
        Some(params)
    }

    fn parse_param(&mut self) -> Option<FuncParam> {
        match self.current_token.token() {
            Token::Identifier(name) => {
                let (kind, param_type) = if self.is_next_token(&Token::Lbracket) {
                    // 配列引数定義
                    self.bump();
                    let k = if self.is_next_token_expected(Token::Rbracket) {
                        while self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if !self.is_next_token_expected(Token::Rbracket) {
                                return None;
                            }
                        }
                        ParamKind::Identifier
                    } else {
                        return None;
                    };
                    let t = if self.is_next_token(&Token::Colon) {
                        self.bump(); // : に移動
                        match self.parse_param_type() {
                            Some(t) => t,
                            None => return None
                        }
                    } else {
                        ParamType::Any
                    };
                    (k, t)
                } else {
                    // 型指定の有無
                    let t = if self.is_next_token(&Token::Colon) {
                        self.bump(); // : に移動
                        match self.parse_param_type() {
                            Some(t) => t,
                            None => return None
                        }
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
                            match self.parse_expression(Precedence::Lowest, false) {
                                Some(e) => e,
                                None => return None,
                            }
                        };
                        ParamKind::Default(e)
                    } else {
                        ParamKind::Identifier
                    };
                    (k, t)
                };
                Some(FuncParam::new_with_type(Some(name), kind, param_type))
            },
            Token::Ref => {
                match self.next_token.token() {
                    Token::Identifier(name) => {
                        self.bump();
                        let kind= if self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if self.is_next_token_expected(Token::Rbracket) {
                                while self.is_next_token(&Token::Lbracket) {
                                    self.bump();
                                    if ! self.is_next_token_expected(Token::Rbracket) {
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
                            match self.parse_param_type() {
                                Some(t) => t,
                                None => return None
                            }
                        } else {
                            ParamType::Any
                        };
                        Some(FuncParam::new_with_type(Some(name), kind, param_type))
                    }
                    _ => {
                        self.error_got_unexpected_next_token();
                        None
                    }
                }
            },
            Token::Variadic => {
                if let Token::Identifier(name) = self.next_token.token() {
                    self.bump();
                    Some(FuncParam::new(Some(name), ParamKind::Variadic))
                } else {
                    self.error_got_unexpected_next_token();
                    None
                }
            },
            _ => {
                self.error_got_unexpected_token();
                None
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
                self.error_got_unexpected_token();
                None
            }
        }
    }

    fn parse_await_func_call_expression(&mut self) -> Option<Expression> {
        self.bump();
        let pos = self.current_token.pos;
        match self.parse_expression(Precedence::Lowest, false) {
            Some(Expression::FuncCall{func,args,is_await:_}) => {
                Some(Expression::FuncCall{func,args,is_await:true})
            },
            _ => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::FunctionCallRequiredAfterAwait,
                    pos,
                    self.script_name()
                ));
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
                if let Some(e) = self.parse_expression(Precedence::Lowest, false) {
                    list.push(e);
                } else {
                    return None;
                }
                self.skip_next_eol();
                if self.is_next_token(&Token::Comma) {
                    self.bump();
                } else {
                    if ! self.is_next_token_expected(end) {
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
    use crate::lexer::Lexer;
    use crate::parser::{Parser, PareseErrors};

    impl StatementWithRow {
        fn new_expected(statement: Statement, row: usize) -> Self{
            Self { statement, row, line: "dummy".into(), script_name: None }
        }
    }

    fn print_errors(errors: PareseErrors, out: bool, input: &str, msg: &str) {
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
        let parser = Parser::new(Lexer::new(input));
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
        let parser = Parser::new(Lexer::new(input));
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
                        ]
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
    statement1
    statement2
    statement3
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
                            3
                        ),
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement2")))),
                            4
                        ),
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement3")))),
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
                "if a then b",
                vec![
                    StatementWithRow::new_expected(
                        Statement::IfSingleLine {
                            condition: Expression::Identifier(Identifier(String::from("a"))),
                            consequence: Box::new(
                                StatementWithRow::new_expected(
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("b")))),
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
                "if a then b else c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::IfSingleLine {
                            condition: Expression::Identifier(Identifier(String::from("a"))),
                            consequence: Box::new(
                                StatementWithRow::new_expected(
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("b")))),
                                    1
                                )
                            ),
                            alternative: Box::new(Some(
                                StatementWithRow::new_expected(
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("c")))),
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
    statement1
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If{
                    condition: Expression::Identifier(Identifier(String::from("b"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
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
    statement1
else
    statement2_1
    statement2_2
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::If {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
                            3
                        ),
                    ],
                    alternative: Some(vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement2_1")))),
                            5
                        ),
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement2_2")))),
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
    statement1
elseif b then
    statement2
elseif c then
    statement3
elseif d then
    statement4
else
    statement5
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::ElseIf {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
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
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement2")))),
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
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement3")))),
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
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement4")))),
                                    9
                                )
                            ],
                        ),
                        (
                            None,
                            vec![
                                StatementWithRow::new_expected(
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement5")))),
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
    statement1
elseif b then
    statement2
endif
"#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::ElseIf {
                    condition: Expression::Identifier(Identifier(String::from("a"))),
                    consequence: vec![
                        StatementWithRow::new_expected(
                            Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
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
                                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement2")))),
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
! hoge
-1
+1
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Prefix(
                    Prefix::Not,
                    Box::new(Expression::Identifier(Identifier(String::from("hoge"))))
                )),
                2
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Prefix(
                    Prefix::Minus,
                    Box::new(Expression::Literal(Literal::Num(1 as f64)))
                )),
                3
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Prefix(
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
3 + 3
3 - 3
3 * 3
3 / 3
3 > 3
3 < 3
3 = 3
3 == 3
3 != 3
3 <> 3
3 >= 3
3 <= 3
        "#;
        parser_test(input, vec![
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Plus,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                2
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Minus,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                3
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Multiply,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                4
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Divide,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                5
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::GreaterThan,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                6
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::LessThan,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                7
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Equal,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                8
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::Equal,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                9
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::NotEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                10
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::NotEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                11
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::GreaterThanEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                12
            ),
            StatementWithRow::new_expected(
                Statement::Expression(Expression::Infix(
                    Infix::LessThanEqual,
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                    Box::new(Expression::Literal(Literal::Num(3 as f64))),
                )),
                13
            ),
        ], vec![]);

    }

    #[test]
    fn test_precedence() {
        let tests = vec![
            (
                "-a * b",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "!-a",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Prefix(
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
                "a + b + c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a + b - c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a * b * c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a * b / c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a + b / c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a + b * c + d / e - f",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "5 > 4 == 3 < 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "5 < 4 != 3 > 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "5 >= 4 = 3 <= 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "3 > 5 == FALSE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "3 < 5 = TRUE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "1 + (2 + 3) + 4",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "(5 + 5) * 2",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "2 / (5 + 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(
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
                "-(5 + 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Prefix(
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
                "!(5 = 5)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Prefix(
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
                "a + add(b * c) + d",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a * [1, 2, 3, 4][b * c] * d",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a or b and c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "1 + 5 mod 3",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "3 * 2 and 2 xor (2 or 4)",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Infix(
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
                "a ? b : c",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Ternary{
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
                "hoge[a?b:c]",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Index(
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
                "x + y * a ? b + q : c / r",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Ternary{
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
                "a ? b: c ? d: e",
                vec![
                    StatementWithRow::new_expected(
                        Statement::Expression(Expression::Ternary{
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
                                None, false
                            )
                        ]), 1
                    )
                ]
            ),
            (
                "hashtbl hoge = HASH_CASECARE",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                Some(Expression::Identifier(Identifier("HASH_CASECARE".to_string()))),
                                false
                            )
                        ]), 1
                    )
                ]
            ),
            (
                "hashtbl hoge = HASH_SORT",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                Some(Expression::Identifier(Identifier("HASH_SORT".to_string()))),
                                false
                            )
                        ]), 1
                    )
                ]
            ),
            (
                "public hashtbl hoge",
                vec![
                    StatementWithRow::new_expected(
                        Statement::HashTbl(vec![
                            (
                                Identifier(String::from("hoge")),
                                None, true
                            )
                        ]), 1
                    )
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
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
                ]
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
                ]
            ),
            (
                r#"
                hash public hoge
                endhash
                "#,
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
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected, vec![]);
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
                ]), 2
            ),
            StatementWithRow::new_expected(
                Statement::Dim(vec![
                    (Identifier("d2".to_string()), Expression::Literal(Literal::Num(1.0)))
                ]), 7
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
                            ]), 3
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
                            ]), 15
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
                        ]),
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

}