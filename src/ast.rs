use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub struct Identifier(pub String);

#[derive(PartialEq, Clone, Debug)]
pub enum Prefix {
    Plus,
    Minus,
    Not
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Prefix::Plus => write!(f, "+"),
            Prefix::Minus => write!(f, "-"),
            Prefix::Not => write!(f, "!"),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Infix {
    Plus,
    Minus,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    GreaterThanEqual,
    GreaterThan,
    LessThanEqual,
    LessThan,
    And,
    Or,
    Xor,
    Mod,
    Assign,
}

impl fmt::Display for Infix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Infix::Plus => write!(f, "+"),
            Infix::Minus => write!(f, "-"),
            Infix::Multiply => write!(f, "*"),
            Infix::Divide => write!(f, "/"),
            Infix::Equal => write!(f, "=="),
            Infix::NotEqual => write!(f, "<>"),
            Infix::GreaterThan => write!(f, ">"),
            Infix::GreaterThanEqual => write!(f, ">="),
            Infix::LessThan => write!(f, "<"),
            Infix::LessThanEqual => write!(f, "<="),
            Infix::And => write!(f, "and"),
            Infix::Or => write!(f, "or"),
            Infix::Xor => write!(f, "xor"),
            Infix::Mod => write!(f, "mod"),
            Infix::Assign => write!(f, "="),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    Identifier(Identifier),
    Literal(Literal),
    Prefix(Prefix, Box<Expression>),
    Infix(Infix, Box<Expression>, Box<Expression>),
    Index(Box<Expression>, Box<Expression>),
    HashTbl(Identifier, Box<Option<Expression>>),
    Function {
        params: Vec<Identifier>,
        body: BlockStatement
    },
    FuncCall {
        func: Box<Expression>,
        args: Vec<Expression>,
    },
    Assign(Box<Expression>, Box<Expression>),
    Ternary { // ?: 三項演算子
        condition: Box<Expression>,
        consequence: Box<Expression>,
        alternative: Box<Expression>,
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Literal {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Expression>),
    Hash(Vec<(Expression, Expression)>),
    // Path(String),
    Empty,
    Null,
    Nothing,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Statement {
    Blank,
    Dim(Identifier, Expression),
    DimArray(Identifier, Expression, Vec<Expression>),
    Public(Identifier, Expression),
    Const(Identifier, Expression),
    Result(Expression),
    Print(Expression),
    Call(String),
    DefDll(String),
    Expression(Expression),
    For {
        loopvar: Identifier,
        from: Expression,
        to: Expression,
        step: Option<Expression>,
        block: BlockStatement
    },
    ForIn {
        loopvar: Identifier,
        collection: Expression,
        block: BlockStatement
    },
    While(Expression, BlockStatement),
    Repeat(Expression, BlockStatement),
    IfSingleLine {
        condition: Expression,
        consequence: Box<Statement>,
        alternative: Box<Option<Statement>>
    },
    If {
        condition: Expression,
        consequence: BlockStatement,
        alternative: Option<BlockStatement>
    },
    ElseIf {
        condition: Expression,
        consequence: BlockStatement,
        alternatives: Vec<(Option<Expression>, BlockStatement)>
    },
}

pub type BlockStatement = Vec<Statement>;
pub type Program = BlockStatement;

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Precedence {
    Lowest,
    Ternary,        // ?:
    Or,             // or xor
    And,            // and
    Equality,       // == != <>
    Relational,     // > < >= <=
    Additive,       // + -
    Multiplicative, // * / mod
    Prefix,         // X or !X
    FuncCall,       // myfunc(x)
    Index,          // array[index]
}