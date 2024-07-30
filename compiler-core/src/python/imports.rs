use itertools::Itertools;

use crate::docvec;
use crate::pretty;
use crate::pretty::{Document, Documentable};
use crate::python::INDENT;

#[derive(Debug, Default)]
pub(crate) struct Imports<'a> {
    imports: Vec<Import<'a>>,
}

impl<'a> Imports<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_module(&mut self, from: String, names: impl IntoIterator<Item = Member<'a>>) {
        let import = Import::new(from, names.into_iter().collect());
        self.imports.push(import);
    }

    pub fn into_doc(self) -> Document<'a> {
        if self.imports.len() == 0 {
            return Document::Line(0);
        }

        docvec![
            pretty::concat(
                self.imports
                    .into_iter()
                    .sorted_by(|a, b| a.from.cmp(&b.from))
                    .map(|import| Import::into_doc(import)),
            ),
            pretty::line(),
            pretty::line(),
        ]
    }
}

#[derive(Debug)]
struct Import<'a> {
    from: String,
    names: Vec<Member<'a>>,
}

impl<'a> Import<'a> {
    fn new(from: String, names: Vec<Member<'a>>) -> Self {
        Self { from, names }
    }

    fn into_doc(self) -> Document<'a> {
        if self.from == "" {
            assert!(
                self.names.len() == 1,
                "Only one name expected if from is blank"
            );
            docvec!["import ", self.names.into_iter().next().unwrap().into_doc()]
        } else if self.names.is_empty() {
            docvec!["import ", Document::String(self.from)]
        } else {
            let members = self.names.into_iter().map(Member::into_doc);
            let members = pretty::join(members, pretty::break_(",", ", "));
            let members = docvec![
                docvec![pretty::break_("", " "), members].nest(INDENT),
                pretty::break_(",", " ")
            ]
            .group();
            docvec![
                "from ",
                Document::String(self.from),
                " import (",
                members,
                ")",
                pretty::line()
            ]
        }
    }
}

#[derive(Debug)]
pub struct Member<'a> {
    pub name: Document<'a>,
    pub alias: Option<Document<'a>>,
}

impl<'a> Member<'a> {
    pub fn new(name: Document<'a>, alias: Option<Document<'a>>) -> Self {
        Self { name, alias }
    }

    fn into_doc(self) -> Document<'a> {
        match self.alias {
            None => self.name,
            Some(alias) => docvec![self.name, " as ", alias],
        }
    }
}
