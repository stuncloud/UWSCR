use crate::token::Token;
use std::i64;
use std::f64;
use std::fmt;

#[derive(Debug,Clone)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}, {}",self.row, self.column)
    }
}

impl Position {
    pub fn new() -> Self {
        Position{row: 0, column: 0}
    }
}

#[derive(Debug, Clone)]
pub struct TokenWithPos {
    pub token: Token,
    pub pos: Position,
}

impl TokenWithPos {
    pub fn new(token: Token) -> Self {
        TokenWithPos {
            token,
            pos: Position::new()
        }
    }
    pub fn new_with_pos(token: Token, pos: Position) -> Self {
        TokenWithPos{token, pos}
    }
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    next_pos: usize,
    ch: char,
    position: Position,
    position_before: Position,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut lexer: Lexer = Lexer {
            input: input.chars().collect::<Vec<char>>(),
            pos: 0,
            next_pos: 0,
            ch: '\0',
            position: Position {row: 1, column: 0},
            position_before: Position{row: 0, column:0},
        };
        lexer.read_char();

        lexer
    }

    fn to_next_row(&mut self) {
        self.position_before = self.position.clone();
        self.position.row += 1;
        self.position.column = 0;
    }

    fn read_char(&mut self) {
        if self.next_pos >= self.input.len() {
            self.ch = '\0';
        } else {
            self.ch = self.input[self.next_pos];
        }
        self.pos = self.next_pos;
        self.next_pos += 1;
        self.position.column += 1;
    }

    fn nextch(&mut self) -> char {
        if self.next_pos >= self.input.len() {
            '\0'
        } else {
            self.input[self.next_pos]
        }
    }

    fn nextch_is(&mut self, ch: char) -> bool {
        self.nextch() == ch
    }

    fn _ch_after_is(&mut self, n: usize, ch: char) -> bool {
        if self.pos + n >= self.input.len() {
            false
        } else {
            self.input[self.pos + n] == ch
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.ch {
                ' ' | '\t' | '　' => {
                    self.read_char();
                },
                _ => {
                    break;
                }
            }
        }
    }

    pub fn next_token(&mut self) -> TokenWithPos {
        self.skip_whitespace();
        let p: Position = self.position.clone();

        let token: Token = match self.ch {
            '=' => {
                if self.nextch_is('=') {
                    self.read_char();
                    Token::Equal
                } else {
                    Token::EqualOrAssign
                }
            },
            '+' => if self.nextch_is('=') {
                self.read_char();
                Token::AddAssign
            } else {
                Token::Plus
            },
            '-' => if self.nextch_is('=') {
                self.read_char();
                Token::SubtractAssign
            } else {
                Token::Minus
            },
            '/' => {
                if self.nextch_is('/') {
                    // Token::Comment
                    while ! self.nextch_is('\0') {
                        self.read_char();
                        if self.nextch_is('\r') {
                            self.read_char();
                            if self.nextch_is('\n'){
                                self.read_char();
                            }
                            break;
                        }
                        if self.nextch_is('\n') {
                            break;
                        }
                    }
                    self.to_next_row();
                    Token::Eol
                } else if self.nextch_is('=') {
                    self.read_char();
                    Token::DivideAssign
                } else {
                    Token::Slash
                }
            },
            '*' => if self.nextch_is('=') {
                self.read_char();
                Token::MultiplyAssign
            } else {
                Token::Asterisk
            },
            '!' => {
                if self.nextch_is('=') {
                    self.read_char();
                    Token::NotEqual
                } else {
                    Token::Bang
                }
            },
            '<' => {
                if self.nextch_is('=') {
                    self.read_char();
                    Token::LessThanEqual
                } else if self.nextch_is('>') {
                    self.read_char();
                    Token::NotEqual
                } else {
                    Token::LessThan
                }
            },
            '>' => {
                if self.nextch_is('=') {
                    self.read_char();
                    Token::GreaterThanEqual
                } else {
                    Token::GreaterThan
                }
            },
            '(' => Token::Lparen,
            ')' => Token::Rparen,
            '{' => Token::Lbrace,
            '}' => Token::Rbrace,
            '[' => Token::Lbracket,
            ']' => Token::Rbracket,
            '?' => Token::Question,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            ',' => Token::Comma,
            '.' => Token::Period,
            '_' => {
                if self.nextch_is('\n') {
                    Token::LineContinue
                } else {
                    return TokenWithPos::new_with_pos(self.consume_identifier(), p);
                }
            },
            '\\' => Token::BackSlash,
            'a'..='z' | 'A'..='Z' | '#' => {
                return TokenWithPos::new_with_pos(self.consume_identifier(), p);
            },
            '0'..='9' => {
                return TokenWithPos::new_with_pos(self.consume_number(), p);
            },
            '$' => {
                return TokenWithPos::new_with_pos(self.consume_hexadecimal(), p);
            },
            '"' => {
                return TokenWithPos::new_with_pos(self.consume_string(), p);
            },
            '\'' => return TokenWithPos::new_with_pos(self.consume_single_quote_string(), p),
            '\n' => {
                self.to_next_row();
                Token::Eol
            },
            '\r' => {
                if self.nextch_is('\n') {
                    self.read_char();
                }
                self.to_next_row();
                Token::Eol
            },
            '\0' => Token::Eof,
            '\x01'..=' ' => Token::Illegal(self.ch),
            _ => {
                return TokenWithPos::new_with_pos(self.consume_identifier(), p);
            },
        };
        self.read_char();

        return TokenWithPos::new_with_pos(token, p);
    }


    fn consume_special_statement(&mut self) -> String {
        self.skip_whitespace();
        let start_pos = self.pos;
        loop {
            match self.ch {
                '\n' => {
                    break;
                },
                '/' => {
                    if self.nextch_is('/') {
                        break;
                    } else {
                        self.read_char();
                    }
                },
                _ => {
                    self.read_char();
                }
            }
        }

        let sp_statement = self.input[start_pos..self.pos].into_iter().collect();
        sp_statement
    }

    fn consume_identifier(&mut self) -> Token {
        let start_pos = self.pos;
        loop {
            match self.ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '#' => {
                    self.read_char();
                },
                '\0'..=' ' | '　' => {
                    break;
                },
                _ => {
                    self.read_char();
                }
            }
        }
        let literal: &String = &self.input[start_pos..self.pos].into_iter().collect();

        match literal.to_ascii_lowercase().as_str() {
            "if" => Token::If,
            "ifb" => Token::IfB,
            "then" => Token::Then,
            "else" => Token::Else,
            "elseif" => Token::ElseIf,
            "endif" => Token::EndIf,
            "select" => Token::Select,
            "case" => Token::Case,
            "default" => Token::Default,
            "selend" => Token::Selend,
            "print" => Token::Print,
            "call" => {
                Token::Call(self.consume_special_statement())
            },
            "def_dll" => {
                Token::DefDll(self.consume_special_statement())
            },
            "while" => Token::While,
            "wend" => Token::Wend,
            "repeat" => Token::Repeat,
            "until" => Token::Until,
            "for" => Token::For,
            "to" => Token::To,
            "in" => Token::In,
            "step" => Token::Step,
            "next" => Token::Next,
            "continue" => Token::Continue,
            "break" => Token::Break,
            "with" => Token::With,
            "endwith" => Token::EndWith,
            "textblock" => self.consume_textblock(),
            "endtextblock" => Token::EndTextBlock,
            "function" => Token::Function,
            "procedure" => Token::Procedure,
            "fend" => Token::Fend,
            "exit" => Token::Exit,
            "module" => Token::Module,
            "endmodule" => Token::EndModule,
            "class" => Token::Class,
            "endclass" => Token::EndClass,
            "dim" => Token::Dim,
            "public" => Token::Public,
            "const" => Token::Const,
            "hashtbl" => Token::HashTable,
            "mod" => Token::Mod,
            "and" => Token::And,
            "or" => Token::Or,
            "xor" => Token::Xor,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "null" => Token::Null,
            "empty" => Token::Empty,
            "nothing" => Token::Nothing,
            "var" | "ref" => Token::Ref,
            "args" | "prms" => Token::Variadic,
            _ => Token::Identifier(literal.to_string()),
        }
    }

    fn consume_number(&mut self) -> Token {
        let start_pos = self.pos;
        let mut has_period = false;
        loop {
            match self.ch {
                '0'..='9' => {
                    if self.nextch_is('.') {
                        self.read_char();
                        if ! has_period {
                            has_period = true;
                        } else {
                            break;
                        }
                    }
                    self.read_char();
                },
                _ => {
                    break;
                }
            }
        }
        let literal: &String = &self.input[start_pos..self.pos].into_iter().collect();
        Token::Num(literal.parse::<f64>().unwrap())
    }

    fn consume_hexadecimal(&mut self) -> Token {
        self.read_char(); // $の次から読む
        let start_pos = self.pos;
        loop {
            match self.ch {
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    self.read_char();
                },
                _ => {
                    break;
                }
            }
        }
        let literal: &String = &self.input[start_pos..self.pos].into_iter().collect();
        // Token::Hex(literal.to_string())
        let parsed = i64::from_str_radix(literal, 16).unwrap();
        Token::Num(parsed as f64)
    }

    fn consume_string(&mut self) -> Token {
        self.read_char();
        let start_pos = self.pos;
        loop {
            match self.ch {
                '"' | '\0' => {
                    let literal: &String = &self.input[start_pos..self.pos].into_iter().collect();
                    self.read_char();
                    return Token::ExpandableString(literal.to_string());
                },
                _ => {
                    self.read_char();
                }
            }
        }
    }

    fn consume_single_quote_string(&mut self) -> Token {
        self.read_char();
        let start_pos = self.pos;
        loop {
            match self.ch {
                '\'' | '\0' => {
                    let literal: &String = &self.input[start_pos..self.pos].into_iter().collect();
                    self.read_char();
                    return Token::String(literal.to_string());
                },
                _ => {
                    self.read_char();
                }
            }
        }
    }

    fn consume_textblock(&mut self) -> Token {
        // eolまで進める
        let mut name = None;
        loop {
            let t = self.next_token();
            match t.token {
                Token::Eol => break,
                Token::Identifier(s) => {
                    name = Some(s);
                }
                _ => {},
            }
        }
        let start_tb = self.pos;
        let mut end_tb = self.pos;
        loop {
            let t = self.next_token();
            if t.token == Token::EndTextBlock {
                break;
            }
            loop {
                end_tb = self.pos;
                let t = self.next_token();
                if t.token == Token::Eol {
                    break;
                }
            }
        }
        let body: String = self.input[start_tb..end_tb].into_iter().collect();
        Token::TextBlock(name, body)
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::token::Token;

    fn test_next_token(input:&str, expected_tokens:Vec<Token>) {
        let mut  lexer = Lexer::new(input);
        for expected_token in expected_tokens {
            let t = lexer.next_token();
            assert_eq!(t.token, expected_token);
        }
    }

    #[test]
    fn test_dim() {
        let input = "dim hoge = 123";
        let tokens = vec![
            Token::Dim,
            Token::Identifier("hoge".to_string()),
            Token::EqualOrAssign,
            Token::Num(123 as f64),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_dim_array() {
        let input = "dim array[] = 1, 3, 5, 7, 9";
        let tokens = vec![
            Token::Dim,
            Token::Identifier("array".to_string()),
            Token::Lbracket,
            Token::Rbracket,
            Token::EqualOrAssign,
            Token::Num(1 as f64),
            Token::Comma,
            Token::Num(3 as f64),
            Token::Comma,
            Token::Num(5 as f64),
            Token::Comma,
            Token::Num(7 as f64),
            Token::Comma,
            Token::Num(9 as f64),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_public() {
        let input = "public fuga = 123";
        let tokens = vec![
            Token::Public,
            Token::Identifier("fuga".to_string()),
            Token::EqualOrAssign,
            Token::Num(123 as f64),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_uppercase() {
        let input = "PUBLIC fuga = 123";
        let tokens = vec![
            Token::Public,
            Token::Identifier("fuga".to_string()),
            Token::EqualOrAssign,
            Token::Num(123 as  f64),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_numeric() {
        use std::i64;

        let input = r#"print 123
print $1234AB
print 123.456
"#;
        let tokens = vec![
            Token::Print,
            Token::Num(123 as f64),
            Token::Eol,
            Token::Print,
            // Token::Hex("$1234AB".to_string()),
            Token::Num(i64::from_str_radix("1234AB", 16).unwrap() as f64),
            Token::Eol,
            Token::Print,
            Token::Num(123.456),
            Token::Eol,
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_string_literal() {
        let input = "print \"あいうえお\"";
        test_next_token(input, vec![
            Token::Print,
            Token::String(String::from("あいうえお"))
        ]);
    }

    #[test]
    fn test_fullwidth_space() {
        let input = "print　\"全角スペースはホワイトスペース\"";
        test_next_token(input, vec![
            Token::Print,
            Token::String(String::from("全角スペースはホワイトスペース"))
        ]);
    }

    #[test]
    fn test_multibyte_identifier() {
        let input = "変数A = 関数¢1()";
        test_next_token(input, vec![
            Token::Identifier(String::from("変数A")),
            Token::EqualOrAssign,
            Token::Identifier(String::from("関数¢1")),
            Token::Lparen,
            Token::Rparen,
        ]);
    }

    #[test]
    fn test_operators1() {
        let input = "print (1 + 2) * 4 / 3";
        let tokens = vec![
            Token::Print,
            Token::Lparen,
            Token::Num(1 as f64),
            Token::Plus,
            Token::Num(2 as f64),
            Token::Rparen,
            Token::Asterisk,
            Token::Num(4 as f64),
            Token::Slash,
            Token::Num(3 as f64),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_function() {
        let input = r#"function hoge(foo, bar)
    result = foo + bar
fend
"#;
        let tokens = vec![
            Token::Function,
            Token::Identifier("hoge".to_string()),
            Token::Lparen,
            Token::Identifier("foo".to_string()),
            Token::Comma,
            Token::Identifier("bar".to_string()),
            Token::Rparen,
            Token::Eol,
            Token::Identifier("result".to_string()),
            Token::EqualOrAssign,
            Token::Identifier("foo".to_string()),
            Token::Plus,
            Token::Identifier("bar".to_string()),
            Token::Eol,
            Token::Fend,
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_special_statement() {
        let input = r#"
call C:\hoge\fuga\test.uws
def_dll hogefunc(int, int):int: hoge.dll
"#;
        test_next_token(input, vec![
            Token::Eol,
            Token::Call(String::from(r"C:\hoge\fuga\test.uws")),
            Token::Eol,
            Token::DefDll(String::from(r"hogefunc(int, int):int: hoge.dll")),
        ]);
    }

    #[test]
    fn test_hashtbl() {
        let input = "hashtbl hoge";
        test_next_token(input, vec!{
            Token::HashTable,
            Token::Identifier(String::from("hoge"))
        })
    }

    #[test]
    fn test_calc_assign() {
        let test_cases = vec![
            (
                "a += 1",
                vec![
                    Token::Identifier("a".to_string()),
                    Token::AddAssign,
                    Token::Num(1.0)
                ]
            ),
            (
                "a -= 1",
                vec![
                    Token::Identifier("a".to_string()),
                    Token::SubtractAssign,
                    Token::Num(1.0)
                ]
            ),
            (
                "a *= 1",
                vec![
                    Token::Identifier("a".to_string()),
                    Token::MultiplyAssign,
                    Token::Num(1.0)
                ]
            ),
            (
                "a /= 1",
                vec![
                    Token::Identifier("a".to_string()),
                    Token::DivideAssign,
                    Token::Num(1.0)
                ]
            ),
        ];
        for (input, expected) in test_cases {
            test_next_token(input, expected);
        }
    }

    #[test]
    fn text_textblock() {
        let test_cases = vec![
            (
r#"textblock
hoge
endtextblock"#,
                vec![
                    Token::TextBlock(None, "hoge".into())
                ]
            ),
            (
r#"textblock hoge
hoge
fuga
endtextblock"#,
                vec![
                    Token::TextBlock(Some("hoge".into()), "hoge\nfuga".into())
                ]
            ),
            (
r#"
        textblock hoge
        hoge
        fuga
        endtextblock
"#,
                vec![
                    Token::Eol,
                    Token::TextBlock(Some("hoge".into()), "        hoge\n        fuga".into())
                ]
            ),
        ];
        for (input, expected) in test_cases {
            test_next_token(input, expected);
        }

    }
}