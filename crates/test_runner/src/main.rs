use std::{env, path::Path};

use test_runner::{discover, run_test};

fn main() {
    let Some(arg) = env::args().nth(1) else {
        panic!("Missing path to TypeScript repo");
    };
    let repo = Path::new(&arg);
    discover(repo, run_test);
}
