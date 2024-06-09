use std::{collections::HashMap, fmt::Display};

use klex::Loc;
use thiserror::Error;

use crate::{
    ast::{AstBase, AstItem, AstNode, MatchInType, MatchOutType},
    opcodes, parser,
};

#[derive(Error, Debug)]
pub enum AnalyzerErr {
    #[error("type error: {0} at {1}")]
    TypeErr(String, Loc),
    #[error("compiler error: {0}")]
    CompilerErr(String, Loc),
}

#[derive(Clone, Debug)]
pub struct AstAnalysis {
    pub stack: Vec<StackElement>,
    pub vars: HashMap<String, LocalVar>,
    pub max_stack_size: usize,
    pub max_vars_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalVar {
    pub index: usize,
    pub elem: StackElement,
}

#[derive(Clone, PartialEq)]
pub struct StackElement {
    pub ty: Type,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    String,
    List(Box<Type>),
    Object(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    String(String),
    List(Vec<Option<Value>>),
}

impl AstBase {
    pub fn analyze(&mut self, debug: bool) -> Result<AstAnalysis, AnalyzerErr> {
        let mut analyzer = AstAnalysis::new();
        for node in &mut self.nodes {
            node.analyze(&mut analyzer, debug)?;
        }
        if analyzer.stack.is_empty() {
            Ok(analyzer)
        } else {
            Err(AnalyzerErr::TypeErr(
                format!(
                    "stack is not empty when programm finishes but has {:?}!",
                    analyzer.stack
                ),
                self.nodes
                    .last()
                    .map(|n| n.loc)
                    .unwrap_or(Loc::start_of_file(self.file_index))
                    .into(),
            ))
        }
    }
}

macro_rules! analyse_head {
    ($self:expr, $head:expr, $sub_analysis:expr, $debug:expr) => {
        if let Some(head) = $head {
            // `head` is getting its own `stack` and `vars` here
            head.analyze(&mut $sub_analysis, $debug)?;
            // but we have to update `self.stack` and `self.vars`
            $self.stack = Some($sub_analysis.stack.clone());
            $self.vars = Some($sub_analysis.vars.clone());
        }
    };
}

impl AstNode {
    /// TODO: a lot of const analysis can be done here
    pub fn analyze(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        self.stack = Some(analysis.stack.clone());
        self.vars = Some(analysis.vars.clone());
        match &mut self.inner {
            AstItem::PushInt(n) => analysis.push(Type::Int, Some(Value::Int(*n))),
            AstItem::PushString(s) => analysis.push(Type::String, Some(Value::String(s.clone()))),
            AstItem::List(_) => self.item_list(analysis),
            AstItem::ListLiteral(_) => self.item_list_literal(analysis, debug)?,
            AstItem::If { .. } => self.item_if(analysis, debug)?,
            AstItem::Switch { .. } => self.item_switch(analysis, debug)?,
            AstItem::While { .. } => self.item_while(analysis, debug)?,
            AstItem::For { .. } => self.item_for(analysis, debug)?,
            AstItem::Block(children) => {
                for c in children {
                    c.analyze(analysis, debug)?;
                }
            }
            AstItem::Store { .. } => self.item_store(analysis, debug)?,
            AstItem::Load(_) => self.item_load(analysis)?,
            AstItem::Jasmin { .. } => self.item_jasmin(analysis)?,
            AstItem::TypeSwitch { .. } => self.item_type_switch(analysis)?,
            AstItem::CmpErr(msg) => {
                return Err(AnalyzerErr::CompilerErr(format!("{msg} at {}\n\tstack: {:?}", self.loc, analysis.types().collect::<Vec<_>>()), self.loc))
            }
        }
        if debug {
            println!(
                "stack after `{}` at {}:\n\t{:?}",
                self.inner.short_spelling(),
                self.loc,
                analysis.types().collect::<Vec<_>>()
            );
        }
        Ok(())
    }

    fn item_list(&self, analysis: &mut AstAnalysis) {
        let AstItem::List(ref ty) = self.inner else {
            unreachable!();
        };
        analysis.push(
            Type::List(Box::new(ty.clone())),
            Some(Value::List(Vec::new())),
        );
        analysis.require_additional_stack_size(1);
    }

    fn item_list_literal(
        &mut self,
        analysis: &mut AstAnalysis,
        debug: bool,
    ) -> Result<(), AnalyzerErr> {
        let AstItem::ListLiteral(ref mut nodes) = self.inner else {
            unreachable!();
        };
        if nodes.is_empty() {
            return Err(AnalyzerErr::TypeErr(
                "empty `List` literal has unknown type, use `list<type>`!".into(),
                self.loc,
            ));
        }
        let mut sub_analysis = analysis.clone();
        nodes[0].analyze(&mut sub_analysis, debug)?;
        let ty = sub_analysis
            .expect_any(
                "item 0 doesn't result in anything in list literal",
                self.loc,
            )?
            .ty;
        if analysis.types().ne(sub_analysis.types()) {
            return Err(AnalyzerErr::TypeErr("items in list literal may not alter the stack except pushing their element. (Error in elemtent 0)".into(), self.loc));
        }
        analysis.max_max_values_with(&sub_analysis);

        for (i, node) in nodes.iter_mut().skip(1).enumerate() {
            node.analyze(&mut sub_analysis, debug)?;
            analysis.max_max_values_with(&sub_analysis);
            sub_analysis.expect(
                &ty,
                format!("expected {ty:?} in element {i} of list literal (inferred type {ty:?})"),
                self.loc,
            )?;
            if sub_analysis.types().ne(analysis.types()) {
                return Err(AnalyzerErr::TypeErr("items in list literal may not alter the stack except pushing their element. (Error in elemtent {i})".into(), self.loc));
            }
        }

        analysis.push(Type::List(Box::new(ty)), None);
        // code gen has to know the type of list
        self.stack = Some(analysis.stack.clone());
        // FIXME: this has to be incremented for each further nesting, how come???
        analysis.require_additional_stack_size(4);
        Ok(())
    }

    fn item_if(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        let AstItem::If {head, body, else_body} = &mut self.inner else {
            unreachable!();
        };
        // don't leak local variables into outer scope
        let mut sub_analysis = analysis.clone();
        analyse_head!(self, head, sub_analysis, debug);
        sub_analysis.expect(
            &Type::Int,
            "expected Int (implicit boolean) on stack for `If`-condition",
            self.loc,
        )?;
        analysis.stack = sub_analysis.stack.clone();
        body.analyze(&mut sub_analysis, debug)?;
        if let Some(else_body) = else_body {
            let mut else_body_analysis = analysis.clone();
            else_body.analyze(&mut else_body_analysis, debug)?;
            if sub_analysis.types().ne(else_body_analysis.types()) {
                return Err(AnalyzerErr::TypeErr(format!(
                    "`if` and `else` don't alter the stack the same way:\n\t`if` results in {:?}({})\n\t`else` results in {:?}({})",
                    sub_analysis.types().collect::<Vec<_>>(),
                    sub_analysis.types().len(),
                    else_body_analysis.types().collect::<Vec<_>>(),
                    else_body_analysis.types().len(),
                ), self.loc));
            }
            analysis.max_max_values_with(&else_body_analysis);
        } else {
            if sub_analysis.types().ne(analysis.types()) {
                return Err(AnalyzerErr::TypeErr(
                    format!(
                        "`if` alters the stack but has no `else`\n\tstack before `if`: {:?}({})\n\tstack after `if`-body: {:?}({})",
                        analysis.types().collect::<Vec<_>>(),
                        analysis.types().len(),
                        sub_analysis.types().collect::<Vec<_>>(),
                        sub_analysis.types().len(),
                    ),
                    self.loc,
                ));
            }
        }
        analysis.max_max_values_with(&sub_analysis);
        analysis.stack = sub_analysis.stack;
        analysis.forget_const_values();
        Ok(())
    }

    fn item_switch(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        let AstItem::Switch {arms, default} = &mut self.inner else {
            unreachable!();
        };

        arms.sort_by_key(|(n, _)| *n);
        analysis.expect(&Type::Int, "`switch` requires an `Int` on stack!", self.loc)?;
        let mut sub_analysis = analysis.clone();
        let stack_before_arms = analysis.stack.clone();
        let vars_before_arms = analysis.vars.clone();
        default.analyze(&mut sub_analysis, debug)?;
        let mut expected_stack = sub_analysis.clone();
        for (_, body) in arms {
            sub_analysis.stack = stack_before_arms.clone();
            sub_analysis.vars = vars_before_arms.clone();
            body.analyze(&mut sub_analysis, debug)?;
            if sub_analysis.types().ne(expected_stack.types()) {
                return Err(AnalyzerErr::TypeErr(format!(
                    "`switch`-arms don't alter the stack the same way\n\tdefault branch results in {:?}({})\n\tarm at {} results in {:?}({})",
                    expected_stack.types().collect::<Vec<_>>(),
                    expected_stack.types().len(),
                    body.loc,
                    sub_analysis.types().collect::<Vec<_>>(),
                    sub_analysis.types().len(),
                ), self.loc));
            }
        }
        expected_stack.vars = vars_before_arms;
        *analysis = expected_stack;
        Ok(())
    }

    fn item_while(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        let AstItem::While { head, body } = &mut self.inner else {
            unreachable!();
        };
        // don't leak local variables into outer scope
        let vars = analysis.vars.clone();
        if let Some(head) = head {
            // `head` is getting its own `stack` and `vars` here
            head.analyze(analysis, debug)?;
            // but we have to update `self.stack` and `self.vars`
            self.stack = Some(analysis.stack.clone());
            self.vars = Some(analysis.vars.clone());
        }
        analysis.expect(
            &Type::Int,
            "expected Int (implicit boolean) on stack before `while`-condition",
            self.loc,
        )?;
        let mut expected_types = analysis.types().cloned().collect::<Vec<_>>();
        expected_types.push(Type::Int);
        body.analyze(analysis, debug)?;
        if let Some(head) = head {
            // `head` is getting its own `stack` and `vars` here
            head.analyze(analysis, debug)?;
            // but we have to update `self.stack` and `self.vars`
            self.stack = Some(analysis.stack.clone());
            self.vars = Some(analysis.vars.clone());
        }
        if analysis.types().ne(expected_types.iter()) {
            return Err(AnalyzerErr::TypeErr(
                format!("`while` loop may not alter the stack beyond pushing the condition!\n\tfound (after 1st iteration): {:?}({})",
                    analysis.types().collect::<Vec<_>>(),
                    analysis.types().len(),
                ),
                self.loc,
            ));
        }
        // pop the expected while condition
        analysis.pop();
        analysis.vars = vars;
        analysis.forget_const_values();
        Ok(())
    }

    fn item_for(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        let AstItem::For { init, condition, modifier, body } = &mut self.inner else {
            unreachable!();
        };
        let mut sub_analysis = analysis.clone();
        // don't leak local variables to outer scope
        init.analyze(&mut sub_analysis, debug)?;
        // the expected bool
        analysis.push(Type::Int, None);
        let mut expected_types = analysis.types().cloned().collect::<Vec<_>>();
        macro_rules! type_check {
            ($err_fmt_str:literal) => {
                if sub_analysis.types().ne(expected_types.iter()) {
                    return Err(AnalyzerErr::TypeErr(
                        format!(
                            $err_fmt_str,
                            expected_types.len(),
                            sub_analysis.types().collect::<Vec<_>>(),
                            sub_analysis.types().len()
                        ),
                        condition.loc,
                    ));
                }
            };
        }
        condition.analyze(&mut sub_analysis, debug)?;
        type_check!("`for` condition may only push a single Int\n\texpected {expected_types:?}({})\n\tbut found {:?}({})");
        sub_analysis.pop();
        analysis.stack = sub_analysis.stack.clone();
        expected_types.pop();
        modifier.analyze(&mut sub_analysis, debug)?;
        type_check!("`for` modifier may not alter the stack\n\texpected {expected_types:?}({})\n\tbut found {:?}({})");
        body.analyze(&mut sub_analysis, debug)?;
        type_check!("`for` loop may not alter the stack\n\texpected {expected_types:?}({}) from the before the loop\n\tbut found {:?}({})");
        analysis.max_max_values_with(&sub_analysis);

        Ok(())
    }

    fn item_store(&mut self, analysis: &mut AstAnalysis, debug: bool) -> Result<(), AnalyzerErr> {
        let AstItem::Store { initializer, name } = &mut self.inner else {
            unreachable!();
        };
        if let Some(init) = initializer {
            init.analyze(analysis, debug)?;
            self.stack = Some(analysis.stack.clone());
            self.vars = Some(analysis.vars.clone());
        }
        analysis.max_vars_count += 1;
        let elem = analysis.expect_any(
            format!("stack is empty when `= {name}` is reached"),
            self.loc,
        )?;
        if let Some(var) = analysis.vars.get_mut(name) {
            if elem.ty == var.elem.ty {
                var.elem.value = elem.value;
            } else {
                return Err(AnalyzerErr::TypeErr(
                    format!(
                        "cannot override type {:?} of variable {name} to {:?}",
                        var.elem.ty, elem.ty
                    ),
                    self.loc,
                ));
            }
        } else {
            let index = analysis.vars.len();
            analysis
                .vars
                .insert(name.to_owned(), LocalVar { index, elem });
        }
        self.vars = Some(analysis.vars.clone());
        Ok(())
    }

    fn item_load(&self, analysis: &mut AstAnalysis) -> Result<(), AnalyzerErr> {
        let AstItem::Load(name) = &self.inner else {
            unreachable!();
        };
        if let Some(var) = analysis.vars.get(name) {
            analysis.push(var.elem.ty.clone(), var.elem.value.clone());
            Ok(())
        } else {
            Err(AnalyzerErr::TypeErr(
                format!("unknown variable {name}"),
                self.loc,
            ))
        }
    }

    fn item_jasmin(&self, analysis: &mut AstAnalysis) -> Result<(), AnalyzerErr> {
        let AstItem::Jasmin { input, output, extra_stack, name, .. } = &self.inner else {
            unreachable!();
        };
        analysis.require_additional_stack_size(*extra_stack);
        let mut generics = HashMap::new();
        for t in input.iter().rev() {
            let ty = analysis.expect_any(format!("{name} expected some {t:?} on stack, found nothing\n\tcaptured generics: {generics:?}"), self.loc)?.ty;
            if !t.matches_and_capture_generics(&ty, &mut generics) {
                return Err(AnalyzerErr::TypeErr(format!("{name} expected some {t:?} on stack, {ty:?} doesn't match!\n\tcaptured generics: {generics:?}"), self.loc));
            }
        }
        for t in output {
            let ty = t.try_resolve(&mut generics).map_err(|_| {
                AnalyzerErr::TypeErr(
                    format!(
                        "cannot resolve type {t:?} in {name}\n\tcaptured generics: {generics:?}"
                    ),
                    self.loc,
                )
            })?;
            analysis.push(ty.clone(), None);
        }
        Ok(())
    }

    fn item_type_switch(&mut self, analysis: &mut AstAnalysis) -> Result<(), AnalyzerErr> {
        let AstItem::TypeSwitch { arms, chosen_index } = &mut self.inner else {
            unreachable!();
        };
        let prev_analysis = analysis.clone();
        let mut generics = HashMap::new();
        'arms_loop: for (i, arm) in arms.iter().enumerate() {
            *analysis = prev_analysis.clone();
            for ty in arm.0.iter().rev() {
                if let Ok(stack_ty) = analysis.expect_any("", self.loc).map(|e| e.ty) {
                    if !ty.matches_and_capture_generics(&stack_ty, &mut generics) {
                        continue 'arms_loop;
                    }
                } else {
                    continue 'arms_loop;
                }
            }
            *chosen_index = Some(i);
            break;
        }
        if let Some(index) = chosen_index {
            *analysis = prev_analysis;
            // FIXME: doesn't respect `debug` flag
            arms[*index].1.analyze(analysis, false)?;
            Ok(())
        } else {
            Err(AnalyzerErr::TypeErr(
                format!(
                    "no arm in `typeswitch` matches on {:?}",
                    prev_analysis.types().collect::<Vec<_>>()
                ),
                self.loc,
            ))
        }
    }
}

impl AstAnalysis {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            max_stack_size: 0,
            vars: HashMap::new(),
            max_vars_count: 0,
        }
    }

    /// Used for debug-printing the stack
    pub fn types(&self) -> impl ExactSizeIterator<Item = &Type> + std::fmt::Debug {
        self.stack.iter().map(|e| &e.ty)
    }

    pub fn require_additional_stack_size(&mut self, x: usize) {
        if self.stack.len() + x > self.max_stack_size {
            self.max_stack_size = self.stack.len() + x;
        }
    }

    pub fn push(&mut self, ty: Type, value: Option<Value>) {
        self.stack.push(StackElement { ty, value });
        if self.stack.len() > self.max_stack_size {
            self.max_stack_size = self.stack.len();
        }
    }

    pub fn pop(&mut self) -> Option<StackElement> {
        self.stack.pop()
    }

    pub fn expect_any(
        &mut self,
        reason: impl Display,
        loc: Loc,
    ) -> Result<StackElement, AnalyzerErr> {
        if let Some(t) = self.pop() {
            Ok(t)
        } else {
            Err(AnalyzerErr::TypeErr(format!("{reason}"), loc))
        }
    }

    pub fn expect_list(
        &mut self,
        reason: impl Display,
        loc: Loc,
    ) -> Result<StackElement, AnalyzerErr> {
        if let Some(e) = self.pop() {
            if let Type::List(_) = e.ty {
                Ok(e)
            } else {
                Err(AnalyzerErr::TypeErr(format!("{reason}, found {e:?}!"), loc))
            }
        } else {
            Err(AnalyzerErr::TypeErr(
                format!("{reason}, found an empty stack!"),
                loc,
            ))
        }
    }

    pub fn expect(
        &mut self,
        ty: &Type,
        reason: impl Display,
        loc: Loc,
    ) -> Result<StackElement, AnalyzerErr> {
        if let Some(e) = self.pop() {
            if e.ty == *ty {
                Ok(e)
            } else {
                Err(AnalyzerErr::TypeErr(
                    format!("{reason}, found {:?}!", e.ty),
                    loc,
                ))
            }
        } else {
            Err(AnalyzerErr::TypeErr(
                format!("{reason}, found an empty stack!"),
                loc,
            ))
        }
    }

    pub fn max_max_values_with(&mut self, other: &AstAnalysis) {
        self.max_stack_size = self.max_stack_size.max(other.max_stack_size);
        self.max_vars_count = self.max_vars_count.max(other.max_vars_count);
    }

    pub fn forget_const_values(&mut self) {
        for e in &mut self.stack {
            e.value = None;
        }
    }
}

impl Type {
    pub fn is_number(&self) -> bool {
        match self {
            Self::Int => true,
            Self::String => false,
            Self::List(_) => false,
            Self::Object(_) => false,
        }
    }

    pub fn to_opcode(&self) -> String {
        match self {
            Self::Int => opcodes::TYPE_INT.into(),
            Self::String => opcodes::TYPE_STRING.into(),
            Self::List(_) => opcodes::TYPE_OBJECT.into(),
            Self::Object(name) => format!("L{name};"),
        }
    }
}

impl std::fmt::Debug for StackElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref const_val) = self.value {
            write!(f, "({const_val:?})")
        } else {
            write!(f, "{:?}", self.ty)
        }
    }
}

impl AnalyzerErr {
    pub fn loc(&self) -> Option<Loc> {
        Some(match self {
            Self::TypeErr(_, loc) | Self::CompilerErr(_, loc) => *loc,
        })
    }
}
