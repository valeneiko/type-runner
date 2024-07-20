use core::str;
use std::{
    ffi::{OsStr, OsString},
    fs::read,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
};

use crate::{Baseline, TestUnit, TestVariant};

const THREADS: u8 = 24;

struct WorkQueue<'a, T> {
    queue: &'a mut Vec<T>,
    workers: u8,
}

impl<'a, T> WorkQueue<'a, T> {
    fn new(init_queue: &'a mut Vec<T>) -> Self {
        Self { queue: init_queue, workers: 0 }
    }
}

fn quick_walk(paths: Vec<PathBuf>) -> impl Iterator<Item = PathBuf> {
    let mut result: [Vec<PathBuf>; THREADS as usize] = Default::default();

    let mut queue = paths;
    std::thread::scope(|s| {
        let queue = Arc::new((Mutex::new(WorkQueue::new(&mut queue)), Condvar::new()));

        for result in &mut result {
            let queue = Arc::clone(&queue);
            s.spawn(move || {
                let (queue, cvar) = &*queue;
                let mut working: u8 = 0;
                loop {
                    let path = {
                        let mut queue = queue.lock().unwrap();
                        loop {
                            if let Some(path) = queue.queue.pop() {
                                queue.workers += 1 - working;
                                working = 1;
                                break path;
                            }

                            queue.workers -= working;
                            working = 0;

                            if queue.workers == 0 {
                                cvar.notify_all();
                                return;
                            }
                            queue = cvar.wait(queue).unwrap();
                        }
                    };

                    let Ok(dir) = path.read_dir() else {
                        continue;
                    };
                    for entry in dir {
                        let Ok(entry) = entry else {
                            continue;
                        };

                        let path = entry.path();
                        if !path.is_dir() {
                            result.push(path);
                            continue;
                        }

                        queue.lock().unwrap().queue.push(path);
                        cvar.notify_one();
                    }
                }
            });
        }
    });

    result.into_iter().flatten()
}

#[derive(Debug)]
enum FileReadError {
    IO(std::io::Error),
    FromUtf8Error(std::str::Utf8Error),
    FromUtf16Error(std::string::FromUtf16Error),
}

impl std::fmt::Display for FileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileReadError::IO(err) => err.fmt(f),
            FileReadError::FromUtf8Error(err) => err.fmt(f),
            FileReadError::FromUtf16Error(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for FileReadError {}

impl From<std::io::Error> for FileReadError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<std::str::Utf8Error> for FileReadError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::FromUtf8Error(value)
    }
}

impl From<std::string::FromUtf16Error> for FileReadError {
    fn from(value: std::string::FromUtf16Error) -> Self {
        Self::FromUtf16Error(value)
    }
}

fn read_file(path: &Path) -> Result<String, FileReadError> {
    let data = read(path)?;
    let result = match data.get(0..3) {
        // UTF8
        Some([0xef, 0xbb, 0xbf]) => str::from_utf8(&data[3..])?.to_string(),
        // UTF16 BE
        Some([0xfe, 0xff, _]) => {
            let data: Vec<_> =
                data[2..].chunks(2).map(|x| u16::from_be_bytes([x[0], x[1]])).collect();
            String::from_utf16(&data)?
        }
        // UTF16 LE
        Some([0xff, 0xfe, _]) => {
            let data: Vec<_> =
                data[2..].chunks(2).map(|x| u16::from_le_bytes([x[0], x[1]])).collect();
            String::from_utf16(&data)?
        }
        // Anything else
        _ => str::from_utf8(&data)?.to_string(),
    };

    Ok(result)
}

/// # Panics
pub fn discover<F: Fn(&TestUnit<'_>, &TestVariant<'_>, &Baseline<'_>, &Path)>(repo: &Path, run: F) {
    let test_paths = vec![repo.join("tests/cases/compiler"), repo.join("tests/cases/conformance")];
    let discovered_files = {
        let mut files: Vec<_> = quick_walk(test_paths).collect();
        files.sort();
        files
    };
    for test_file in discovered_files {
        // Ignore these 2 tests
        if test_file.ends_with("compiler/corrupted.ts")
            || test_file.ends_with("compiler/TransportStream.ts")
            || test_file.ends_with("compiler/checkJsFiles6.ts")
            || test_file.ends_with("compiler/jsFileCompilationWithoutJsExtensions.ts")
        {
            continue;
        }

        let Ok(data) = read_file(&test_file) else {
            panic!("Failed to read test file: {}", test_file.strip_prefix(repo).unwrap().display());
        };
        let unit = TestUnit::parse(&test_file, data.as_bytes());
        if unit.settings.no_types_and_symbols {
            continue;
        }

        let name = test_file.file_stem().expect("path to be a file");
        for variant in unit.variations.iter() {
            let variant_name = &variant.name;
            let types_file = get_baseline_path(repo, name, variant_name, "types");
            let Ok(types_data) = read_file(&types_file) else {
                panic!(
                    "Failed to read types baseline file:\n  case: {}\n  baseline: {}\n  variant: {:?}",
                    test_file.strip_prefix(repo).unwrap().display(),
                    types_file.strip_prefix(repo).unwrap().display(),
                    variant
                );
            };

            let errors_file = get_baseline_path(repo, name, variant_name, "errors.txt");
            let errors_data = read_file(&errors_file).ok();

            let baseline = Baseline::parse(
                types_file.strip_prefix(repo).unwrap(),
                types_data.as_bytes(),
                errors_file.strip_prefix(repo).unwrap(),
                errors_data.as_ref().map(std::string::String::as_bytes),
            );

            run(&unit, &variant, &baseline, repo);
        }
    }
}

fn get_baseline_path(repo: &Path, name: &OsStr, variant: &str, kind: &str) -> PathBuf {
    // let filename = format!("{}{}.{}", name, variant, kind);
    let mut filename = OsString::with_capacity(name.len() + variant.len() + kind.len() + 1);
    filename.push(name);
    filename.push(variant);
    filename.push(".");
    filename.push(kind);
    repo.join("tests/baselines/reference").join(filename)
}
