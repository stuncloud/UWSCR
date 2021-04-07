use crate::ast::*;
use crate::lexer::{Lexer, Position, TokenWithPos};
use crate::token::Token;

use std::fmt;

use serde_json;

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    UnexpectedToken,
    BlockNotClosedCorrectly,
    ValueMustBeDefined,
    BadParameter,
    OutOfWith,
    OutOfLoop,
    InvalidStatement,
    ClassHasNoConstructor,
    InvalidJson,
    InvalidFilePath,
    InvalidDllType,
    DllPathNonFound,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    kind: ParseErrorKind,
    msg: String,
    pos: Position,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErrorKind::UnexpectedToken => write!(f, "Unexpected Token"),
            ParseErrorKind::BlockNotClosedCorrectly => write!(f, "Block is not closing correctly"),
            ParseErrorKind::ValueMustBeDefined => write!(f, "constant must define value"),
            ParseErrorKind::BadParameter => write!(f, "bad parameter"),
            ParseErrorKind::OutOfWith => write!(f, "Not in WITH block"),
            ParseErrorKind::OutOfLoop => write!(f, "Not in Loop block"),
            ParseErrorKind::InvalidStatement => write!(f, "Invalid Statement"),
            ParseErrorKind::ClassHasNoConstructor => write!(f, "Constructor required"),
            ParseErrorKind::InvalidJson => write!(f, "Invalid json format"),
            ParseErrorKind::InvalidFilePath => write!(f, "Invalid file path"),
            ParseErrorKind::InvalidDllType => write!(f, "Invalid dll type"),
            ParseErrorKind::DllPathNonFound => write!(f, "Dll path not found"),
        }
    }
}

impl ParseError {
    pub fn new<S: Into<String>>(kind: ParseErrorKind, msg: S, pos: Position) -> Self {
        ParseError {kind, msg: msg.into(), pos}
    }

    pub fn get_kind(self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} [{}]: {}", self.kind, self.pos, self.msg)
    }
}

pub type PareseErrors = Vec<ParseError>;

pub struct Parser {
    lexer: Lexer,
    current_token: TokenWithPos,
    next_token: TokenWithPos,
    errors: PareseErrors,
    with: Option<Expression>,
    with_count: usize,
    in_loop: bool,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        let mut parser = Parser {
            lexer,
            current_token: TokenWithPos::new(Token::Eof),
            next_token: TokenWithPos::new(Token::Eof),
            errors: vec![],
            with: None,
            with_count: 0,
            in_loop: false,
        };
        parser.bump();
        parser.bump();

        parser
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
            Token::And => Precedence::And,
            Token::Or | Token::Xor => Precedence::Or,
            Token::Question => Precedence::Ternary,
            Token::Assign => Precedence::Assign,
            _ => Precedence::Lowest,
        }
    }

    pub fn get_errors(&mut self) -> PareseErrors {
        self.errors.clone()
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
        let eobtokens = vec![
            Token::Else,
            Token::ElseIf,
            Token::EndIf,
            Token::Case,
            Token::Default,
            Token::Selend,
            Token::Wend,
            Token::Until,
            Token::Next,
            Token::EndWith,
            Token::Fend,
            Token::EndModule,
            Token::EndClass,
            Token::Rbrace,
            Token::Except,
            Token::Finally,
            Token::EndTry,
        ];
        self.is_current_token_in(eobtokens)
    }

    fn is_next_token(&mut self, token: &Token) -> bool {
        self.next_token.token == *token
    }

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

    fn error_got_invalid_next_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "expected token was {:?}, but got {:?} instead.",
                token, self.next_token.token,
            ),
            self.next_token.pos.clone()
        ))
    }

    fn error_got_invalid_close_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::BlockNotClosedCorrectly,
            format!(
                "this block requires {:?} to close but got {:?}",
                token, self.current_token.token
            ),
            self.current_token.pos.clone()
        ))
    }

    fn error_got_invalid_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "expected token was {:?}, but got {:?} instead.",
                token, self.current_token.token
            ),
            self.current_token.pos.clone()
        ))
    }

    fn error_token_is_not_identifier(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "expected token was Identifier, but got {:?} instead.",
                self.current_token.token
            ),
            self.current_token.pos.clone()
        ))
    }

    fn error_got_unexpected_token(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "unexpected token: {:?}.",
                self.current_token.token
            ),
            self.current_token.pos.clone()
        ))
    }

    fn error_got_unexpected_next_token(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "unexpected token: {:?}.",
                self.next_token.token
            ),
            self.next_token.pos.clone()
        ))
    }

    fn error_got_bad_parameter(&mut self, msg: String) {
        self.errors.push(ParseError::new(
            ParseErrorKind::BadParameter,
            msg,
            self.current_token.pos.clone()
        ))
    }

    fn error_got_invalid_dlltype(&mut self, name: String) {
        self.errors.push(ParseError::new(
            ParseErrorKind::InvalidDllType,
            format!("{} is not valid dll type", name),
            self.current_token.pos.clone()
        ))
    }

    fn error_got_invalid_dllpath(&mut self, pos: Position) {
        self.errors.push(ParseError::new(
            ParseErrorKind::DllPathNonFound,
            "path to dll is required",
            pos
        ))
    }

    fn current_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.current_token.token)
    }

    fn next_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.next_token.token)
    }

    fn error_no_prefix_parser(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "no prefix parser found for \"{:?}\"",
                self.current_token.token
            ),
            self.current_token.pos.clone()
        ))
    }

    pub fn parse(&mut self) -> Program {
        let mut program: Program = vec![];
        let mut pub_counter = 0;
        let mut func_counter = 0;

        while ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(s) => match s {
                    Statement::Public(_) |
                    Statement::Const(_) |
                    Statement::TextBlock(_, _) => {
                        program.insert(pub_counter, s);
                        pub_counter += 1;
                    },
                    Statement::Function{name, params, body, is_proc} => {
                        let mut new_body = Vec::new();
                        for statement in body {
                            match statement {
                                Statement::Public(_) |
                                Statement::Const(_) |
                                Statement::TextBlock(_, _) => {
                                    program.insert(pub_counter, statement);
                                    pub_counter += 1;
                                },
                                _ => new_body.push(statement)
                            }
                        }
                        program.insert(pub_counter + func_counter, Statement::Function {
                            name, params, body: new_body, is_proc
                        });
                        func_counter += 1;
                    },
                    Statement::Module(_, _) |
                    Statement::Class(_, _) => {
                        program.insert(pub_counter + func_counter, s);
                        func_counter += 1;
                    },
                    _ => program.push(s)
                },
                None => {}
            }
            self.bump();
        }

        program
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

    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token.token {
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
            Token::Call(_) => self.parse_special_statement(),
            Token::DefDll => self.parse_def_dll_statemennt(),
            Token::HashTable => self.parse_hashtable_statement(false),
            Token::Function => self.parse_function_statement(false),
            Token::Procedure => self.parse_function_statement(true),
            Token::Exit => Some(Statement::Exit),
            Token::ExitExit => self.parse_exitexit_statement(),
            Token::Module => self.parse_module_statement(),
            Token::Class => self.parse_class_statement(),
            Token::TextBlock(ref name, ref body, is_ex) => {
                name.clone().map(|s| Statement::TextBlock(Identifier(s), Literal::TextBlock(body.clone(), is_ex)))
            },
            Token::NoEndTextBlock => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::BlockNotClosedCorrectly,
                    format!("endtextblock required"),
                    self.current_token.pos.clone()
                ));
                None
            },
            Token::With => self.parse_with_statement(),
            Token::Try => self.parse_try_statement(),
            _ => self.parse_expression_statement(),
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
                self.bump();
                let index = if self.is_next_token(&Token::Rbracket) {
                    // 添字省略
                    Expression::Literal(Literal::Empty)
                } else {
                    self.bump();
                    match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => e,
                        None => return None
                    }
                };
                if ! self.is_next_token_expected(Token::Rbracket) {
                    return None;
                };
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    // 代入演算子がなければ配列宣言のみ
                    if value_required {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::ValueMustBeDefined,
                            format!("{} has no value.", var_name),
                            self.next_token.pos.clone()
                        ));
                        return None;
                    } else {
                        Expression::Array(Vec::new(), Box::new(index))
                    }
                } else {
                    self.bump();
                    let list = match self.parse_expression_list(Token::Eol) {
                        Some(vec_e) => vec_e,
                        None => return None
                    };
                    Expression::Array(list, Box::new(index))
                }
            } else {
                // 変数定義
                // 代入演算子がなければ変数宣言のみ
                if ! self.is_next_token(&Token::EqualOrAssign) {
                    if value_required {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::ValueMustBeDefined,
                            format!("{} has no value.", var_name),
                            self.next_token.pos.clone()
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
            Token::Identifier(_) => self.bump(),
            Token::HashTable => {
                self.bump();
                return self.parse_hashtable_statement(true);
            },
            _ => return None,
        }
        match self.parse_variable_definition(false) {
            Some(v) => Some(Statement::Public(v)),
            None => None
        }
    }

    fn parse_dim_statement(&mut self) -> Option<Statement> {
        match &self.next_token.token {
            Token::Identifier(_) => self.bump(),
            _ => return None,
        }
        match self.parse_variable_definition(false) {
            Some(v) => Some(Statement::Dim(v)),
            None => None
        }
    }

    fn parse_const_statement(&mut self) -> Option<Statement> {
        match &self.next_token.token {
            Token::Identifier(_) => self.bump(),
            _ => return None,
        }
        match self.parse_variable_definition(true) {
            Some(v) => Some(Statement::Const(v)),
            None => None
        }
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

        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => Expression::Literal(Literal::String("".to_string()))
        };

        Some(Statement::Print(expression))
    }

    fn parse_special_statement(&mut self) -> Option<Statement> {
        match self.current_token.token {
            Token::Call(ref mut s) => {
                Some(Statement::Call(s.clone()))
            },
            _ => None
        }
    }

    fn parse_def_dll_statemennt(&mut self) -> Option<Statement> {
        self.bump();
        let name = match self.current_token.token {
            Token::Identifier(ref s) => s.clone(),
            _ => {
                self.error_token_is_not_identifier();
                return None;
            }
        };
        if ! self.is_next_token_expected(Token::Lparen) {
            return None;
        }
        self.bump();
        let mut params = Vec::new();
        while ! self.is_current_token_in(vec![Token::Rparen, Token::Eol, Token::Eof]) {
            match self.current_token.token {
                Token::Identifier(_) => {
                    let def_dll_param = self.parse_dll_param(false);
                    if def_dll_param.is_none() {
                        return None;
                    }
                    params.push(def_dll_param.unwrap());
                },
                Token::Ref => {
                    self.bump();
                    if let Token::Identifier(_) = self.current_token.token.clone() {
                        let def_dll_param = self.parse_dll_param(true);
                        if def_dll_param.is_none() {
                            return None;
                        }
                        params.push(def_dll_param.unwrap());
                    }
                },
                // 構造体
                Token::Lbrace => match self.parse_dll_struct() {
                    Some(p) => params.push(p),
                    None => return None,
                },
                Token::Comma => {},
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
                let t: DllType = s.parse().unwrap();
                if let DllType::Unknown(_) = t.clone() {
                    // :パス
                    match self.parse_dll_path() {
                        Some(p) => (DllType::Void, p),
                        None => return None,
                    }
                } else {
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
                }
            },
            _ => {
                self.error_got_unexpected_token();
                return None;
            },
        };

        Some(Statement::DefDll {
            name,
            params,
            ret_type,
            path
        })
    }

    fn parse_dll_struct(&mut self) -> Option<DefDllParam> {
        self.bump();
        let mut s = Vec::new();
        while ! self.is_current_token_in(vec![Token::Rbrace, Token::Eol, Token::Eof]) {
            match self.current_token.token {
                Token::Identifier(_) => {
                    let def_dll_param = self.parse_dll_param(false);
                    if def_dll_param.is_none() {
                        return None;
                    }
                    s.push(def_dll_param.unwrap());
                },
                Token::Ref => {
                    self.bump();
                    if let Token::Identifier(_) = self.current_token.token.clone() {
                        let def_dll_param = self.parse_dll_param(true);
                        if def_dll_param.is_none() {
                            return None;
                        }
                        s.push(def_dll_param.unwrap());
                    }
                },
                Token::Lbrace => match self.parse_dll_struct() {
                    Some(p) => s.push(p),
                    None => {
                        self.error_got_unexpected_token();
                        return None;
                    },
                }
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
        let pos = self.current_token.pos.clone();
        let mut path = String::new();
        while ! self.is_current_token(&Token::Eol) {
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
        let t = if let Token::Identifier(s) = self.current_token.token.clone() {
            s.parse::<DllType>().unwrap()
        } else {
            return None;
        };
        if let DllType::Unknown(unknown) = t {
            self.error_got_invalid_dlltype(unknown);
            return None;
        }
        if self.is_next_token(&Token::Lbracket) {
            self.bump();
            self.bump();
            match self.current_token.token {
                Token::Rbracket => if is_ref {
                    Some(DefDllParam::VarArray(t, None))
                } else {
                    Some(DefDllParam::Array(t, None))
                },
                Token::Num(n) => {
                    if ! self.is_next_token_expected(Token::Rbracket) {
                        return None;
                    }
                    self.bump();
                    if is_ref {
                        Some(DefDllParam::VarArray(t, Some(n as usize)))
                    } else {
                        Some(DefDllParam::Array(t, Some(n as usize)))
                    }
                },
                _ => {
                    self.error_got_unexpected_token();
                    return None;
                },
            }
        } else if is_ref {
            Some(DefDllParam::Var(t))
        } else {
            Some(DefDllParam::Param(t))
        }
    }

    fn parse_continue_statement(&mut self) -> Option<Statement> {
        if ! self.in_loop {
            self.errors.push(ParseError::new(
                ParseErrorKind::OutOfLoop,
                "continue is not allowd.",
                self.current_token.pos.clone()
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
                ParseErrorKind::OutOfLoop,
                "break is not allowd.",
                self.current_token.pos.clone()
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
        match self.next_token.token {
            Token::EqualOrAssign => {
                // for
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

                if ! self.is_current_token(&Token::Next) {
                    self.error_got_invalid_close_token(Token::Next);
                    return None;
                }
                Some(Statement::For{
                    loopvar, from, to, step, block
                })
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

                if ! self.is_current_token(&Token::Next) {
                    self.error_got_invalid_close_token(Token::Next);
                    return None;
                }
                Some(Statement::ForIn{loopvar, collection, block})
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
        if ! self.is_current_token(&Token::Wend) {
            self.error_got_invalid_close_token(Token::Wend);
            return None;
        }
        Some(Statement::While(expression, block))
    }

    fn parse_repeat_statement(&mut self) -> Option<Statement> {
        self.bump();
        let block = self.parse_loop_block_statement();
        if ! self.is_current_token(&Token::Until) {
            self.error_got_invalid_close_token(Token::Until);
            return None;
        }
        self.bump();
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };

        Some(Statement::Repeat(expression, block))
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
                Expression::FuncCall{func:_, args:_} => {
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
            block.insert(0, with_temp_assignment.unwrap());
        }
        if ! self.is_current_token(&Token::EndWith) {
            self.error_got_invalid_close_token(Token::EndWith);
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
            Token::Except => {
                self.bump();
                except = Some(self.parse_block_statement());
            },
            Token::Finally => {},
            t => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::UnexpectedToken,
                    format!("should have except or finally, but got {:?}", t),
                    self.current_token.pos
                ));
                return None;
            },
        }
        match self.current_token.token.clone() {
            Token::Finally => {
                self.bump();
                finally = match self.parse_finally_block_statement() {
                    Ok(b) => Some(b),
                    Err(s) => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidStatement,
                            format!("you can not use {} in finally", s),
                            self.current_token.pos
                        ));
                        return None;
                    }
                };
            },
            Token::EndTry => {},
            t => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::BlockNotClosedCorrectly,
                    format!("should have finally or endtry, but got {:?}", t),
                    self.current_token.pos
                ));
                return None;
            },
        }
        if ! self.is_current_token(&Token::EndTry) {
            self.error_got_invalid_close_token(Token::EndTry);
            return None;
        }

        Some(Statement::Try {trys, except, finally})
    }

    fn parse_finally_block_statement(&mut self) -> Result<BlockStatement, String> {
        self.bump();
        let mut block: BlockStatement  = vec![];

        while ! self.is_current_token_end_of_block() && ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(Statement::Exit) => return Err("exit".into()),
                Some(Statement::Continue(_)) => return Err("continue".into()),
                Some(Statement::Break(_)) => return Err("break".into()),
                Some(s) => block.push(s),
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
                ParseErrorKind::UnexpectedToken,
                format!("Exit code should be number"),
                self.current_token.pos
            ));
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
            Token::ExpandableString(_) |
            Token::String(_) => self.parse_string_expression(),
            Token::Bool(_) => self.parse_bool_expression(),
            Token::Lbracket => self.parse_array_expression(),
            Token::Bang | Token::Minus | Token::Plus => self.parse_prefix_expression(),
            Token::Lparen => self.parse_grouped_expression(),
            Token::Function => self.parse_function_expression(false),
            Token::Procedure => self.parse_function_expression(true),
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
                match serde_json::from_str::<serde_json::Value>(s.as_str()) {
                    Ok(v) => Some(Expression::UObject(v)),
                    Err(e) => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidJson,
                            format!("{}", e),
                            self.current_token.pos.clone()
                        ));
                        return None;
                    }
                }
            },
            Token::UObjectNotClosing => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::BlockNotClosedCorrectly,
                    format!("}} required"),
                    self.current_token.pos.clone()
                ));
                return None
            },
            _ => {
                self.error_no_prefix_parser();
                return None;
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
                Token::Plus
                | Token::Minus
                | Token::Slash
                | Token::Asterisk
                | Token::Equal
                | Token::EqualOrAssign
                | Token::NotEqual
                | Token::LessThan
                | Token::LessThanEqual
                | Token::GreaterThan
                | Token::GreaterThanEqual
                | Token::And
                | Token::Or
                | Token::Xor
                | Token::Mod
                | Token::To
                | Token::Step
                | Token::In => {
                    self.bump();
                    left = self.parse_infix_expression(left.unwrap());
                },
                Token::Assign => left = self.parse_assign_expression(left.unwrap()),
                Token::Lbracket => {
                    self.bump();
                    left = {
                        let index = self.parse_index_expression(left.unwrap());
                        if is_sol {
                            if let Some(e) = self.parse_assignment(self.next_token.token.clone(), index.clone().unwrap()) {
                                return Some(e);
                            }
                        }
                        index
                    }
                },
                Token::Lparen => {
                    self.bump();
                    left = self.parse_function_call_expression(left.unwrap());
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
        match self.current_token.token {
            Token::Identifier(ref mut i) => Some(Identifier(i.clone())),
            _ => None,
        }
    }

    fn parse_identifier_expression(&mut self) -> Option<Expression> {
        match self.parse_identifier() {
            Some(i) => Some(Expression::Identifier(i)),
            None => None
        }
    }

    fn parse_with_dot_expression(&mut self) -> Option<Expression> {
        match self.get_current_with() {
            Some(e) => self.parse_dotcall_expression(e),
            None => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::OutOfWith,
                    format!(""),
                    self.current_token.pos.clone()
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

    fn parse_expression_list(&mut self, end: Token) -> Option<Vec<Expression>> {
        let mut list:Vec<Expression> = vec![];

        if self.is_next_token(&end) {
            self.bump();
            return Some(list);
        }

        self.bump();

        match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => list.push(e),
            None => return None
        }

        while self.is_next_token(&Token::Comma) {
            self.bump();
            self.bump();

            match self.parse_expression(Precedence::Lowest, false) {
                Some(e) => list.push(e),
                None => return None
            }
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
            let alternative = if self.is_next_token(&Token::Else) {
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

        if self.is_current_token(&Token::EndIf) {
            return Some(Statement::If {
                condition,
                consequence,
                alternative: None
            });
        }

        if self.is_current_token(&Token::Else) {
            let alternative:Option<BlockStatement> = Some(self.parse_block_statement());
            if ! self.is_current_token(&Token::EndIf) {
                self.error_got_invalid_close_token(Token::EndIf);
                return None;
            }
            return Some(Statement::If {
                condition,
                consequence,
                alternative
            });
        }

        let mut alternatives: Vec<(Option<Expression>, BlockStatement)> = vec![];
        while self.is_current_token_in(vec![Token::Else, Token::ElseIf]) {
            if self.is_current_token(&Token::Else) {
                alternatives.push(
                    (None, self.parse_block_statement())
                );
                // break;
            } else {
                if self.is_current_token(&Token::ElseIf) {
                    self.bump();
                    let elseifcond = match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => e,
                        None => return None
                    };
                    alternatives.push(
                        (Some(elseifcond), self.parse_block_statement())
                    );
                }
            }
        }
        if ! self.is_current_token(&Token::EndIf) {
            self.error_got_invalid_close_token(Token::EndIf);
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
        while self.is_current_token_in(vec![Token::Case, Token::Default]) {
            match self.current_token.token {
                Token::Case => {
                    let case_values = match self.parse_expression_list(Token::Eol) {
                        Some(list) => list,
                        None => return None
                    };
                    cases.push((
                        case_values,
                        self.parse_block_statement()
                    ));
                },
                Token::Default => {
                    self.bump();
                    default = Some(self.parse_block_statement());
                },
                _ => return None
            }
        }
        if ! self.is_current_token(&Token::Selend) {
            self.error_got_invalid_close_token(Token::Selend);
            return None;
        }
        Some(Statement::Select {expression, cases, default})
    }

    fn parse_function_statement(&mut self, is_proc: bool) -> Option<Statement> {
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
            match self.parse_function_parameters() {
                Some(p) => p,
                None => return None
            }
        } else {
            vec![]
        };

        self.bump();
        let body = self.parse_block_statement();

        if ! self.is_current_token(&Token::Fend) {
            self.error_got_invalid_close_token(Token::Fend);
            return None;
        }
        Some(Statement::Function{name, params, body, is_proc})
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
        while ! self.is_current_token(&Token::EndModule) {
            if self.is_current_token(&Token::Eof) {
                self.errors.push(ParseError::new(
                    ParseErrorKind::BlockNotClosedCorrectly,
                    format!(
                        "module should be closed by endmodule.",
                    ),
                    self.current_token.pos.clone()
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
        let class_statement_pos = self.current_token.pos.clone();
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
        while ! self.is_current_token(&Token::EndClass) {
            if self.is_current_token(&Token::Eof) {
                self.errors.push(ParseError::new(
                    ParseErrorKind::BlockNotClosedCorrectly,
                    format!(
                        "class should be closed by endclass.",
                    ),
                    self.current_token.pos.clone()
                ));
                return None;
            }
            let cur_pos = self.current_token.pos.clone();
            match self.parse_statement() {
                Some(s) => match s {
                    Statement::Dim(_) |
                    Statement::Public(_) |
                    Statement::Const(_) |
                    Statement::TextBlock(_, _) |
                    Statement::HashTbl(_) => block.push(s),
                    Statement::Function{ref name, params: _, body: _, is_proc: _} => {
                        if name == &identifier {
                            has_constructor = true;
                        }
                        block.push(s);
                    },
                    _ => {
                        self.errors.push(ParseError::new(
                            ParseErrorKind::InvalidStatement,
                            format!("you can not define {:?} in Class", s),
                            cur_pos
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
                ParseErrorKind::ClassHasNoConstructor,
                format!("procedure {}() must be defined", identifier),
                class_statement_pos
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

        let params = match self.parse_function_parameters() {
            Some(p) => p,
            None => return None
        };

        let body = self.parse_block_statement();

        if ! self.is_current_token(&Token::Fend) {
            self.error_got_invalid_close_token(Token::Fend);
            return None;
        }

        Some(Expression::AnonymusFunction {params, body, is_proc})
    }

    fn parse_function_parameters(&mut self) -> Option<Vec<Expression>> {
        let mut params = vec![];
        if self.is_next_token(&Token::Rparen) {
            self.bump();
            return Some(params);
        }
        let mut with_default_flg = false;
        let mut variadic_flg = false;
        self.bump();
        loop {
            match self.parse_param() {
                Some(param) => {
                    match param.clone() {
                        Params::Identifier(i) |
                        Params::Reference(i) |
                        Params::Array(i, _) => if with_default_flg {
                            self.error_got_bad_parameter(format!("{}: only argument with default is allowed after argument with default", i));
                            return None;
                        } else if variadic_flg {
                            self.error_got_bad_parameter(format!("{}: no arguments are allowed after variadic argument", i));
                            return None;
                        },
                        Params::WithDefault(i, _) => if variadic_flg {
                            self.error_got_bad_parameter(format!("{}: no arguments are allowed after variadic argument", i));
                            return None;
                        } else {
                            with_default_flg = true;
                        },
                        Params::Variadic(i) => if with_default_flg {
                            self.error_got_bad_parameter(format!("&{}: variadic argument is not allowed after argument with default value", i));
                            return None;
                        } else if variadic_flg {
                            self.error_got_bad_parameter(format!("&{}: no arguments are allowed after variadic argument", i));
                            return None;
                        } else {
                            variadic_flg = true;
                        },
                        Params::VariadicDummy => continue
                    }
                    params.push(Expression::Params(param))
                },
                None => return None
            }
            if self.is_next_token(&Token::Comma) {
                self.bump();
                self.bump();
            } else {
                break;
            }
        }
        if ! self.is_next_token_expected(Token::Rparen) {
            // self.error_got_invalid_close_token(Token::Rparen);
            return None;
        }

        Some(params)
    }

    fn parse_param(&mut self) -> Option<Params> {
        match &self.current_token.token {
            Token::Identifier(_) => {
                let i = self.parse_identifier().unwrap();
                if self.is_next_token(&Token::Lbracket) {
                    self.bump();
                    if self.is_next_token_expected(Token::Rbracket) {
                        while self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if !self.is_next_token_expected(Token::Rbracket) {
                                return None;
                            }
                        }
                        return Some(Params::Array(i, false));
                    } else {
                        return None;
                    }
                } else if self.is_next_token(&Token::EqualOrAssign) {
                    self.bump();
                    if self.is_next_token(&Token::Comma) || self.is_next_token(&Token::Rparen) {
                        // 代入する値を省略した場合はEmptyが入る
                        return Some(Params::WithDefault(i, Box::new(Expression::Literal(Literal::Empty))));
                    }
                    self.bump();
                    match self.parse_expression(Precedence::Lowest, false) {
                        Some(e) => return Some(Params::WithDefault(i, Box::new(e))),
                        None => {}
                    };
                } else {
                    return Some(Params::Identifier(i));
                }
            },
            Token::Ref => {
                match self.next_token.token {
                    Token::Identifier(_) => {
                        self.bump();
                        let i = self.parse_identifier().unwrap();
                        if self.is_next_token(&Token::Lbracket) {
                            self.bump();
                            if self.is_next_token_expected(Token::Rbracket) {
                                while self.is_next_token(&Token::Lbracket) {
                                    self.bump();
                                    if ! self.is_next_token_expected(Token::Rbracket) {
                                        return None;
                                    }
                                }
                                return Some(Params::Array(i, true));
                            } else {
                                return None;
                            }
                        } else {
                            return Some(Params::Reference(i));
                        }
                    }
                    _ =>{}
                }
            },
            Token::Variadic => {
                if let Token::Identifier(s) = self.next_token.token.clone() {
                    self.bump();
                    return Some(Params::Variadic(Identifier(s.clone())))
                }
            },
            _ => {}
        }
        self.error_got_bad_parameter(format!("unexpected token: {:?}", self.current_token));
        None
    }

    fn parse_function_call_expression(&mut self, func: Expression) -> Option<Expression> {
        let args = match self.parse_expression_list(Token::Rparen) {
            Some(a) => a,
            None => return None
        };

        Some(Expression::FuncCall {
            func: Box::new(func),
            args
        })
    }

}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check_parse_errors(parser: &mut Parser, out: bool, msg: String) {
        let errors = parser.get_errors();
        if errors.len() == 0 {
            return;
        }

        if out {
            println!("parser has {} errors", errors.len());
            for error in errors {
                println!("{:?}", error);
            }
        }

        panic!(msg);
    }

    fn parser_test(input: &str, expected: Vec<Statement>) {
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse();
        check_parse_errors(&mut parser, true, String::from("test failed"));
        assert_eq!(program, expected);
    }

    fn parser_panic_test(input: &str, expected: Vec<Statement>, msg: String) {
        let mut parser = Parser::new(Lexer::new(input));
        let program = parser.parse();
        check_parse_errors(&mut parser, false, msg);
        assert_eq!(program, expected);
    }

    #[test]
    fn test_blank_row() {
        let input = r#"
print 1


print 2
        "#;
        parser_test(input, vec![
            Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
            Statement::Print(Expression::Literal(Literal::Num(2 as f64))),
        ])
    }

    #[test]
    fn test_dim_statement() {
        let testcases = vec![
            (
                "dim hoge = 1", vec![
                    Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("hoge")),
                                Expression::Literal(Literal::Num(1 as f64))
                            ),
                        ]
                    ),
                ]
            ),
            (
                "dim fuga", vec![
                    Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("fuga")),
                                Expression::Literal(Literal::Empty)
                            )
                        ]
                    ),
                ]
            ),
            (
                "dim piyo = EMPTY", vec![
                    Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("piyo")),
                                Expression::Literal(Literal::Empty)
                            )
                        ]
                    ),
                ]
            ),
            (
                "dim arr1[] = 1, 3, 5, 7, 9", vec![
                    Statement::Dim(
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
                                    Box::new(Expression::Literal(Literal::Empty)),
                                )
                            )
                        ]
                    ),
                ]
            ),
            (
                "dim arr2[4]", vec![
                    Statement::Dim(
                        vec![
                            (
                                Identifier(String::from("arr2")),
                                Expression::Array(
                                    vec![],
                                    Box::new(Expression::Literal(Literal::Num(4 as f64))),
                                )
                            )
                        ]
                    ),
                ]
            ),
            (
                "dim a = 1, b, c[1], d[] = 1,2", vec![
                    Statement::Dim(
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
                                    Box::new(Expression::Literal(Literal::Num(1.0))),
                                )
                            ),
                            (
                                Identifier(String::from("d")),
                                Expression::Array(
                                    vec![
                                        Expression::Literal(Literal::Num(1.0)),
                                        Expression::Literal(Literal::Num(2.0)),
                                    ],
                                    Box::new(Expression::Literal(Literal::Empty)),
                                )
                            )
                        ]
                    ),
                ]
            ),
        ];
        for (input, expected) in testcases {
            println!("{}", &input);
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_special() {
        let input = r#"
call C:\hoge\fuga\test.uws
call C:\hoge\fuga\test.uws(1, 2)
        "#;
        parser_test(input, vec![
            Statement::Call(
                String::from(r"C:\hoge\fuga\test.uws")
            ),
            Statement::Call(
                String::from(r"C:\hoge\fuga\test.uws(1, 2)")
            ),
        ]);
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
            Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
            Statement::Print(Expression::Literal(Literal::Num(1.23))),
            Statement::Print(Expression::Literal(
                Literal::Num(i64::from_str_radix("12AB", 16).unwrap() as f64)
            )),
            Statement::Print(Expression::Literal(Literal::Bool(true))),
            Statement::Print(Expression::Literal(Literal::Bool(false))),
            Statement::Print(Expression::Literal(
                Literal::ExpandableString(String::from("展開可能文字列リテラル"))
            )),
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
            Statement::Print(Expression::Literal(
                Literal::Array(vec![])
            )),
        ]);
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
            Statement::If {
                condition: Expression::Identifier(Identifier(String::from("a"))),
                consequence: vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement2")))),
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement3")))),
                ],
                alternative: None
            },
        ]);
    }

    #[test]
    fn test_single_line_if() {
        let tests = vec![
            (
                "if a then b",
                vec![
                    Statement::IfSingleLine {
                        condition: Expression::Identifier(Identifier(String::from("a"))),
                        consequence: Box::new(Statement::Expression(Expression::Identifier(Identifier(String::from("b"))))),
                        alternative: Box::new(None)
                    }
                ]
            ),
            (
                "if a then b else c",
                vec![
                    Statement::IfSingleLine {
                        condition: Expression::Identifier(Identifier(String::from("a"))),
                        consequence: Box::new(Statement::Expression(Expression::Identifier(Identifier(String::from("b"))))),
                        alternative: Box::new(Some(Statement::Expression(Expression::Identifier(Identifier(String::from("c")))))),
                    }
                ]
            ),
            (
                "if a then print 1 else b = c",
                vec![
                    Statement::IfSingleLine {
                        condition: Expression::Identifier(Identifier(String::from("a"))),
                        consequence: Box::new(Statement::Print(Expression::Literal(Literal::Num(1 as f64)))),
                        alternative: Box::new(Some(Statement::Expression(Expression::Assign(
                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            Box::new(Expression::Identifier(Identifier(String::from("c")))),
                        )))),
                    }
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
            Statement::If{
                condition: Expression::Identifier(Identifier(String::from("b"))),
                consequence: vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement1")))),
                ],
                alternative: None
            }
        ]);
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
            Statement::If {
                condition: Expression::Identifier(Identifier(String::from("a"))),
                consequence: vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement1"))))
                ],
                alternative: Some(vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement2_1")))),
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement2_2")))),
                ])
            },
        ]);

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
            Statement::ElseIf {
                condition: Expression::Identifier(Identifier(String::from("a"))),
                consequence: vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement1"))))
                ],
                alternatives: vec![
                    (
                        Some(Expression::Identifier(Identifier(String::from("b")))),
                        vec![Statement::Expression(Expression::Identifier(Identifier(String::from("statement2"))))],
                    ),
                    (
                        Some(Expression::Identifier(Identifier(String::from("c")))),
                        vec![Statement::Expression(Expression::Identifier(Identifier(String::from("statement3"))))],
                    ),
                    (
                        Some(Expression::Identifier(Identifier(String::from("d")))),
                        vec![Statement::Expression(Expression::Identifier(Identifier(String::from("statement4"))))],
                    ),
                    (
                        None,
                        vec![Statement::Expression(Expression::Identifier(Identifier(String::from("statement5"))))],
                    ),
                ]
            },
        ]);
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
            Statement::ElseIf {
                condition: Expression::Identifier(Identifier(String::from("a"))),
                consequence: vec![
                    Statement::Expression(Expression::Identifier(Identifier(String::from("statement1"))))
                ],
                alternatives: vec![
                    (
                        Some(Expression::Identifier(Identifier(String::from("b")))),
                        vec![Statement::Expression(Expression::Identifier(Identifier(String::from("statement2"))))],
                    ),
                ]
            },
        ]);
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
                    Statement::Select {
                        expression: Expression::Literal(Literal::Num(1.0)),
                        cases: vec![
                            (
                                vec![
                                    Expression::Literal(Literal::Num(1.0)),
                                    Expression::Literal(Literal::Num(2.0))
                                ],
                                vec![
                                    Statement::Print(Expression::Identifier(Identifier("a".to_string())))
                                ]
                            ),
                            (
                                vec![
                                    Expression::Literal(Literal::Num(3.0))
                                ],
                                vec![
                                    Statement::Print(Expression::Identifier(Identifier("b".to_string())))
                                ]
                            ),
                        ],
                        default: Some(vec![
                            Statement::Print(Expression::Identifier(Identifier("c".to_string())))
                        ])
                    }
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
                    Statement::Select {
                        expression: Expression::Literal(Literal::Num(1.0)),
                        cases: vec![],
                        default: Some(vec![
                            Statement::Print(Expression::Identifier(Identifier("c".to_string())))
                        ])
                    }
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
                    Statement::Select {
                        expression: Expression::Literal(Literal::Num(1.0)),
                        cases: vec![
                            (
                                vec![
                                    Expression::Literal(Literal::Num(1.0)),
                                ],
                                vec![
                                    Statement::Print(Expression::Identifier(Identifier("a".to_string())))
                                ]
                            ),
                        ],
                        default: None
                    }
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
            Statement::Expression(Expression::Prefix(
                Prefix::Not,
                Box::new(Expression::Identifier(Identifier(String::from("hoge"))))
            )),
            Statement::Expression(Expression::Prefix(
                Prefix::Minus,
                Box::new(Expression::Literal(Literal::Num(1 as f64)))
            )),
            Statement::Expression(Expression::Prefix(
                Prefix::Plus,
                Box::new(Expression::Literal(Literal::Num(1 as f64)))
            ))
        ]);
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
            Statement::Expression(Expression::Infix(
                Infix::Plus,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::Minus,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::Multiply,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::Divide,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::GreaterThan,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::LessThan,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::Equal,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::Equal,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::NotEqual,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::NotEqual,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::GreaterThanEqual,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
            Statement::Expression(Expression::Infix(
                Infix::LessThanEqual,
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
                Box::new(Expression::Literal(Literal::Num(3 as f64))),
            )),
        ]);

    }

    #[test]
    fn test_precedence() {
        let tests = vec![
            (
                "-a * b",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Multiply,
                        Box::new(Expression::Prefix(
                            Prefix::Minus,
                            Box::new(Expression::Identifier(Identifier(String::from("a"))))
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("b"))))
                    ))
                ]
            ),
            (
                "!-a",
                vec![
                    Statement::Expression(Expression::Prefix(
                        Prefix::Not,
                        Box::new(Expression::Prefix(
                            Prefix::Minus,
                            Box::new(Expression::Identifier(Identifier(String::from("a"))))
                        ))
                    ))
                ]
            ),
            (
                "a + b + c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Plus,
                        Box::new(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b"))))
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("c"))))
                    ))
                ]
            ),
            (
                "a + b - c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Minus,
                        Box::new(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b"))))
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("c"))))
                    ))
                ]
            ),
            (
                "a * b * c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Multiply,
                        Box::new(Expression::Infix(
                            Infix::Multiply,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b"))))
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("c"))))
                    ))
                ]
            ),
            (
                "a * b / c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Divide,
                        Box::new(Expression::Infix(
                            Infix::Multiply,
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Identifier(Identifier(String::from("b"))))
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("c"))))
                    ))
                ]
            ),
            (
                "a + b / c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Plus,
                        Box::new(Expression::Identifier(Identifier(String::from("a")))),
                        Box::new(Expression::Infix(
                            Infix::Divide,
                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            Box::new(Expression::Identifier(Identifier(String::from("c"))))
                        )),
                    ))
                ]
            ),
            (
                "a + b * c + d / e - f",
                vec![
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
                    ))
                ]
            ),
            (
                "5 > 4 == 3 < 4",
                vec![
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
                    )
                ]
            ),
            (
                "5 < 4 != 3 > 4",
                vec![
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
                    )
                ]
            ),
            (
                "5 >= 4 = 3 <= 4",
                vec![
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
                    )
                ]
            ),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                vec![
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
                    )
                ]
            ),
            (
                "3 > 5 == FALSE",
                vec![
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
                    )
                ]
            ),
            (
                "3 < 5 = TRUE",
                vec![
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
                    )
                ]
            ),
            (
                "1 + (2 + 3) + 4",
                vec![
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
                    )
                ]
            ),
            (
                "(5 + 5) * 2",
                vec![
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
                    )
                ]
            ),
            (
                "2 / (5 + 5)",
                vec![
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
                    )
                ]
            ),
            (
                "-(5 + 5)",
                vec![
                    Statement::Expression(Expression::Prefix(
                        Prefix::Minus,
                        Box::new(Expression::Infix(
                            Infix::Plus,
                            Box::new(Expression::Literal(Literal::Num(5 as f64))),
                            Box::new(Expression::Literal(Literal::Num(5 as f64))),
                        ))
                    ))
                ]
            ),
            (
                "!(5 = 5)",
                vec![
                    Statement::Expression(Expression::Prefix(
                        Prefix::Not,
                        Box::new(Expression::Infix(
                            Infix::Equal,
                            Box::new(Expression::Literal(Literal::Num(5 as f64))),
                            Box::new(Expression::Literal(Literal::Num(5 as f64))),
                        ))
                    ))
                ]
            ),
            (
                "a + add(b * c) + d",
                vec![
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
                                ]
                            })
                        )),
                        Box::new(Expression::Identifier(Identifier(String::from("d")))),
                    ))
                ]
            ),
            (
                "add(a, b, 1, 2 * 3, 4 + 5, add(6, 7 * 8))",
                vec![
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
                                ]
                            }
                        ]
                    })
                ]
            ),
            (
                "a * [1, 2, 3, 4][b * c] * d",
                vec![
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
                    ))
                ]
            ),
            (
                "add(a * b[2], b[1], 2 * [1, 2][1])",
                vec![
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
                        ]
                    })
                ]
            ),
            (
                "a or b and c",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Or,
                        Box::new(Expression::Identifier(Identifier(String::from("a")))),
                        Box::new(Expression::Infix(
                            Infix::And,
                            Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            Box::new(Expression::Identifier(Identifier(String::from("c")))),
                        ))
                    ))
                ]
            ),
            (
                "1 + 5 mod 3",
                vec![
                    Statement::Expression(Expression::Infix(
                        Infix::Plus,
                        Box::new(Expression::Literal(Literal::Num(1 as f64))),
                        Box::new(Expression::Infix(
                            Infix::Mod,
                            Box::new(Expression::Literal(Literal::Num(5 as f64))),
                            Box::new(Expression::Literal(Literal::Num(3 as f64))),
                        )),
                    ))
                ]
            ),
            (
                "3 * 2 and 2 xor (2 or 4)",
                vec![
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
                    ))
                ]
            ),
            (
                r#"
if a = b = c then
    print 1
endif
                "#,
                vec![
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
                            Statement::Print(Expression::Literal(Literal::Num(1 as f64))),
                        ],
                        alternative: None
                    }
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_assign() {
        let tests = vec![
            (
                "a = 1",
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Identifier(Identifier(String::from("a")))),
                        Box::new(Expression::Literal(Literal::Num(1 as f64)))
                    ))
                ]
            ),
            (
                "a[0] = 1",
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Index(
                            Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            Box::new(Expression::Literal(Literal::Num(0 as f64))),
                            Box::new(None)
                        )),
                        Box::new(Expression::Literal(Literal::Num(1 as f64)))
                    ))
                ]
            ),
            (
                "a = 1 = 2", // a に 1 = 2 を代入
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Identifier(Identifier(String::from("a")))),
                        Box::new(Expression::Infix(
                            Infix::Equal,
                            Box::new(Expression::Literal(Literal::Num(1 as f64))),
                            Box::new(Expression::Literal(Literal::Num(2 as f64))),
                    ))
                    ))
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
                    Statement::For {
                        loopvar: Identifier(String::from("i")),
                        from: Expression::Literal(Literal::Num(0 as f64)),
                        to: Expression::Literal(Literal::Num(5 as f64)),
                        step: None,
                        block: vec![
                            Statement::Print(Expression::Identifier(Identifier(String::from("i"))))
                        ]
                    }
                ]
            ),
            (
                r#"
for i = 5 to 0 step -1
    print i
next
                "#,
                vec![
                    Statement::For {
                        loopvar: Identifier(String::from("i")),
                        from: Expression::Literal(Literal::Num(5 as f64)),
                        to: Expression::Literal(Literal::Num(0 as f64)),
                        step: Some(Expression::Prefix(
                            Prefix::Minus,
                            Box::new(Expression::Literal(Literal::Num(1 as f64)))
                        )),
                        block: vec![
                            Statement::Print(Expression::Identifier(Identifier(String::from("i"))))
                        ]
                    }
                ]
            ),
            (
                r#"
for item in col
    print item
next
                "#,
                vec![
                    Statement::ForIn {
                        loopvar: Identifier(String::from("item")),
                        collection: Expression::Identifier(Identifier(String::from("col"))),
                        block: vec![
                            Statement::Print(Expression::Identifier(Identifier(String::from("item"))))
                        ]
                    }
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
            Statement::ForIn {
                loopvar: Identifier(String::from("item")),
                collection: Expression::Identifier(Identifier(String::from("col"))),
                block: vec![
                    Statement::Print(Expression::Identifier(Identifier(String::from("item"))))
                ]
            }
        ];
        parser_panic_test(input, expected, String::from("end of block should be NEXT"));
    }

    #[test]
    fn test_while() {
        let input  = r#"
while (a == b) and (c >= d)
    dosomething()
wend
        "#;
        parser_test(input, vec![
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
                    Statement::Expression(Expression::FuncCall {
                        func: Box::new(Expression::Identifier(Identifier(String::from("dosomething")))),
                        args: vec![]
                    })
                ]
            )
        ]);
    }

    #[test]
    fn test_repeat() {
        let input  = r#"
repeat
    dosomething()
until (a == b) and (c >= d)
        "#;
        parser_test(input, vec![
            Statement::Repeat(
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
                    Statement::Expression(Expression::FuncCall {
                        func: Box::new(Expression::Identifier(Identifier(String::from("dosomething")))),
                        args: vec![]
                    })
                ]
            )
        ]);
    }

    #[test]
    fn test_ternary_operator() {
        let tests = vec![
            (
                "a ? b : c",
                vec![
                    Statement::Expression(Expression::Ternary{
                        condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                        consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                        alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                    })
                ]
            ),
            (
                "x = a ? b : c",
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Identifier(Identifier(String::from("x")))),
                        Box::new(Expression::Ternary{
                            condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                        })
                    ))
                ]
            ),
            (
                "hoge[a?b:c]",
                vec![
                    Statement::Expression(Expression::Index(
                        Box::new(Expression::Identifier(Identifier(String::from("hoge")))),
                        Box::new(Expression::Ternary{
                            condition: Box::new(Expression::Identifier(Identifier(String::from("a")))),
                            consequence: Box::new(Expression::Identifier(Identifier(String::from("b")))),
                            alternative: Box::new(Expression::Identifier(Identifier(String::from("c")))),
                        }),
                        Box::new(None)
                    ))
                ]
            ),
            (
                "x + y * a ? b + q : c / r",
                vec![
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
                    })
                ]
            ),
            (
                "a ? b: c ? d: e",
                vec![
                    Statement::Expression(Expression::Ternary{
                        condition: Box::new(Expression::Identifier(Identifier("a".to_string()))),
                        consequence: Box::new(Expression::Identifier(Identifier("b".to_string()))),
                        alternative: Box::new(Expression::Ternary{
                            condition: Box::new(Expression::Identifier(Identifier("c".to_string()))),
                            consequence: Box::new(Expression::Identifier(Identifier("d".to_string()))),
                            alternative: Box::new(Expression::Identifier(Identifier("e".to_string())))
                        })
                    })
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_hashtbl() {
        let tests = vec![
            (
                "hashtbl hoge",
                vec![
                    Statement::HashTbl(vec![
                        (
                            Identifier(String::from("hoge")),
                            None, false
                        )
                    ])
                ]
            ),
            (
                "hashtbl hoge = HASH_CASECARE",
                vec![
                    Statement::HashTbl(vec![
                        (
                            Identifier(String::from("hoge")),
                            Some(Expression::Identifier(Identifier("HASH_CASECARE".to_string()))),
                            false
                        )
                    ])
                ]
            ),
            (
                "hashtbl hoge = HASH_SORT",
                vec![
                    Statement::HashTbl(vec![
                        (
                            Identifier(String::from("hoge")),
                            Some(Expression::Identifier(Identifier("HASH_SORT".to_string()))),
                            false
                        )
                    ])
                ]
            ),
            (
                "public hashtbl hoge",
                vec![
                    Statement::HashTbl(vec![
                        (
                            Identifier(String::from("hoge")),
                            None, true
                        )
                    ])
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
                vec![
                    Statement::Function {
                        name: Identifier("hoge".to_string()),
                        params: vec![
                            Expression::Params(Params::Identifier(Identifier("foo".to_string()))),
                            Expression::Params(Params::Identifier(Identifier("bar".to_string()))),
                            Expression::Params(Params::Identifier(Identifier("baz".to_string()))),
                        ],
                        body: vec![
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
                            )
                        ],
                        is_proc: false,
                    }
                ]
            ),
            (
                r#"
procedure hoge(foo, var bar, baz[], qux = 1)
fend
                "#,
                vec![
                    Statement::Function {
                        name: Identifier("hoge".to_string()),
                        params: vec![
                            Expression::Params(Params::Identifier(Identifier("foo".to_string()))),
                            Expression::Params(Params::Reference(Identifier("bar".to_string()))),
                            Expression::Params(Params::Array(Identifier("baz".to_string()), false)),
                            Expression::Params(Params::WithDefault(
                                Identifier("qux".to_string()),
                                Box::new(Expression::Literal(Literal::Num(1.0))),
                            )),
                        ],
                        body: vec![],
                        is_proc: true,
                    }
                ]
            ),
            (
                r#"
procedure hoge(ref foo, args bar)
fend
                "#,
                vec![
                    Statement::Function {
                        name: Identifier("hoge".to_string()),
                        params: vec![
                            Expression::Params(Params::Reference(Identifier("foo".to_string()))),
                            Expression::Params(Params::Variadic(Identifier("bar".to_string()))),
                        ],
                        body: vec![],
                        is_proc: true,
                    }
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
                    Statement::Function {
                        name: Identifier("hoge".to_string()),
                        params: vec![
                            Expression::Params(Params::Identifier(Identifier("a".to_string()))),
                        ],
                        body: vec![
                            Statement::Expression(
                                Expression::Assign(
                                    Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                    Box::new(Expression::Identifier(Identifier("a".to_string()))),
                                )
                            )
                        ],
                        is_proc: false,
                    },
                    Statement::Print(Expression::FuncCall{
                        func: Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                        args: vec![
                            Expression::Literal(Literal::Num(1.0)),
                        ],
                    })
                ]
            ),
            (
                r#"
hoge = function(a)
    result = a
fend
                "#,
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                        Box::new(Expression::AnonymusFunction{
                            params: vec![
                                Expression::Params(Params::Identifier(Identifier("a".to_string()))),
                            ],
                            body: vec![
                                Statement::Expression(Expression::Assign(
                                    Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                    Box::new(Expression::Identifier(Identifier("a".to_string())))
                                ))
                            ],
                            is_proc: false
                        }),
                    ))
                ]
            ),
            (
                r#"
hoge = procedure(a)
    print a
fend
                "#,
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::Identifier(Identifier("hoge".to_string()))),
                        Box::new(Expression::AnonymusFunction{
                            params: vec![
                                Expression::Params(Params::Identifier(Identifier("a".to_string()))),
                            ],
                            body: vec![
                                Statement::Print(
                                    Expression::Identifier(Identifier("a".to_string()))
                                )
                            ],
                            is_proc: true,
                        }),
                    ))
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_compound_assign() {
        let tests = vec![
            (
                r#"
a += 1
                "#,
                vec![
                    Statement::Expression(Expression::CompoundAssign(
                        Box::new(Expression::Identifier(Identifier("a".to_string()))),
                        Box::new(Expression::Literal(Literal::Num(1.0))),
                        Infix::Plus,
                    ))
                ]
            ),
            (
                r#"
a -= 1
                "#,
                vec![
                    Statement::Expression(Expression::CompoundAssign(
                        Box::new(Expression::Identifier(Identifier("a".to_string()))),
                        Box::new(Expression::Literal(Literal::Num(1.0))),
                        Infix::Minus,
                    ))
                ]
            ),
            (
                r#"
a *= 1
                "#,
                vec![
                    Statement::Expression(Expression::CompoundAssign(
                        Box::new(Expression::Identifier(Identifier("a".to_string()))),
                        Box::new(Expression::Literal(Literal::Num(1.0))),
                        Infix::Multiply,
                    ))
                ]
            ),
            (
                r#"
a /= 1
                "#,
                vec![
                    Statement::Expression(Expression::CompoundAssign(
                        Box::new(Expression::Identifier(Identifier("a".to_string()))),
                        Box::new(Expression::Literal(Literal::Num(1.0))),
                        Infix::Divide,
                    ))
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_dotcall() {
        let tests = vec![
            (
                r#"
print hoge.a
                "#,
                vec![
                    Statement::Print(Expression::DotCall(
                        Box::new(Expression::Identifier(Identifier("hoge".into()))),
                        Box::new(Expression::Identifier(Identifier("a".into()))),
                    ))
                ]
            ),
            (
                r#"
print hoge.b()
                "#,
                vec![
                    Statement::Print(Expression::FuncCall{
                        func: Box::new(Expression::DotCall(
                            Box::new(Expression::Identifier(Identifier("hoge".into()))),
                            Box::new(Expression::Identifier(Identifier("b".into()))),
                        )),
                        args: vec![]
                    })
                ]
            ),
            (
                r#"
hoge.a = 1
                "#,
                vec![
                    Statement::Expression(Expression::Assign(
                        Box::new(Expression::DotCall(
                            Box::new(Expression::Identifier(Identifier("hoge".into()))),
                            Box::new(Expression::Identifier(Identifier("a".into()))),
                        )),
                        Box::new(Expression::Literal(Literal::Num(1.0))),
                    ))
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

    #[test]
    fn test_def_dll() {
        let tests = vec![
            (
                r#"
def_dll hoge(int, dword[], byte[128], var string, var long[], {word,word}):bool:hoge.dll
                "#,
                vec![
                    Statement::DefDll {
                        name: "hoge".into(),
                        params: vec![
                            DefDllParam::Param(DllType::Int),
                            DefDllParam::Array(DllType::Dword, None),
                            DefDllParam::Array(DllType::Byte, Some(128)),
                            DefDllParam::Var(DllType::String),
                            DefDllParam::VarArray(DllType::Long, None),
                            DefDllParam::Struct(vec![
                                DefDllParam::Param(DllType::Word),
                                DefDllParam::Param(DllType::Word),
                            ]),
                        ],
                        ret_type: DllType::Bool,
                        path: "hoge.dll".into()
                    }
                ]
            ),
            (
                r#"
def_dll hoge()::hoge.dll
                "#,
                vec![
                    Statement::DefDll {
                        name: "hoge".into(),
                        params: vec![],
                        ret_type: DllType::Void,
                        path: "hoge.dll".into()
                    }
                ]
            ),
            (
                r#"
def_dll hoge():hoge.dll
                "#,
                vec![
                    Statement::DefDll {
                        name: "hoge".into(),
                        params: vec![],
                        ret_type: DllType::Void,
                        path: "hoge.dll".into()
                    }
                ]
            ),
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
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
        parser_test(input, vec![
            Statement::Public(vec![
                (Identifier("p1".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
            Statement::Const(vec![
                (Identifier("c1".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
            Statement::Const(vec![
                (Identifier("c2".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
            Statement::Public(vec![
                (Identifier("p2".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
            Statement::Public(
                vec![(Identifier("p3".to_string()), Expression::Literal(Literal::Num(1.0)))]
            ),
            Statement::Function {
                name: Identifier("f1".to_string()),
                params: vec![],
                body: vec![],
                is_proc: false,
            },
            Statement::Function {
                name: Identifier("p1".to_string()),
                params: vec![],
                body: vec![],
                is_proc: true,
            },
            Statement::Function {
                name: Identifier("f2".to_string()),
                params: vec![],
                body: vec![],
                is_proc: false,
            },
            Statement::Dim(vec![
                (Identifier("d1".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
            Statement::Dim(vec![
                (Identifier("d2".to_string()), Expression::Literal(Literal::Num(1.0)))
            ]),
        ]);
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
        parser_test(input, vec![
            Statement::Module(
                Identifier("Hoge".to_string()),
                vec![
                    Statement::Dim(vec![
                        (Identifier("a".to_string()), Expression::Literal(Literal::Num(1.0)))
                    ]),
                    Statement::Public(vec![
                        (Identifier("b".to_string()), Expression::Literal(Literal::Num(1.0)))
                    ]),
                    Statement::Const(vec![
                        (Identifier("c".to_string()), Expression::Literal(Literal::Num(1.0)))
                    ]),
                    Statement::Function {
                        name: Identifier("Hoge".to_string()),
                        params: vec![],
                        body: vec![
                            Statement::Expression(Expression::Assign(
                                Box::new(Expression::DotCall(
                                    Box::new(Expression::Identifier(Identifier("this".to_string()))),
                                    Box::new(Expression::Identifier(Identifier("a".to_string()))),
                                )),
                                Box::new(Expression::Identifier(Identifier("c".to_string()))),
                            ))
                        ],
                        is_proc: true,
                    },
                    Statement::Function {
                        name: Identifier("f".to_string()),
                        params: vec![
                            Expression::Params(Params::Identifier(Identifier("x".to_string()))),
                            Expression::Params(Params::Identifier(Identifier("y".to_string())))
                        ],
                        body: vec![
                            Statement::Expression(Expression::Assign(
                                Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                Box::new(Expression::Infix(
                                    Infix::Plus,
                                    Box::new(Expression::Identifier(Identifier("x".to_string()))),
                                    Box::new(Expression::FuncCall{
                                        func: Box::new(Expression::Identifier(Identifier("_f".to_string()))),
                                        args: vec![
                                            Expression::Identifier(Identifier("y".to_string()))
                                        ]
                                    }),
                                )),
                            ))
                        ],
                        is_proc: false,
                    },
                    Statement::Dim(vec![
                        (
                            Identifier("_f".to_string()),
                            Expression::AnonymusFunction {
                                params: vec![
                                    Expression::Params(Params::Identifier(Identifier("z".to_string()))),
                                ],
                                body: vec![
                                    Statement::Expression(Expression::Assign(
                                        Box::new(Expression::Identifier(Identifier("result".to_string()))),
                                        Box::new(Expression::Infix(
                                            Infix::Plus,
                                            Box::new(Expression::Identifier(Identifier("z".to_string()))),
                                            Box::new(Expression::Literal(Literal::Num(1.0))),
                                        )),
                                    )),
                                ],
                                is_proc: false
                            }
                        )
                    ]),
                ],
            ),
        ]);
    }

}