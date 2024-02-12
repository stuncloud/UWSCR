use parser::lexer::{Lexer, TokenInfo};
use parser::token::Token;

use std::sync::OnceLock;
use std::collections::HashMap;

use tower_lsp::lsp_types::{SemanticToken, SemanticTokensLegend, SemanticTokenType, SemanticTokenModifier};

static SEMANTIC_TOKEN_LEGEND: OnceLock<SemanticTokensLegend> = OnceLock::new();
static SEMANTIC_TOKEN_MAP: OnceLock<HashMap<USemanticTokenType, (u32, u32)>> = OnceLock::new();

#[derive(PartialEq, Eq, Hash)]
enum USemanticTokenType {
    Number,
    String,
    Comment,
    Variable,
    Constant,
    Public,
    Keyword,
    Operator,
    Function,
    FunctionDef,
    Option,
    Property,
    Parameter,
}
impl USemanticTokenType {
    fn as_tuple(&self) -> (u32, u32) {
        let map = SEMANTIC_TOKEN_MAP.get_or_init(|| {
            let legend = SemanticTokenParser::legend();
            let mut map = HashMap::new();
            map.insert(USemanticTokenType::Number, (
                legend.type_offset_of(&SemanticTokenType::NUMBER),
                0
            ));
            map.insert(USemanticTokenType::String, (
                legend.type_offset_of(&SemanticTokenType::STRING),
                0
            ));
            map.insert(USemanticTokenType::Comment, (
                legend.type_offset_of(&SemanticTokenType::COMMENT),
                0
            ));
            map.insert(USemanticTokenType::Variable, (
                legend.type_offset_of(&SemanticTokenType::VARIABLE),
                0
            ));
            map.insert(USemanticTokenType::Constant, (
                legend.type_offset_of(&SemanticTokenType::VARIABLE),
                legend.modifier_offset_of(&SemanticTokenModifier::READONLY)
            ));
            map.insert(USemanticTokenType::Constant, (
                legend.type_offset_of(&SemanticTokenType::VARIABLE),
                legend.modifier_offset_of(&SemanticTokenModifier::STATIC)
            ));
            map.insert(USemanticTokenType::Keyword, (
                legend.type_offset_of(&SemanticTokenType::KEYWORD),
                0
            ));
            map.insert(USemanticTokenType::Operator, (
                legend.type_offset_of(&SemanticTokenType::OPERATOR),
                0
            ));
            map.insert(USemanticTokenType::Function, (
                legend.type_offset_of(&SemanticTokenType::FUNCTION),
                0
            ));
            map.insert(USemanticTokenType::FunctionDef, (
                legend.type_offset_of(&SemanticTokenType::FUNCTION),
                legend.modifier_offset_of(&SemanticTokenModifier::DECLARATION),
            ));
            map.insert(USemanticTokenType::Option, (
                legend.type_offset_of(&SemanticTokenType::VARIABLE),
                legend.modifier_offset_of(&SemanticTokenModifier::STATIC),
            ));
            map.insert(USemanticTokenType::Property, (
                legend.type_offset_of(&SemanticTokenType::PROPERTY),
                0,
            ));
            map.insert(USemanticTokenType::Parameter, (
                legend.type_offset_of(&SemanticTokenType::VARIABLE),
                legend.modifier_offset_of(&SemanticTokenModifier::DECLARATION),
            ));
            map
        });
        map[self]
    }
}

pub struct SemanticTokenParser {
    lexer: Lexer,
    semantic_tokens: Vec<SemanticToken>,
}

impl SemanticTokenParser {
    pub fn legend() -> &'static SemanticTokensLegend {
        let legend = SEMANTIC_TOKEN_LEGEND.get_or_init(|| {
            SemanticTokensLegend {
                token_types: vec![
                    SemanticTokenType::KEYWORD,
                    SemanticTokenType::OPERATOR,
                    SemanticTokenType::NUMBER,
                    SemanticTokenType::STRING,
                    SemanticTokenType::VARIABLE,
                    SemanticTokenType::CLASS,
                    SemanticTokenType::STRUCT,
                    SemanticTokenType::PARAMETER,
                    SemanticTokenType::ENUM,
                    SemanticTokenType::ENUM_MEMBER,
                    SemanticTokenType::FUNCTION,
                    SemanticTokenType::PROPERTY,
                    SemanticTokenType::METHOD,
                    SemanticTokenType::COMMENT,
                ],
                token_modifiers: vec![
                    SemanticTokenModifier::READONLY,
                    SemanticTokenModifier::STATIC,
                    SemanticTokenModifier::DECLARATION,
                ]
            }
        });
        legend
    }
    pub fn new(lexer: Lexer) -> Self {

        Self {
            lexer,
            semantic_tokens: vec![],
        }
    }
    fn next(&mut self) -> TokenInfo {
        self.lexer.next_token()
    }
    fn is_next_token_lparen(&mut self) -> bool {
        self.lexer.nextch_is('(')
    }
    fn info_as_token(&mut self, info: &TokenInfo, t: USemanticTokenType) {
        let token = info.as_semantic_token(t);
        self.semantic_tokens.push(token);
    }
    fn set_token(&mut self, row: usize, start: usize, length: usize, t: USemanticTokenType) {
        let (token_type, token_modifiers_bitset) = t.as_tuple();
        let token = SemanticToken {
            delta_line: row as u32 - 1,
            delta_start: start as u32 - 1,
            length: length as u32,
            token_type,
            token_modifiers_bitset
        };
        self.semantic_tokens.push(token);
    }

    pub fn parse(mut self) -> Vec<SemanticToken> {
        let mut variables = vec![];
        let mut publics = vec![];
        let mut constants = vec![];
        let mut dim_flg = false;
        let mut public_flg = false;
        let mut const_flg = false;
        let mut period_flg = false;
        let mut func_def_flg = false;
        let mut param_flg = false;
        loop {
            let info = self.next();
            match info.token {
                Token::Illegal(_) => {},
                Token::Blank => {},
                Token::Eol => {
                    dim_flg = false;
                    public_flg = false;
                    const_flg = false;
                },
                Token::Eof => break,
                Token::Identifier(ref ident) => {
                    if self.is_next_token_lparen() {
                        if func_def_flg {
                            func_def_flg = false;
                            param_flg = true;
                            self.info_as_token(&info, USemanticTokenType::FunctionDef);
                        } else {
                            self.info_as_token(&info, USemanticTokenType::Function);
                        }
                    } else {
                        if param_flg {
                            self.info_as_token(&info, USemanticTokenType::Parameter);
                        } else if period_flg {
                            period_flg = false;
                            self.info_as_token(&info, USemanticTokenType::Property);
                        } else if dim_flg {
                            self.info_as_token(&info, USemanticTokenType::Variable);
                            variables.push(ident.to_ascii_uppercase());
                        } else if public_flg {
                            self.info_as_token(&info, USemanticTokenType::Public);
                            publics.push(ident.to_ascii_uppercase());
                        } else if const_flg {
                            self.info_as_token(&info, USemanticTokenType::Constant);
                            constants.push(ident.to_ascii_uppercase());
                        } else {
                            if publics.contains(&ident.to_ascii_uppercase()) {
                                self.info_as_token(&info, USemanticTokenType::Public);
                            } else if constants.contains(&ident.to_ascii_uppercase()) {
                                self.info_as_token(&info, USemanticTokenType::Constant);
                            } else {
                                self.info_as_token(&info, USemanticTokenType::Variable);
                            }
                        }
                    }
                },
                Token::Num(_) |
                Token::Hex(_) => self.info_as_token(&info, USemanticTokenType::Number),
                Token::String(_) => self.info_as_token(&info, USemanticTokenType::String),
                Token::ExpandableString(mut s) => {
                    // <#hoge>対応
                    // すでに存在する変数なら色を付ける
                    // "aaaaa<#CR>aaa<#foo>aaaa<#bar>aaaa"
                    // fooのみ存在する場合
                    // "aaaaa → STRING
                    // <# → COMMENT
                    // CR → VARIABLE + READONLY
                    // > → COMMENT
                    // aaa → STRING
                    // <# → COMMENT
                    // foo → VARIABLE
                    // > → COMMENT
                    // aaaa<#bar>aaaa → STRING
                    let row = info.pos.row;
                    let mut start = info.pos.column;
                    while s.len() > 0 {
                        match s.find("<#") {
                            Some(p) => {
                                let d = s.drain(..p).collect::<String>();
                                self.set_token(row, start, d.len(), USemanticTokenType::String);
                                start += p;
                                match s.find(">") {
                                    Some(p2) => {
                                        let d = s.drain(..p2).collect::<String>();
                                        let var = (&d[2..p2-1]).to_ascii_uppercase();
                                        if variables.contains(&var) || ["CR", "TAB", "DBL"].contains(&var.as_str()) {
                                            // <#
                                            self.set_token(row, start, 2, USemanticTokenType::Comment);
                                            start += 2;
                                            // 変数名
                                            match var.as_str() {
                                                "CR" => {
                                                    self.set_token(row, start, 2, USemanticTokenType::Constant);
                                                    start += 2;
                                                },
                                                "TAB" | "DBL" => {
                                                    self.set_token(row, start, 3, USemanticTokenType::Constant);
                                                    start += 3;
                                                },
                                                v => {
                                                    let length = v.len();
                                                    self.set_token(row, start, length, USemanticTokenType::Variable);
                                                    start += length;
                                                }
                                            }
                                            // >
                                            self.set_token(row, start, 1, USemanticTokenType::Comment);
                                            start += 1;
                                        } else {
                                            self.set_token(row, start, p2, USemanticTokenType::String);
                                            start += p2;
                                        }
                                    },
                                    None => {
                                        self.set_token(row, start, s.len(), USemanticTokenType::String);
                                        break;
                                    },
                                }
                            },
                            None => {
                                self.set_token(row, start, s.len(), USemanticTokenType::String);
                                break;
                            },
                        }
                    }
                },
                Token::Bool(_) |
                Token::Null |
                Token::Empty |
                Token::NaN |
                Token::Nothing => self.info_as_token(&info, USemanticTokenType::Constant),
                Token::UObject(json) => {
                    let mut row = info.pos.row;
                    let mut start = info.pos.column;
                    // @
                    self.set_token(row, start, 1, USemanticTokenType::Operator);
                    start += 1;
                    // json
                    for (i, line) in json.lines().enumerate() {
                        row = info.pos.row + i;
                        self.set_token(row, start, line.len(), USemanticTokenType::String);
                        start = 0;
                    }
                    // @
                    self.set_token(row, start, 1, USemanticTokenType::Operator);
                },
                Token::UObjectNotClosing => {},
                Token::Dim => {
                    dim_flg = true;
                },
                Token::Public => {
                    public_flg = true;
                },
                Token::Const => {
                    const_flg = true;
                },
                Token::Print |
                Token::Thread |
                Token::Await |
                Token::Async |
                Token::HashTable |
                Token::Call => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::Uri(uri) => {
                    let row = info.pos.row;
                    let mut start = info.pos.column;
                    // url[
                    self.set_token(row, start, 4, USemanticTokenType::Keyword);
                    start += 4;
                    // uri
                    let length = uri.len();
                    self.set_token(row, start, length, USemanticTokenType::String);
                    start += length;
                    // ]
                    self.set_token(row, start, 1, USemanticTokenType::Keyword);
                },
                Token::Path(_, _) => self.info_as_token(&info, USemanticTokenType::String),
                Token::DefDll => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::Plus |
                Token::Minus |
                Token::Bang |
                Token::Asterisk |
                Token::Slash |
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
                Token::GreaterThanEqual => self.info_as_token(&info, USemanticTokenType::Operator),
                Token::Question => {},
                Token::Colon => {},
                Token::Comma => {},
                Token::Period => {
                    period_flg = true;
                },
                Token::Semicolon => {},
                Token::Lparen => {},
                Token::Rparen => {
                    if param_flg {
                        param_flg = false;
                    }
                },
                Token::Lbrace => {},
                Token::Rbrace => {},
                Token::Lbracket => {},
                Token::Rbracket => {},
                Token::LineContinue => {},
                Token::BackSlash => {},
                Token::ColonBackSlash => {},
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
                Token::Try => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::TextBlock(_) => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::EndTextBlock => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::TextBlockBody(body, comment) => {
                    let length = body.len();
                    let row = info.pos.row;
                    let start = info.pos.column;
                    let t = if comment {USemanticTokenType::Comment} else {USemanticTokenType::String};
                    self.set_token(row, start, length, t);
                },
                Token::Function |
                Token::Procedure => {
                    func_def_flg = true;
                    self.info_as_token(&info, USemanticTokenType::FunctionDef);
                },
                Token::Module |
                Token::Class |
                Token::Enum |
                Token::Struct |
                Token::Hash |
                Token::BlockEnd(_) => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::Option(name, name_pos) => {
                    let row = info.pos.row;
                    let start = info.pos.column;
                    // OPTION
                    self.set_token(row, start, 6, USemanticTokenType::Option);
                    // オプション名
                    self.set_token(row, name_pos, name.len(), USemanticTokenType::Option);
                },
                Token::ComErrIgn |
                Token::ComErrRet => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::ComErrFlg => self.info_as_token(&info, USemanticTokenType::Constant),
                Token::Exit |
                Token::ExitExit => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::Comment => {},
                Token::Ref |
                Token::Variadic => self.info_as_token(&info, USemanticTokenType::Keyword),
                Token::Pipeline => {},
                Token::Arrow => {},
            }
        }
        self.semantic_tokens
    }
}

trait SemanticTokensLegendExt {
    fn type_offset_of(&self, t: &SemanticTokenType) -> u32;
    fn modifier_offset_of(&self, m: &SemanticTokenModifier) -> u32;
}

impl SemanticTokensLegendExt for SemanticTokensLegend {
    fn type_offset_of(&self, t: &SemanticTokenType) -> u32 {
        self.token_types.iter().position(|t0| t0 == t).unwrap_or(0) as u32
    }
    fn modifier_offset_of(&self, m: &SemanticTokenModifier) -> u32 {
        self.token_modifiers.iter().position(|m0| m0 == m).unwrap_or(0) as u32
    }
}

trait TokenInfoExt {
    fn as_semantic_token(&self, t: USemanticTokenType) -> SemanticToken;
}

impl TokenInfoExt for TokenInfo {
    fn as_semantic_token(&self, t: USemanticTokenType) -> SemanticToken {
        let (token_type, token_modifiers_bitset) = t.as_tuple();
        let length = match self.token {
            // Token::String(_) => self.token.len() + 2,
            _ => self.token.len()
        } as u32;
        SemanticToken {
            delta_line: self.pos.row as u32 - 1,
            delta_start: self.pos.column as u32 - 1,
            length,
            token_type,
            token_modifiers_bitset
        }
    }
}