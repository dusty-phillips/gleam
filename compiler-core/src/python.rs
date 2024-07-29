mod expression;

use crate::analyse::TargetSupport;
use crate::ast;
use crate::build::Target;
use crate::docvec;
use crate::line_numbers::LineNumbers;
use crate::pretty;
use crate::pretty::{Document, Documentable};
use camino::Utf8Path;
use ecow::EcoString;
use itertools::Itertools;

const INDENT: isize = 2;

pub type Output<'a> = Result<Document<'a>, Error>;

#[derive(Debug)]
pub struct Generator<'a> {
    line_numbers: &'a LineNumbers,
    module: &'a ast::TypedModule,
    module_scope: im::HashMap<EcoString, usize>,
    current_module_name_segments_count: usize,
    target_support: TargetSupport,
}

impl<'a> Generator<'a> {
    pub fn new(
        line_numbers: &'a LineNumbers,
        module: &'a ast::TypedModule,
        target_support: TargetSupport,
    ) -> Self {
        let current_module_name_segments_count = module.name.split('/').count();

        Self {
            current_module_name_segments_count,
            line_numbers,
            module,
            module_scope: Default::default(),
            target_support,
        }
    }

    pub fn compile(&mut self) -> Output<'a> {
        let statements = self
            .module
            .definitions
            .iter()
            .flat_map(|s| self.statement(s));
        let statements: Vec<_> =
            Itertools::intersperse(statements, Ok(pretty::lines(2))).try_collect()?;

        Ok(docvec![statements])
    }

    pub fn statement(&mut self, statement: &'a ast::TypedDefinition) -> Option<Output<'a>> {
        match statement {
            ast::Definition::TypeAlias(ast::TypeAlias { .. }) => None,

            // Handled in collect_imports
            ast::Definition::Import(ast::Import { .. }) => None,

            // Handled in collect_definitions
            ast::Definition::CustomType(ast::CustomType { .. }) => None,

            ast::Definition::ModuleConstant(ast::ModuleConstant {
                publicity,
                name,
                value,
                ..
            }) => None, // TODO: This should be something

            ast::Definition::Function(function) => {
                // If there's an external JavaScript implementation then it will be imported,
                // so we don't need to generate a function definition.
                if function.external_python.is_some() {
                    return None;
                }

                // If the function does not support JavaScript then we don't need to generate
                // a function definition.
                if !function.implementations.supports(Target::Python) {
                    return None;
                }

                self.module_function(function)
            }
        }
    }

    fn module_function(&mut self, function: &'a ast::TypedFunction) -> Option<Output<'a>> {
        let (_, name) = function
            .name
            .as_ref()
            .expect("A module's function must be named");
        let argument_names = function
            .arguments
            .iter()
            .map(|arg| arg.names.get_variable_name())
            .collect();
        let mut generator = expression::Generator::new(
            self.module.name.clone(),
            self.line_numbers,
            name.clone(),
            argument_names,
            self.module_scope.clone(),
        );
        let head = if function.publicity.is_private() {
            // TODO: Should probably prefix private functions with _,
            // but I haven't looked into what that would like like at the call
            // site
            "def "
        } else {
            "def "
        };

        let body = match generator.function_body(&function.body, function.arguments.as_slice()) {
            // No error, let's continue!
            Ok(body) => body,

            // There is an error coming from some expression that is not supported on JavaScript
            // and the target support is not enforced. In this case we do not error, instead
            // returning nothing which will cause no function to be generated.
            Err(error) if error.is_unsupported() && !self.target_support.is_enforced() => {
                return None
            }

            // Some other error case which will be returned to the user.
            Err(error) => return Some(Err(error)),
        };

        let document = docvec![
            head,
            maybe_escape_identifier_doc(name.as_str()),
            fun_args(function.arguments.as_slice(), generator.tail_recursion_used),
            ":",
            docvec![pretty::line(), body].nest(INDENT).group(),
            pretty::line(),
        ];
        Some(Ok(document))
    }
}

pub fn module(
    module: &ast::TypedModule,
    line_numbers: &LineNumbers,
    path: &Utf8Path,
    src: &EcoString,
    target_support: TargetSupport,
) -> Result<String, crate::Error> {
    let document = Generator::new(line_numbers, module, target_support)
        .compile()
        .map_err(|error| crate::Error::Python {
            path: path.to_path_buf(),
            src: src.clone(),
            error,
        })?;
    Ok(document.to_pretty_string(80))
}
fn is_usable_python_identifier(word: &str) -> bool {
    !matches!(
        word,
        // Keywords and reserved words
        // python -c "import keyword ; print(keyword.kwlist)"
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}

fn escape_identifier(word: &str) -> String {
    format!("{word}_")
}

fn maybe_escape_identifier_doc(word: &str) -> Document<'_> {
    if is_usable_python_identifier(word) {
        word.to_doc()
    } else {
        Document::String(escape_identifier(word))
    }
}

fn fun_args(args: &'_ [ast::TypedArg], tail_recursion_used: bool) -> Document<'_> {
    let mut discards = 0;
    wrap_args(args.iter().map(|a| match a.get_variable_name() {
        None => {
            let doc = if discards == 0 {
                "_".to_doc()
            } else {
                Document::String(format!("_{discards}"))
            };
            discards += 1;
            doc
        }
        Some(name) if tail_recursion_used => Document::String(format!("loop${name}")),
        Some(name) => maybe_escape_identifier_doc(name),
    }))
}

fn wrap_args<'a, I>(args: I) -> Document<'a>
where
    I: IntoIterator<Item = Document<'a>>,
{
    pretty::break_("", "")
        .append(pretty::join(args, pretty::break_(",", ", ")))
        .nest(INDENT)
        .append(pretty::break_("", ""))
        .surround("(", ")")
        .group()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Unsupported {
        feature: String,
        location: ast::SrcSpan,
    },
}

impl Error {
    /// Returns `true` if the error is [`Unsupported`].
    ///
    /// [`Unsupported`]: Error::Unsupported
    #[must_use]
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported { .. })
    }
}
