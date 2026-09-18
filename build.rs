use std::{collections::HashMap, path::PathBuf};

fn main() {
    // thunk::thunk();
    let library = HashMap::from([("lucide".to_string(), PathBuf::from(lucide_slint::lib()))]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library);
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint build failed");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("ui/res/app.ico");
        // pub const LANG_CHINESE_SIMPLIFIED: u32 = 4u32;
        res.set_language(4u16);
        res.compile().expect("资源文件信息编译失败");

        #[cfg(feature = "win8")]
        {
            cc::Build::new()
                .file("compat/win8_compat.c")
                .compile("win8_compat");
            println!("cargo:rustc-link-arg=-Wl,--wrap=SystemParametersInfoForDpi");
        }
    }
}
