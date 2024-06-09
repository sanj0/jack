use std::collections::HashMap;

use klex::Loc;
use thiserror::Error;

use crate::{
    analyzer::{AnalyzerErr, Type, Value},
    ast::{AstBase, AstItem, AstNode},
    opcodes, *,
};

use self::ast::MatchInType;

pub struct ClassWriter {
    /// The name of the source file. "Name" is to be taken literally, meaning this should not
    /// include a path.
    source: String,
    /// The name of this class
    name: String,
    /// The superclass of this class
    extends: String,
    /// Gets written first in the class assembly
    header: String,
    /// Gets written into the initializer, after calling the superclass constructor
    init: String,
    /// The contens of the main method
    /// If this stays empty, no main method will be generated
    main: String,
    /// Gets written last in the class assembly
    footer: String,
}

impl AstBase {
    /// max_stack_size is obtained from the analysis ...
    pub fn code_gen(
        &self,
        class: &mut ClassWriter,
        max_stack_size: usize,
        max_vars_count: usize,
    ) -> Result<(), CodeGenErr> {
        class
            .push_main(opcodes::DIR_STACK_LIMIT)
            .append_main(&max_stack_size.to_string())
            .main_endl();
        class
            .push_main(opcodes::DIR_LOCALS_LIMIT)
            .append_main(&max_vars_count.to_string())
            .main_endl();
        let mut n_vars = 0;
        for node in &self.nodes {
            n_vars = n_vars.max(expect_var_info!(node).len());
            node.code_gen(class)?;
        }
        Ok(())
    }
}

impl AstNode {
    pub fn code_gen(&self, class: &mut ClassWriter) -> Result<(), CodeGenErr> {
        //class.line_directive(self.loc);
        match &self.inner {
            AstItem::PushInt(i) => class.push_int(i),
            AstItem::PushString(s) => class.push_string(s),
            AstItem::List(_) => new_list!(self, class),
            AstItem::ListLiteral(nodes) => {
                let is_int_list = matches!(&expect_type_info!(self).last().unwrap().ty, Type::List(a) if **a == Type::Int);
                new_list!(self, class);
                for node in nodes {
                    dup!(class);
                    node.code_gen(class)?;
                    if is_int_list {
                        int_to_integer!(self, class);
                    }
                    //class.swap();
                    invoke!(
                        generic: self,
                        class,
                        opcodes::INVOKE_VIRTUAL,
                        "java/util/ArrayList/add",
                        1,
                        opcodes::TYPE_BOOL
                    );
                    // drop the boolean returned by `add`
                    class.append_main(opcodes::POP).main_endl();
                }
            }
            AstItem::If {
                head,
                body,
                else_body,
            } => {
                class.append_main(&line_directive!(self)).main_endl();
                if let Some(head) = head {
                    head.code_gen(class)?;
                }
                let body_label = format!("If{}", class.main.len());
                let else_body_label = format!("Else{}", class.main.len());
                let end_if_label = format!("EndIf{}", class.main.len());
                class
                    .push_main(opcodes::IF_NE)
                    .append_main(&body_label)
                    .main_endl();
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&else_body_label)
                    .main_endl();
                class.push_main(&body_label).append_main(":").main_endl();
                body.code_gen(class)?;
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&end_if_label)
                    .main_endl();
                class
                    .push_main(&else_body_label)
                    .append_main(":")
                    .main_endl();
                if let Some(else_body) = else_body {
                    else_body.code_gen(class)?;
                }
                class.push_main(&end_if_label).append_main(":").main_endl();
            }
            AstItem::Switch {
                ref arms,
                ref default,
            } => class.switch(arms, default)?,
            AstItem::While { head, body } => {
                class.append_main(&line_directive!(self)).main_endl();
                let head_label = format!("WhileHead{}", class.main.len());
                let body_label = format!("While{}", class.main.len());
                let end_label = format!("EndWhile{}", class.main.len());
                class.push_main(&head_label).append_main(":").main_endl();
                if let Some(head) = head {
                    head.code_gen(class)?;
                }
                class
                    .push_main(opcodes::IF_NE)
                    .append_main(&body_label)
                    .main_endl();
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&end_label)
                    .main_endl();
                class.push_main(&body_label).append_main(":").main_endl();
                body.code_gen(class)?;
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&head_label)
                    .main_endl();
                class.push_main(&end_label).append_main(":").main_endl();
            }
            AstItem::For {
                init,
                condition,
                modifier,
                body,
            } => {
                class.append_main(&line_directive!(self)).main_endl();
                let end_label = format!("ForEnd{}", class.main.len());
                let body_label = format!("ForBody{}", class.main.len());
                let condition_label = format!("ForCond{}", class.main.len());
                init.code_gen(class)?;
                class
                    .push_main(&condition_label)
                    .append_main(":")
                    .main_endl();
                condition.code_gen(class)?;
                class
                    .push_main(opcodes::IF_NE)
                    .append_main(&body_label)
                    .main_endl();
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&end_label)
                    .main_endl();
                class.push_main(&body_label).append_main(":").main_endl();
                body.code_gen(class)?;
                modifier.code_gen(class)?;
                class
                    .push_main(opcodes::GOTO)
                    .append_main(&condition_label)
                    .main_endl();
                class.push_main(&end_label).append_main(":").main_endl();
            }
            AstItem::Block(children) => {
                for c in children {
                    c.code_gen(class)?;
                }
            }
            AstItem::Store { initializer, name } => {
                if let Some(init) = initializer {
                    init.code_gen(class)?;
                }
                class.append_main(&line_directive!(self)).main_endl();
                let vars = expect_var_info!(self);
                class
                    .push_main(match vars.get(name).unwrap().elem.ty {
                        Type::Int => opcodes::I_STORE,
                        Type::String | Type::List(_) | Type::Object(_) => opcodes::A_STORE,
                    })
                    .append_main(&vars.get(name).unwrap().index.to_string())
                    .main_endl();
            }
            AstItem::Load(name) => {
                class.append_main(&line_directive!(self)).main_endl();
                let vars = expect_var_info!(self);
                class
                    .push_main(match vars.get(name).unwrap().elem.ty {
                        Type::Int => opcodes::I_LOAD,
                        Type::String | Type::List(_) | Type::Object(_) => opcodes::A_LOAD,
                    })
                    .append_main(&vars.get(name).unwrap().index.to_string())
                    .main_endl();
            }
            AstItem::Jasmin { body, .. } => class.jasmin(body),
            AstItem::TypeSwitch { arms, chosen_index } => class.type_switch(arms, chosen_index.as_ref())?,
            AstItem::CmpErr(_) => unreachable!(),
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum CodeGenErr {
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("analyzer error: {0}")]
    AnalyzerErr(#[from] AnalyzerErr),
    #[error("node at {0} has not been analyzed yet!")]
    NotAnalyzedErr(Loc),
}

impl ClassWriter {
    pub fn new(source: String, name: String, extends: String) -> Self {
        Self {
            source,
            name,
            extends,
            header: String::new(),
            init: String::new(),
            main: String::new(),
            footer: String::new(),
        }
    }

    pub fn write(&self) -> String {
        format!(
            class_template!(),
            source = self.source,
            name = self.name,
            extends = self.extends,
            header = self.header,
            init = self.init,
            main = self.main,
            footer = self.footer
        )
    }

    pub fn push_stmt(&mut self, statement: &[&str]) {
        for (i, part) in statement.iter().enumerate() {
            self.main.push_str(part);
            self.main
                .push(if i == statement.len() - 1 { '\n' } else { ' ' });
        }
    }

    pub fn push_main(&mut self, s: &str) -> &mut Self {
        self.main.push_str(s);
        self.main.push_str(" ");
        self
    }

    /// Doesn't add a space after
    pub fn append_main(&mut self, s: &str) -> &mut Self {
        self.main.push_str(s);
        self
    }

    pub fn main_endl(&mut self) {
        self.main.push('\n');
    }

    pub fn line_directive(&mut self, loc: Loc) {
        self.push_stmt(&[opcodes::DIR_LINE, &loc.row.to_string()])
    }

    pub fn push_int(&mut self, n: &i32) {
        if (-1..=5).contains(n) {
            self.push_stmt(&[match n {
                -1 => opcodes::ICONST_M1,
                0 => opcodes::ICONST_0,
                1 => opcodes::ICONST_1,
                2 => opcodes::ICONST_2,
                3 => opcodes::ICONST_3,
                4 => opcodes::ICONST_4,
                5 => opcodes::ICONST_5,
                _ => unreachable!(),
            }]);
        } else {
            self.push_stmt(&[
                match n {
                    -128..=127 => opcodes::BIPUSH,
                    -32768..=32767 => opcodes::SIPUSH,
                    _ => opcodes::LDC,
                },
                &n.to_string(),
            ]);
        }
    }

    pub fn push_string(&mut self, s: &str) {
        self.push_stmt(&[opcodes::LDC, &format!("{:?}", s)]);
    }

    pub fn dup(&mut self) {
        self.append_main(opcodes::DUP).main_endl();
    }

    pub fn dupx1(&mut self) {
        self.append_main(opcodes::DUPX1).main_endl();
    }

    pub fn swap(&mut self) {
        self.append_main(opcodes::SWAP).main_endl();
    }

    pub fn new_list(&mut self, node: &AstNode) {
        self.push_main(opcodes::NEW)
            .append_main(opcodes::CLASS_ARRAY_LIST)
            .main_endl();
        self.dup();
        invoke!(
            node,
            self,
            opcodes::INVOKE_SPECIAL,
            "java/util/ArrayList/<init>",
            0,
            opcodes::TYPE_VOID
        );
    }

    pub fn set(&mut self, node: &AstNode) {
        let e = node.stack.as_ref().unwrap().last().unwrap();
        if e.ty == Type::Int {
            int_to_integer!(node, self);
        }
        invoke!(
            node,
            self,
            opcodes::INVOKE_VIRTUAL,
            "java/util/ArrayList/set",
            types: [opcodes::TYPE_INT, opcodes::TYPE_OBJECT],
            opcodes::TYPE_OBJECT
        );
        // pop off the redundant bool
        self.append_main(opcodes::POP).main_endl();
    }

    pub fn to_char_list(&mut self, node: &AstNode) {
        self.push_main(opcodes::NEW)
            .append_main(opcodes::CLASS_ARRAY_LIST)
            .main_endl();
        self.dupx1();
        self.swap();
        invoke!(
            node,
            self,
            opcodes::INVOKE_VIRTUAL,
            "java/lang/String/codePoints",
            0,
            "Ljava/util/stream/IntStream;"
        );
        self.append_main(
            "invokeinterface java/util/stream/IntStream/boxed()Ljava/util/stream/Stream; 1",
        )
        .main_endl();
        invoke!(
            node,
            self,
            opcodes::INVOKE_STATIC,
            "java/util/stream/Collectors/toList",
            0,
            "Ljava/util/stream/Collector;"
        );
        self.append_main("invokeinterface java/util/stream/Stream/collect(Ljava/util/stream/Collector;)Ljava/lang/Object; 2").main_endl();
        self.push_main(opcodes::CHECK_CAST)
            .append_main("java/util/Collection")
            .main_endl();
        invoke!(
            node,
            self,
            opcodes::INVOKE_SPECIAL,
            "java/util/ArrayList/<init>",
            types: ["Ljava/util/Collection;"],
            opcodes::TYPE_VOID
        );
    }

    pub fn switch(&mut self, arms: &[(i32, AstNode)], default: &AstNode) -> Result<(), CodeGenErr> {
        self.push_stmt(&[opcodes::LOOKUP_SWITCH]);
        let label = format!("Switch{}", self.main.len());
        let labels = arms
            .iter()
            .map(|(n, _)| (*n, format!("{label}{n}")))
            .collect::<HashMap<i32, String>>();
        let default_label = &format!("{label}default");
        let end_label = &format!("End{label}");
        for (n, _) in arms.iter() {
            self.push_stmt(&[&n.to_string(), ":", &labels[n]]);
        }
        self.push_stmt(&[opcodes::DEFAULT, ":", default_label]);
        for (n, body) in arms {
            self.push_stmt(&[&labels[n], ":"]);
            body.code_gen(self)?;
            self.push_stmt(&[opcodes::GOTO, end_label]);
        }
        self.push_stmt(&[default_label, ":"]);
        default.code_gen(self)?;
        self.push_stmt(&[end_label, ":"]);
        Ok(())
    }

    pub fn jasmin(&mut self, code: &String) {
        self.push_main(code).main_endl();
    }

    pub fn type_switch(&mut self, arms: &[(Vec<MatchInType>, Box<AstNode>)], chosen_index: Option<&usize>) -> Result<(), CodeGenErr>{
        if let Some(index) = chosen_index {
            arms[*index].1.code_gen(self)
        } else {
            panic!("fatal: `typeswitch` has not been analyzed!");
        }
    }
}
