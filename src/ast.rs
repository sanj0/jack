use std::collections::HashMap;

use klex::Loc;

use crate::analyzer::{AstAnalysis, LocalVar, StackElement, Type};

#[derive(Clone, Debug)]
pub struct AstBase {
    pub(crate) nodes: Vec<AstNode>,
    pub file_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstNode {
    pub(crate) inner: AstItem,
    pub(crate) loc: Loc,
    pub(crate) stack: Option<Vec<StackElement>>,
    pub(crate) vars: Option<HashMap<String, LocalVar>>,
}

impl AstNode {
    pub fn new(inner: AstItem, loc: Loc) -> Self {
        Self {
            inner,
            loc,
            stack: None,
            vars: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstItem {
    /// Pushes an int onto the stack
    PushInt(i32),
    /// Pushes a string onto the stack
    PushString(String),
    /// Initializes a new list
    List(Type),
    ListLiteral(Vec<AstNode>),
    If {
        head: Option<Box<AstNode>>,
        body: Box<AstNode>,
        else_body: Option<Box<AstNode>>,
    },
    Switch {
        arms: Vec<(i32, AstNode)>,
        default: Box<AstNode>,
    },
    While {
        head: Option<Box<AstNode>>,
        body: Box<AstNode>,
    },
    For {
        init: Box<AstNode>,
        condition: Box<AstNode>,
        modifier: Box<AstNode>,
        body: Box<AstNode>,
    },
    Block(Vec<AstNode>),
    Store {
        initializer: Option<Box<AstNode>>,
        name: String
    },
    Load(String),
    Jasmin {
        name: String,
        extra_stack: usize,
        input: Vec<MatchInType>,
        output: Vec<MatchOutType>,
        body: String,
    },
    TypeSwitch {
        arms: Vec<(Vec<MatchInType>, Box<AstNode>)>,
        chosen_index: Option<usize>,
    },
    CmpErr(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchInType {
    Any,
    List(Box<MatchInType>),
    Type(Type),
    Generic(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchOutType {
    Type(Type),
    List(Box<MatchOutType>),
    Generic(String),
}

impl AstItem {
    pub fn short_spelling(&self) -> String {
        match self {
            Self::PushInt(i) => format!("push({i})"),
            Self::PushString(s) => format!("push{s:?}"),
            Self::List(t) => format!("list<{t:?}>"),
            Self::ListLiteral(xs) => format!("{:?}", xs.iter().map(|n| n.inner.short_spelling()).collect::<Vec<_>>()),
            Self::If { .. } => "if".into(),
            Self::Switch { .. } => "switch".into(),
            Self::While { .. } => "while".into(),
            Self::For { .. } => "for".into(),
            Self::Block(_) => "block".into(),
            Self::Store { name, .. } => format!("store({name})"),
            Self::Load(s) => s.into(),
            Self::Jasmin { name, .. } => format!("{name}"),
            Self::TypeSwitch { .. } => "typeswitch".into(),
            Self::CmpErr(_) => "cmperr".into(),
        }
    }
}

impl MatchInType {
    pub fn matches_and_capture_generics(&self, ty: &Type, generics: &mut HashMap<String, Type>) -> bool {
        match self {
            Self::Any => true,
            Self::List(xs) => {
                if let Type::List(inner) = ty {
                    xs.matches_and_capture_generics(inner, generics)
                } else {
                    false
                }
            }
            Self::Type(x) => {
                x == ty
            }
            Self::Generic(name) => {
                if let Some(already_captured) = generics.get(name) {
                    already_captured == ty
                } else {
                    generics.insert(name.to_owned(), ty.clone());
                    true
                }
            }
        }
    }
}

impl MatchOutType {
    pub fn try_resolve(&self, generics: &mut HashMap<String, Type>) -> Result<Type, ()> {
        Ok(match self {
            Self::Type(x) => x.clone(),
            Self::Generic(name) => generics.get(name).cloned().ok_or(())?,
            Self::List(xs) => Type::List(Box::new(xs.try_resolve(generics)?))
        })
    }
}
