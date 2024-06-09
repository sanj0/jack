use std::collections::HashMap;

pub use klex::RichToken;
use klex::{KlexError, Lexer, Loc, Token};

pub const KW_MACRO: &str = "macro";
pub const KW_INCLUDE: &str = "#include";

#[derive(Clone, Debug)]
pub struct Macro {
    args: Vec<String>,
    body: Vec<RichToken>,
}

pub fn lex_and_macronize(
    src: String,
    file_index: usize,
    debug: bool,
) -> Result<(Vec<RichToken>, HashMap<Token, Macro>, String), String> {
    // TODO: this seems very inefficient
    let mut expanded_src = String::new();
    for line in src.lines() {
        if line.starts_with(KW_INCLUDE) {
            if let Some((_, file_name)) = line.split_once(" ") {
                expanded_src
                    .push_str(&std::fs::read_to_string(file_name).expect("cannot read file"));
            }
        } else {
            expanded_src.push_str(line);
            expanded_src.push('\n');
        }
    }
    let src_to_return = expanded_src.clone();

    let tokens = Lexer::new(&expanded_src, file_index)
        .lex()
        .map_err(|e| e.to_string())?;
    let mut token_iter = tokens.into_iter();
    let mut macros = HashMap::new();
    let mut tokens_after_macro_parse = Vec::new();
    while let Some(t0) = token_iter.next() {
        if matches!(&t0.inner, Token::Sym(kw) if kw == KW_MACRO) {
            let Some(key) = token_iter.next().map(|rt| rt.inner) else {
                return Err(format!("expected key token after `{KW_MACRO}`-keyword at {}", t0.loc));
            };
            // TODO: parse args
            let mut body = Vec::new();
            while let Some(t) = token_iter.next() {
                if matches!(t.inner, Token::SemiSemi) {
                    break;
                } else {
                    body.push(t.clone());
                }
            }
            macros.insert(key, Macro::new(Vec::new(), body));
        } else {
            tokens_after_macro_parse.push(t0);
        }
    }
    let mut mod_count = 1;
    let mut depth = 0;
    while mod_count != 0 {
        if depth > 500 {
            return Err("hit macro expansion depth limit!".into());
        }
        depth += 1;
        mod_count = 0;
        tokens_after_macro_parse = tokens_after_macro_parse
            .into_iter()
            .flat_map(|rt| {
                if let Some(m) = macros.get(&rt.inner) {
                    mod_count += 1;
                    // TODO: args
                    m.invoke(Vec::new(), rt.loc).expect("macro invoke error")
                } else {
                    vec![rt]
                }
            })
            .collect();
    }
    if debug {
        println!("// DEBUG INFO: macro expansion depth = {depth}");
    }
    Ok((tokens_after_macro_parse, macros, src_to_return))
}

impl Macro {
    pub fn new(args: Vec<String>, body: Vec<RichToken>) -> Self {
        Self { args, body }
    }

    // might be ineffective
    pub fn invoke(&self, args: Vec<Vec<RichToken>>, loc: Loc) -> Result<Vec<RichToken>, ()> {
        if args.len() != self.args.len() {
            return Err(());
        }
        Ok(self
            .body
            .iter()
            .flat_map(|t| {
                if let Token::Sym(s) = &t.inner {
                    if let Some(i) = self.args.iter().position(|arg| arg == s) {
                        args[i].clone()
                    } else {
                        vec![RichToken::new(t.inner.clone(), loc, t.len)]
                    }
                } else {
                    vec![RichToken::new(t.inner.clone(), loc, t.len)]
                }
            })
            .collect())
    }
}
