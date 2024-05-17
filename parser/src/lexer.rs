use crate::token::BlockEnd;
use crate::token::Token;
use std::f64;
use std::fmt;
use std::cmp::Ordering;

#[derive(Debug,Clone,Copy,Default,PartialEq)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}, {}",self.row, self.column)
    }
}
impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.row.partial_cmp(&other.row) {
            Some(Ordering::Equal) => {
                self.column.partial_cmp(&other.column)
            }
            ord => ord,
        }
    }
}

impl Position {
    pub fn new(row: usize, column: usize) -> Self {
        Position{row, column}
    }
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub pos: Position,
    pub skipped_whitespace: bool,
}

impl TokenInfo {
    pub fn new(token: Token) -> Self {
        TokenInfo {
            token,
            pos: Position::default(),
            skipped_whitespace: false,
        }
    }
    pub fn new_with_pos(token: Token, pos: Position, skipped_whitespace: bool) -> Self {
        TokenInfo{token, pos, skipped_whitespace}
    }
    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn token_len(&self) -> usize {
        self.token.len()
    }
    pub fn get_end_pos(&self) -> Position {
        Position {
            row: self.pos.row,
            column: self.pos.column + self.token_len()
        }
    }

    // Semantic Tokens用
    pub fn as_token_len(&self) -> u32 {
        self.token.len() as u32
    }
    pub fn as_delta_line(&self) -> u32 {
        self.pos.row as u32 - 1
    }
    pub fn as_delta_start(&self) -> u32 {
        self.pos.column as u32 - 1
    }
}

pub struct Lexer {
    input: Vec<char>,
    pub lines: Vec<String>,
    pos: usize,
    next_pos: usize,
    ch: char,
    position: Position,
    position_before: Position,
    textblock_flg: bool,
    is_textblock: bool,
    is_comment_textblock: bool,
    is_call: bool,
    /// ( と ) のペア数カウント, dllパスの終端位置
    def_dll: Option<(u32, usize)>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut lexer: Lexer = Lexer {
            input: input.chars().collect::<Vec<char>>(),
            lines: input.lines().map(|s| s.to_string()).collect(),
            pos: 0,
            next_pos: 0,
            ch: '\0',
            position: Position {row: 1, column: 0},
            position_before: Position{row: 0, column:0},
            textblock_flg: false,
            is_textblock: false,
            is_comment_textblock: true,
            is_call: false,
            def_dll: None,
        };
        lexer.read_char();

        lexer
    }
    pub fn get_line(&self, row: usize) -> String {
        if row > 0 && row <= self.lines.len() {
            let line = self.lines[row - 1].clone();
            line
        } else {
            String::new()
        }
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

    /// 同一行の指定位置に移動
    fn move_to(&mut self, pos: usize) {
        if pos >= self.input.len() {
            self.ch = '\0';
        } else {
            self.ch = self.input[pos];
        }
        self.pos = pos;
        self.next_pos = pos + 1;
        self.position.column = pos;
    }

    fn nextch(&mut self) -> char {
        if self.next_pos >= self.input.len() {
            '\0'
        } else {
            self.input[self.next_pos]
        }
    }

    pub fn nextch_is(&mut self, ch: char) -> bool {
        self.nextch() == ch
    }

    fn ch_nth_after_is(&mut self, n: usize, ch: char) -> bool {
        if self.pos + n >= self.input.len() {
            false
        } else {
            self.input.get(self.pos+n)
                .map(|c| *c == ch)
                .unwrap_or(false)
        }
    }
    fn as_string(&self, from: usize, to: usize) -> Option<String> {
        self.input.get(from..to)
            .map(|slice| slice.iter().collect())
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        loop {
            match self.ch {
                ' ' | '\t' | '　' => {
                    self.read_char();
                    skipped = true;
                },
                _ => {
                    break;
                }
            }
        }
        skipped
    }

    pub fn next_token(&mut self) -> TokenInfo {
        if self.is_textblock {
            let p = self.position.clone();
            let body = self.get_textblock_body();
            let is_comment = self.is_comment_textblock;
            self.is_comment_textblock = true;
            return TokenInfo::new_with_pos(Token::TextBlockBody(body, is_comment), p, false);
        }
        let skipped = self.skip_whitespace();
        let p: Position = self.position.clone();

        if self.def_dll.is_some_and(|(_, len)| len > 0 ) {
            let token = self.get_dll_path();
            return TokenInfo::new_with_pos(token, p, skipped);
        }
        if self.is_call {
            self.is_call = false;
            let token = self.consume_call_path();
            return TokenInfo::new_with_pos(token, p, skipped);
        }

        let token: Token = match self.ch {
            '=' => {
                if self.nextch_is('=') {
                    self.read_char();
                    Token::Equal
                } else if self.nextch_is('>') {
                    self.read_char();
                    Token::Arrow
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
                    // //- の場合トークンを作らない
                    if self.ch_nth_after_is(2, '-') {
                        self.read_char();
                        self.read_char();
                        self.read_char();
                        return self.next_token();
                    }
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
            '(' => {
                if let Some((n, _)) = self.def_dll.as_mut() {
                    *n += 1;
                }
                Token::Lparen
            },
            ')' => {
                if let Some((n, _)) = self.def_dll.as_mut() {
                    *n = n.saturating_sub(1);
                }
                Token::Rparen
            },
            '{' => Token::Lbrace,
            '}' => Token::Rbrace,
            '[' => Token::Lbracket,
            ']' => Token::Rbracket,
            '?' => Token::Question,
            ':' => if self.nextch_is('\\') {
                self.read_char();
                Token::ColonBackSlash
            } else if self.nextch_is('=') {
                self.read_char();
                Token::Assign
            } else if self.def_dll.is_some_and(|(n, _)| n == 0) {
                self.is_dll_path();
                Token::Colon
            } else {
                Token::Colon
            },
            ';' => Token::Eol,
            ',' => Token::Comma,
            '.' => Token::Period,
            '_' => {
                match self.nextch() {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '#' => {
                        return TokenInfo::new_with_pos(self.consume_identifier(), p, skipped);
                    },
                    _ => {
                        self.read_char();
                        let tp = self.next_token();
                        match tp.clone().token {
                            Token::Eol => return self.next_token(),
                            _ => return tp,
                        }
                    },
                }
            },
            '@' => self.consume_uobject(),
            // '\\' => Token::BackSlash,
            'a'..='z' | 'A'..='Z' | '#' | '\\' => {
                return TokenInfo::new_with_pos(self.consume_identifier(), p, skipped);
            },
            '0'..='9' => {
                return TokenInfo::new_with_pos(self.consume_number(), p, skipped);
            },
            '$' => {
                return TokenInfo::new_with_pos(self.consume_hexadecimal(), p, skipped);
            },
            '"' => {
                return TokenInfo::new_with_pos(self.consume_string(), p, skipped);
            },
            '|' => Token::Pipeline,
            '\'' => return TokenInfo::new_with_pos(self.consume_single_quote_string(), p, skipped),
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
                return TokenInfo::new_with_pos(self.consume_identifier(), p, skipped);
            },
        };
        if token == Token::Eol && self.textblock_flg {
            self.is_textblock = true;
            self.textblock_flg = false;
        }
        self.read_char();
        return TokenInfo::new_with_pos(token, p, skipped);
    }

    fn is_dll_path(&mut self) {
        let mut pos = self.pos + 1;
        let pos = loop {
            if let Some(c) = self.input.get(pos) {
                match c {
                    ':' => {
                        if ! self.input.get(pos+1).is_some_and(|c2| *c2 == '\\') {
                            break 0;
                        }
                    },
                    // 行末
                    '\r' | '\n' => break pos,
                    // コメント
                    '/' => if self.input.get(pos+1).is_some_and(|c2| *c2 == '/') {
                        if ! self.input.get(pos+2).is_some_and(|c3| *c3 == '-') {
                            break pos;
                        }
                    },
                    _ => {},
                }
                pos += 1;
            } else {
                // 文末
                break pos;
            }
        };
        if let Some((_, is_dll_path)) = self.def_dll.as_mut() {
            *is_dll_path = pos;
        }
    }
    fn get_dll_path(&mut self) -> Token {
        let end = self.def_dll.unwrap_or_default().1;
        let token = if end > 0 {
            let start = self.pos;
            self.move_to(end);
            let path: String = self.input[start..self.pos].into_iter().collect();
            let path = path.trim_end();
            Token::DllPath(path.into())
        } else {
            Token::DllPath(String::new())
        };
        self.def_dll = None;
        token
    }

    fn consume_call_path(&mut self) -> Token {
        if let Some(slice) = self.input.get(self.pos..self.pos+4) {
            if slice == ['u', 'r', 'l', '['] {
                // url解析
                self.pos = self.pos + 4;
                self.next_pos = self.pos + 1;
                self.ch = match self.input.get(self.pos) {
                    Some(c) => *c,
                    None => return Token::Eof,
                };
                let start_pos = self.pos;
                loop {
                    match self.nextch() {
                        '\r' | '\n' | '\0' => {
                            // 書式が不正
                            return Token::Illegal(self.nextch())
                        },
                        ']' => {
                            self.read_char();
                            break;
                        },
                        _ => self.read_char(),
                    }
                }
                let uri = self.as_string(start_pos, self.pos).unwrap_or_default();
                self.read_char();
                return Token::Uri(uri);
            }
        }
        // パスの解析
        // 現在地から行末までに \ (バックスラッシュ)がなければファイル名とする
        // \ (スラッシュ) もパス区切りとして扱う
        // ファイル名部分の最後に ( があればその直前までをパスとする
        // ( からはまたnext_tokenさせる
        let mut start_pos = self.pos;
        let mut back_slash_pos: usize = 0;
        let mut lparen_pos: usize = 0;
        // "path.uws" の場合に対応
        if let Some(char) = self.input.get(self.pos) {
            if '"' == *char {
                self.input[self.pos] = ' ';
                start_pos += 1;
                self.read_char();
            }
        }

        loop {
            match self.nextch() {
                '\r' | '\n' | '\0' => {
                    break;
                },
                '"' => {
                    self.input[self.next_pos] = ' ';
                },
                '/' => if self.ch_nth_after_is(2, '/') {
                    // コメントなので抜ける
                    break;
                } else {
                    // \と同じ扱い
                    back_slash_pos = self.pos + 1
                },
                '\\' => back_slash_pos = self.pos + 1,
                '(' => lparen_pos = self.pos + 1,
                _ => {}
            }
            self.read_char();
        }
        let end_pos = if lparen_pos > 0 {
            // ( がある場合は現在地を戻す
            self.pos = lparen_pos;
            self.next_pos = lparen_pos + 1;
            self.ch = match self.input.get(lparen_pos) {
                Some(c) => *c,
                None => return Token::Eof,
            };
            lparen_pos
        } else {
            self.read_char();
            // if self.ch == '\0' {
            if ['\r', '\n', '\0'].contains(&self.ch) {
                // 行・分末の場合
                self.pos
            } else {
                self.pos - 1
            }
        };
        let (dir, name) = if back_slash_pos > 0 {
            (
                self.as_string(start_pos, back_slash_pos),
                self.as_string(back_slash_pos+1, end_pos).unwrap_or_default()
            )
        } else {
            (
                None,
                // self.input[start_pos..end_pos].into_iter().collect::<String>()
                self.as_string(start_pos, end_pos).unwrap_or_default()
            )
        };

        Token::Path(dir, name)
    }

    fn get_identifier(&mut self) -> String {
        let start_pos = self.pos;
        loop {
            match self.ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '#' | '\\' => {
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
        self.as_string(start_pos, self.pos).unwrap_or_default()
    }

    fn consume_identifier(&mut self) -> Token {
        if self.textblock_flg {
            self.is_comment_textblock = false;
        }
        let literal = self.get_identifier();

        match literal.to_ascii_lowercase().as_str() {
            "if" => Token::If,
            "ifb" => Token::IfB,
            "then" => Token::Then,
            "else" => Token::BlockEnd(BlockEnd::Else),
            "elseif" => Token::BlockEnd(BlockEnd::ElseIf),
            "endif" => Token::BlockEnd(BlockEnd::EndIf),
            "select" => Token::Select,
            "case" => Token::BlockEnd(BlockEnd::Case),
            "default" => Token::BlockEnd(BlockEnd::Default),
            "selend" => Token::BlockEnd(BlockEnd::Selend),
            "print" => Token::Print,
            "call" => {
                self.is_call = true;
                Token::Call
            },
            "def_dll" => {
                self.def_dll = Some((0, 0));
                Token::DefDll
            },
            "while" => Token::While,
            "wend" => Token::BlockEnd(BlockEnd::Wend),
            "repeat" => Token::Repeat,
            "until" => Token::BlockEnd(BlockEnd::Until),
            "for" => Token::For,
            "to" => Token::To,
            "in" => Token::In,
            "step" => Token::Step,
            "next" => Token::BlockEnd(BlockEnd::Next),
            "endfor" => Token::BlockEnd(BlockEnd::EndFor),
            "continue" => Token::Continue,
            "break" => Token::Break,
            "with" => Token::With,
            "endwith" => Token::BlockEnd(BlockEnd::EndWith),
            "try" => Token::Try,
            "except" => Token::BlockEnd(BlockEnd::Except),
            "finally" => Token::BlockEnd(BlockEnd::Finally),
            "endtry" => Token::BlockEnd(BlockEnd::EndTry),
            "textblock" => {
                self.textblock_flg = true;
                Token::TextBlock(false)
            },
            "textblockex" => {
                self.textblock_flg = true;
                Token::TextBlock(true)
            },
            "endtextblock" => Token::EndTextBlock,
            "enum" => Token::Enum,
            "endenum" => Token::BlockEnd(BlockEnd::EndEnum),
            "struct" => Token::Struct,
            "endstruct" => Token::BlockEnd(BlockEnd::EndStruct),
            "function" => Token::Function,
            "procedure" => Token::Procedure,
            "fend" => Token::BlockEnd(BlockEnd::Fend),
            "exit" => Token::Exit,
            "exitexit" => Token::ExitExit,
            "module" => Token::Module,
            "endmodule" => Token::BlockEnd(BlockEnd::EndModule),
            "class" => Token::Class,
            "endclass" => Token::BlockEnd(BlockEnd::EndClass),
            "dim" => Token::Dim,
            "public" => Token::Public,
            "const" => Token::Const,
            "hashtbl" => Token::HashTable,
            "hash" => Token::Hash,
            "endhash" => Token::BlockEnd(BlockEnd::EndHash),
            "mod" => Token::Mod,
            "and" => Token::And,
            "or" => Token::Or,
            "xor" => Token::Xor,
            "andl" => Token::AndL,
            "orl" => Token::OrL,
            "xorl" => Token::XorL,
            "andb" => Token::AndB,
            "orb" => Token::OrB,
            "xorb" => Token::XorB,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "null" => Token::Null,
            "empty" => Token::Empty,
            "nothing" => Token::Nothing,
            "nan" => Token::NaN,
            "var" | "ref" => Token::Ref,
            "args" | "prms" => Token::Variadic,
            "option" => self.consume_option(),
            "thread" => Token::Thread,
            "async" => Token::Async,
            "await" => Token::Await,
            "com_err_ign" => Token::ComErrIgn,
            "com_err_ret" => Token::ComErrRet,
            "com_err_flg" => Token::ComErrFlg,
            _ => Token::Identifier(literal.to_string()),
        }
    }

    fn consume_option(&mut self) -> Token {
        self.skip_whitespace();
        let pos = self.pos;
        let ident = self.get_identifier();
        Token::Option(ident.to_ascii_lowercase(), pos)
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
        Token::Hex(literal.to_string())
    }

    fn consume_string(&mut self) -> Token {
        self.read_char();
        let start_pos = self.pos;
        let mut join = false;
        loop {
            match self.ch {
                '"' | '\0' => {
                    let literal: &String = &self.input[start_pos..self.pos].into_iter()
                        .filter(|c| **c != '\0')
                        .collect();
                    self.read_char();
                    return Token::ExpandableString(literal.to_string());
                },
                '\n' => {
                    if join {
                        self.input[self.pos] = '\0';
                        join = false;
                    }
                    self.read_char();
                    self.to_next_row();
                },
                '_' => {
                    if self.will_end_line() {
                        join = true;
                        self.input[self.pos] = '\0';
                    }
                    self.read_char();
                },
                _ => {
                    if join {
                        self.input[self.pos] = '\0';
                    }
                    self.read_char();
                }
            }
        }
    }

    fn consume_single_quote_string(&mut self) -> Token {
        self.read_char();
        let start_pos = self.pos;
        let mut join = false;
        loop {
            match self.ch {
                '\'' | '\0' => {
                    let literal: &String = &self.input[start_pos..self.pos].into_iter()
                        .filter(|c| **c != '\0')
                        .collect();
                    self.read_char();
                    return Token::String(literal.to_string());
                },
                '\n' => {
                    if join {
                        self.input[self.pos] = '\0';
                        join = false;
                    }
                    self.read_char();
                    self.to_next_row();
                },
                '_' => {
                    if self.will_end_line() {
                        join = true;
                        self.input[self.pos] = '\0';
                    }
                    self.read_char();
                },
                _ => {
                    if join {
                        self.input[self.pos] = '\0';
                    }
                    self.read_char();
                }
            }
        }
    }
    fn will_end_line(&self) -> bool {
        let mut pos = self.pos + 1;
        loop {
            match &self.input[pos] {
                // ホワイトスペース
                ' ' | '\t' | '　' => {},
                '\r' | '\n' => break true,
                '/' => {
                    if self.input[pos+1] == '/' {
                        break true
                    }
                },
                _ => break false,
            }
            pos += 1
        }
    }

    fn consume_uobject(&mut self) -> Token {
        // jsonじゃなさそうならIllegal
        let start_char = self.nextch();
        match start_char {
            '{' | '[' => self.read_char(),
            _ => return Token::Illegal('@'),
        };
        let start_pos = self.pos;
        loop {
            match self.nextch() {
                '"' => {
                    self.read_char();
                    while ! ['"', '\0'].contains(&self.nextch()) {
                        // 文字列が閉じられるか、文末まで進める
                        self.read_char();
                    }
                },
                '/' => {
                    if self.input[self.pos + 2] == '/' {
                        // コメントなので行末まで消す
                        while ! ['\r', '\n', '\0'].contains(&self.nextch()) {
                            self.input.remove(self.pos);
                        }
                        self.input.remove(self.pos);
                    }
                },
                '}' | ']' => {

                    self.read_char();
                    if self.nextch_is('@') {
                        break;
                    }
                    continue;
                },
                // 文末まで来てしまった場合
                '\0' => return Token::UObjectNotClosing,
                _ => {},
            }
            self.read_char();
        }
        self.read_char();
        let json: String = self.input[start_pos..self.pos].into_iter().collect();
        Token::UObject(json)
    }

    fn is_endtextblock(&mut self) -> bool {
        let pos = self.pos;
        let endtextblock = "endtextblock";
        self.skip_whitespace();
        let result = if ['e', 'E'].contains(&self.ch) {
            match self.as_string(self.pos, self.pos + endtextblock.len()) {
                Some(s) => s.eq_ignore_ascii_case(endtextblock),
                None => false,
            }
        } else {
            false
        };
        self.pos = pos;
        self.next_pos = pos + 1;
        self.ch = if self.input.len() > pos {
            self.input[pos]
        } else {
            self.input[self.input.len()- 1]
        };
        result
    }

    fn get_textblock_body(&mut self) -> String {
        /*
        textblock hoge // parserはToken::Textblock後のEoLに来たらこれを呼ぶ
        hoge
        fuga
        piyo           // endtextblock前のEoLまでを返す
        endtextblock
        */
        let start_pos = self.pos;
        let mut end_pos = self.pos;

        // 即endtextblockで閉じられているかどうか
        if self.is_endtextblock() {
            self.is_textblock = false;
            return "".to_string();
        }
        loop {
            match self.nextch() {
                // 行末が来たら次がendtextblockかどうかを見る
                '\r' | '\n' => {
                    end_pos = self.pos + 1;
                    self.read_char();
                    if self.nextch_is('\n') {
                        self.read_char();
                    }
                    self.read_char();
                    self.position.row += 1;
                    if self.is_endtextblock() {
                        break;
                    } else {
                        continue;
                    }
                },
                '\0' => break,
                _ => self.read_char()
            };
        }
        self.position.column = 0;
        self.is_textblock = false;
        let body: String = self.input[start_pos..end_pos].into_iter().collect();
        body
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::token::{Token, BlockEnd};

    fn test_next_token(input:&str, expected_tokens:Vec<Token>) {
        let mut  lexer = Lexer::new(input);
        for expected_token in expected_tokens {
            let t = lexer.next_token();
            println!("debug output on test: {:?}", &t);
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

        let input = r#"print 123
print $1234AB
print 123.456
"#;
        let tokens = vec![
            Token::Print,
            Token::Num(123 as f64),
            Token::Eol,
            Token::Print,
            Token::Hex("1234AB".to_string()),
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
            Token::ExpandableString(String::from("あいうえお"))
        ]);
    }

    #[test]
    fn test_fullwidth_space() {
        let input = "print　\"全角スペースはホワイトスペース\"";
        test_next_token(input, vec![
            Token::Print,
            Token::ExpandableString(String::from("全角スペースはホワイトスペース"))
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
            Token::BlockEnd(BlockEnd::Fend),
        ];
        test_next_token(input, tokens);
    }

    #[test]
    fn test_def_dll() {
        let testcases = vec![
            (
                "def_dll hogefunc(int, var long):bool: hoge.dll",
                vec![
                    Token::DefDll,
                    Token::Identifier("hogefunc".into()),
                    Token::Lparen,
                    Token::Identifier("int".into()),
                    Token::Comma,
                    Token::Ref,
                    Token::Identifier("long".into()),
                    Token::Rparen,
                    Token::Colon,
                    Token::Identifier("bool".into()),
                    Token::Colon,
                    Token::DllPath("hoge.dll".into()),
                ]
            ),
            (
                r#"def_dll hogefunc():C:\hoge.dll"#,
                vec![
                    Token::DefDll,
                    Token::Identifier("hogefunc".into()),
                    Token::Lparen,
                    Token::Rparen,
                    Token::Colon,
                    Token::DllPath("C:\\hoge.dll".into())
                ]
            ),
            (
                r#"def_dll hogefunc()::C:\hoge\hoge.dll"#,
                vec![
                    Token::DefDll,
                    Token::Identifier("hogefunc".into()),
                    Token::Lparen,
                    Token::Rparen,
                    Token::Colon,
                    Token::Colon,
                    Token::DllPath("C:\\hoge\\hoge.dll".into())
                ]
            ),
        ];
        for (input, expected) in testcases {
            test_next_token(input, expected);
        }
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
comment
endtextblock"#,
                vec![
                    Token::TextBlock(false),
                    Token::Eol,
                    Token::TextBlockBody("comment".into(), true),
                    Token::EndTextBlock,
                ]
            ),
            (
r#"
    textblock
    endtextblock
"#,
                vec![
                    Token::Eol,
                    Token::TextBlock(false),
                    Token::Eol,
                    Token::TextBlockBody("".into(), true),
                    Token::EndTextBlock,
                    Token::Eol,
                ]
            ),
            (
r#"textblock
endtextblock"#,
                vec![
                    Token::TextBlock(false),
                    Token::Eol,
                    Token::TextBlockBody("".into(), true),
                    Token::EndTextBlock,
                ]
            ),
            (
r#"textblock hoge
hoge
fuga
endtextblock"#,
                vec![
                    Token::TextBlock(false),
                    Token::Identifier("hoge".into()),
                    Token::Eol,
                    Token::TextBlockBody("hoge\nfuga".into(), false),
                    Token::EndTextBlock,
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
                    Token::TextBlock(false),
                    Token::Identifier("hoge".into()),
                    Token::Eol,
                    Token::TextBlockBody("    hoge\n    fuga".into(), false),
                    Token::EndTextBlock,
                    Token::Eol,
                ]
            ),
            (
r#"textblockex foo
bar
baz
endtextblock"#,
                vec![
                    Token::TextBlock(true),
                    Token::Identifier("foo".into()),
                    Token::Eol,
                    Token::TextBlockBody("bar\nbaz".into(), false),
                    Token::EndTextBlock,
                ]
            ),
            (
                "textblockex foo\r\nbar\r\nbaz\r\nendtextblock",
                vec![
                    Token::TextBlock(true),
                    Token::Identifier("foo".into()),
                    Token::Eol,
                    Token::TextBlockBody("bar\r\nbaz".into(), false),
                    Token::EndTextBlock,
                ]
            ),
        ];
        for (input, expected) in test_cases {
            test_next_token(input, expected);
        }

    }

    #[test]
    fn test_call() {
        let test_cases = vec![
            (
                "call hoge.uws",
                vec![
                    Token::Call,
                    Token::Path(None, "hoge.uws".into())
                ],
            ),
            (
                "call c:\\test\\hoge.uws",
                vec![
                    Token::Call,
                    Token::Path(Some("c:\\test".into()), "hoge.uws".into())
                ],
            ),
            (
                "call .\\hoge.uws",
                vec![
                    Token::Call,
                    Token::Path(Some(".".into()), "hoge.uws".into())
                ],
            ),
            (
                "call hoge.uws(1, 2)",
                vec![
                    Token::Call,
                    Token::Path(None, "hoge.uws".into()),
                    Token::Lparen,
                    Token::Num(1.0),
                    Token::Comma,
                    Token::Num(2.0),
                    Token::Rparen
                ],
            ),
        ];
        for (input, expected) in test_cases {
            test_next_token(input, expected);
        }

    }

}