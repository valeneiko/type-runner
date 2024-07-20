use std::{collections::VecDeque, path::Path};

use memchr::{memchr, memchr_iter, memchr2, memrchr};

use crate::byte_utils::trim_space_start;

use super::line_iter::LineIter;

#[expect(clippy::if_same_then_else)]
fn cmp_file(a: &str, b: &str) -> std::cmp::Ordering {
    let result = a.cmp(b);
    if result.is_eq() {
        std::cmp::Ordering::Equal
    } else if a == "tsconfig.json" {
        std::cmp::Ordering::Less
    } else if b == "tsconfig.json" {
        std::cmp::Ordering::Greater
    } else if a.starts_with("lib.") && a.ends_with(".d.ts") {
        std::cmp::Ordering::Greater
    } else if b.starts_with("lib.") && b.ends_with(".d.ts") {
        std::cmp::Ordering::Less
    } else {
        result
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct ErrorsBaseline<'a> {
    config_errors: Vec<ConfigError<'a>>,
    file_errors: Vec<FileError<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError<'a> {
    code: &'a str,
    message: &'a str,
    hint: Vec<(u8, &'a str)>,
}

impl<'a> ConfigError<'a> {
    fn parse(path: &'_ Path, line: &'a [u8]) -> Self {
        let code_start = 8;
        let Some(code_end) = memchr(b':', &line[code_start..]) else {
            panic!(
                "Failed to find end of error code\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(code_start)
            );
        };

        let message_start = code_start + code_end + 2;
        ConfigError {
            code: std::str::from_utf8(&line[code_start..code_start + code_end])
                .expect("error code to be UTF8"),
            message: std::str::from_utf8(&line[message_start..]).expect("message to be UTF8"),
            hint: vec![],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FileError<'a> {
    file: &'a str,
    // line: u32,
    // column: u32,
    loc: Option<(u32, u32)>,
    length: Option<u32>,
    code: &'a str,
    message: &'a str,
    hint: Vec<(u8, &'a str)>,
    related: Vec<Self>,
}

impl<'a> FileError<'a> {
    fn parse(path: &'_ Path, line: &'a [u8]) -> Self {
        let Some(name_end) = memchr(b'(', line) else {
            panic!(
                "Failed to find end of file name\n  path: {}\n  line: {}\n      : >",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
            );
        };

        let line_start = name_end + 1;
        let Some(line_end) = memchr(b',', &line[line_start..]) else {
            panic!(
                "Failed to find end of line number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(line_start)
            );
        };

        let column_start = line_start + line_end + 1;
        let Some(column_end) = memchr(b')', &line[column_start..]) else {
            panic!(
                "Failed to find end of column number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(column_start)
            );
        };

        let code_start = column_start + column_end + 11;
        let Some(code_end) = memchr(b':', &line[code_start..]) else {
            panic!(
                "Failed to find end of error code\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(code_start)
            );
        };

        let message_start = code_start + code_end + 2;
        Self {
            file: std::str::from_utf8(&line[..name_end]).expect("file name to be UTF8"),
            loc: if let Ok(line_num) = std::str::from_utf8(&line[line_start..line_start + line_end])
                .expect("line number to be UTF8")
                .parse()
            {
                let column = std::str::from_utf8(&line[column_start..column_start + column_end])
                    .expect("column number to be UTF8")
                    .parse()
                    .expect("column number to be integer");
                Some((line_num, column))
            } else {
                None
            },
            length: None,
            code: std::str::from_utf8(&line[code_start..code_start + code_end])
                .expect("error code to be UTF8"),
            message: std::str::from_utf8(&line[message_start..]).expect("message to be UTF8"),
            hint: vec![],
            related: vec![],
        }
    }

    fn parse_related(path: &'_ Path, line: &'a [u8], parent: &'_ FileError<'a>) -> Self {
        let code_start = 14;
        let Some(code_end) = memchr2(b' ', b':', &line[code_start..]) else {
            panic!(
                "Failed to find end of error code\n  path: {}\n  line: {}\n      : >{}",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(code_start)
            );
        };

        if line[code_start + code_end] == b' ' {
            let name_start = code_start + code_end + 1;
            let mut delim_iter = memchr_iter(b':', &line[name_start..]);
            let Some(name_end) = delim_iter.next() else {
                panic!(
                    "Failed to find end of file name\n  path: {}\n  line: {}\n      : {}>",
                    path.display(),
                    std::str::from_utf8(line).unwrap().escape_debug(),
                    " ".repeat(name_start)
                );
            };

            let line_start = name_start + name_end + 1;
            let Some(line_end) = delim_iter.next() else {
                panic!(
                    "Failed to find end of line number\n  path: {}\n  line: {}\n      : {}>",
                    path.display(),
                    std::str::from_utf8(line).unwrap().escape_debug(),
                    " ".repeat(line_start)
                );
            };

            let column_start = name_start + line_end + 1;
            let Some(column_end) = delim_iter.next() else {
                panic!(
                    "Failed to find end of column number\n  path: {}\n  line: {}\n      : {}>",
                    path.display(),
                    std::str::from_utf8(line).unwrap().escape_debug(),
                    " ".repeat(column_start)
                );
            };

            let message_start = name_start + column_end + 2;
            Self {
                file: std::str::from_utf8(&line[name_start..name_start + name_end])
                    .expect("file name to be UTF8"),
                loc: if let Ok(line_num) =
                    std::str::from_utf8(&line[line_start..name_start + line_end])
                        .expect("line number to be UTF8")
                        .parse::<u32>()
                {
                    let column = std::str::from_utf8(&line[column_start..name_start + column_end])
                        .expect("column number to be UTF8")
                        .parse()
                        .expect("column number to be integer");
                    Some((line_num, column))
                } else {
                    None
                },
                length: None,
                code: std::str::from_utf8(&line[code_start..code_start + code_end])
                    .expect("error code to be UTF8"),
                message: std::str::from_utf8(&line[message_start..]).expect("message to be UTF8"),
                hint: vec![],
                related: vec![],
            }
        } else {
            let code_end = code_start + code_end;
            Self {
                file: parent.file,
                loc: parent.loc,
                length: parent.length,
                code: std::str::from_utf8(&line[code_start..code_end])
                    .expect("error code to be UTF8"),
                message: std::str::from_utf8(&line[code_end + 2..]).expect("message to be UTF8"),
                hint: vec![],
                related: vec![],
            }
        }
    }

    fn parse_pretty(path: &'_ Path, line: &'a [u8]) -> Self {
        let name_start = 5;
        let mut delim_iter = memchr_iter(b'', &line[name_start..]);
        let Some(name_end) = delim_iter.next() else {
            panic!(
                "Failed to find end of file name\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start]).unwrap().escape_debug().count()
                )
            );
        };

        let Some(line_end) = delim_iter.nth(2) else {
            panic!(
                "Failed to find end of line number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + name_end])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        let Some(column_end) = delim_iter.next() else {
            panic!(
                "Failed to find end of column number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + line_end])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        let Some(code_start) = delim_iter.nth(2) else {
            panic!(
                "Failed to find start of error code\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + column_end])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        let Some(message_start) = delim_iter.next() else {
            panic!(
                "Failed to find start of error message\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + code_start])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        Self {
            file: std::str::from_utf8(&line[name_start..name_start + name_end])
                .expect("file name to be UTF8"),
            loc: if let Ok(line_num) =
                std::str::from_utf8(&line[name_start + name_end + 10..name_start + line_end - 5])
                    .expect("line number to be UTF8")
                    .parse()
            {
                let column =
                    std::str::from_utf8(&line[name_start + line_end + 5..name_start + column_end])
                        .expect("column number to be UTF8")
                        .parse()
                        .expect("column number to be integer");
                Some((line_num, column))
            } else {
                None
            },
            length: None,
            code: std::str::from_utf8(
                &line[name_start + code_start + 8..name_start + message_start - 2],
            )
            .expect("error code to be UTF8"),
            message: std::str::from_utf8(&line[name_start + message_start + 4..])
                .expect("message to be UTF8"),
            hint: vec![],
            related: vec![],
        }
    }

    fn parse_pretty_related<T: Iterator<Item = (usize, usize, &'a [u8])>>(
        path: &'_ Path,
        mut iter: T,
    ) -> Self {
        let line = iter.next().expect("related error first line").2;
        let name_start = 7;
        let mut delim_iter = memchr_iter(b'', &line[name_start..]);

        let Some(name_end) = delim_iter.next() else {
            panic!(
                "Failed to find end of file name\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start]).unwrap().escape_debug().count()
                )
            );
        };

        let Some(line_end) = delim_iter.nth(2) else {
            panic!(
                "Failed to find end of line number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + name_end])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        let Some(column_end) = delim_iter.next() else {
            panic!(
                "Failed to find end of column number\n  path: {}\n  line: {}\n      : {}>",
                path.display(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                " ".repeat(
                    std::str::from_utf8(&line[..name_start + line_end])
                        .unwrap()
                        .escape_debug()
                        .count()
                )
            );
        };

        let underline = iter.nth(1).expect("related error third line").2;

        let mut err = Self {
            file: std::str::from_utf8(&line[name_start..name_start + name_end])
                .expect("file name to be UTF8"),
            loc: if let Ok(line_num) =
                std::str::from_utf8(&line[name_start + name_end + 10..name_start + line_end - 5])
                    .expect("line number to be UTF8")
                    .parse()
            {
                let column =
                    std::str::from_utf8(&line[name_start + line_end + 5..name_start + column_end])
                        .expect("column number to be UTF8")
                        .parse()
                        .expect("column number to be integer");
                Some((line_num, column))
            } else {
                None
            },
            length: None,
            code: "",
            message: std::str::from_utf8(
                &iter.next().expect("related error last fourth line").2[4..],
            )
            .expect("message to be UTF8"),
            hint: vec![],
            related: vec![],
        };

        if let Some(loc) = err.loc {
            err.length = memrchr(b'~', underline).map(
                #[expect(clippy::cast_possible_truncation)]
                |x| {
                    x as u32
                        - 17
                        - memchr(b'', &underline[9..]).expect("delimiter after line number") as u32
                        - loc.1
                },
            );
        }

        err
    }
}

impl<'a> ErrorsBaseline<'a> {
    /// # Panics
    pub fn parse(path: &'_ Path, data: &'a [u8]) -> Self {
        if data[0] == 0x1B {
            return Self::parse_formatted(path, data);
        }

        let mut result = Self::default();
        let mut iter = LineIter::new(data);
        while let Some((_line_idx, _line_start, line)) = iter.next() {
            if line.is_empty() {
                let (_, _, line) = iter.next().unwrap();
                assert!(
                    line.is_empty(),
                    "Expected 2 empty lines at the end of summary block:\n  path: {}\n  line: {}",
                    path.display(),
                    std::str::from_utf8(line).unwrap().escape_debug()
                );
                break;
            }

            match line[0] {
                b' ' => {
                    let err = if result.file_errors.is_empty() {
                        &mut result
                            .config_errors
                            .last_mut()
                            .expect("Error to be created before hint line")
                            .hint
                    } else {
                        &mut result
                            .file_errors
                            .last_mut()
                            .expect("Error to be created before hint line")
                            .hint
                    };

                    let hint = trim_space_start(line);
                    let spaces = line.len() - hint.len();

                    #[expect(clippy::cast_possible_truncation)]
                    err.push((
                        (spaces / 2) as u8,
                        std::str::from_utf8(hint).expect("hint to be UTF8"),
                    ));
                }
                _ => {
                    if line.starts_with(b"error TS") {
                        assert!(
                            result.file_errors.is_empty(),
                            "Expected all config errors to be before any file errors\n  path: {}\n  line: {}",
                            path.display(),
                            std::str::from_utf8(line).unwrap().escape_debug()
                        );
                        result.config_errors.push(ConfigError::parse(path, line));
                    } else {
                        result.file_errors.push(FileError::parse(path, line));
                    }
                }
            }
        }

        // Skip until we encounter the first file: ==== file.ts (0 errors) ====
        let mut file = "";
        for (_, _, line) in iter.by_ref() {
            if !line.is_empty() && line[0] == b'=' {
                file = std::str::from_utf8(
                    &line[5..5 + memchr(b' ', &line[5..])
                        .expect("file name to be followed by space")],
                )
                .expect("file name to be UTF8");
                if file.starts_with("./") {
                    file = &file[2..];
                }
                break;
            }
        }

        let mut err_queue = {
            let partition_start =
                result.file_errors.partition_point(|x| cmp_file(x.file, file).is_lt());
            let err = &mut result.file_errors[partition_start..];
            let partition_end = err.partition_point(|x| cmp_file(x.file, file).is_le());
            err[..partition_end].iter_mut().collect::<VecDeque<_>>()
        };
        assert!(
            err_queue.iter().all(|x| x.file == file),
            "Expected errors to be ordered:\n  path: {}\n  file: {}",
            path.display(),
            file
        );
        let mut code_line = 0u32;
        while let Some((_, _, line)) = iter.next() {
            if !line.is_empty() && line[0] == b'=' {
                file = std::str::from_utf8(
                    &line[5..5 + memchr(b' ', &line[5..])
                        .expect("file name to be followed by space")],
                )
                .expect("file name to be UTF8");
                if file.starts_with("./") {
                    file = &file[2..];
                }
                err_queue = {
                    let partition_start =
                        result.file_errors.partition_point(|x| cmp_file(x.file, file).is_lt());
                    let err = &mut result.file_errors[partition_start..];
                    let partition_end = err.partition_point(|x| cmp_file(x.file, file).is_le());
                    err[..partition_end].iter_mut().collect::<VecDeque<_>>()
                };
                assert!(
                    err_queue.iter().all(|x| x.file == file),
                    "Expected errors to be ordered:\n  path: {}\n  file: {}",
                    path.display(),
                    file
                );
                code_line = 0;
                continue;
            }

            if err_queue.is_empty() {
                continue;
            }

            code_line += 1;

            let mut err_done: Vec<usize> = vec![];
            for (idx, err) in err_queue.iter_mut().enumerate().take_while(|(_, err)| {
        let Some(loc) = err.loc else {
          panic!(
            "Expected error location to exist:\n  path: {}\n  err: {:?}\n  file: {}\n  line: {}",
            path.display(),
            err,
            file,
            std::str::from_utf8(line).unwrap().escape_debug()
          );
        };
        loc.0 <= code_line
      }) {
        let loc = err.loc.expect("error location to exist");
        let last_line = iter.next().expect("underline line to exist").2;
        assert!(data.len() > iter.line_start,
          "Expected error or code line after underline:\n  path: {}\n  err: {:?}\n  line({:>2}): {}\n  ____    : {}\n",
          path.display(),
          err,
          code_line,
          std::str::from_utf8(line).unwrap().escape_debug(),
          std::str::from_utf8(last_line).unwrap().escape_debug(),
        );
        if data[iter.line_start] == b'!' {
          err_done.push(idx);
          if code_line == loc.0 {
            err.length = memrchr(b'~', last_line).map(
              #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
              |x| (x as u32).checked_sub(2+loc.1).unwrap_or_else(|| panic!(
                "Expected length to be positive:\n  path: {}\n  err: {:?}\n  line:  {}\n  line: {}\n  expr: {} - 2 - {} = {}",
                path.display(),
                err,
                std::str::from_utf8(last_line).unwrap().escape_debug(),
                std::str::from_utf8(line).unwrap().escape_debug(),
                x, loc.1, x as i32 - 2 - loc.1 as i32
              )),
            );
          }

          while iter.line_start < data.len() && data[iter.line_start] == b'!' {
            let (_, _, line) = iter.next().unwrap();
            if line[4] != b'r' {
              continue;
            }

            err.related.push(FileError::parse_related(path, line, err));
          }
        }
      }

            for (counter, idx) in err_done.into_iter().enumerate() {
                err_queue.remove(idx - counter);
            }
        }

        result
    }

    fn parse_formatted(path: &'_ Path, data: &'a [u8]) -> Self {
        // Need to skip ANSI escape sequences: \u001b[.{1,2}m
        // Starts with `0x1B` (ESC), followed by `[`, followed by 1-2 digits and termiated by `m`
        let mut result = Self::default();
        let mut iter = LineIter::new(data);
        while let Some((_, _, line)) = iter.next() {
            if !line.starts_with(b"[96m") {
                break;
            }

            let mut err = FileError::parse_pretty(path, line);

            while let Some((_, _, line)) = iter.next() {
                if data[iter.line_start] == b'' {
                    iter.next();
                    break;
                }

                let hint = trim_space_start(line);
                let spaces = line.len() - hint.len();

                #[expect(clippy::cast_possible_truncation)]
                err.hint.push((
                    (spaces / 2) as u8,
                    std::str::from_utf8(hint).expect("hint to be UTF8"),
                ));
            }

            let underline = iter.next().expect("underline to exist").2;
            if let Some(loc) = err.loc {
                err.length = memrchr(b'~', underline).map(
                    #[expect(clippy::cast_possible_truncation)]
                    |x| {
                        x as u32
                            - 13
                            - memchr(b'', &underline[5..]).expect("delimiter after line number")
                                as u32
                            - loc.1
                    },
                );
            }

            // Next line start a new error
            if data[iter.line_start] == b'' {
                continue;
            }

            // Skip empty line
            iter.next();

            // Parse related errors
            while &data[iter.line_start..iter.line_start + 3] == b"  " {
                let related = FileError::parse_pretty_related(path, iter.by_ref().take(4));
                err.related.push(related);
            }

            result.file_errors.push(err);
        }

        // Skip until we encounter the first file: ==== file.ts (0 errors) ====
        let mut file = "";
        for (_, _, line) in iter.by_ref() {
            if !line.is_empty() && line[0] == b'=' {
                file = std::str::from_utf8(
                    &line[5..5 + memchr(b' ', &line[5..])
                        .expect("file name to be followed by space")],
                )
                .expect("file name to be UTF8");
                if file.starts_with("./") {
                    file = &file[2..];
                }
                break;
            }
        }

        let mut err_queue = {
            let partition_start =
                result.file_errors.partition_point(|x| cmp_file(x.file, file).is_lt());
            let err = &mut result.file_errors[partition_start..];
            let partition_end = err.partition_point(|x| cmp_file(x.file, file).is_le());
            err[..partition_end].iter_mut().collect::<VecDeque<_>>()
        };
        assert!(
            err_queue.iter().all(|x| x.file == file),
            "Expected errors to be ordered:\n  path: {}\n  file: {}",
            path.display(),
            file
        );
        let mut code_line = 0u32;
        while let Some((_, _, line)) = iter.next() {
            if !line.is_empty() && line[0] == b'=' {
                file = std::str::from_utf8(
                    &line[5..5 + memchr(b' ', &line[5..])
                        .expect("file name to be followed by space")],
                )
                .expect("file name to be UTF8");
                if file.starts_with("./") {
                    file = &file[2..];
                }
                err_queue = {
                    let partition_start =
                        result.file_errors.partition_point(|x| cmp_file(x.file, file).is_lt());
                    let err = &mut result.file_errors[partition_start..];
                    let partition_end = err.partition_point(|x| cmp_file(x.file, file).is_le());
                    err[..partition_end].iter_mut().collect::<VecDeque<_>>()
                };
                assert!(
                    err_queue.iter().all(|x| x.file == file),
                    "Expected errors to be ordered:\n  path: {}\n  file: {}",
                    path.display(),
                    file
                );
                code_line = 0;
                continue;
            }

            if err_queue.is_empty() {
                continue;
            }

            code_line += 1;

            let mut err_done: Vec<usize> = vec![];
            for (idx, err) in err_queue
                .iter_mut()
                .enumerate()
                .take_while(|(_, err)| err.loc.expect("error location to exist").0 <= code_line)
            {
                if data[iter.line_start] == b'!' {
                    err_done.push(idx);

                    let mut related = err.related.iter_mut();
                    while iter.line_start < data.len() && data[iter.line_start] == b'!' {
                        let (_, _, line) = iter.next().unwrap();
                        if line[4] != b'r' {
                            continue;
                        }

                        let related = related.next().expect("number of related errors to match");
                        related.code = std::str::from_utf8(
                            &line[14..14
                                + memchr(b' ', &line[14..]).expect("error code to end with space")],
                        )
                        .expect("error code to be UTF8");
                    }
                }
            }

            for (counter, idx) in err_done.into_iter().enumerate() {
                err_queue.remove(idx - counter);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use super::*;

    #[test]
    fn single_file() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br"ClassDeclaration26.ts(2,18): error TS1440: Variable declaration not allowed at this location.
ClassDeclaration26.ts(4,5): error TS1068: Unexpected token. A constructor, method, accessor, or property was expected.
ClassDeclaration26.ts(4,20): error TS1005: ',' expected.
ClassDeclaration26.ts(4,23): error TS1005: '=>' expected.
ClassDeclaration26.ts(5,1): error TS1128: Declaration or statement expected.


==== ClassDeclaration26.ts (5 errors) ====
    class C {
        public const var export foo = 10;
                     ~~~
!!! error TS1440: Variable declaration not allowed at this location.

        var constructor() { }
        ~~~
!!! error TS1068: Unexpected token. A constructor, method, accessor, or property was expected.
                       ~
!!! error TS1005: ',' expected.
                          ~
!!! error TS1005: '=>' expected.
    }
    ~
!!! error TS1128: Declaration or statement expected.";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![
                    FileError {
                        file: "ClassDeclaration26.ts",
                        loc: Some((2, 18)),
                        length: Some(3),
                        code: "1440",
                        message: "Variable declaration not allowed at this location.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "ClassDeclaration26.ts",
                        loc: Some((4, 5)),
                        length: Some(3),
                        code: "1068",
                        message: "Unexpected token. A constructor, method, accessor, or property was expected.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "ClassDeclaration26.ts",
                        loc: Some((4, 20)),
                        length: Some(1),
                        code: "1005",
                        message: "',' expected.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "ClassDeclaration26.ts",
                        loc: Some((4, 23)),
                        length: Some(1),
                        code: "1005",
                        message: "'=>' expected.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "ClassDeclaration26.ts",
                        loc: Some((5, 1)),
                        length: Some(1),
                        code: "1128",
                        message: "Declaration or statement expected.",
                        hint: vec![],
                        related: vec![]
                    },
                ]
            }
        );
    }

    #[test]
    fn multiple_file() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br#"file2.ts(5,16): error TS2671: Cannot augment module './file1' because it resolves to a non-module entity.
file3.ts(3,8): error TS2503: Cannot find namespace 'x'.


==== file3.ts (1 errors) ====
    import x = require("./file1");
    import "./file2";
    let a: x.A; // should not work
           ~
!!! error TS2503: Cannot find namespace 'x'.
==== file1.ts (0 errors) ====
    var x = 1;
    export = x;

==== file2.ts (1 errors) ====
    import x = require("./file1");

    // augmentation for './file1'
    // should error since './file1' does not have namespace meaning
    declare module "./file1" {
                   ~~~~~~~~~
!!! error TS2671: Cannot augment module './file1' because it resolves to a non-module entity.
        interface A { a }
    }
    "#;
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![
                    FileError {
                        file: "file2.ts",
                        loc: Some((5, 16)),
                        length: Some(9),
                        code: "2671",
                        message: "Cannot augment module './file1' because it resolves to a non-module entity.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "file3.ts",
                        loc: Some((3, 8)),
                        length: Some(1),
                        code: "2503",
                        message: "Cannot find namespace 'x'.",
                        hint: vec![],
                        related: vec![]
                    }
                ]
            }
        );
    }

    #[test]
    fn with_hint() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br"addMoreOverloadsToBaseSignature.ts(5,11): error TS2430: Interface 'Bar' incorrectly extends interface 'Foo'.
  Types of property 'f' are incompatible.
    Type '(key: string) => string' is not assignable to type '() => string'.
      Target signature provides too few arguments. Expected 1 or more, but got 0.


==== addMoreOverloadsToBaseSignature.ts (1 errors) ====
    interface Foo {
        f(): string;
    }

    interface Bar extends Foo {
              ~~~
!!! error TS2430: Interface 'Bar' incorrectly extends interface 'Foo'.
!!! error TS2430:   Types of property 'f' are incompatible.
!!! error TS2430:     Type '(key: string) => string' is not assignable to type '() => string'.
!!! error TS2430:       Target signature provides too few arguments. Expected 1 or more, but got 0.
        f(key: string): string;
    }
    ";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![FileError {
                    file: "addMoreOverloadsToBaseSignature.ts",
                    loc: Some((5, 11)),
                    length: Some(3),
                    code: "2430",
                    message: "Interface 'Bar' incorrectly extends interface 'Foo'.",
                    hint: vec![
                        (1, r"Types of property 'f' are incompatible."),
                        (
                            2,
                            r"Type '(key: string) => string' is not assignable to type '() => string'."
                        ),
                        (
                            3,
                            r"Target signature provides too few arguments. Expected 1 or more, but got 0."
                        ),
                    ],
                    related: vec![]
                }]
            }
        );
    }

    #[test]
    fn with_config_error() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br"error TS5102: Option 'noImplicitUseStrict' has been removed. Please remove it from your configuration.
alwaysStrictNoImplicitUseStrict.ts(3,13): error TS1100: Invalid use of 'arguments' in strict mode.


!!! error TS5102: Option 'noImplicitUseStrict' has been removed. Please remove it from your configuration.
==== alwaysStrictNoImplicitUseStrict.ts (1 errors) ====
    module M {
        export function f() {
            var arguments = [];
                ~~~~~~~~~
!!! error TS1100: Invalid use of 'arguments' in strict mode.
        }
    }";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![ConfigError {
                    code: "5102",
                    message: "Option 'noImplicitUseStrict' has been removed. Please remove it from your configuration.",
                    hint: vec![],
                }],
                file_errors: vec![FileError {
                    file: "alwaysStrictNoImplicitUseStrict.ts",
                    loc: Some((3, 13)),
                    length: Some(9),
                    code: "1100",
                    message: "Invalid use of 'arguments' in strict mode.",
                    hint: vec![],
                    related: vec![]
                }]
            }
        );
    }

    #[test]
    fn with_related() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br#"a.ts(1,8): error TS1259: Module '"b"' can only be default-imported using the 'esModuleInterop' flag


==== b.d.ts (0 errors) ====
    declare class Foo {
    	member: string;
    }
    export = Foo;

==== a.ts (1 errors) ====
    import Foo from "./b";
           ~~~
!!! error TS1259: Module '"b"' can only be default-imported using the 'esModuleInterop' flag
!!! related TS2594 b.d.ts:4:1: This module is declared with 'export =', and can only be used with a default import when using the 'esModuleInterop' flag.
    export var x = new Foo();
    "#;
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![FileError {
                    file: "a.ts",
                    loc: Some((1, 8)),
                    length: Some(3),
                    code: "1259",
                    message: r#"Module '"b"' can only be default-imported using the 'esModuleInterop' flag"#,
                    hint: vec![],
                    related: vec![FileError {
                        file: "b.d.ts",
                        loc: Some((4, 1)),
                        length: None,
                        code: "2594",
                        message: r"This module is declared with 'export =', and can only be used with a default import when using the 'esModuleInterop' flag.",
                        hint: vec![],
                        related: vec![]
                    }]
                }]
            }
        );
    }

    #[test]
    fn overlapping_span() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br#"constructorWithIncompleteTypeAnnotation.ts(11,13): error TS2503: Cannot find namespace 'module'.
constructorWithIncompleteTypeAnnotation.ts(11,13): error TS2580: Cannot find name 'module'. Do you need to install type definitions for node? Try `npm i --save-dev @types/node`.
constructorWithIncompleteTypeAnnotation.ts(11,19): error TS1005: ';' expected.
constructorWithIncompleteTypeAnnotation.ts(22,35): error TS1005: ')' expected.
constructorWithIncompleteTypeAnnotation.ts(22,39): error TS2363: The right-hand side of an arithmetic operation must be of type 'any', 'number', 'bigint' or an enum type.
constructorWithIncompleteTypeAnnotation.ts(24,28): error TS1005: ':' expected.
constructorWithIncompleteTypeAnnotation.ts(24,29): error TS1005: ',' expected.


==== constructorWithIncompleteTypeAnnotation.ts (7 errors) ====
    declare module "fs" {
        export class File {
            constructor(filename: string);
            public ReadAllText(): string;
        }
        export interface IFile {
            [index: number]: string;
        }
    }

    import fs = module("fs");
                ~~~~~~
!!! error TS2503: Cannot find namespace 'module'.
                ~~~~~~
!!! error TS2580: Cannot find name 'module'. Do you need to install type definitions for node? Try `npm i --save-dev @types/node`.
                      ~
!!! error TS1005: ';' expected.


    module TypeScriptAllInOne {
        export class Program {
            static Main(...args: string[]) {
                try {
                    var bfs = new BasicFeatures();
                    var retValue: number = 0;

                    retValue = bfs.VARIABLES();
                    if (retValue != 0 ^=  {
                                      ~~
!!! error TS1005: ')' expected.
!!! related TS1007 constructorWithIncompleteTypeAnnotation.ts:22:20: The parser expected to find a ')' to match the '(' token here.
                                          ~


                        return 1;
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
                               ~
!!! error TS1005: ':' expected.
                                ~
!!! error TS1005: ',' expected.
                    }
    ~~~~~~~~~~~~~~~~~
!!! error TS2363: The right-hand side of an arithmetic operation must be of type 'any', 'number', 'bigint' or an enum type.

"#;
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((11, 13)),
                        length: Some(6),
                        code: "2503",
                        message: "Cannot find namespace 'module'.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((11, 13)),
                        length: Some(6),
                        code: "2580",
                        message: "Cannot find name 'module'. Do you need to install type definitions for node? Try `npm i --save-dev @types/node`.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((11, 19)),
                        length: Some(1),
                        code: "1005",
                        message: "';' expected.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((22, 35)),
                        length: Some(2),
                        code: "1005",
                        message: "')' expected.",
                        hint: vec![],
                        related: vec![FileError {
                            file: "constructorWithIncompleteTypeAnnotation.ts",
                            loc: Some((22, 20)),
                            length: None,
                            code: "1007",
                            message: "The parser expected to find a ')' to match the '(' token here.",
                            hint: vec![],
                            related: vec![]
                        }]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((22, 39)),
                        length: None, // multi-line
                        code: "2363",
                        message: "The right-hand side of an arithmetic operation must be of type 'any', 'number', 'bigint' or an enum type.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((24, 28)),
                        length: Some(1),
                        code: "1005",
                        message: "':' expected.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "constructorWithIncompleteTypeAnnotation.ts",
                        loc: Some((24, 29)),
                        length: Some(1),
                        code: "1005",
                        message: "',' expected.",
                        hint: vec![],
                        related: vec![]
                    }
                ]
            }
        );
    }

    #[test]
    fn related_without_location() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br"regularExpressionGroupNameSuggestions.ts(1,18): error TS1503: Named capturing groups are only available when targeting 'ES2018' or later.
regularExpressionGroupNameSuggestions.ts(1,27): error TS1532: There is no capturing group named 'Foo' in this regular expression.


==== regularExpressionGroupNameSuggestions.ts (2 errors) ====
    const regex = /(?<foo>)\k<Foo>/;
                     ~~~~~
!!! error TS1503: Named capturing groups are only available when targeting 'ES2018' or later.
                              ~~~
!!! error TS1532: There is no capturing group named 'Foo' in this regular expression.
!!! related TS1369: Did you mean 'foo'?
    ";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![
                    FileError {
                        file: "regularExpressionGroupNameSuggestions.ts",
                        loc: Some((1, 18)),
                        length: Some(5),
                        code: "1503",
                        message: "Named capturing groups are only available when targeting 'ES2018' or later.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "regularExpressionGroupNameSuggestions.ts",
                        loc: Some((1, 27)),
                        length: Some(3),
                        code: "1532",
                        message: "There is no capturing group named 'Foo' in this regular expression.",
                        hint: vec![],
                        related: vec![FileError {
                            file: "regularExpressionGroupNameSuggestions.ts",
                            loc: Some((1, 27)),
                            length: Some(3),
                            code: "1369",
                            message: "Did you mean 'foo'?",
                            hint: vec![],
                            related: vec![]
                        }]
                    },
                ]
            }
        );
    }

    #[test]
    fn summary_not_sorted() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br#"tsconfig.json(2,5): error TS5095: Option 'bundler' can only be used when 'module' is set to 'preserve' or to 'es2015' or later.
test.ts(1,19): error TS5097: An import path can only end with a '.ts' extension when 'allowImportingTsExtensions' is enabled.


==== tsconfig.json (1 errors) ====
    {
        "compilerOptions": {
        ~~~~~~~~~~~~~~~~~
!!! error TS5095: Option 'bundler' can only be used when 'module' is set to 'preserve' or to 'es2015' or later.
            "paths": {
                "foo/*": ["./dist/*"],
                "baz/*.ts": ["./types/*.d.ts"]
            }
        }
    }

==== dist/bar.ts (0 errors) ====
    export const a = 1234;

==== types/main.d.ts (0 errors) ====
    export const b: string;

==== test.ts (1 errors) ====
    import { a } from "foo/bar.ts";
                      ~~~~~~~~~~~~
!!! error TS5097: An import path can only end with a '.ts' extension when 'allowImportingTsExtensions' is enabled.
    import { b } from "baz/main.ts";
    "#;
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![
                    FileError {
                        file: "tsconfig.json",
                        loc: Some((2, 5)),
                        length: Some(17),
                        code: "5095",
                        message: "Option 'bundler' can only be used when 'module' is set to 'preserve' or to 'es2015' or later.",
                        hint: vec![],
                        related: vec![]
                    },
                    FileError {
                        file: "test.ts",
                        loc: Some((1, 19)),
                        length: Some(12),
                        code: "5097",
                        message: "An import path can only end with a '.ts' extension when 'allowImportingTsExtensions' is enabled.",
                        hint: vec![],
                        related: vec![]
                    },
                ]
            }
        );
    }

    #[test]
    fn with_pretty() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data = br"[96mmultiLineContextDiagnosticWithPretty.ts[0m:[93m2[0m:[93m5[0m - [91merror[0m[90m TS2353: [0mObject literal may only specify known properties, and 'a' does not exist in type '{ c: string; }'.

[7m2[0m     a: {
[7m [0m [91m    ~[0m


==== multiLineContextDiagnosticWithPretty.ts (1 errors) ====
    const x: {c: string} = {
        a: {
        ~
!!! error TS2353: Object literal may only specify known properties, and 'a' does not exist in type '{ c: string; }'.
            b: '',
        }
    };

Found 1 error in multiLineContextDiagnosticWithPretty.ts[90m:2[0m

";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![FileError {
                    file: "multiLineContextDiagnosticWithPretty.ts",
                    loc: Some((2, 5)),
                    length: Some(1),
                    code: "2353",
                    message: r"Object literal may only specify known properties, and 'a' does not exist in type '{ c: string; }'.",
                    hint: vec![],
                    related: vec![]
                }]
            }
        );
    }

    #[test]
    fn with_related_and_pretty() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data =
      br"[96mindex.ts[0m:[93m2[0m:[93m1[0m - [91merror[0m[90m TS1005: [0m'}' expected.

[7m2[0m
[7m [0m [91m[0m

  [96mindex.ts[0m:[93m1[0m:[93m11[0m
    [7m1[0m if (true) {
    [7m [0m [96m          ~[0m
    The parser expected to find a '}' to match the '{' token here.


==== index.ts (1 errors) ====
    if (true) {


!!! error TS1005: '}' expected.
!!! related TS1007 index.ts:1:11: The parser expected to find a '}' to match the '{' token here.
Found 1 error in index.ts[90m:2[0m

";
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![FileError {
                    file: "index.ts",
                    loc: Some((2, 1)),
                    length: None,
                    code: "1005",
                    message: r"'}' expected.",
                    hint: vec![],
                    related: vec![FileError {
                        file: "index.ts",
                        loc: Some((1, 11)),
                        length: Some(1),
                        code: "1007",
                        message: r"The parser expected to find a '}' to match the '{' token here.",
                        hint: vec![],
                        related: vec![],
                    }]
                }]
            }
        );
    }

    #[test]
    fn with_hint_and_pretty() {
        let path = PathBuf::from_str("tests/baselines/reference/unit1.errors.txt").unwrap();
        let data =
      br#"[96mindex.ts[0m:[93m3[0m:[93m8[0m - [91merror[0m[90m TS2345: [0mArgument of type '{ default: () => void; }' is not assignable to parameter of type '() => void'.
  Type '{ default: () => void; }' provides no match for the signature '(): void'.

[7m3[0m invoke(foo);
[7m [0m [91m       ~~~[0m

  [96mindex.ts[0m:[93m1[0m:[93m1[0m
    [7m1[0m import * as foo from "./foo";
    [7m [0m [96m~~~~~~~~~~~~~~~~~~~~~~~~~~~~~[0m
    Type originates at this import. A namespace-style import cannot be called or constructed, and will cause a failure at runtime. Consider using a default import or import require here instead.


==== foo.d.ts (0 errors) ====
    declare function foo(): void;
    declare namespace foo {}
    export = foo;
==== index.ts (1 errors) ====
    import * as foo from "./foo";
    function invoke(f: () => void) { f(); }
    invoke(foo);
           ~~~
!!! error TS2345: Argument of type '{ default: () => void; }' is not assignable to parameter of type '() => void'.
!!! error TS2345:   Type '{ default: () => void; }' provides no match for the signature '(): void'.
!!! related TS7038 index.ts:1:1: Type originates at this import. A namespace-style import cannot be called or constructed, and will cause a failure at runtime. Consider using a default import or import require here instead.

Found 1 error in index.ts[90m:3[0m

"#;
        let baseline = ErrorsBaseline::parse(&path, data);
        assert_eq!(
            baseline,
            ErrorsBaseline {
                config_errors: vec![],
                file_errors: vec![FileError {
                    file: "index.ts",
                    loc: Some((3, 8)),
                    length: Some(3),
                    code: "2345",
                    message: r"Argument of type '{ default: () => void; }' is not assignable to parameter of type '() => void'.",
                    hint: vec![(
                        1,
                        r"Type '{ default: () => void; }' provides no match for the signature '(): void'."
                    )],
                    related: vec![FileError {
                        file: "index.ts",
                        loc: Some((1, 1)),
                        length: Some(29),
                        code: "7038",
                        message: r"Type originates at this import. A namespace-style import cannot be called or constructed, and will cause a failure at runtime. Consider using a default import or import require here instead.",
                        hint: vec![],
                        related: vec![],
                    }]
                }]
            }
        );
    }
}
