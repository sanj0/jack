use std::{fmt::Debug, iter::Peekable, num::ParseIntError};

use klex::{KlexError, Loc, RichToken, Token};
use thiserror::Error;

use crate::{
    analyzer::Type,
    ast::{AstBase, AstItem, AstNode, MatchInType, MatchOutType},
};

pub const KW_LIST: &str = "list";
pub const KW_GET: &str = "get";
pub const KW_SET: &str = "set";
pub const KW_PRINT: &str = "print";
pub const KW_IF: &str = "if";
pub const KW_ELSE: &str = "else";
pub const KW_SWITCH: &str = "switch";
pub const KW_TYPE_SWITCH: &str = "typeswitch";
pub const KW_DEFAULT: &str = "default";
pub const KW_WHILE: &str = "while";
pub const KW_DOWHILE: &str = "dowhile";
pub const KW_FOR: &str = "for";
pub const KW_TO_INT: &str = "@int";
pub const KW_TO_STRING: &str = "@string";
pub const KW_TO_CHAR_LIST: &str = "@charlist";
pub const KW_CMP_ERR: &str = "cmperr";

pub const TYPE_NAME_INT: &str = "int";
pub const TYPE_NAME_STRING: &str = "string";
pub const TYPE_NAME_ANY: &str = "any";
pub const TYPE_NAME_LIST: &str = "list";
pub const TYPE_NAME_OBJECT: &str = "object";

#[derive(Error, Debug)]
pub enum ParserErr {
    #[error("error while lexing at {1}: {0}")]
    LexerErr(KlexError, Loc),
    #[error("unexpected EOF: {0}")]
    UnexpectedEOF(String, Loc),
    #[error("illegal start of item: {:?} at {}", .0.inner, .0.loc)]
    IllegalStartOfItem(RichToken),
    #[error("error parsing an int literal at {1}: {0}")]
    IntParseError(ParseIntError, Loc),
    #[error("unknown keyword: '{0}' at {1}")]
    UnknownKeyword(String, Loc),
    #[error("unexpected token: {0}, found {1:?} at {2}")]
    UnexpectedToken(String, Token, Loc),
    #[error("parser error at {1}: {0}")]
    Error(String, Loc),
}

pub fn parse<I>(src: I, file_index: usize) -> Result<AstBase, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let mut base = AstBase {
        nodes: Vec::new(),
        file_index,
    };
    let mut tokens = Tokens {
        inner: src.peekable(),
        loc: Loc::start_of_file(0),
    };

    while tokens.peek_skip_comments()?.is_some() {
        base.nodes.push(next_node(&mut tokens)?);
    }

    Ok(base)
}

/// Wraps the token stream in order to skip comments
#[derive(Clone, Debug)]
struct Tokens<I>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    inner: Peekable<I>,
    loc: Loc,
}

fn next_node<I>(tokens: &mut Tokens<I>) -> Result<AstNode, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let t0 = tokens.next_skip_comments()?;
    let loc = t0.loc;

    let item = match t0.inner {
        Token::Num(ref n) => {
            if n.contains(".") {
                AstItem::PushString(n.to_owned())
            } else {
                AstItem::PushInt(
                    n.parse()
                        .map_err(|e| ParserErr::IntParseError(e, tokens.loc))?,
                )
            }
        }
        Token::Str(s) => AstItem::PushString(s),
        Token::Chr(c) => AstItem::PushInt(c as u32 as i32),
        Token::Sym(ref sym) => parse_symbol(tokens, sym)?,
        Token::Colon => parse_no_init_store(tokens)?,
        Token::LBrace => parse_block(tokens, Token::RBrace)?,
        Token::LBrack => parse_list_lit(tokens)?,
        Token::Dollar => parse_jasmin(tokens)?,
        _ => return Err(ParserErr::IllegalStartOfItem(t0)),
    };
    Ok(AstNode::new(item, loc))
}

fn parse_jasmin<I>(tokens: &mut Tokens<I>) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let name = if let Some(RichToken {
        inner: Token::Str(s),
        ..
    }) = tokens.peek_skip_comments()?
    {
        let s = s.to_owned();
        tokens.next()?;
        s
    } else {
        "jasmin-literal".to_owned()
    };
    expect_token(tokens, Token::LBrace, "expected `{` after `$`")?;
    let extra_stack = if let Some(RichToken {
        inner: Token::Num(n),
        loc,
        ..
    }) = tokens.peek_skip_comments()?
    {
        let n = n
            .parse::<usize>()
            .map_err(|e| ParserErr::IntParseError(e, *loc))?;
        tokens.next()?;
        n
    } else {
        0
    };
    let input = expect_match_in_type_list(tokens, "jasmin input type list")?;
    let mut output = Vec::new();
    expect_token(
        tokens,
        Token::Arrow,
        "expected `->` after jasmin input list",
    )?;
    let loc = expect_token(
        tokens,
        Token::LBrack,
        "expected `[` as start of jasmin output list",
    )?;
    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == Token::RBrack {
            tokens.next()?;
            break;
        } else {
            output.push(parse_match_out_type(tokens)?);
            if let Some(Token::Comma) = tokens.peek_skip_comments()?.map(|t| &t.inner) {
                tokens.next_skip_comments()?;
            }
        }
    }

    let Token::Str(body) = tokens.next_skip_comments()?.inner else {
        return Err(ParserErr::Error("expected double quoted jasmin literal _here_ `${ [...] -> [...] _here_ }`".into(), loc));
    };

    expect_token(
        tokens,
        Token::RBrace,
        "expected `}` to close jasmin literal",
    )?;
    Ok(AstItem::Jasmin {
        name,
        extra_stack,
        input,
        output,
        body,
    })
}

fn parse_list_lit<I>(tokens: &mut Tokens<I>) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let mut items = Vec::new();
    let mut curr_item = Vec::new();
    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == Token::RBrack {
            tokens.next()?;
            items.push(AstNode::new(AstItem::Block(curr_item), tokens.loc));
            break;
        }
        if t.inner == Token::Comma {
            tokens.next()?;
            items.push(AstNode::new(AstItem::Block(curr_item), tokens.loc));
            curr_item = Vec::new();
        }
        curr_item.push(next_node(tokens)?);
    }
    Ok(AstItem::ListLiteral(items))
}

fn parse_no_init_store<I>(tokens: &mut Tokens<I>) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    expect_token(
        tokens,
        Token::Equal,
        "var collect: `... := name`; var init: `name = ...`",
    )?;
    let token = tokens.next_skip_comments()?;
    let Token::Sym(name) = token.inner else {
        return Err(ParserErr::UnexpectedToken("expected variable name after `=`".into(), token.inner, tokens.loc));
    };
    Ok(AstItem::Store {
        initializer: None,
        name,
    })
}

fn parse_type_switch<I>(tokens: &mut Tokens<I>) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    expect_token(
        tokens,
        Token::LBrace,
        "`{` expected after `typeswitch` keyword"
    )?;
    let mut arms = Vec::new();
    loop {
        if let Some(t) = tokens.peek_skip_comments()? {
            if t.inner == Token::RBrace {
                tokens.next()?;
                break;
            }
            let types = expect_match_in_type_list(tokens, "stack pattern for `typeswitch` arm")?;
            expect_token(tokens, Token::Arrow, "`->` expected between stack pattern and body in `typeswitch` arm")?;
            let body = next_node(tokens)?;
            arms.push((types, Box::new(body)));
        } else {
            return Err(ParserErr::UnexpectedEOF("hit EOF while waiting for closing `}` in `typeswitch`".into(), tokens.loc))
        }
    }
    Ok(AstItem::TypeSwitch {
        arms,
        chosen_index: None,
    })
}

fn parse_block<I>(tokens: &mut Tokens<I>, closing: Token) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let mut nodes = Vec::new();

    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == closing {
            tokens.next()?;
            break;
        }
        nodes.push(next_node(tokens)?);
    }
    Ok(AstItem::Block(nodes))
}

fn parse_symbol<I>(tokens: &mut Tokens<I>, sym: &str) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    Ok(match sym {
        KW_LIST => AstItem::List(parse_type_in_angles(tokens)?),
        KW_IF => {
            let head = if matches!(
                tokens.peek_skip_comments(),
                Ok(Some(RichToken {
                    inner: Token::LParen,
                    ..
                }))
            ) {
                let loc = tokens.next()?.loc;
                Some(Box::new(AstNode::new(
                    parse_block(tokens, Token::RParen)?,
                    loc,
                )))
            } else {
                None
            };
            let body = next_node(tokens)?;
            let mut else_body = None;
            if let Some(t) = tokens.peek_skip_comments()? {
                if let Token::Sym(s) = &t.inner {
                    if s == KW_ELSE {
                        tokens.next()?;
                        else_body = Some(Box::new(next_node(tokens)?));
                    }
                }
            }
            AstItem::If {
                head,
                body: Box::new(body),
                else_body,
            }
        }
        KW_SWITCH => {
            expect_token(tokens, Token::LBrace, "expected `{` after `switch`}")?;
            let mut arms = Vec::new();
            let mut default = None;
            loop {
                let t = tokens.peek_skip_comments()?;
                if let Some(t) = t {
                    if t.inner == Token::RBrace {
                        let _ = tokens.next();
                        break;
                    }
                } else {
                    return Err(ParserErr::UnexpectedEOF(
                        "`switch` is missing closing delimiter `}`".into(),
                        tokens.loc,
                    ));
                }
                let label = next_node(tokens)?.inner;
                tokens.peek_skip_comments()?;
                expect_token(
                    tokens,
                    Token::BigArrow,
                    "expected `=>` after `switch`-label value}",
                )?;
                let body = next_node(tokens)?;
                if let AstItem::PushInt(n) = label {
                    arms.push((n, body));
                } else {
                    if matches!(label, AstItem::Load(s) if s == KW_DEFAULT) {
                        if default.is_some() {
                            return Err(ParserErr::Error(
                                "`switch` contains a second `default`-arm".into(),
                                tokens.loc,
                            ));
                        } else {
                            default = Some(body);
                        }
                    } else {
                        return Err(ParserErr::Error(
                            "expected 'char', int literal or `default` as start of `switch`-arm"
                                .into(),
                            tokens.loc,
                        ));
                    }
                }
            }
            if let Some(default) = default {
                AstItem::Switch {
                    arms,
                    default: Box::new(default),
                }
            } else {
                return Err(ParserErr::Error(
                    "`switch` is missing a `default`-arm!".into(),
                    tokens.loc,
                ));
            }
        }
        KW_TYPE_SWITCH => parse_type_switch(tokens)?,
        KW_WHILE | KW_DOWHILE => {
            let head = if matches!(
                tokens.peek_skip_comments(),
                Ok(Some(RichToken {
                    inner: Token::LParen,
                    ..
                }))
            ) {
                let loc = tokens.next()?.loc;
                Some(Box::new(AstNode::new(
                    parse_block(tokens, Token::RParen)?,
                    loc,
                )))
            } else {
                None
            };
            let body = next_node(tokens)?;
            let while_item = AstItem::While {
                head,
                body: Box::new(body),
            };
            if sym == KW_DOWHILE {
                AstItem::Block(vec![
                    AstNode::new(AstItem::PushInt(1), tokens.loc),
                    AstNode::new(while_item, tokens.loc),
                ])
            } else {
                while_item
            }
        }
        KW_FOR => parse_for(tokens)?,
        KW_ELSE => {
            return Err(ParserErr::UnknownKeyword(
                "else without if!".into(),
                tokens.loc,
            ))
        }
        KW_CMP_ERR => {
            if let Token::Str(msg) = tokens.next_skip_comments()?.inner {
                AstItem::CmpErr(msg.to_owned())
            } else {
                return Err(ParserErr::Error("expected string literal after `cmperr`".into(), tokens.loc))
            }
        }
        name => {
            if matches!(
                tokens.peek_skip_comments()?.map(|t| &t.inner),
                Some(Token::Equal)
            ) {
                tokens.next()?;
                let initializer = next_node(tokens)?;
                AstItem::Store {
                    initializer: Some(Box::new(initializer)),
                    name: name.into(),
                }
            } else {
                AstItem::Load(name.into())
            }
        }
    })
}

fn parse_for<I>(tokens: &mut Tokens<I>) -> Result<AstItem, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    expect_token(tokens, Token::LParen, "expected `(` after `for`")?;
    let mut init_nodes = Vec::new();
    let mut cond_nodes = Vec::new();
    let mut mod_nodes = Vec::new();
    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == Token::SemiColon {
            tokens.next()?;
            break;
        }
        init_nodes.push(next_node(tokens)?);
    }
    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == Token::SemiColon {
            tokens.next()?;
            break;
        }
        cond_nodes.push(next_node(tokens)?);
    }
    while let Some(t) = tokens.peek_skip_comments()? {
        if t.inner == Token::SemiColon || t.inner == Token::RParen {
            tokens.next()?;
            break;
        }
        mod_nodes.push(next_node(tokens)?);
    }

    let body = next_node(tokens)?;

    Ok(AstItem::For {
        init: Box::new(AstNode::new(AstItem::Block(init_nodes), tokens.loc)),
        condition: Box::new(AstNode::new(AstItem::Block(cond_nodes), tokens.loc)),
        modifier: Box::new(AstNode::new(AstItem::Block(mod_nodes), tokens.loc)),
        body: Box::new(body),
    })
}

fn expect_token<I>(
    tokens: &mut Tokens<I>,
    expected: Token,
    reason: impl std::fmt::Display,
) -> Result<Loc, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let t = tokens.next_skip_comments()?;
    if expected == t.inner {
        Ok(t.loc)
    } else {
        return Err(ParserErr::UnexpectedToken(
            format!("{}", reason),
            t.inner,
            t.loc,
        ));
    }
}

fn parse_type_in_angles<I>(tokens: &mut Tokens<I>) -> Result<Type, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let less = tokens.next_skip_comments()?;
    let Token::LBrack = less.inner else {
        return Err(ParserErr::UnexpectedToken("expected `[` followed by a type and `]`".into(), less.inner, less.loc));
    };
    let ty = parse_type(tokens)?;
    let greater = tokens.next_skip_comments()?;
    let Token::RBrack = greater.inner else {
        return Err(ParserErr::UnexpectedToken("expected `]` after `[` + type".into(), greater.inner, greater.loc));
    };
    Ok(ty)
}

fn parse_match_in_type_in_angles<I>(tokens: &mut Tokens<I>) -> Result<MatchInType, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let less = tokens.next_skip_comments()?;
    let Token::LBrack = less.inner else {
        return Err(ParserErr::UnexpectedToken("expected `[` followed by a type and `]`".into(), less.inner, less.loc));
    };
    let ty = parse_match_in_type(tokens)?;
    let greater = tokens.next_skip_comments()?;
    let Token::RBrack = greater.inner else {
        return Err(ParserErr::UnexpectedToken("expected `]` after `[` + type".into(), greater.inner, greater.loc));
    };
    Ok(ty)
}

fn parse_match_out_type_in_angles<I>(tokens: &mut Tokens<I>) -> Result<MatchOutType, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let less = tokens.next_skip_comments()?;
    let Token::LBrack = less.inner else {
        return Err(ParserErr::UnexpectedToken("expected `[` followed by a type and `]`".into(), less.inner, less.loc));
    };
    let ty = parse_match_out_type(tokens)?;
    let greater = tokens.next_skip_comments()?;
    let Token::RBrack = greater.inner else {
        return Err(ParserErr::UnexpectedToken("expected `]` after `[` + type".into(), greater.inner, greater.loc));
    };
    Ok(ty)
}

fn parse_type<I>(tokens: &mut Tokens<I>) -> Result<Type, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let type_name = tokens.next_skip_comments()?;
    if let Token::Sym(ref name) = type_name.inner {
        Ok(match name.as_str() {
            TYPE_NAME_INT => Type::Int,
            TYPE_NAME_STRING => Type::String,
            TYPE_NAME_OBJECT => parse_object_after_kw(tokens)?,
            TYPE_NAME_LIST => Type::List(Box::new(parse_type_in_angles(tokens)?)),
            _ => {
                return Err(ParserErr::UnexpectedToken(
                    "not a type!".into(),
                    type_name.inner,
                    type_name.loc,
                ))
            }
        })
    } else {
        Err(ParserErr::UnexpectedToken(
            "expected a type name!".into(),
            type_name.inner,
            type_name.loc,
        ))
    }
}

fn expect_match_in_type_list<I>(tokens: &mut Tokens<I>, reason: &str) -> Result<Vec<MatchInType>, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    expect_token(tokens, Token::LBrack, format!("`[` expected as start of type list: {reason}"))?;
    let mut types = Vec::new();
    loop {
        if let Some(t) = tokens.peek_skip_comments()? {
            if t.inner == Token::RBrack {
                tokens.next()?;
                break;
            } else {
                types.push(parse_match_in_type(tokens)?);
                if let Some(Token::Comma) = tokens.peek_skip_comments()?.map(|t| &t.inner) {
                    tokens.next_skip_comments()?;
                }
            }
        } else {
            return Err(ParserErr::UnexpectedEOF(format!("hit EOF while parsing type list: {reason}"), tokens.loc));
        }
    }
    Ok(types)
}

/// Parses a Type::Object(name) after the object keyword:
/// ("name") -> Type::Object(name)
fn parse_object_after_kw<I>(tokens: &mut Tokens<I>) -> Result<Type, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    expect_token(tokens, Token::LParen, "expected `(name)` where `name` is a string literal after `object` keyword!")?;
    if let Token::Str(name) = tokens.next_skip_comments()?.inner {
        expect_token(tokens, Token::RParen, "missing closing `(` after `object` name")?;
        Ok(Type::Object(name))
    } else {
        return Err(ParserErr::Error("expected a string literal after `object(`".into(), tokens.loc));
    }
}

fn parse_match_in_type<I>(tokens: &mut Tokens<I>) -> Result<MatchInType, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let type_name = tokens.next_skip_comments()?;
    if let Token::Sym(sym) = &type_name.inner {
        Ok(match sym.as_str() {
            TYPE_NAME_INT => MatchInType::Type(Type::Int),
            TYPE_NAME_STRING => MatchInType::Type(Type::String),
            TYPE_NAME_OBJECT => MatchInType::Type(parse_object_after_kw(tokens)?),
            TYPE_NAME_LIST => MatchInType::List(Box::new(parse_match_in_type_in_angles(tokens)?)),
            TYPE_NAME_ANY => MatchInType::Any,
            generic => MatchInType::Generic(generic.to_owned()),
        })
    } else {
        Err(ParserErr::UnexpectedToken(
            "expected a type name!".into(),
            type_name.inner.clone(),
            type_name.loc,
        ))
    }
}

fn parse_match_out_type<I>(tokens: &mut Tokens<I>) -> Result<MatchOutType, ParserErr>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    let type_name = tokens.next_skip_comments()?;
    if let Token::Sym(sym) = &type_name.inner {
        Ok(match sym.as_str() {
            TYPE_NAME_INT => MatchOutType::Type(Type::Int),
            TYPE_NAME_STRING => MatchOutType::Type(Type::String),
            TYPE_NAME_OBJECT => MatchOutType::Type(parse_object_after_kw(tokens)?),
            TYPE_NAME_LIST => MatchOutType::List(Box::new(parse_match_out_type_in_angles(tokens)?)),
            TYPE_NAME_ANY => return Err(ParserErr::UnexpectedToken("type `any` not allowed here!".into(), type_name.inner.clone(), type_name.loc)),
            generic => MatchOutType::Generic(generic.to_owned()),
        })
    } else {
        Err(ParserErr::UnexpectedToken(
            "expected a type name!".into(),
            type_name.inner.clone(),
            type_name.loc,
        ))
    }
}

impl<I> Tokens<I>
where
    I: Iterator<Item = Result<RichToken, KlexError>> + Debug + Clone,
{
    pub fn next(&mut self) -> Result<RichToken, ParserErr> {
        match self.inner.next() {
            Some(Ok(t)) => {
                self.loc = t.loc;
                Ok(t)
            }
            Some(Err(e)) => Err(ParserErr::LexerErr(e, self.loc)),
            None => Err(ParserErr::UnexpectedEOF(
                "expected a token!".into(),
                self.loc,
            )),
        }
    }

    pub fn next_skip_comments(&mut self) -> Result<RichToken, ParserErr> {
        let mut t = self.next()?;
        while matches!(t.inner, Token::Comment(_)) {
            t = self.next()?;
        }
        Ok(t)
    }

    pub fn peek(&mut self) -> Result<Option<&RichToken>, ParserErr> {
        match self.inner.peek() {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(ParserErr::LexerErr(e.clone(), self.loc)),
            None => Ok(None),
        }
    }

    pub fn peek_skip_comments(&mut self) -> Result<Option<&RichToken>, ParserErr> {
        // TODO: is cloning here really necessary??
        let mut t = self.peek()?.map(|x| x.inner.clone());
        while let Some(Token::Comment(_)) = t {
            self.next()?;
            t = self.peek()?.map(|x| x.inner.clone());
        }
        self.peek()
    }
}

impl ParserErr {
    pub fn loc(&self) -> Option<Loc> {
        Some(match self {
            Self::LexerErr(_, loc)
            | Self::UnexpectedEOF(_, loc)
            | Self::IntParseError(_, loc)
            | Self::UnknownKeyword(_, loc)
            | Self::UnexpectedToken(_, _, loc)
            | Self::Error(_, loc) => *loc,
            Self::IllegalStartOfItem(_) => return None,
        })
    }
}
