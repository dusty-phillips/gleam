use crate::analyse::TargetSupport;
use crate::{ast::TypedModule, line_numbers::LineNumbers};
use camino::Utf8Path;
use ecow::EcoString;

pub fn module(
    module: &TypedModule,
    line_numbers: &LineNumbers,
    path: &Utf8Path,
    src: &EcoString,
    target_support: TargetSupport,
) -> Result<String, crate::Error> {
    tracing::debug!(
        "{:?}, {:?}, {:?}, {:?}, {:?}",
        module,
        line_numbers,
        path,
        src,
        target_support
    );
    Ok("NOT PYTHON YET".to_string())
}
