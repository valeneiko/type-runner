use std::path::Path;

use errors_baseline::ErrorsBaseline;
use types_baseline::TypesBaseline;

mod errors_baseline;
mod line_iter;
pub mod types_baseline;

pub struct Baseline<'a> {
    pub types: TypesBaseline<'a>,
    pub errors: Option<ErrorsBaseline<'a>>,
}

impl<'a> Baseline<'a> {
    pub fn parse(
        types_path: &'_ Path,
        types_data: &'a [u8],
        errors_path: &'_ Path,
        errors_data: Option<&'a [u8]>,
    ) -> Self {
        Self {
            types: TypesBaseline::parse(types_path, types_data),
            errors: errors_data.map(|x| ErrorsBaseline::parse(errors_path, x)),
        }
    }
}
