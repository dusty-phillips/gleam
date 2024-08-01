use super::maybe_escape_identifier_doc;
use super::{Output, INDENT};
use crate::ast;
use crate::docvec;
use crate::line_numbers::LineNumbers;
use crate::pretty;
use crate::pretty::{Document, Documentable};
use crate::type_;
use ecow::EcoString;
use itertools::Itertools;

#[derive(Debug, Clone, Copy)]
pub enum Position {
    Tail,
    NotTail,
}

impl Position {
    /// Returns `true` if the position is [`Tail`].
    ///
    /// [`Tail`]: Position::Tail
    #[must_use]
    pub fn is_tail(&self) -> bool {
        matches!(self, Self::Tail)
    }
}

#[derive(Debug)]
pub(crate) struct Generator<'module> {
    module_name: EcoString,
    line_numbers: &'module LineNumbers,
    function_name: Option<EcoString>,
    function_arguments: Vec<Option<&'module EcoString>>,
    current_scope_vars: im::HashMap<EcoString, usize>,
    pub function_position: Position,
    pub scope_position: Position,
    // We track whether tail call recursion is used so that we can render a loop
    // at the top level of the function to use in place of pushing new stack
    // frames.
    pub tail_recursion_used: bool,
}

impl<'module> Generator<'module> {
    #[allow(clippy::too_many_arguments)] // TODO: FIXME
    pub fn new(
        module_name: EcoString,
        line_numbers: &'module LineNumbers,
        function_name: EcoString,
        function_arguments: Vec<Option<&'module EcoString>>,
        mut current_scope_vars: im::HashMap<EcoString, usize>,
    ) -> Self {
        let mut function_name = Some(function_name);
        for &name in function_arguments.iter().flatten() {
            // Initialise the function arguments
            let _ = current_scope_vars.insert(name.clone(), 0);

            // If any of the function arguments shadow the current function then
            // recursion is no longer possible.
            if function_name.as_ref() == Some(name) {
                function_name = None;
            }
        }
        Self {
            module_name,
            line_numbers,
            function_name,
            function_arguments,
            tail_recursion_used: false,
            current_scope_vars,
            function_position: Position::Tail,
            scope_position: Position::Tail,
        }
    }

    pub fn local_var<'a>(&mut self, name: &'a EcoString) -> Document<'a> {
        match self.current_scope_vars.get(name) {
            None => {
                let _ = self.current_scope_vars.insert(name.clone(), 0);
                maybe_escape_identifier_doc(name)
            }
            Some(0) => maybe_escape_identifier_doc(name),
            Some(n) => Document::String(format!("{name}${n}")),
        }
    }

    pub fn function_body<'a>(
        &mut self,
        body: &'a [ast::TypedStatement],
        args: &'a [ast::TypedArg],
    ) -> Output<'a> {
        let body = self.statements(body)?;
        Ok(body)
        // if self.tail_recursion_used {
        //     self.tail_call_loop(body, args)
        // } else {
        //     Ok(body)
        // }
    }

    fn variable<'a>(
        &mut self,
        name: &'a EcoString,
        constructor: &'a type_::ValueConstructor,
    ) -> Output<'a> {
        match &constructor.variant {
            type_::ValueConstructorVariant::ModuleFn { .. }
            | type_::ValueConstructorVariant::ModuleConstant { .. }
            | type_::ValueConstructorVariant::LocalVariable { .. } => Ok(self.local_var(name)),
            _ => todo!(
                "Python doesn't know how to handle variable {:#?} yet",
                constructor
            ),
        }
    }

    pub fn statements<'a>(&mut self, statements: &'a [ast::TypedStatement]) -> Output<'a> {
        let count = statements.len();
        let mut documents = Vec::with_capacity(count * 3);
        for (i, statement) in statements.iter().enumerate() {
            documents.push(self.statement(statement)?);
            documents.push(pretty::line());
        }
        if count == 1 {
            Ok(documents.to_doc())
        } else {
            Ok(documents.to_doc().force_break())
        }
    }

    pub fn statement<'a>(&mut self, statement: &'a ast::TypedStatement) -> Output<'a> {
        match statement {
            ast::Statement::Expression(expression) => self.expression(expression),
            ast::Statement::Assignment(assignment) => todo!("Python assignments not supported yet"),
            ast::Statement::Use(_use) => todo!("Python Use not supported yet"),
        }
    }

    pub fn expression<'a>(&mut self, expression: &'a ast::TypedExpr) -> Output<'a> {
        match expression {
            ast::TypedExpr::String { value, .. } => Ok(string(value)),
            ast::TypedExpr::Call { fun, args, .. } => self.call(fun, args),
            ast::TypedExpr::Var {
                name, constructor, ..
            } => self.variable(name, constructor),
            _ => todo!(
                "Python doesn't support this expression yet {:#?}",
                expression
            ),
        }
    }

    fn call<'a>(
        &mut self,
        fun: &'a ast::TypedExpr,
        arguments: &'a [ast::CallArg<ast::TypedExpr>],
    ) -> Output<'a> {
        let arguments: Vec<Document<'_>> = arguments
            .iter()
            .map(|element| self.expression(&element.value))
            .try_collect()?;

        self.call_with_doc_args(fun, arguments)
    }

    fn call_with_doc_args<'a>(
        &mut self,
        fun: &'a ast::TypedExpr,
        arguments: Vec<Document<'a>>,
    ) -> Output<'a> {
        match fun {
            ast::TypedExpr::Var { name, .. } => {
                let fun_doc = self.expression(fun)?;
                let arguments_doc = call_arguments(arguments.into_iter().map(Ok))?;
                Ok(docvec![fun_doc, arguments_doc])
            }
            _ => todo!("function type not supported in python yet {:#?}", fun),
        }
    }
}

pub fn string(value: &str) -> Document<'_> {
    if value.contains('\n') {
        Document::String(value.replace('\n', r"\n")).surround("\"", "\"")
    } else {
        value.to_doc().surround("\"", "\"")
    }
}

fn call_arguments<'a, Elements: IntoIterator<Item = Output<'a>>>(elements: Elements) -> Output<'a> {
    let elements = Itertools::intersperse(elements.into_iter(), Ok(pretty::break_(",", ", ")))
        .collect::<Result<Vec<_>, _>>()?
        .to_doc();
    if elements.is_empty() {
        return Ok("()".to_doc());
    }
    Ok(docvec![
        "(",
        docvec![pretty::break_("", ""), elements].nest(INDENT),
        pretty::break_(",", ""),
        ")"
    ]
    .group())
}
