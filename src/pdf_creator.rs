use std::u16;

use crate::booklet;
use crate::booklet::BindingRule;
use crate::pdf_render::PdfDocumentHolder;
use oxidize_pdf::Color;
use oxidize_pdf::Document;
use oxidize_pdf::Font;
use oxidize_pdf::Page;
use oxidize_pdf::graphics::LineDashPattern;
use pdfium_render::prelude::PdfDocumentMetadataTagType;

/// 创建册子
///
/// # 参数
/// * `src_pdf` - 源PDF文档容器
/// * `binding_rule` - 装订规则
/// * `booklet_num` - 册子编号
/// * `is_last_booklet` - 是否是最后一册
/// * `booklet_start_page` - 小册子开始页索引(包含)
/// * `booklet_end_page` - 小册子结束页索引(不包含)
pub fn create_booklet(
    src_pdf: &PdfDocumentHolder,
    binding_rule: &BindingRule,
    booklet_num: u16,
    is_last_booklet: bool,
    booklet_start_page: u16,
    booklet_end_page: u16,
) {
    let mut doc = Document::new();
    write_pdf_metadata(src_pdf, &mut doc);
    let file_name = binding_rule
        .input_path
        .file_prefix()
        .expect("没有文件名")
        .to_string_lossy();
    doc.set_title(format!("booklet #{}", booklet_num));
    let mut page_idx = booklet_start_page;
    while page_idx < booklet_end_page {
        if let Some(page) = create_page(
            src_pdf,
            page_idx,
            booklet_start_page,
            booklet_end_page,
            booklet_num,
            is_last_booklet,
            binding_rule,
        ) {
            doc.add_page(page);
        } else {
            break;
        }
        page_idx += 1;
    }

    doc.save(format!(
        "{}/{}_{:02}.pdf",
        binding_rule.output_dir.display(),
        file_name,
        booklet_num
    ))
    .unwrap();

    println!(
        "完成第{}册，共{}页, 开始页: {}, 结束页: {}",
        booklet_num,
        booklet_end_page - booklet_start_page,
        booklet_start_page,
        booklet_end_page
    );
}

/// 设置PDF文档的元数据
///
/// # 参数
/// * `src_pdf` - 源PDF文档容器
/// * `doc` - 目标PDF文档对象
fn write_pdf_metadata(src_pdf: &PdfDocumentHolder<'_>, doc: &mut Document) {
    let creator = format!(
        "{} v{} - {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION")
    );
    doc.set_creator(creator);
    // doc.set_producer(pkg_name);

    if let Some(author) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Author) {
        let author_value = author.value();
        if !author_value.is_empty() {
            doc.set_author(author_value);
        }
    }

    if let Some(subject) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Subject) {
        let subject_value = subject.value();
        if !subject_value.is_empty() {
            doc.set_subject(subject_value);
        }
    }

    if let Some(keywords) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Keywords) {
        let keywords_value = keywords.value();
        if !keywords_value.is_empty() {
            doc.set_keywords(keywords_value);
        }
    }
}

fn create_page(
    src_pdf: &PdfDocumentHolder,
    page_idx: u16,
    group_start_idx: u16,
    group_end_idx: u16,
    booklet_num: u16,
    // padded_page_count: &mut u16,
    is_last_booklet: bool,
    binding_rule: &BindingRule,
) -> Option<Page> {
    let src_pdf_page_count = src_pdf.get_page_count();
    let page_low_idx;
    let page_high_idx;
    let is_sheet_back;
    if let Some((li, hi, is_back)) = calc_sheet_lh_page_idx(
        src_pdf_page_count,
        page_idx,
        group_start_idx,
        group_end_idx,
        booklet_num == 1,
        is_last_booklet,
        binding_rule,
    ) {
        page_low_idx = li;
        page_high_idx = hi;
        is_sheet_back = is_back;
    } else {
        // 本册结束了
        return None;
    };
    let binding_at_middle = binding_rule.binding_at_middle;
    let img_low = if page_low_idx >= src_pdf_page_count {
        // 空白的情况，没有低页
        None
    } else {
        // 获取低页的图像数据
        let reverse_image = is_sheet_back;
        let (page_low_width, page_low_height, page_low_rgba) =
            src_pdf.get_page_image(page_low_idx, reverse_image);
        Some(
            oxidize_pdf::Image::from_rgba_data(page_low_rgba, page_low_width, page_low_height)
                .unwrap(),
        )
    };
    println!("{}, {}, {}", page_low_idx, page_high_idx, is_sheet_back);
    let img_high = if page_high_idx >= src_pdf_page_count {
        // 空白的情况，没有高页
        None
    } else {
        // 获取高页的图像数据
        let reverse_image = !(is_sheet_back ^ binding_at_middle);
        let (page_high_width, page_high_height, page_high_rgba) =
            src_pdf.get_page_image(page_high_idx, reverse_image);
        let img_high =
            oxidize_pdf::Image::from_rgba_data(page_high_rgba, page_high_width, page_high_height)
                .unwrap();
        Some(img_high)
    };
    let v_1mm_to_pt = 72.0 / 25.4;
    // 3mm
    let margin = 3.0 * v_1mm_to_pt;
    let mut new_page = Page::a4();
    let (w, h) = (new_page.width(), new_page.height());
    let half_h = h / 2.0;
    let half_w = w / 2.0;
    // 等比缩放
    // let margin_tb = margin * h / w / 2.0; ==> margin * h / 2.0 / w; ==> margin * (h / 2.0) / w;
    let margin_tb = margin * half_h / w;
    let margin_tb2 = 2.0 * margin_tb;
    // let v_1d5mm = 1.5*v_1mm_to_pt;
    let small_margin_tb = 0.6 * margin_tb;
    let ((img_bottom, img_bottom_idx, bottom_y), (img_top, img_top_idx, top_y)) =
        if binding_rule.binding_at_middle {
            (
                (img_low, page_low_idx, small_margin_tb),
                (
                    img_high,
                    page_high_idx,
                    half_h + margin_tb2 - small_margin_tb,
                ),
            )
        } else {
            (
                (img_high, page_high_idx, margin_tb2 - small_margin_tb),
                (img_low, page_low_idx, half_h + small_margin_tb),
            )
        };
    let img_width = w - 2.0 * margin;
    let img_height = half_h - margin_tb2;
    let v_12mm = 12.0 * v_1mm_to_pt;
    if let Some(img) = img_bottom {
        new_page.add_image(format!("{}", img_bottom_idx), img);
        new_page
            .draw_image(
                format!("{}", img_bottom_idx).as_str(),
                margin,
                bottom_y,
                img_width,
                img_height,
            )
            .unwrap();
    }
    if let Some(img) = img_top {
        new_page.add_image(format!("{}", img_top_idx), img);
        new_page
            .draw_image(
                format!("{}", img_top_idx).as_str(),
                margin,
                top_y,
                img_width,
                img_height,
            )
            .unwrap();
    }

    // 间隔12mm
    let dot_space = v_12mm;
    let padding = 6.0 * v_1mm_to_pt;
    let ((start_x, start_y), (to_x, to_y)) = if is_sheet_back {
        ((padding, half_h), (w, half_h))
    } else {
        ((w - padding, half_h), (0.0, half_h))
    };
    new_page
        .graphics()
        .set_stroke_color(Color::Gray(0.3))
        .move_to(start_x, start_y)
        .line_to(to_x, to_y)
        .set_line_dash_pattern(LineDashPattern::dotted(1.0, dot_space))
        .stroke();
    if !is_sheet_back {
        let _ = new_page
            .text()
            .set_font(Font::TimesRoman, 6.0)
            .at(half_w - 9.0 * v_1mm_to_pt, half_h)
            .write(format!("^- {} -^", booklet_num).as_str());
    }
    return Some(new_page);
}

fn calc_sheet_lh_page_idx(
    page_count: u16,
    page_idx: u16,
    group_start_idx: u16,
    group_end_idx: u16,
    // booklet_num: u16,
    is_first_booklet: bool,
    is_last_booklet: bool,
    binding_rule: &BindingRule,
) -> Option<(u16, u16, bool)> {
    let has_cover = binding_rule.has_cover;
    let keep_cover = binding_rule.keep_cover;
    let mut page_low_idx = page_idx;
    // let mut page_idx = page_idx;
    let binding_at_middle = binding_rule.binding_at_middle;
    let mut page_high_idx = group_end_idx - page_idx + group_start_idx - 1;
    let mut is_sheet_back = page_idx % 2 != 0;
    // 第一册
    if is_first_booklet {
        if has_cover && keep_cover {
            if page_idx == 1 {
                page_low_idx = u16::MAX;
                // is_sheet_back = true;
            } else if page_idx > 1 {
                // page_idx -= 1;
                page_low_idx = page_idx - 1;
            } else {
                // == 0
                // is_sheet_back = false;
            }
        } else if has_cover && !keep_cover {
            if page_idx == 0 {
                // group_start_idx = 1;
            }
            is_sheet_back = !is_sheet_back;
        }
    } else {
        if has_cover {
            is_sheet_back = !is_sheet_back;
        }
    }

    if page_low_idx < u16::MAX && page_low_idx >= page_high_idx {
        // 本册结束了
        return None;
    }
    // 边缘装订
    if !binding_at_middle {
        if has_cover && keep_cover {}
        if is_first_booklet && is_last_booklet {
            page_high_idx = (group_end_idx - group_start_idx - 1) / 2 + page_idx;
        } else if is_first_booklet || is_last_booklet {
            page_high_idx = (group_end_idx - group_start_idx) / 2 + page_idx;
        } else {
            page_high_idx = (group_end_idx - group_start_idx + 1) / 2 + page_idx;
        }
    }
    if is_last_booklet {
        if has_cover && keep_cover {
            if page_high_idx == page_count - 1 {
                page_high_idx = u16::MAX;
            } else if page_high_idx == group_end_idx - 1 {
                page_high_idx = page_count - 1;
                is_sheet_back = false;
            } else if page_high_idx == group_end_idx - 2 {
                is_sheet_back = true;
            }
        } else if has_cover && !keep_cover {
            if page_high_idx >= page_count {
                page_high_idx = u16::MAX;
                is_sheet_back = page_idx % 2 == 0;
            }
        }
    }
    Some((page_low_idx, page_high_idx, is_sheet_back))
}

fn calc_page_on_sheet(
    page_count: u16,
    group_start_idx: u16,
    group_end_idx: u16,
    // booklet_num: u16,
    is_first_booklet: bool,
    is_last_booklet: bool,
    binding_rule: &BindingRule,
) -> Vec<(u16, u16)> {
    let mut res = Vec::new();
    let has_cover = binding_rule.has_cover;
    let keep_cover = binding_rule.keep_cover;
    let binding_at_middle = binding_rule.binding_at_middle;
    let mut page_idx = group_start_idx;
    while page_idx < group_end_idx {
        let mut page_low_idx = page_idx;
        let mut page_high_idx = group_end_idx - page_idx + group_start_idx - 1;
        // 第一册
        if is_first_booklet {
            if has_cover && keep_cover {
                if page_idx == 1 {
                    page_low_idx = u16::MAX;
                    // is_sheet_back = true;
                } else if page_idx > 1 {
                    // page_idx -= 1;
                    page_low_idx = page_idx - 1;
                } else {
                    // == 0
                    // is_sheet_back = false;
                }
            } else if has_cover && !keep_cover {
                if page_idx == 0 {
                    // group_start_idx = 1;
                }
            }
        }

        if page_low_idx < u16::MAX && page_low_idx >= page_high_idx {
            // 本册结束了
            // return None;
            break;
        }

        // 边缘装订
        if !binding_at_middle {
            if has_cover && keep_cover {}
            if is_first_booklet && is_last_booklet {
                page_high_idx = (group_end_idx - group_start_idx - 1) / 2 + page_idx;
            } else if is_first_booklet || is_last_booklet {
                page_high_idx = (group_end_idx - group_start_idx) / 2 + page_idx;
            } else {
                page_high_idx = (group_end_idx - group_start_idx + 1) / 2 + page_idx;
            }
        }
        if is_last_booklet {
            if has_cover && keep_cover {
                if page_high_idx == page_count - 1 {
                    page_high_idx = u16::MAX;
                } else if page_high_idx == group_end_idx - 1 {
                    page_high_idx = page_count - 1;
                } else if page_high_idx == group_end_idx - 2 {
                }
            } else if has_cover && !keep_cover {
                if page_high_idx >= page_count {
                    page_high_idx = u16::MAX;
                }
            }
        }
        res.push((page_low_idx, page_high_idx));
        page_idx += 1;
    }
    res
}

pub fn create_booklet_v2(
    src_pdf: &PdfDocumentHolder,
    binding_rule: &BindingRule,
    booklet_num: u16,
    is_last_booklet: bool,
    booklet_start_page: u16,
    booklet_end_page: u16,
    // booklet_doc: &mut Document,
) {
    let is_first_booklet = booklet_num == 1;

    let page_count = src_pdf.get_page_count();
    let is_auto_double_side = binding_rule.auto_double_side;
    let sheet_pages_vec = calc_page_on_sheet(
        page_count,
        booklet_start_page,
        booklet_end_page,
        is_first_booklet,
        is_last_booklet,
        binding_rule,
    );
    let len = sheet_pages_vec.len();
    let mut front_idx = 0;
    let mut back_idx: i32 = if is_auto_double_side {
        1
    } else {
        len as i32 - 1
    };
    // let mut high = 1;
    let mut front_doc = Document::new();
    let mut back_doc = Document::new();
    let mut booklet_doc = Document::new();
    write_pdf_metadata(src_pdf, &mut booklet_doc);
    if !is_auto_double_side {
        write_pdf_metadata(src_pdf, &mut front_doc);
        write_pdf_metadata(src_pdf, &mut back_doc);
        front_doc.set_title(format!("front booklet #{}", booklet_num));
        back_doc.set_title(format!("back booklet #{}", booklet_num));
    }
    let file_name = binding_rule
        .input_path
        .file_prefix()
        .expect("没有文件名")
        .to_string_lossy();
    booklet_doc.set_title(format!("booklet #{}", booklet_num));
    // let mut page_idx = booklet_start_page;
    while front_idx < len {
        let front_page_pair = sheet_pages_vec.get(front_idx).unwrap();
        let back_page_pair = sheet_pages_vec.get(back_idx as usize).unwrap();
        let front_sheet = create_front_sheet(src_pdf, booklet_num, front_page_pair, binding_rule);
        let back_sheet = crate_back_sheet(src_pdf, booklet_num, back_page_pair, binding_rule);
        front_idx = front_idx + 2;
        if is_auto_double_side {
            booklet_doc.add_page(front_sheet);
            booklet_doc.add_page(back_sheet);
            back_idx = back_idx + 2;
        } else {
            front_doc.add_page(front_sheet);
            back_doc.add_page(back_sheet);
            back_idx = back_idx - 2;
        }
    }
    if is_auto_double_side {
        booklet_doc
            .save(format!(
                "{}/{}_{:02}.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ))
            .unwrap();
    } else {
        front_doc
            .save(format!(
                "{}/{}_{:02}_po2.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ))
            .unwrap();
        back_doc
            .save(format!(
                "{}/{}_{:02}_po1.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ))
            .unwrap();
    }

    println!(
        "完成第{}册，共{}页, 开始页: {}, 结束页: {}",
        booklet_num,
        booklet_end_page - booklet_start_page,
        booklet_start_page,
        booklet_end_page
    );
}

fn create_front_sheet(
    src_pdf: &PdfDocumentHolder,
    booklet_num: u16,
    page_pair: &(u16, u16),
    binding_rule: &BindingRule,
) -> Page {
    create_one_sheet(src_pdf, booklet_num, page_pair, false, binding_rule)
}

fn crate_back_sheet(
    src_pdf: &PdfDocumentHolder,
    booklet_num: u16,
    page_pair: &(u16, u16),
    binding_rule: &BindingRule,
) -> Page {
    create_one_sheet(src_pdf, booklet_num, page_pair, true, binding_rule)
}

fn create_one_sheet(
    src_pdf: &PdfDocumentHolder,
    booklet_num: u16,
    page_pair: &(u16, u16),
    is_sheet_back: bool,
    binding_rule: &BindingRule,
) -> Page {
    let (page_low_idx, page_high_idx) = *page_pair;
    let binding_at_middle = binding_rule.binding_at_middle;
    let src_pdf_page_count = src_pdf.get_page_count();
    let img_low = if page_low_idx >= src_pdf_page_count {
        // 空白的情况，没有低页
        None
    } else {
        // 获取低页的图像数据
        let reverse_image = is_sheet_back;
        let (page_low_width, page_low_height, page_low_rgba) =
            src_pdf.get_page_image(page_low_idx, reverse_image);
        Some(
            oxidize_pdf::Image::from_rgba_data(page_low_rgba, page_low_width, page_low_height)
                .unwrap(),
        )
    };
    println!("{}, {}, {}", page_low_idx, page_high_idx, is_sheet_back);
    let img_high = if page_high_idx >= src_pdf_page_count {
        // 空白的情况，没有高页
        None
    } else {
        // 获取高页的图像数据
        let reverse_image = !(is_sheet_back ^ binding_at_middle);
        let (page_high_width, page_high_height, page_high_rgba) =
            src_pdf.get_page_image(page_high_idx, reverse_image);
        let img_high =
            oxidize_pdf::Image::from_rgba_data(page_high_rgba, page_high_width, page_high_height)
                .unwrap();
        Some(img_high)
    };
    let v_1mm_to_pt = 72.0 / 25.4;
    // 3mm
    let margin = 3.0 * v_1mm_to_pt;
    let mut new_page = Page::a4();
    let (w, h) = (new_page.width(), new_page.height());
    let half_h = h / 2.0;
    let half_w = w / 2.0;
    // 等比缩放
    // let margin_tb = margin * h / w / 2.0; ==> margin * h / 2.0 / w; ==> margin * (h / 2.0) / w;
    let margin_tb = margin * half_h / w;
    let margin_tb2 = 2.0 * margin_tb;
    // let v_1d5mm = 1.5*v_1mm_to_pt;
    let small_margin_tb = 0.6 * margin_tb;
    let ((img_bottom, img_bottom_idx, bottom_y), (img_top, img_top_idx, top_y)) =
        if binding_rule.binding_at_middle {
            (
                (img_low, page_low_idx, small_margin_tb),
                (
                    img_high,
                    page_high_idx,
                    half_h + margin_tb2 - small_margin_tb,
                ),
            )
        } else {
            (
                (img_high, page_high_idx, margin_tb2 - small_margin_tb),
                (img_low, page_low_idx, half_h + small_margin_tb),
            )
        };
    let img_width = w - 2.0 * margin;
    let img_height = half_h - margin_tb2;
    let v_12mm = 12.0 * v_1mm_to_pt;
    if let Some(img) = img_bottom {
        new_page.add_image(format!("{}", img_bottom_idx), img);
        new_page
            .draw_image(
                format!("{}", img_bottom_idx).as_str(),
                margin,
                bottom_y,
                img_width,
                img_height,
            )
            .unwrap();
    }
    if let Some(img) = img_top {
        new_page.add_image(format!("{}", img_top_idx), img);
        new_page
            .draw_image(
                format!("{}", img_top_idx).as_str(),
                margin,
                top_y,
                img_width,
                img_height,
            )
            .unwrap();
    }

    // 间隔12mm
    let dot_space = v_12mm;
    let padding = 6.0 * v_1mm_to_pt;
    let ((start_x, start_y), (to_x, to_y)) = if is_sheet_back {
        ((padding, half_h), (w, half_h))
    } else {
        ((w - padding, half_h), (0.0, half_h))
    };
    new_page
        .graphics()
        .set_stroke_color(Color::Gray(0.3))
        .move_to(start_x, start_y)
        .line_to(to_x, to_y)
        .set_line_dash_pattern(LineDashPattern::dotted(1.0, dot_space))
        .stroke();
    if !is_sheet_back {
        let _ = new_page
            .text()
            .set_font(Font::TimesRoman, 6.0)
            .at(half_w - 9.0 * v_1mm_to_pt, half_h)
            .write(format!("^- {} -^", booklet_num).as_str());
    }
    // let is_sheet_back;
    return new_page;
}

// fn calc_sheet_lh_page_idx(
//     page_count: u16,
//     page_idx: u16,
//     group_start_idx: u16,
//     group_end_idx: u16,
//     // booklet_num: u16,
//     is_first_booklet: bool,
//     is_last_booklet: bool,
//     binding_rule: &BindingRule,
// ) -> Option<(u16, u16, bool)> {
//     let has_cover = binding_rule.has_cover;
//     let keep_cover = binding_rule.keep_cover;
//     let mut page_low_idx = page_idx;
//     // let mut page_idx = page_idx;
//     let binding_at_middle = binding_rule.binding_at_middle;
//     let mut is_sheet_back = page_idx % 2 != 0;
//     // if true {
//     //     // todo 要考虑封面空白填充页的情况
//     //     let mut is_back = is_sheet_back;
//     //     let adjust_idx = if has_cover && keep_cover && page_idx >= 1 {
//     //         // 正面变为背面了
//     //         is_back = !is_back;
//     //         if page_idx == 1 { 0 } else { 1 }
//     //     } else {
//     //         0
//     //     };
//     //     if is_back {
//     //         page_low_idx = group_start_idx
//     //             + ((group_end_idx - group_start_idx) as f32 / 2.0f32).ceil() as u16
//     //             - (page_idx + adjust_idx - group_start_idx);
//     //     }
//     // }
//     let mut page_high_idx = group_end_idx - page_low_idx + group_start_idx - 1;

//     // 第一册
//     if is_first_booklet {
//         if has_cover && keep_cover {
//             if page_low_idx == 1 {
//                 page_low_idx = u16::MAX;
//                 // is_sheet_back = true;
//             } else if page_low_idx > 1 {
//                 // page_idx -= 1;
//                 page_low_idx = page_low_idx - 1;
//             } else {
//                 // == 0
//                 // is_sheet_back = false;
//             }
//         } else if has_cover && !keep_cover {
//             if page_low_idx == 0 {
//                 // group_start_idx = 1;
//             }
//             is_sheet_back = !is_sheet_back;
//         }
//     } else {
//         if has_cover {
//             is_sheet_back = !is_sheet_back;
//         }
//     }

//     if page_low_idx < group_start_idx || (page_low_idx < u16::MAX && page_low_idx >= page_high_idx)
//     {
//         // 本册结束了
//         return None;
//     }
//     // 边缘装订
//     if !binding_at_middle {
//         if has_cover && keep_cover {}
//         if is_first_booklet && is_last_booklet {
//             page_high_idx = (group_end_idx - group_start_idx - 1) / 2 + page_idx;
//         } else if is_first_booklet || is_last_booklet {
//             page_high_idx = (group_end_idx - group_start_idx) / 2 + page_idx;
//         } else {
//             page_high_idx = (group_end_idx - group_start_idx + 1) / 2 + page_idx;
//         }
//     }
//     if is_last_booklet {
//         // todo 结合自动双面判断
//         if has_cover && keep_cover {
//             if page_high_idx == page_count - 1 {
//                 page_high_idx = u16::MAX;
//             } else if page_high_idx == group_end_idx - 1 {
//                 page_high_idx = page_count - 1;
//                 is_sheet_back = false;
//             } else if page_high_idx == group_end_idx - 2 {
//                 is_sheet_back = true;
//             }
//         } else if has_cover && !keep_cover {
//             if page_high_idx >= page_count {
//                 page_high_idx = u16::MAX;
//                 is_sheet_back = page_idx % 2 == 0;
//             }
//         }
//     }
//     Some((page_low_idx, page_high_idx, is_sheet_back))
// }
