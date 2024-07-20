use std::{io, path::Path};

use oxc::{allocator::Allocator, diagnostics::NamedSource, semantic::Semantic};
use oxc_index::IndexVec;
use oxc_resolver::FileSystem;

pub struct TypeCheck<'fs, Fs>
where
    &'fs Fs: FileSystem,
{
    fs: &'fs Fs,
}

oxc_index::define_index_type! {
  pub struct ModuleId = u32;
}

pub struct TSProgram<'a> {
    pub modules: IndexVec<ModuleId, &'a str>,
    pub semantic: IndexVec<ModuleId, Semantic<'a>>,
}

#[derive(Debug)]
pub enum ParseError {
    IO(io::Error),
    UnknownExtension(oxc::span::UnknownExtension),
    Parser(Vec<oxc::diagnostics::OxcDiagnostic>, NamedSource<String>),
    Semantic(Vec<oxc::diagnostics::OxcDiagnostic>, NamedSource<String>),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::IO(err) => err.fmt(f),
            ParseError::UnknownExtension(err) => err.fmt(f),
            ParseError::Semantic(vec, source) | ParseError::Parser(vec, source) => {
                let reporter = oxc::diagnostics::GraphicalReportHandler::new();
                for err in vec {
                    reporter
                        .render_report(f, err.clone().with_source_code(source.clone()).as_ref())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<oxc::span::UnknownExtension> for ParseError {
    fn from(value: oxc::span::UnknownExtension) -> Self {
        Self::UnknownExtension(value)
    }
}

impl From<io::Error> for ParseError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}

#[derive(Debug)]
pub enum TSProgramError<'a> {
    ParseError(Vec<(&'a str, ParseError)>),
}

impl std::fmt::Display for TSProgramError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TSProgramError::ParseError(err) => {
                f.write_fmt(format_args!("Failed to parse {} files\n\n", err.len()))?;

                for (path, err) in err {
                    let err_type = match err {
                        ParseError::IO(_) => "IO",
                        ParseError::UnknownExtension(_) => "Unknown Extension",
                        ParseError::Parser(_, _) => "parser",
                        ParseError::Semantic(_, _) => "semantic",
                    };
                    f.write_fmt(format_args!(
                        "---------------- {path} ----------------\n  type: {err_type}{err}\n"
                    ))?;
                }

                Ok(())
            }
        }
    }
}

impl std::error::Error for TSProgramError<'_> {}

impl<'fs, Fs> TypeCheck<'fs, Fs>
where
    &'fs Fs: FileSystem,
{
    pub fn new(fs: &'fs Fs) -> Self {
        Self { fs }
    }

    /// # Errors
    ///
    /// Will return `Err` if parsing any of the root files fails
    pub fn create_program<'a>(
        &'a self,
        root_files: &'a [&'a str],
        alloc: &'a Allocator,
    ) -> Result<TSProgram<'a>, TSProgramError<'a>> {
        let mut result = TSProgram {
            modules: IndexVec::with_capacity(root_files.len()),
            semantic: IndexVec::with_capacity(root_files.len()),
        };

        // Parse root files
        {
            let mut parse_err = Vec::new();
            for &path in root_files {
                match self.parse_file(path, alloc) {
                    Ok(semantic) => {
                        result.modules.push(path);
                        result.semantic.push(semantic);
                    }
                    Err(err) => parse_err.push((path, err)),
                }
            }

            if !parse_err.is_empty() {
                return Err(TSProgramError::ParseError(parse_err));
            }
        };

        // Resolve imports and add parsed resolved modules to the result
        // TODO

        Ok(result)
    }

    fn parse_file<'a>(
        &'_ self,
        path: &'_ str,
        alloc: &'a Allocator,
    ) -> Result<Semantic<'a>, ParseError> {
        let source_text = self.fs.read_to_string(Path::new(path))?;
        let source_text = alloc.alloc_str(&source_text);
        let source_type = oxc::span::SourceType::from_path(path)?;
        let parser = oxc::parser::Parser::new(alloc, source_text, source_type);

        let parse_result = parser.parse();
        if parse_result.panicked {
            return Err(ParseError::Parser(
                parse_result.errors,
                NamedSource::new(path, source_text.to_owned()),
            ));
        }

        // TODO: parse_result.errors should be available
        // if !parse_result.errors.is_empty() {
        //   return Err(ParseError::Parser(
        //     parse_result.errors,
        //     NamedSource::new(path, source_text.to_owned()),
        //   ));
        // }

        let program = alloc.alloc(parse_result.program);
        let builder = oxc::semantic::SemanticBuilder::new().with_check_syntax_error(true);
        let semantic_result = builder.build(program);

        // TODO: semantic_result.errors should be available
        // if !semantic_result.errors.is_empty() {
        //   return Err(ParseError::Semantic(
        //     semantic_result.errors,
        //     NamedSource::new(path, source_text.to_owned()),
        //   ));
        // }

        Ok(semantic_result.semantic)
    }
}
