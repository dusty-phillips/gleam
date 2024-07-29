use super::*;
use crate::line_numbers::LineNumbers;
use ecow::EcoString;

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

    pub fn function_body<'a>(
        &mut self,
        body: &'a [ast::TypedStatement],
        args: &'a [ast::TypedArg],
    ) -> Output<'a> {
        let body = docvec!["print('hard coded function body')"];
        Ok(body)
        // let body = self.statements(body)?;
        // if self.tail_recursion_used {
        //     self.tail_call_loop(body, args)
        // } else {
        //     Ok(body)
        // }
    }
}
