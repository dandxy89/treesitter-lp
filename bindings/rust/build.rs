//! Compiles the generated parser and external scanner.

use std::path::{Path, PathBuf};

fn main() {
    let src_dir = Path::new("src");

    let mut c_config = cc::Build::new();
    c_config.std("c11").include(src_dir);

    #[cfg(target_env = "msvc")]
    c_config.flag("-utf-8");

    if std::env::var("TARGET").expect("cargo sets TARGET") == "wasm32-unknown-unknown" {
        let wasm_headers = std::env::var("DEP_TREE_SITTER_LANGUAGE_WASM_HEADERS")
            .expect("DEP_TREE_SITTER_LANGUAGE_WASM_HEADERS must be set by the language crate");
        let wasm_src = PathBuf::from(
            std::env::var("DEP_TREE_SITTER_LANGUAGE_WASM_SRC")
                .expect("DEP_TREE_SITTER_LANGUAGE_WASM_SRC must be set by the language crate"),
        );

        c_config.include(&wasm_headers);
        c_config.files(["stdio.c", "stdlib.c", "string.c"].map(|f| wasm_src.join(f)));
    }

    for file in ["parser.c", "scanner.c"] {
        let path = src_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        c_config.file(path);
    }

    c_config.compile("tree-sitter-lp");
}
