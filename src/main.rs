#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
use native_dialog::DialogBuilder;

mod booklet;
mod gui;
mod pdf_creator;
mod pdf_render;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gui::start_gui()
}

fn start_dialog() {
    let path = DialogBuilder::file()
        // .set_location("~/Desktop")
        .add_filter("PDF", ["pdf"])
        .set_title("选择源文件")
        .open_single_file()
        .show()
        .unwrap()
        .expect("必须选择一个文件");
    println!("{}", path.to_string_lossy());

    let out_path = DialogBuilder::file()
        .set_title("选择输出目标文件夹")
        .open_single_dir()
        .show()
        .unwrap();
    let auto_double = DialogBuilder::message()
        .set_level(native_dialog::MessageLevel::Info)
        .set_title("打印参数")
        .set_text("是否自动双面")
        .confirm()
        .show()
        .unwrap_or(true);
    println!("自动双面：{}", auto_double);
    let has_cover = DialogBuilder::message()
        .set_level(native_dialog::MessageLevel::Info)
        .set_title("装订参数")
        .set_text("是否有封面")
        .confirm()
        .show()
        .unwrap_or(false);
    let mut keep_cover = false;
    if has_cover {
        keep_cover = DialogBuilder::message()
            .set_level(native_dialog::MessageLevel::Info)
            .set_title("装订参数")
            .set_text("是否保留封面")
            .confirm()
            .show()
            .unwrap_or(false);
    }

    // println!("{}", out_path.to_string_lossy());
    let pdfium = pdf_render::init_pdfium();
    let input_path = path;
    let binding_rule = booklet::BindingRule::new(&input_path);
    let binding_rule = booklet::BindingRule {
        binding_at_middle: true,
        sheets_per_booklet: 10,
        has_cover,
        keep_cover,
        auto_double_side: auto_double,
        ..binding_rule
    }
    .set_output_path(&out_path);
    let src_pdf = pdf_render::PdfDocumentHolder::new(&pdfium, &input_path, None);
    dbg!(src_pdf.get_page_count());
    booklet::create_booklet(&src_pdf, &binding_rule);
}
