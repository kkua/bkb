use native_dialog::{DialogBuilder, MessageLevel};
use pdfium_render::prelude::*;
use std::{path::PathBuf, sync::OnceLock};

/// PDF文档持有者，同时保存Pdfium和PdfDocument以确保生命周期
pub struct PdfDocumentHolder<'a> {
    document: PdfDocument<'a>,
}

impl<'a> PdfDocumentHolder<'a> {
    /// 创建新的PDF文档持有者
    ///
    /// # 参数
    /// * `path` - PDF文件路径
    /// * `password` - 可选的密码
    ///
    /// # Panics
    /// 如果无法绑定到pdfium库或无法读取PDF文件，会触发panic
    pub fn new(pdfium: &'a Pdfium, path: &PathBuf, password: Option<&'a str>) -> Self {
        // 先加载文档
        let document = pdfium
            .load_pdf_from_file(path, password)
            .expect("无法读取PDF文件");

        // 将document转换为'static生命周期
        // let document: PdfDocument<'static> = unsafe { std::mem::transmute(document) };

        Self { document }
    }

    /// 获取页面对象的引用
    pub fn pages(&self) -> &PdfPages<'_> {
        self.document.pages()
    }

    pub fn metadata(&self) -> &PdfMetadata<'_> {
        self.document.metadata()
    }
    /// 获取指定页面的图像数据
    ///
    /// # 参数
    /// * `page_idx` - 页面索引（从0开始）
    ///
    /// # 返回
    /// 返回 (width, height, rgba_bytes) 元组
    pub fn get_page_image(&self, page_idx: i32, reverse_image: bool) -> (u32, u32, Vec<u8>) {
        let rotate = if reverse_image {
            //旋转270°
            PdfPageRenderRotation::Degrees270
        } else {
            // 旋转90°
            PdfPageRenderRotation::Degrees90
        };
        let page = self.pages().get(page_idx).unwrap();
        // 72 DPI: 595 x 842 像素
        // 150 DPI: 1240 x 1754 像素
        // 300 DPI: 2480 x 3508 像素
        // 长边 最后会为 210mm -  2*3mm = 204mm，按300dpi换算为像素 204/25.4*300=2409.448
        // 400dpi换算 204/25.4*400=3212.598
        const TARGET_HEIGHT: i32 = 3212;
        let render_config = PdfRenderConfig::new()
            .set_target_height(TARGET_HEIGHT)
            .set_maximum_height(TARGET_HEIGHT)
            .rotate(rotate, true);
        let bitmap = page.render_with_config(&render_config).unwrap();
        let width = bitmap.width() as u32;
        let height = bitmap.height() as u32;
        let rgba = bitmap.as_rgba_bytes();
        (width, height, rgba)
    }

    /// 获取PDF总页数
    pub fn get_page_count(&self) -> i32 {
        self.pages().len()
    }
}

static PDFIUM_REF: OnceLock<Pdfium> = OnceLock::new();

pub fn init_pdfium() -> &'static Pdfium {
    PDFIUM_REF.get_or_init(|| {
        let lib_path = std::env::current_dir().unwrap().join("lib");
        let lib = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path));
        if lib.is_err() {
            eprintln!("无法绑定到pdfium库, Error: {}", lib.err().unwrap());
            eprintln!(
                "请前往下载适合的版本，解压后将动态链接库文件放入文件夹 {}",
                lib_path.to_string_lossy()
            );
            let url = "https://github.com/bblanchon/pdfium-binaries/releases";
            eprintln!("下载地址 {}", url);
            let yes = DialogBuilder::message()
                .set_level(MessageLevel::Error)
                .set_title("出错啦!")
                .set_text(format!(
                    "请下载适合的版本，解压后放入文件夹 {}\n下载地址 {}",
                    lib_path.to_string_lossy(),
                    url
                ))
                .confirm()
                .show()
                .unwrap();

            if yes {
                let _ = webbrowser::open(url);
            }
            // sleep(Duration::from_secs(5));
            // panic!("请按上述提示操作后重新运行")
            std::process::exit(0)
        }
        Pdfium::new(lib.unwrap())
    })
}
