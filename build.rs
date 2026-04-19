use std::{collections::HashMap, path::PathBuf};

fn main() {
    // println!("cargo:rustc-env=PDFIUM_BUNDLE_LIB={}", "lib");
    let library = HashMap::from([("lucide".to_string(), PathBuf::from(lucide_slint::lib()))]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library);
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint build failed");
}
