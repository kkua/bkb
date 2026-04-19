use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    thread,
    time::Duration,
};

use native_dialog::DialogBuilder;
use slint::{
    ComponentHandle, Image, Model, SharedPixelBuffer, SharedString, VecModel, Weak,
};

use crate::{
    booklet::{self, BindingRule},
    pdf_render::{PdfDocumentHolder, init_pdfium},
};
slint::include_modules!();
static APP_REF: OnceLock<Weak<App>> = OnceLock::new();

macro_rules! def_cb {

    ($ui:expr, $event_handler:ident, $logic_fn:ident) => {{
        // 1. 创建弱引用
        let ui_handle = $ui.as_weak();

        // 2. 注册回调，闭包负责提升句柄并调用逻辑函数
        $ui.$event_handler({
            move || {
            $logic_fn(ui_handle.clone())
        }});
    }};
    ($ui:expr, $event_handler:ident, $logic_fn:ident, $($args:ident),+) => {{
        let ui_handle = $ui.as_weak();
        $ui.$event_handler(move |$($args),+| {
            $logic_fn(ui_handle.clone(), $($args),+)
        });
    }};
}

pub fn start_gui() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new()?;

    // if let Some(img) = get_app_icon() {
    //     app.set_app_icon(img);
    // }
    bind_all_callback(&app);
    app.run()?;
    Ok(())
}

pub fn bind_all_callback(app: &App) {
    // let app_ref = app.as_weak();
    // def_cb!(app, on_add_task, choose_pdf);
    // def_cb!(app, on_clear_queue, cb_clear_queue);
    let _init = APP_REF.set(app.as_weak().clone());
    // println!("APP_REF init {}", _init.is_ok());
    def_cb!(app, on_load_app_icon, on_load_app_icon);
    def_cb!(app, on_add_pdf, on_add_pdf);
    def_cb!(app, on_change_pdf, on_change_pdf, idx);
    def_cb!(app, on_change_out_dir, on_change_out_dir, idx);
    def_cb!(app, on_start_task, on_start_task, idx);
    def_cb!(app, on_open_out_dir, on_open_out_dir, idx);
    // def_cb!(app, on_update_task, on_update_task, idx, conf);
}

// fn choose_pdf(app_ref: Weak<App>) {
//     if let Some(app) = app_ref.upgrade() {
//         let tq = app.get_task_queue();
//         if let Some(model) =tq.as_any().downcast_ref::<VecModel<AppRowObj>>() {
//             // model.push(AppRowObj{model., shared});
//             let idx = model.row_count() as i32;
//             let row = AppRowObj{idx,path: SharedString::new()};
//             model.push(row);
//         }
//     }
// }

fn on_add_pdf(ui_handle: slint::Weak<App>) {
    // 尝试提升弱引用为强引用
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return, // UI 可能已经被销毁，直接返回
    };
    let path_vec = DialogBuilder::file()
        // .set_location("~/Desktop")
        .add_filter("PDF", ["pdf"])
        .set_title("选择源文件")
        .open_multiple_file()
        .show()
        .unwrap();

    if path_vec.is_empty() {
        return;
    }
    let template_conf = ui.get_template_conf();
    let task_list = ui.get_task_list();
    if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
        for pdf_path in path_vec {
            let out_path = pdf_path.clone();
            let out_dir = out_path.parent().unwrap_or(&Path::new("./out/"));
            let cf = CreateConfig {
                enable: true,
                src_path: SharedString::from(pdf_path.to_string_lossy().to_string()),
                out_dir: SharedString::from(out_dir.to_string_lossy().to_string()),
                idx: model.row_count() as i32,
                ..template_conf
            };
            model.push(cf);
            //    CreateConfig
        }
    }

    // ui.set_task_queue(ModelRc::from([]));
}

fn on_change_pdf(ui_handle: slint::Weak<App>, idx: i32) -> SharedString {
    // 尝试提升弱引用为强引用
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return SharedString::new(), // UI 可能已经被销毁，直接返回
    };
    println!("change pdf idx:{}", idx);
    let pdf_path = DialogBuilder::file()
        // .set_location("~/Desktop")
        .add_filter("PDF", ["pdf"])
        .set_title("选择源文件")
        .open_single_file()
        .show()
        .unwrap();
    if let Some(pdf_path) = pdf_path {
        let out_path = pdf_path.clone();
        let out_path = out_path.parent();
        let out_dir = out_path.map_or_else(
            || SharedString::new(),
            |path| SharedString::from(path.to_string_lossy().to_string()),
        );
        let task_list = ui.get_task_list();
        if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
            let idx = idx as isize;
            let model_cnt = model.row_count() as isize;
            if idx > model_cnt || model_cnt <= 0 {
                return SharedString::new();
            }
            if let Some(conf) = model.row_data(idx as usize) {
                let new_conf = CreateConfig {
                    src_path: SharedString::from(pdf_path.to_string_lossy().to_string()),
                    out_dir,
                    // progress: 1.0f32,
                    ..conf
                };
                // send_update_task(idx, conf);

                model.set_row_data(idx as usize, new_conf);
                return SharedString::from(pdf_path.to_string_lossy().to_string());
            } else {
                return SharedString::new();
            }
        } else {
            return SharedString::new();
        }
        // return SharedString::from(pdf_path.to_string_lossy().to_string());
    } else {
        return SharedString::new();
    }
}

fn on_start_task(ui_handle: slint::Weak<App>, idx: i32) {
    let idx = idx as isize;
    let ui_ref = ui_handle.clone();
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return, // UI 可能已经被销毁，直接返回
    };
    let task_list = ui.get_task_list();
    if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
        let model_cnt = model.row_count() as isize;
        if idx > model_cnt || model_cnt <= 0 {
            return;
        }
        let mut tasks = Vec::<CreateConfig>::with_capacity(4);
        if idx < 0 {
            // 遍历
            for i in 0..model_cnt {
                if let Some(conf) = model.row_data(i as usize) {
                    if !conf.enable {
                        continue;
                    }
                    let new_conf = CreateConfig {
                        progress: 0.0f32,
                        ..conf.clone()
                    };
                    model.set_row_data(i as usize, new_conf);
                    tasks.push(conf);
                    // let ui_ref = ui_ref.clone();
                    // let _ = thread::Builder::new().spawn(move || {
                    //     create_booklet(&ui_ref, i, &conf);
                    // });
                }
            }
        } else {
            if let Some(conf) = model.row_data(idx as usize) {
                // let _ = thread::Builder::new().spawn(move || {
                //     create_booklet(&ui_ref, idx, &conf);
                // });
                let new_conf = CreateConfig {
                    progress: 0.0f32,
                    ..conf.clone()
                };
                model.set_row_data(idx as usize, new_conf);
                tasks.push(conf);
            }
        }
        batch_create_booklet(&ui_ref, tasks);
    }
}

fn on_update_task(ui_handle: slint::Weak<App>, idx: i32, conf: CreateConfig) {
    let idx = idx as isize;
    // let ui_ref = ui_handle.clone();
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return, // UI 可能已经被销毁，直接返回
    };
    let task_list = ui.get_task_list();
    if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
        model.set_row_data(idx as usize, conf);
    }
}

fn on_change_out_dir(ui_handle: slint::Weak<App>, idx: i32) {
    let idx = idx as isize;
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return, // UI 可能已经被销毁，直接返回
    };
    let task_list = ui.get_task_list();
    if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
        if let Some(conf) = model.row_data(idx as usize) {
            let dir = DialogBuilder::file()
                // .set_location("~/Desktop")
                // .add_filter("PDF", ["pdf"])
                .set_title("选择输出目录")
                .open_single_dir()
                .show()
                .unwrap_or_else(|_| None);
            if let Some(dir_path) = dir {
                let out_dir = SharedString::from(dir_path.to_string_lossy().to_string());
                let new_conf = CreateConfig { out_dir, ..conf };
                model.set_row_data(idx as usize, new_conf);
            }
        }
    }
}

fn on_open_out_dir(ui_handle: slint::Weak<App>, idx: i32) {
    let idx = idx as isize;
    let ui = match ui_handle.upgrade() {
        Some(ui) => ui,
        None => return, // UI 可能已经被销毁，直接返回
    };
    let task_list = ui.get_task_list();
    if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
        if let Some(conf) = model.row_data(idx as usize) {
            let out_dir = conf.out_dir.to_string();
            #[cfg(windows)]
            {
                use std::process::Command;
                let _ = Command::new("explorer").arg(out_dir).spawn();
            }
        }
    }
}

fn send_update_task(idx: isize, conf: CreateConfig) {
    println!("call send_update_task");
    // let idx = idx as isize;
    if let Some(ui_ref) = APP_REF.get() {
        let _3 = ui_ref.upgrade_in_event_loop(move |ui| {
            let task_list = ui.get_task_list();
            if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
                // model.row_data_tracked(idx as usize);
                model.set_row_data(idx as usize, conf.clone());
                ui.set_update_task_idx(idx as i32);
                // ui.invoke_send_update_task(idx as i32, conf);
                ui.window().request_redraw();
                // if let Some(row) = model.row_data(idx as usize) {
                //         let new_conf = CreateConfig {
                //         progress: 1.0f32,
                //         ..row
                //     };
                // }
            }
        });
    } else {
        println!("APP_REF is None");
    }
}

fn batch_create_booklet(ui_ref: &Weak<App>, tasks: Vec<CreateConfig>) {
    let ui_ref = ui_ref.clone();
    let _ = thread::Builder::new().spawn(move || {
        for conf in tasks {
            let ui_ref = ui_ref.clone();
            let idx = conf.idx as isize;
            create_booklet(&ui_ref.clone(), idx, &conf);
            // ui.set_update_task_idx(conf.idx);
        }
        thread::sleep(Duration::from_millis(150));
        let _ = ui_ref.clone().upgrade_in_event_loop(move |ui| {
            ui.set_pending(false);
        });
    });
}

fn create_booklet(ui_ref: &Weak<App>, idx: isize, conf: &CreateConfig) {
    if conf.src_path.is_empty() || conf.out_dir.is_empty() {
        return;
    }
    let src_path = conf.src_path.to_string();
    let out_dir = conf.out_dir.to_string();
    let src_pdf_cb = src_path.clone();
    let br = BindingRule {
        has_cover: conf.has_cover,
        keep_cover: conf.keep_cover,
        auto_double_side: conf.auto_doble_side,
        sheets_per_booklet: conf.sheet_per_booklet as usize,
        input_path: PathBuf::from(src_pdf_cb),
        output_dir: PathBuf::from(out_dir),
        binding_at_middle: conf.binding_at_middle,
    };

    let ui_ref = ui_ref.clone();

    // let _r = thread::spawn(move || {
    let pdfium = init_pdfium();
    let src_pdf = PdfDocumentHolder::new(&pdfium, &PathBuf::from(src_path), None);
    booklet::create_booklet(&src_pdf, &br);
    let _3 = ui_ref.upgrade_in_event_loop(move |ui| {
        let task_list = ui.get_task_list();
        if let Some(model) = task_list.as_any().downcast_ref::<VecModel<CreateConfig>>() {
            if let Some(row) = model.row_data(idx as usize) {
                let new_conf = CreateConfig {
                    progress: 1.0f32,
                    enable: false,
                    ..row
                };
                // send_update_task(idx, new_conf);
                model.set_row_data(idx as usize, new_conf.clone());
                // ui.invoke_send_update_task(idx as i32, new_conf);
                ui.set_update_task_idx(idx as i32);
            }
        }
    });
    // })
    // .join();
}

fn on_load_app_icon(ui_handle: slint::Weak<App>) -> slint::Image {
    // let ui = match ui_handle.upgrade() {
    //     Some(ui) => ui,
    //     None => return, // UI 可能已经被销毁，直接返回
    // };
    println!("load app icon");
    if let Some(img) = get_app_icon() {
        img
    } else {
        Image::default()
    }
}

fn get_app_icon() -> Option<slint::Image> {
    let icon_data = include_bytes!("../ui/res/app.ico");
    // IcoDecoder::new(BufReader::from(icon_data)).unwrap();
    if let Ok(img) = image::load_from_memory_with_format(icon_data, image::ImageFormat::Ico) {
        let rgba = img.to_rgba8();
        img.save("debug_icon_check.png").unwrap();
        let (width, height) = rgba.dimensions();
        println!("图标尺寸: {}x{}", width, height);
        let buf = SharedPixelBuffer::clone_from_slice(&rgba, width, height);
        let slint_image = Image::from_rgba8(buf);
        println!("get_app_icon");
        Some(slint_image)
    } else {
        None
    }
}
