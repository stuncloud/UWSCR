use std::fmt;

#[derive(PartialEq, Clone, Debug)]
pub struct Identifier(pub String);

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Identifier(name) = self;
        write!(f, "{}", name)
    }
}

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
    Array(Vec<Expression>, Box<Expression>), // 配列、配列宣言時の添字
    Literal(Literal),
    Prefix(Prefix, Box<Expression>),
    Infix(Infix, Box<Expression>, Box<Expression>),
    Index(Box<Expression>, Box<Expression>),
    AnonymusFunction {
        params: Vec<Identifier>,
        body: BlockStatement
    },
    AnonymusProcedure {
        params: Vec<Identifier>,
        body: BlockStatement
    },
    FuncCall {
        func: Box<Expression>,
        args: Vec<Expression>,
    },
    Assign(Box<Expression>, Box<Expression>),
    CompoundAssign(Box<Expression>, Box<Expression>, Infix), // += -= *= /=
    Ternary { // ?: 三項演算子
        condition: Box<Expression>,
        consequence: Box<Expression>,
        alternative: Box<Expression>,
    },
    DotCall(Box<Expression>, Box<Expression>), // hoge.fuga hoge.piyo()
}

#[derive(PartialEq, Clone, Debug)]
pub enum Literal {
    // Int(i64),
    Num(f64),
    String(String),
    Bool(bool),
    Array(Vec<Expression>),
    // Path(String),
    Empty,
    Null,
    Nothing,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Statement {
    Dim(Identifier, Expression),
    Public(Identifier, Expression),
    Const(Identifier, Expression),
    HashTbl(Identifier, Option<Expression>),
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
    Continue(u32),
    Break(u32),
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
    Select {
        expression: Expression,
        cases: Vec<(Vec<Expression>, BlockStatement)>,
        default: Option<BlockStatement>
    },
    Function {
        name: Identifier,
        params: Vec<Identifier>,
        body: BlockStatement
    },
    Procedure {
        name: Identifier,
        params: Vec<Identifier>,
        body: BlockStatement
    },
    ModuleFunction {
        module_name: String,
        name: String,
        params: Vec<Identifier>,
        body: BlockStatement
    },
    ModuleProcedure {
        module_name: String,
        name: String,
        params: Vec<Identifier>,
        body: BlockStatement
    },
    Exit,
    Module(Identifier, BlockStatement), // public, private
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
    DotCall,        // hoge.fuga
}