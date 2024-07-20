use std::path::Path;

use type_info::TypeCheck;

use crate::{
    Baseline, TestUnit, TestVariant, file_system::TestFileSystem, type_visitor::TypeVisitor,
};

/// # Panics
pub fn run_test(
    unit: &TestUnit<'_>,
    variant: &TestVariant<'_>,
    baseline: &Baseline<'_>,
    root_dir: &Path,
) {
    let compile = if let Some(compile) = unit.file_names.iter().find_map(|&name| {
        if name == "tsconfig.json" {
            // Not sure about this. In theory we should read the list from compilerOptions.
            Some(Vec::new())
        } else {
            None
        }
    }) {
        compile
    } else if unit.settings.no_implicit_references {
        vec![unit.file_names.last_idx()]
    } else {
        assert!(
            !unit.file_names.is_empty(),
            "Test with no files: {}",
            relative_path(unit.path, root_dir).display()
        );

        let last_idx = unit.file_names.last_idx();
        let last_content = unit.file_contents
            [if unit.file_names[last_idx] == "tsconfig.json" { last_idx - 1 } else { last_idx }];
        if last_content.contains("require(") || last_content.contains("reference path") {
            vec![unit.file_names.last_idx()]
        } else {
            unit.file_names.indices().collect()
        }
    };

    let fs = TestFileSystem { unit };
    let type_check = TypeCheck::new(&fs);
    let alloc = oxc::allocator::Allocator::default();
    let root_files: Vec<_> = compile
        .iter()
        .map(|&x| unit.file_names[x])
        .filter(|&x| {
            !std::path::Path::new(x).extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("map")
            })
        })
        .collect();
    let program = match type_check.create_program(&root_files, &alloc) {
        Ok(program) => program,
        Err(err) => {
            // panic!(
            //   "❌ Failed to create program: \n  path: {}\n  variant: {}\n  error: {}",
            //   relative_path(unit.path, root_dir).display(),
            //   if variant.name.is_empty() { "()" } else { &variant.name },
            //   err
            // );

            println!(
                "⚠  {}{}\n{}",
                relative_path(unit.path, root_dir).display(),
                variant.name,
                err
            );
            return;
        }
    };

    println!("⏷ {}{}", relative_path(unit.path, root_dir).display(), variant.name);
    for (&name, semantic) in program.modules.iter().zip(&program.semantic) {
        println!("  ---------------- {name} ----------------");
        let baseline = &baseline.types.files
            [baseline.types.names.position(|&x| x == name).expect("type baseline to exist")];
        let visitor = TypeVisitor { semantic, baseline };
        visitor.run();
    }

    // println!("✅ {}{}", relative_path(unit.path, root_dir).display(), variant.name);
}

/// # Panics
fn relative_path<'a>(path: &'a Path, root_dir: &Path) -> &'a Path {
    path.strip_prefix(root_dir).unwrap()
}
