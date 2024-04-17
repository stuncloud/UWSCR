use parser::lexer::{Lexer, TokenInfo};
use parser::token::Token;
use evaluator::builtins::{BuiltinName, BuiltinNameDesc};

use tower_lsp::lsp_types::{SemanticToken, SemanticTokensLegend, SemanticTokenType, SemanticTokenModifier};
use once_cell::sync::Lazy;

#[allow(unused)]
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
impl From<USemanticTokenType> for (u32, u32) {
    fn from(t: USemanticTokenType) -> Self {
        match t {
            USemanticTokenType::Number => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::NUMBER),
                0
            ),
            USemanticTokenType::String => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::STRING),
                0
            ),
            USemanticTokenType::Comment => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::COMMENT),
                0
            ),
            USemanticTokenType::Variable => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::VARIABLE),
                0
            ),
            USemanticTokenType::Constant => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::CONST),
                // SEMANTIC_TOKEN_LEGEND.modifier_offset_of(&[
                //     SemanticTokenModifier::READONLY,
                // ])
                0
            ),
            USemanticTokenType::Public => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::VARIABLE),
                SEMANTIC_TOKEN_LEGEND.modifier_offset_of(&[
                    SemanticTokenModifier::STATIC,
                    SemanticTokenModifier::PUBLIC,
                ])
            ),
            USemanticTokenType::Keyword => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::KEYWORD),
                0
            ),
            USemanticTokenType::Operator => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::OPERATOR),
                0
            ),
            USemanticTokenType::Function => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::FUNCTION),
                0
            ),
            USemanticTokenType::FunctionDef => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::FUNCTION),
                SEMANTIC_TOKEN_LEGEND.modifier_offset_of(&[SemanticTokenModifier::DECLARATION]),
            ),
            USemanticTokenType::Option => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::VARIABLE),
                SEMANTIC_TOKEN_LEGEND.modifier_offset_of(&[SemanticTokenModifier::STATIC]),
            ),
            USemanticTokenType::Property => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::PROPERTY),
                0
            ),
            USemanticTokenType::Parameter => (
                SEMANTIC_TOKEN_LEGEND.type_offset_of(&SemanticTokenType::VARIABLE),
                SEMANTIC_TOKEN_LEGEND.modifier_offset_of(&[SemanticTokenModifier::DECLARATION]),
            ),
        }
    }
}
pub const SEMANTIC_TOKEN_LEGEND: Lazy<SemanticTokensLegend> = Lazy::new(|| {
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
            SemanticTokenType::CONST,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::CONST,
            SemanticTokenModifier::PUBLIC,
        ],
    }
});
trait SemanticTokenTypeExt {
    const CONST: SemanticTokenType;
}
impl SemanticTokenTypeExt for SemanticTokenType {
    const CONST: SemanticTokenType = SemanticTokenType::new("constant");
}
trait SemanticTokenModifierExt {
    const CONST: SemanticTokenModifier;
    const PUBLIC: SemanticTokenModifier;
}
impl SemanticTokenModifierExt for SemanticTokenModifier {
    const CONST: SemanticTokenModifier = SemanticTokenModifier::new("constant");
    const PUBLIC: SemanticTokenModifier = SemanticTokenModifier::new("public");
}
pub struct SemanticTokenParser {
    lexer: Lexer,
    semantic_tokens: Vec<SemanticToken>,
}

impl SemanticTokenParser {
    pub fn legend() -> SemanticTokensLegend {
        SEMANTIC_TOKEN_LEGEND.clone()
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
    // fn is_next_token_lparen(&mut self) -> bool {
    //     self.lexer.nextch_is('(')
    // }
    fn set_token(&mut self, info: &TokenInfo, t: USemanticTokenType) {
        let (token_type, token_modifiers_bitset) = t.into();
        let length = match info.token {
            // Token::String(_) => info.token.len() + 2,
            _ => info.token.len()
        } as u32;
        let token = SemanticToken {
            delta_line: info.pos.row as u32 - 1,
            delta_start: info.pos.column as u32 - 1,
            length,
            token_type,
            token_modifiers_bitset,
        };
        self.semantic_tokens.push(token);
    }

    pub fn parse(mut self, builtins: &Vec<BuiltinName>) -> Vec<SemanticToken> {
        loop {
            let info = self.next();
            match info.token {
                Token::Identifier(ref ident) => {
                    let hoge = builtins.iter()
                        .find(|name| name.name().eq_ignore_ascii_case(ident));
                    if let Some(name) = hoge {
                        match name.desc() {
                            Some(desc) => match desc {
                                BuiltinNameDesc::Function(_) => {
                                    self.set_token(&info, USemanticTokenType::Function);
                                },
                                BuiltinNameDesc::Const(_) => {
                                    self.set_token(&info, USemanticTokenType::Constant);
                                },
                            },
                            None => {}
                        }
                    }
                },
                Token::Eof => break,
                _ => {}
            }
        }
        self.semantic_tokens
    }
}

trait SemanticTokensLegendExt {
    fn type_offset_of(&self, token_type: &SemanticTokenType) -> u32;
    fn modifier_offset_of(&self, modifiers: &[SemanticTokenModifier]) -> u32;
}

impl SemanticTokensLegendExt for SemanticTokensLegend {
    fn type_offset_of(&self, token_type: &SemanticTokenType) -> u32 {
        self.token_types.iter().position(|t0| t0 == token_type).unwrap_or(0) as u32
    }
    fn modifier_offset_of(&self, modifiers: &[SemanticTokenModifier]) -> u32 {
        // self.token_modifiers.iter().position(|m0| m0 == m).unwrap_or(0) as u32
        modifiers.iter()
            .filter_map(|m| self.token_modifiers.iter().position(|_m| _m == m))
            .map(|i| 2 ^ i)
            .reduce(|a, b| a + b)
            .unwrap_or(0) as u32
    }
}

// trait TokenInfoExt {
//     fn as_semantic_token(&self, t: USemanticTokenType) -> SemanticToken;
// }

// impl TokenInfoExt for TokenInfo {
//     fn as_semantic_token(&self, t: USemanticTokenType) -> SemanticToken {
//         let (token_type, token_modifiers_bitset) = t.as_tuple();
//         let length = match self.token {
//             // Token::String(_) => self.token.len() + 2,
//             _ => self.token.len()
//         } as u32;
//         SemanticToken {
//             delta_line: self.pos.row as u32 - 1,
//             delta_start: self.pos.column as u32 - 1,
//             length,
//             token_type,
//             token_modifiers_bitset
//         }
//     }
// }