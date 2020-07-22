use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::Token;
use std::fmt;

pub enum ParseErrorKind {
    UnexpectedToken
}

#[derive(Debug, Clone)]
pub struct ParseError {
    kind: ParseErrorKind,
    msg: String,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErrorKind::UnexpectedToken => write!(f, "Unexpected Token"),
        }
    }
}
impl fmt::Debug for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErrorKind::UnexpectedToken => write!(f, "Unexpected Token"),
        }
    }
}

impl Clone for ParseErrorKind {
    fn clone(&self) -> Self {
        match *self {
            ParseErrorKind::UnexpectedToken => ParseErrorKind::UnexpectedToken
        }
    }
}

impl ParseError {
    fn new(kind: ParseErrorKind, msg: String) -> Self {
        ParseError {kind, msg}
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.msg)
    }
}

pub type PareseErrors = Vec<ParseError>;

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    next_token: Token,
    errors: PareseErrors,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        let mut parser = Parser {
            lexer,
            current_token: Token::Eof,
            next_token: Token::Eof,
            errors: vec![],
        };
        parser.bump();
        parser.bump();

        parser
    }

    fn token_to_precedence(token: &Token) -> Precedence {
        match token {
            Token::Or | Token::Xor => Precedence::Or,
            Token::And => Precedence::And,
            Token::Equal | Token::EqualOrAssign | Token::NotEqual => Precedence::Equality,
            Token::LessThan | Token::LessThanEqual => Precedence::Relational,
            Token::GreaterThan | Token::GreaterThanEqual => Precedence::Relational,
            Token::Plus | Token::Minus => Precedence::Additive,
            Token::Slash | Token::Asterisk | Token::Mod => Precedence::Multiplicative,
            Token::Lparen => Precedence::FuncCall,
            Token::Lbracket => Precedence::Index,
            Token::Question => Precedence::Ternary,
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

    fn is_current_token(&mut self, token: &Token) -> bool {
        self.current_token == *token
    }

    fn is_current_token_in(&mut self, tokens: Vec<Token>) -> bool {
        tokens.contains(&self.current_token)
    }

    fn is_current_token_end_of_block(&mut self) -> bool {
        let eobtokens = vec![
            Token::Else,
            Token::ElseIf,
            Token::EndIf,
            Token::Wend,
            Token::Until,
            Token::Next,
            Token::EndWith,
            Token::EndTextBlock,
            Token::Fend,
            Token::EndModule,
            Token::EndClass,
        ];
        self.is_current_token_in(eobtokens)
    }

    fn is_next_token(&mut self, token: &Token) -> bool {
        self.next_token == *token
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
                token, self.next_token
            )
        ))
    }

    fn error_got_invalid_token(&mut self, token: Token) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "expected token was {:?}, but got {:?} instead.",
                token, self.current_token
            )
        ))
    }

    fn error_token_is_not_identifier(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "expected token was Identifier, but got {:?} instead.",
                self.current_token
            )
        ))
    }

    fn error_got_unexpected_token(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "unexpected token: {:?}.",
                self.current_token
            )
        ))
    }

    fn current_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.current_token)
    }

    fn next_token_precedence(&mut self) -> Precedence {
        Self::token_to_precedence(&self.next_token)
    }

    fn error_no_prefix_parser(&mut self) {
        self.errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            format!(
                "no prefix parser found for \"{:?}\"",
                self.current_token
            )
        ))
    }

    pub fn parse(&mut self) -> Program {
        let mut program: Program = vec![];

        while ! self.is_current_token(&Token::Eof) {
            match self.parse_statement() {
                Some(s) => program.push(s),
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
        match self.current_token {
            Token::Dim => self.parse_dim_statement(),
            Token::If => self.parse_if_statement(),
            Token::Print => self.parse_print_statement(),
            Token::Result => self.parse_result_statement(),
            Token::For => self.parse_for_statement(),
            Token::While => self.parse_while_statement(),
            Token::Repeat => self.parse_repeat_statement(),
            Token::Blank => Some(Statement::Blank),
            Token::Call(_) => self.parse_special_statement(),
            Token::DefDll(_) => self.parse_special_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_dim_statement(&mut self) -> Option<Statement> {
        match &self.next_token {
            Token::Identifier(_) => self.bump(),
            _ => return None,
        }

        // 変数名
        let var_name = match self.parse_identifier() {
            Some(e) => e,
            None => return None
        };

        if self.is_next_token(&Token::Lbracket) {
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

            // 代入演算子がなければ配列宣言のみ
            if ! self.is_next_token(&Token::EqualOrAssign) {
                return Some(Statement::DimArray(var_name, index, vec![]));
            };
            self.bump();

            let list = match self.parse_expression_list(Token::Eol) {
                Some(vec_e) => vec_e,
                None => return None
            };

            Some(Statement::DimArray(var_name, index, list))
        } else {
            // 変数定義
            // 代入演算子がなければ変数宣言のみ
            if ! self.is_next_token(&Token::EqualOrAssign) {
                return Some(Statement::Dim(var_name, Expression::Literal(Literal::Empty)));
            };
            self.bump();
            self.bump();
            let expression = match self.parse_expression(Precedence::Lowest, false) {
                Some(e) => e,
                None => return None
            };
            if self.is_next_token(&Token::Semicolon) || self.is_next_token(&Token::Eol) {
                self.bump();
            }

            Some(Statement::Dim(var_name, expression))
        }

    }

    fn parse_result_statement(&mut self) -> Option<Statement> {
        match &self.next_token {
            Token::Identifier(_) => self.bump(),
            _ => return None,
        };

        // 代入演算子かどうか
        if ! self.is_next_token_expected(Token::EqualOrAssign) {
            return None;
        };

        self.bump();

        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };

        if self.is_next_token(&Token::Semicolon) || self.is_next_token(&Token::LineBreak) {
            self.bump();
        }

        Some(Statement::Result(expression))
    }

    fn parse_print_statement(&mut self) -> Option<Statement> {
        self.bump();

        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };

        Some(Statement::Print(expression))
    }

    fn parse_special_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Call(ref mut s) => {
                Some(Statement::Call(s.clone()))
            },
            Token::DefDll(ref mut s) => {
                Some(Statement::DefDll(s.clone()))
            },
            _ => None
        }
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
        match self.next_token {
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
                let block = self.parse_block_statement();

                if ! self.is_current_token(&Token::Next) {
                    self.error_got_invalid_token(Token::Next);
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
                let block = self.parse_block_statement();

                if ! self.is_current_token(&Token::Next) {
                    self.error_got_invalid_token(Token::Next);
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
        let block = self.parse_block_statement();
        if ! self.is_current_token_expected(Token::Wend) {
            return None;
        }
        Some(Statement::While(expression, block))
    }

    fn parse_repeat_statement(&mut self) -> Option<Statement> {
        self.bump();
        let block = self.parse_block_statement();
        if ! self.is_current_token_expected(Token::Until) {
            return None;
        }
        self.bump();
        let expression = match self.parse_expression(Precedence::Lowest, false) {
            Some(e) => e,
            None => return None
        };

        Some(Statement::Repeat(expression, block))
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        match self.parse_expression(Precedence::Lowest, true) {
            Some(e) => {
                if self.is_next_token(&Token::Semicolon) || self.is_next_token(&Token::LineBreak) {
                    self.bump();
                }
                Some(Statement::Expression(e))
            }
            None => None
        }
    }

    fn parse_expression(&mut self, precedence: Precedence, is_sol: bool) -> Option<Expression> {
        // prefix
        let mut left = match self.current_token {
            Token::Identifier(_) => {
                let identifier = self.parse_identifier_expression();
                if is_sol {
                    if self.is_next_token(&Token::EqualOrAssign) {
                        return self.parse_assign_expression(identifier.unwrap());
                    }
                }
                identifier
            },
            Token::Empty => Some(Expression::Literal(Literal::Empty)),
            Token::Null => Some(Expression::Literal(Literal::Null)),
            Token::Nothing => Some(Expression::Literal(Literal::Nothing)),
            Token::Num(_) => self.parse_number_expression(),
            Token::String(_) => self.parse_string_expression(),
            Token::Bool(_) => self.parse_bool_expression(),
            Token::Lbracket => self.parse_array_expression(),
            Token::Bang | Token::Minus | Token::Plus => self.parse_prefix_expression(),
            Token::Lparen => self.parse_grouped_expression(),
            Token::HashTable => self.parse_hashtable_expression(),
            Token::Function => self.parse_function_expression(),
            Token::Then | Token::Eol => return None,
            _ => {
                self.error_no_prefix_parser();
                return None;
            }
        };

        // #[cfg(test)]
        // println!("test:parse_expression left: {:?}, next token: {:?}", left, self.next_token);

        // infix
        while (
            ! self.is_next_token(&Token::Semicolon)
            || ! self.is_next_token(&Token::LineBreak)
        ) && precedence < self.next_token_precedence() {
            match self.next_token {
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
                Token::Lbracket => {
                    self.bump();
                    left = {
                        let index = self.parse_index_expression(left.unwrap());
                        if is_sol {
                            if self.is_next_token(&Token::EqualOrAssign) {
                                return self.parse_assign_expression(index.unwrap());
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
                }
                _ => return left
            }
        }

        left
    }

    fn parse_identifier(&mut self) -> Option<Identifier> {
        match self.current_token {
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

    fn parse_number_expression(&mut self) -> Option<Expression> {
        match self.current_token {
            Token::Num(ref mut num) => Some(
                Expression::Literal(Literal::Num(num.clone()))
            ),
            _ => None
        }
    }

    fn parse_string_expression(&mut self) -> Option<Expression> {
        match self.current_token {
            Token::String(ref mut s) => Some(
                Expression::Literal(Literal::String(s.clone()))
            ),
            _ => None
        }
    }

    fn parse_bool_expression(&mut self) -> Option<Expression> {
        match self.current_token {
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

    fn parse_hashtable_expression(&mut self) -> Option<Expression> {
        // hashtable hoge
        // hashtable hoge = HASH_CASECARE
        // hashtable hoge = HASH_SORT
        self.bump();
        let identifier = match self.parse_identifier() {
            Some(i) => i,
            None => return None
        };
        let expression;
        if self.is_next_token(&Token::EqualOrAssign) {
            self.bump();
            self.bump();
            expression = match self.parse_expression(Precedence::Lowest, false) {
                Some(e) => Some(e),
                None => return None
            };
        } else {
            expression = None;
        }
        Some(Expression::HashTbl(identifier, Box::new(expression)))
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

        if ! self.is_next_token_expected(end) {
            return None;
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

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let prefix = match self.current_token {
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
        let infix = match self.current_token {
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
        if ! self.is_next_token_expected(Token::Rbracket) {
            return None;
        }

        Some(Expression::Index(Box::new(left), Box::new(index)))
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
            self.error_got_invalid_token(Token::EndIf);
            return None;
        }
        Some(Statement::ElseIf {
            condition,
            consequence,
            alternatives
        })

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

    fn parse_function_expression(&mut self) -> Option<Expression> {
        if ! self.is_next_token_expected(Token::Lparen) {
            return None;
        }

        let params = match self.parse_function_parameters() {
            Some(p) => p,
            None => return None
        };

        let body = self.parse_block_statement();

        Some(Expression::Function {
            params,
            body
        })
    }

    fn parse_function_parameters(&mut self) -> Option<Vec<Identifier>> {
        let mut params = vec![];
        if self.is_next_token(&Token::Rparen) {
            self.bump();
            return Some(params);
        }

        self.bump();
        match self.parse_identifier() {
            Some(i) => params.push(i),
            None => return None
        }

        while self.is_next_token(&Token::Comma) {
            self.bump();
            self.bump();
            match self.parse_identifier() {
                Some(i) => params.push(i),
                None => return None
            }
        }

        if ! self.is_next_token_expected(Token::Rparen) {
            return None;
        }

        Some(params)
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
        let input = r#"
dim hoge = 1
dim fuga
dim piyo = EMPTY
dim arr1[] = 1, 3, 5, 7, 9
dim arr2[4]
        "#;
        parser_test(input, vec![
            Statement::Dim(
                Identifier(String::from("hoge")),
                Expression::Literal(Literal::Num(1 as f64))
            ),
            Statement::Dim(
                Identifier(String::from("fuga")),
                Expression::Literal(Literal::Empty)
            ),
            Statement::Dim(
                Identifier(String::from("piyo")),
                Expression::Literal(Literal::Empty)
            ),
            Statement::DimArray(
                Identifier(String::from("arr1")),
                Expression::Literal(Literal::Empty),
                vec![
                    Expression::Literal(Literal::Num(1 as f64)),
                    Expression::Literal(Literal::Num(3 as f64)),
                    Expression::Literal(Literal::Num(5 as f64)),
                    Expression::Literal(Literal::Num(7 as f64)),
                    Expression::Literal(Literal::Num(9 as f64)),
                ]
            ),
            Statement::DimArray(
                Identifier(String::from("arr2")),
                Expression::Literal(Literal::Num(4 as f64)),
                vec![]
            ),
        ]);
    }

    #[test]
    fn test_special() {
        let input = r#"
call C:\hoge\fuga\test.uws
call C:\hoge\fuga\test.uws(1, 2)
def_dll hogefunc(int, int):int: C:\path\to\hoge.dll
        "#;
        parser_test(input, vec![
            Statement::Call(
                String::from(r"C:\hoge\fuga\test.uws")
            ),
            Statement::Call(
                String::from(r"C:\hoge\fuga\test.uws(1, 2)")
            ),
            Statement::DefDll(
                String::from(r"hogefunc(int, int):int: C:\path\to\hoge.dll")
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
print "文字列リテラル"
print ["配", "列", "リ", "テ", "ラ", "ル"]
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
                Literal::String(String::from("文字列リテラル"))
            )),
            Statement::Print(Expression::Literal(
                Literal::Array(vec![
                    Expression::Literal(Literal::String(String::from("配"))),
                    Expression::Literal(Literal::String(String::from("列"))),
                    Expression::Literal(Literal::String(String::from("リ"))),
                    Expression::Literal(Literal::String(String::from("テ"))),
                    Expression::Literal(Literal::String(String::from("ラ"))),
                    Expression::Literal(Literal::String(String::from("ル"))),
                ])
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
                                Box::new(Expression::Literal(Literal::Array(vec![
                                    Expression::Literal(Literal::Num(1 as f64)),
                                    Expression::Literal(Literal::Num(2 as f64)),
                                    Expression::Literal(Literal::Num(3 as f64)),
                                    Expression::Literal(Literal::Num(4 as f64)),
                                ]))),
                                Box::new(Expression::Infix(
                                    Infix::Multiply,
                                    Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                    Box::new(Expression::Identifier(Identifier(String::from("c")))),
                                ))
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
                                    Box::new(Expression::Literal(Literal::Num(2 as f64)))
                                ))
                            ),
                            Expression::Index(
                                Box::new(Expression::Identifier(Identifier(String::from("b")))),
                                Box::new(Expression::Literal(Literal::Num(1 as f64)))
                            ),
                            Expression::Infix(
                                Infix::Multiply,
                                Box::new(Expression::Literal(Literal::Num(2 as f64))),
                                Box::new(Expression::Index(
                                    Box::new(Expression::Literal(Literal::Array(vec![
                                        Expression::Literal(Literal::Num(1 as f64)),
                                        Expression::Literal(Literal::Num(2 as f64)),
                                    ]))),
                                    Box::new(Expression::Literal(Literal::Num(1 as f64)))
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
                            Box::new(Expression::Literal(Literal::Num(0 as f64)))
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
                        })
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
        ];
        for (input, expected) in tests {
            parser_test(input, expected);
        }
    }

}