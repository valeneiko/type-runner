use core::str;
use std::{fmt::Write, ops::Add, path::Path};

use memchr::memchr;
use oxc::syntax::identifier::is_identifier_part;
use oxc_index::IndexVec;

use super::line_iter::LineIter;

oxc_index::define_index_type! {
  pub struct LineId = u16;
}

oxc_index::define_index_type! {
  pub struct BaselineFileId = u8;
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct TypesBaseline<'a> {
    pub names: IndexVec<BaselineFileId, &'a str>,
    pub files: IndexVec<BaselineFileId, TypeBaselineFile<'a>>,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct TypeBaselineFile<'a> {
    pub statements: IndexVec<LineId, &'a str>,
    pub assertions: IndexVec<LineId, Vec<Assertion<'a>>>,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Assertion<'a> {
    pub expr: &'a str,
    pub expected_type: &'a str,
}

impl<'a> TypesBaseline<'a> {
    /// # Panics
    pub fn parse(path: &'_ Path, data: &'a [u8]) -> Self {
        let mut result = Self::default();
        let mut iter = LineIter::new(data);

        {
            let line = iter.next();
            assert!(
                line.is_some()
                    && line.unwrap().2.starts_with(b"//// [")
                    && line.unwrap().2.ends_with(b"] ////"),
                "Expected baseline to start with test unit path\n  path: {}\n  line: {}",
                path.display(),
                str::from_utf8(line.unwrap().2).unwrap_or_default().escape_debug()
            );
        }

        let mut expr_start: Option<usize> = None;
        let mut expr_end = None;

        while let Some((_line_idx, line_start, line)) = iter.next() {
            if line.is_empty() {
                if expr_end.is_some() {
                    expr_end = Some(line_start);
                }
                continue;
            }

            if line.starts_with(b"=== ") {
                assert!(
                    line.ends_with(b" ==="),
                    "Expected filename header\n  path: {}\n  line: {}",
                    path.display(),
                    str::from_utf8(line).unwrap_or_default().escape_debug()
                );

                if let Some(expr_start) = expr_start {
                    if expr_start < line_start {
                        if let Some(expr_end) = expr_end {
                            let expr = {
                                assert!(
                                    expr_start <= expr_end,
                                    "expression bounds reversed:\n  path: {}\n  bounds: [{}, {})\n  data: {}\n      : {}\n      : {}\n  line: {}",
                                    path.display(),
                                    expr_start,
                                    expr_end,
                                    str::from_utf8(
                                        &data[expr_start.saturating_add_signed(-32)
                                            ..expr_start.add(30).clamp(0, data.len())]
                                    )
                                    .unwrap_or_default()
                                    .escape_debug(),
                                    data.iter()
                                        .enumerate()
                                        .skip(expr_start.saturating_add_signed(-32))
                                        .take(62)
                                        .fold(String::new(), |mut output, (i, &c)| {
                                            let _ = write!(
                                                output,
                                                "{}{}",
                                                if c == b'\n' { " " } else { "" },
                                                (i % 10)
                                            );
                                            output
                                        }),
                                    data.iter()
                                        .enumerate()
                                        .skip(expr_start.saturating_add_signed(-32))
                                        .take(62)
                                        .fold(String::new(), |mut output, (i, &c)| {
                                            let _ = write!(
                                                output,
                                                "{}{}",
                                                if c == b'\n' { " " } else { "" },
                                                if i == expr_start {
                                                    "["
                                                } else if i == expr_end {
                                                    ")"
                                                } else {
                                                    " "
                                                }
                                            );
                                            output
                                        }),
                                    str::from_utf8(line).unwrap_or_default().escape_debug(),
                                );
                                let expr = &data[expr_start..expr_end];
                                str::from_utf8(expr).expect("Expression to be UTF8")
                            };
                            let baseline = result.files.last_mut().unwrap();
                            baseline.statements.push(expr);
                            baseline.assertions.push(Vec::new());
                        }
                    }
                }

                let name = &line[4..line.len() - 4];
                result.names.push(str::from_utf8(name).unwrap());
                result.files.push(TypeBaselineFile::default());

                // println!("New file: {} @ {}", result.names.last().unwrap(), iter.line_start);
                expr_start = Some(iter.line_start);
            }

            // Keep reading multi-line expression
            if !line.starts_with(b">")
                || line.len() < 2
                || (!is_identifier_part(line[1] as char) && line[1] != b'\'')
            {
                expr_end = Some(line_start + line.len());
                continue;
            }

            // Add assertion
            let Some(baseline) = result.files.last_mut() else {
                panic!(
                    "Expected baseline file to exist\n  path: {}\n  line: {}",
                    path.display(),
                    str::from_utf8(line).unwrap_or_default().escape_debug()
                );
            };

            let expr = {
                // let Some(expr_end) = expr_end else {
                //   panic!(
                //     "Expected expr_end to be set:\n  path: {}\n  file: {}\n  line: {}\n  expr: {}",
                //     path.display(),
                //     result.names.last().unwrap(),
                //     str::from_utf8(line).unwrap_or_default().escape_debug(),
                //     str::from_utf8(&data[expr_start.unwrap()..line_start])
                //       .unwrap_or_default()
                //       .escape_debug(),
                //   );
                // };
                let expr = &data[expr_start.expect("expr_start to exist")
                    ..expr_end.unwrap_or(expr_start.unwrap())];
                str::from_utf8(expr).expect("Expression to be UTF8")
            };
            expr_end = None;
            baseline.statements.push(expr);
            baseline.assertions.push(Vec::new());

            let mut line = line;
            loop {
                let (line_idx, line_start, underline) =
                    iter.next().expect("assertion should be followed by underline");

                let has_underline = underline.starts_with(b"> ");
                let Some(delim) = memchr(b':', if has_underline { underline } else { line }) else {
                    panic!(
                        "assertion should contain delimiter:\n  path: {}\n  name:{}\n  line: {}\n  underline: {}",
                        path.display(),
                        result.names.last().unwrap(),
                        str::from_utf8(line).unwrap_or_default().escape_debug(),
                        str::from_utf8(underline).unwrap_or_default().escape_debug()
                    );
                };

                assert!(
                    delim <= line.len(),
                    "delimiter should be in bounds\n  path: {}\n line:      {}\n  underline: {}",
                    path.display(),
                    str::from_utf8(line).unwrap_or_default().escape_debug(),
                    str::from_utf8(underline).unwrap_or_default().escape_debug()
                );

                let (expr, expected_type) = {
                    let offset = 1 + str::from_utf8(&line[1..])
                        .expect("line to be UTF8")
                        .char_indices()
                        .scan(1usize, |acc, (offset, ch)| {
                            if *acc >= delim {
                                None
                            } else {
                                *acc += ch.len_utf16();
                                Some(offset)
                            }
                        })
                        .last()
                        .expect("Delimitier to be within line bounds");

                    let assertion = match str::from_utf8(&line[1..offset]) {
                        Ok(assertion) => assertion,
                        Err(err) => panic!(
                            "Expected assertion to be UTF8:\n  path: {}\n  idx: delim={}, offset={}, valid={}, error_len={:?}\n  line: {}\n      : >{}",
                            path.display(),
                            delim,
                            offset,
                            err.valid_up_to(),
                            err.error_len(),
                            str::from_utf8(line).unwrap_or_default().escape_debug(),
                            String::from_utf8_lossy(&line[1..offset]).escape_debug(),
                        ),
                    };

                    (
                        assertion,
                        str::from_utf8(&line[offset + 3..]).expect("expected type to be UTF8"),
                    )
                };
                baseline.assertions.last_mut().unwrap().push(Assertion { expr, expected_type });

                let (_line_idx, line_start, next_line) = if has_underline {
                    iter.next().expect("assertion block should be followed by new line")
                } else {
                    (line_idx, line_start, underline)
                };
                if next_line.starts_with(b">") {
                    line = next_line;
                    continue;
                }

                expr_start = Some(if next_line.is_empty() { iter.line_start } else { line_start });
                break;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use oxc_index::index_vec;

    use super::*;

    #[test]
    fn single_file() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
// type parameter type is not a valid operand of addition operator
enum E { a, b }
>E : E
>  : ^
>a : E.a
>  : ^^^
>b : E.b
>  : ^^^

function foo<T, U>(t: T, u: U) {
>foo : <T, U>(t: T, u: U) => void
>    : ^ ^^ ^^ ^^ ^^ ^^ ^^^^^^^^^
>t : T
>  : ^
>u : U
>  : ^
";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts"],
                files: index_vec![TypeBaselineFile {
                    statements: index_vec![
                        "// type parameter type is not a valid operand of addition operator\nenum E { a, b }",
                        "function foo<T, U>(t: T, u: U) {"
                    ],
                    assertions: index_vec![
                        vec![
                            Assertion { expr: "E", expected_type: "E" },
                            Assertion { expr: "a", expected_type: "E.a" },
                            Assertion { expr: "b", expected_type: "E.b" },
                        ],
                        vec![
                            Assertion { expr: "foo", expected_type: "<T, U>(t: T, u: U) => void" },
                            Assertion { expr: "t", expected_type: "T" },
                            Assertion { expr: "u", expected_type: "U" },
                        ]
                    ]
                }]
            }
        );
    }

    #[test]
    fn multiple_files() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^

=== b.ts ===
const b = 123;
>b : number
>  : ^^^^^^
>123 : 123
>    : ^^^

";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile {
                        statements: index_vec!["const a = 5;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "a", expected_type: "number" },
                            Assertion { expr: "5", expected_type: "5" },
                        ],]
                    },
                    TypeBaselineFile {
                        statements: index_vec!["const b = 123;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "b", expected_type: "number" },
                            Assertion { expr: "123", expected_type: "123" },
                        ],]
                    }
                ]
            }
        );
    }

    #[test]
    fn scoped_assertions() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^

=== b.ts ===
const b = 123;
>b : number
>  : ^^^^^^
>123 : 123
>    : ^^^

";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile {
                        statements: index_vec!["const a = 5;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "a", expected_type: "number" },
                            Assertion { expr: "5", expected_type: "5" },
                        ],]
                    },
                    TypeBaselineFile {
                        statements: index_vec!["const b = 123;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "b", expected_type: "number" },
                            Assertion { expr: "123", expected_type: "123" },
                        ],]
                    }
                ]
            }
        );
    }

    #[test]
    fn empty_fie() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===

=== b.ts ===
const b = 123;
>b : number
>  : ^^^^^^
>123 : 123
>    : ^^^

";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile { statements: index_vec![""], assertions: index_vec![vec![]] },
                    TypeBaselineFile {
                        statements: index_vec!["const b = 123;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "b", expected_type: "number" },
                            Assertion { expr: "123", expected_type: "123" },
                        ]]
                    }
                ]
            }
        );
    }

    #[test]
    fn end_of_scope_on_last_line() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br#"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
declare module 'demoModule' {
>'demoModule' : typeof import("demoModule")
>             : ^^^^^^^^^^^^^^^^^^^^^^^^^^^

    export = alias;
>alias : typeof alias
>      : ^^^^^^^^^^^^
}
=== b.ts ===
const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^
"#;
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile {
                        statements: index_vec![
                            "declare module 'demoModule' {",
                            "    export = alias;"
                        ],
                        assertions: index_vec![
                            vec![Assertion {
                                expr: "'demoModule'",
                                expected_type: r#"typeof import("demoModule")"#
                            },],
                            vec![Assertion { expr: "alias", expected_type: "typeof alias" },]
                        ]
                    },
                    TypeBaselineFile {
                        statements: index_vec!["const a = 5;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "a", expected_type: "number" },
                            Assertion { expr: "5", expected_type: "5" },
                        ]]
                    }
                ]
            }
        );
    }

    #[test]
    fn assertion_without_underline() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br#"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
class C {
>C : C
>  : ^

    public x;
>x : any

    public a = '';
>a : string
>  : ^^^^^^
>'' : ""
}

const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^
"#;
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts"],
                files: index_vec![TypeBaselineFile {
                    statements: index_vec![
                        "class C {",
                        "    public x;",
                        "    public a = '';",
                        "}\n\nconst a = 5;"
                    ],
                    assertions: index_vec![
                        vec![Assertion { expr: "C", expected_type: "C" }],
                        vec![Assertion { expr: "x", expected_type: "any" }],
                        vec![
                            Assertion { expr: "a", expected_type: "string" },
                            Assertion { expr: r"''", expected_type: r#""""# },
                        ],
                        vec![
                            Assertion { expr: "a", expected_type: "number" },
                            Assertion { expr: "5", expected_type: "5" },
                        ]
                    ]
                }]
            }
        );
    }

    #[test]
    fn middle_assertion_without_underline() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
g.prototype.m = function () {
>g.prototype.m = function () {  this;} : () => void
>                                      : ^^^^^^^^^^
>g.prototype.m : any
>g.prototype : any
>            : ^^^
>g : () => void
>  : ^^^^^^^^^^
>prototype : any
>          : ^^^
>m : any
>  : ^^^
>function () {  this;} : () => void
>                      : ^^^^^^^^^^

  this;
>this : any

};
";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts"],
                files: index_vec![TypeBaselineFile {
                    statements: index_vec!["g.prototype.m = function () {", "  this;"],
                    assertions: index_vec![
                        vec![
                            Assertion {
                                expr: "g.prototype.m = function () {  this;}",
                                expected_type: "() => void"
                            },
                            Assertion { expr: "g.prototype.m", expected_type: "any" },
                            Assertion { expr: "g.prototype", expected_type: "any" },
                            Assertion { expr: "g", expected_type: "() => void" },
                            Assertion { expr: "prototype", expected_type: "any" },
                            Assertion { expr: "m", expected_type: "any" },
                            Assertion {
                                expr: "function () {  this;}",
                                expected_type: "() => void"
                            }
                        ],
                        vec![Assertion { expr: "this", expected_type: "any" },]
                    ]
                }]
            }
        );
    }

    #[test]
    fn comment_on_last_line() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^

// Separate file
=== b.ts ===
const b = 123;
>b : number
>  : ^^^^^^
>123 : 123
>    : ^^^

";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile {
                        statements: index_vec!["const a = 5;", "// Separate file",],
                        assertions: index_vec![
                            vec![
                                Assertion { expr: "a", expected_type: "number" },
                                Assertion { expr: "5", expected_type: "5" },
                            ],
                            vec![]
                        ]
                    },
                    TypeBaselineFile {
                        statements: index_vec!["const b = 123;"],
                        assertions: index_vec![vec![
                            Assertion { expr: "b", expected_type: "number" },
                            Assertion { expr: "123", expected_type: "123" },
                        ]]
                    }
                ]
            }
        );
    }

    #[test]
    fn code_line_starts_with_gt() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = br"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
type GenericStructure<
>GenericStructure : GenericStructure<AcceptableKeyType>
>                 : ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

  AcceptableKeyType extends string = string
> = Record<AcceptableKeyType, number>;

const a = 5;
>a : number
>  : ^^^^^^
>5 : 5
>  : ^

=== b.ts ===
    any
>
    ? { children?: React.ReactNode }
>children : React.ReactNode
>         : ^^^^^^^^^^^^^^^
>React : any
>      : ^^^
";
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts", "b.ts"],
                files: index_vec![
                    TypeBaselineFile {
                        statements: index_vec![
                            "type GenericStructure<",
                            "  AcceptableKeyType extends string = string\n> = Record<AcceptableKeyType, number>;\n\nconst a = 5;"
                        ],
                        assertions: index_vec![
                            vec![Assertion {
                                expr: "GenericStructure",
                                expected_type: "GenericStructure<AcceptableKeyType>"
                            },],
                            vec![
                                Assertion { expr: "a", expected_type: "number" },
                                Assertion { expr: "5", expected_type: "5" },
                            ]
                        ]
                    },
                    TypeBaselineFile {
                        statements: index_vec!["    any\n>\n    ? { children?: React.ReactNode }"],
                        assertions: index_vec![vec![
                            Assertion { expr: "children", expected_type: "React.ReactNode" },
                            Assertion { expr: "React", expected_type: "any" },
                        ]]
                    }
                ]
            }
        );
    }

    #[test]
    fn non_ascii() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.types").unwrap();
        let data = r#"//// [tests/cases/compiler/unit1.ts] ////

=== a.ts ===
module 才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123 {
>才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123 : typeof globalThis.才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123
>                                                                          : ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

const 𝓱𝓮𝓵𝓵𝓸 = "𝔀𝓸𝓻𝓵𝓭";
>𝓱𝓮𝓵𝓵𝓸 : "𝔀𝓸𝓻𝓵𝓭"
>           : ^^^^^^^^^^^^
>"𝔀𝓸𝓻𝓵𝓭" : "𝔀𝓸𝓻𝓵𝓭"
>             : ^^^^^^^^^^^^

const 𝘳𝘦𝘨𝘦𝘹 = /(?𝘴𝘪-𝘮:^𝘧𝘰𝘰.)/𝘨𝘮𝘶;
>𝘳𝘦𝘨𝘦𝘹 : RegExp
>           : ^^^^^^
>/(?𝘴𝘪-𝘮:^𝘧𝘰𝘰.)/𝘨𝘮𝘶 : RegExp
>                            : ^^^^^^
"#.as_bytes();
        let baseline = TypesBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            TypesBaseline {
                names: index_vec!["a.ts"],
                files: index_vec![TypeBaselineFile {
                    statements: index_vec![
                        "module 才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123 {",
                        r#"const 𝓱𝓮𝓵𝓵𝓸 = "𝔀𝓸𝓻𝓵𝓭";"#,
                        "const 𝘳𝘦𝘨𝘦𝘹 = /(?𝘴𝘪-𝘮:^𝘧𝘰𝘰.)/𝘨𝘮𝘶;"
                    ],
                    assertions: index_vec![
                        vec![Assertion {
                            expr: "才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123",
                            expected_type: "typeof globalThis.才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüß才能ソЫⅨ蒤郳र्क्ड्राüışğİliيونيكودöÄüßAbcd123"
                        },],
                        vec![
                            Assertion {
                                expr: "𝓱𝓮𝓵𝓵𝓸", expected_type: r#""𝔀𝓸𝓻𝓵𝓭""#
                            },
                            Assertion {
                                expr: r#""𝔀𝓸𝓻𝓵𝓭""#, expected_type: r#""𝔀𝓸𝓻𝓵𝓭""#
                            },
                        ],
                        vec![
                            Assertion { expr: "𝘳𝘦𝘨𝘦𝘹", expected_type: r"RegExp" },
                            Assertion {
                                expr: r"/(?𝘴𝘪-𝘮:^𝘧𝘰𝘰.)/𝘨𝘮𝘶", expected_type: r"RegExp"
                            },
                        ],
                    ]
                }]
            }
        );
    }
}
