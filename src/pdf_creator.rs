use crate::{
    booklet::BindingRule,
};
use std::path::Path;

use lopdf::{
    Dictionary, Document, Object, ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};
// use crate::pdf_render::PdfDocumentHolder;
// use oxidize_pdf::Document;
// use oxidize_pdf::Font;
// use oxidize_pdf::Page;
// use oxidize_pdf::graphics::LineDashPattern;
// use pdfium_render::prelude::PdfDocumentMetadataTagType;

/// 创建册子
///
/// # 参数
/// * `src_pdf` - 源PDF文档容器
/// * `binding_rule` - 装订规则
/// * `booklet_num` - 册子编号
/// * `is_last_booklet` - 是否是最后一册
/// * `booklet_start_page` - 小册子开始页索引(包含)
/// * `booklet_end_page` - 小册子结束页索引(不包含)
pub fn create_booklet_v2(
    src_pdf: &Document,
    binding_rule: &BindingRule,
    booklet_num: i32,
    is_last_booklet: bool,
    booklet_start_page: i32,
    booklet_end_page: i32,
) {
    let is_first_booklet = booklet_num == 1;

    let page_count = src_pdf.get_pages().len() as i32;
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
    let mut front_doc = create_doc(&src_pdf);
    let mut back_doc = create_doc(&src_pdf);
    let mut booklet_doc = create_doc(&src_pdf);
    // write_pdf_metadata(src_pdf, &mut booklet_doc);
    // if !is_auto_double_side {
    //     write_pdf_metadata(src_pdf, &mut front_doc);
    //     write_pdf_metadata(src_pdf, &mut back_doc);
    //     front_doc.set_title(format!("front booklet #{}", booklet_num));
    //     back_doc.set_title(format!("back booklet #{}", booklet_num));
    // }
    let mut dst_page_ids = Vec::with_capacity(200);
    let mut front_dst_page_ids = Vec::with_capacity(200);
    let mut back_dst_page_ids = Vec::with_capacity(200);

    let file_name = binding_rule
        .input_path
        .file_prefix()
        .expect("没有文件名")
        .to_string_lossy();
    // booklet_doc.set_title(format!("booklet #{}", booklet_num));
    // let mut page_idx = booklet_start_page;
    while front_idx < len {
        let front_page_pair = sheet_pages_vec.get(front_idx).unwrap();
        let back_page_pair = sheet_pages_vec.get(back_idx as usize).unwrap();
        // let front_sheet = create_front_sheet(src_pdf, booklet_num, front_page_pair, binding_rule);
        // let back_sheet = crate_back_sheet(src_pdf, booklet_num, back_page_pair, binding_rule);
        front_idx = front_idx + 2;
        if is_auto_double_side {
            add_page(
                &src_pdf,
                &mut booklet_doc,
                &mut dst_page_ids,
                booklet_num,
                *front_page_pair,
                binding_rule.binding_at_middle,
                false,
            );
            add_page(
                &src_pdf,
                &mut booklet_doc,
                &mut dst_page_ids,
                booklet_num,
                *back_page_pair,
                binding_rule.binding_at_middle,
                true,
            );
            // booklet_doc.add_page(front_sheet);
            // booklet_doc.add_page(back_sheet);
            back_idx = back_idx + 2;
        } else {
            add_page(
                &src_pdf,
                &mut front_doc,
                &mut front_dst_page_ids,
                booklet_num,
                *front_page_pair,
                binding_rule.binding_at_middle,
                false,
            );
            add_page(
                &src_pdf,
                &mut back_doc,
                &mut back_dst_page_ids,
                booklet_num,
                *front_page_pair,
                binding_rule.binding_at_middle,
                true,
            );
            // front_doc.add_page(front_sheet);
            // back_doc.add_page(back_sheet);
            back_idx = back_idx - 2;
        }
    }
    if is_auto_double_side {
        save_pdf(
            &mut booklet_doc,
            dst_page_ids,
            format!(
                "{}/{}_{:02}.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ),
        );
    } else {
        save_pdf(
            &mut front_doc,
            front_dst_page_ids,
            format!(
                "{}/{}_{:02}_po2.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ),
        );
        save_pdf(
            &mut back_doc,
            back_dst_page_ids,
            format!(
                "{}/{}_{:02}_po1.pdf",
                binding_rule.output_dir.display(),
                file_name,
                booklet_num
            ),
        );
    }

    println!(
        "完成第{}册，共{}页, 开始页: {}, 结束页: {}",
        booklet_num,
        booklet_end_page - booklet_start_page,
        booklet_start_page,
        booklet_end_page
    );
}

pub fn create_doc(src_doc: &Document) -> Document {
    let mut dst_doc = Document::with_version("1.7");
    // 计算源文档的最大 ID，确保后续生成的对象 ID 不与源文档冲突
    let src_max_id = src_doc.objects.keys().map(|k| k.0).max().unwrap_or(0);
    dst_doc.max_id = src_max_id + 1;

    // 直接将源文档的对象池合并到目标文档中，保留所有原始 ID
    dst_doc.objects.extend(src_doc.objects.clone());
    return dst_doc;
}

/// 设置PDF文档的元数据
///
/// # 参数
/// * `src_pdf` - 源PDF文档容器
/// * `doc` - 目标PDF文档对象
// fn write_pdf_metadata(src_pdf: &PdfDocumentHolder<'_>, doc: &mut Document) {
//     let creator = format!(
//         "{} v{} - {}",
//         env!("CARGO_PKG_NAME"),
//         env!("CARGO_PKG_VERSION"),
//         env!("CARGO_PKG_DESCRIPTION")
//     );
//     doc.set_creator(creator);
//     // doc.set_producer(pkg_name);

//     if let Some(author) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Author) {
//         let author_value = author.value();
//         if !author_value.is_empty() {
//             doc.set_author(author_value);
//         }
//     }

//     if let Some(subject) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Subject) {
//         let subject_value = subject.value();
//         if !subject_value.is_empty() {
//             doc.set_subject(subject_value);
//         }
//     }

//     if let Some(keywords) = src_pdf.metadata().get(PdfDocumentMetadataTagType::Keywords) {
//         let keywords_value = keywords.value();
//         if !keywords_value.is_empty() {
//             doc.set_keywords(keywords_value);
//         }
//     }
// }

fn calc_page_on_sheet(
    page_count: i32,
    group_start_idx: i32,
    group_end_idx: i32,
    // booklet_num: u16,
    is_first_booklet: bool,
    is_last_booklet: bool,
    binding_rule: &BindingRule,
) -> Vec<(i32, i32)> {
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
                    page_low_idx = i32::MAX;
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

        if page_low_idx < i32::MAX && page_low_idx >= page_high_idx {
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
                    page_high_idx = i32::MAX;
                } else if page_high_idx == group_end_idx - 1 {
                    page_high_idx = page_count - 1;
                } else if page_high_idx == group_end_idx - 2 {
                }
            } else if has_cover && !keep_cover {
                if page_high_idx >= page_count {
                    page_high_idx = i32::MAX;
                }
            }
        }
        res.push((page_low_idx, page_high_idx));
        page_idx += 1;
    }
    res
}


pub fn save_pdf<P>(doc: &mut Document, dst_page_ids: Vec<ObjectId>, path: P)
where
    P: AsRef<Path>,
{
    add_pages_catalog(doc, dst_page_ids);
    doc.save(path).unwrap();
}

const A4_H: f32 = 842.0;
const A4_W: f32 = 595.0;
const PAGE_H: f32 = A4_W;
const PAGE_W: f32 = A4_H;
const PAGE_W_HALF: f32 = A4_H / 2.0;
pub fn add_page(
    src_doc: &Document,
    dst_doc: &mut Document,
    dst_page_ids: &mut Vec<ObjectId>,
    booklet_num: i32,
    page_pair: (i32, i32),
    binding_at_middle: bool,
    is_sheet_back: bool,
) {
    dbg!(&page_pair);
    let lpage_num = if is_sheet_back {
        page_pair.1
    } else {
        page_pair.1
    };
    let rpage_num = if is_sheet_back {
        page_pair.0
    } else {
        page_pair.0
    };
    let rotate_180 = !binding_at_middle;

    let mut l_content_flow = Vec::with_capacity(100);
    let l_id = build_half_page(
        src_doc,
        dst_doc,
        lpage_num,
        true,
        false || is_sheet_back,
        &mut l_content_flow,
    );
    let mut r_content_flow = Vec::with_capacity(100);
    let r_id = build_half_page(
        src_doc,
        dst_doc,
        rpage_num,
        false,
        rotate_180 ^ is_sheet_back,
        &mut r_content_flow,
    );
    let l_form = if let Some(id) = l_id {
        Some((id, l_content_flow, lpage_num))
    } else {
        None
    };
    let r_form = if let Some(id) = r_id {
        Some((id, r_content_flow, rpage_num))
    } else {
        None
    };
    add_page_flow(
        dst_doc,
        dst_page_ids,
        l_form,
        r_form,
        booklet_num,
        is_sheet_back,
    );
}

fn build_half_page(
    src_doc: &Document,
    dst_doc: &mut Document,
    page_num: i32,
    left: bool,
    rotate_180: bool,
    content_flow: &mut Vec<Operation>,
) -> Option<ObjectId> {
    let pages = src_doc.get_pages();
    if page_num < 0 || page_num >= pages.len() as i32 {
        dbg!("page_num out of range");
        return None;
    }
    dbg!(page_num + 1);
    let page_id = pages.get(&((page_num + 1) as u32)).unwrap();
    if let Ok(page_obj) = src_doc.get_object(*page_id) {
        let src_page_dict = page_obj.as_dict().unwrap();

        // 提取尺寸并计算缩放因子（假设源页面等比缩放放入 A4 的一半）
        let media_box = src_page_dict.get(b"MediaBox").unwrap().as_array().unwrap();
        let src_width = media_box[2].as_float().unwrap();
        let src_height = media_box[3].as_float().unwrap();
        let scale = (PAGE_W_HALF / src_width).min(PAGE_H / src_height);

        // 4. 提取 TrimBox，如果没有则回退到 MediaBox
        let trim_box = match src_page_dict.get(b"TrimBox").and_then(|v| v.as_array()) {
            Ok(arr) => arr,
            Err(_) => media_box, // 如果源文档没有 TrimBox，则使用 MediaBox 作为裁切边界
        };

        let trim_x = trim_box[0].as_float().unwrap_or(0.0);
        let trim_y = trim_box[1].as_float().unwrap_or(0.0);
        let trim_w = trim_box[2].as_float().unwrap_or(src_width) - trim_x;
        let trim_h = trim_box[3].as_float().unwrap_or(src_height) - trim_y;

        let space_y = 0f32.max((PAGE_H - src_height * scale) / 2.0);
        let space_x = 0f32.max((PAGE_W_HALF - src_width * scale) / 2.0);

        // cos(θ) sin(θ) -sin(θ) cos(θ) tx ty cm
        // a = sx, b = 0, c = 0, d = sy, e = tx, f = ty
        let ctm_sx = if rotate_180 { -scale } else { scale };
        let ctm_b = "0";
        let ctm_c = "0";
        let ctm_sy = if rotate_180 { -scale } else { scale };
        // let ctm_tx = if left { space_x } else { PAGE_W_HALF + space_x };
        let ctm_tx = if left { space_x } else { PAGE_W_HALF + space_x };
        let ctm_tx = if rotate_180 {
            if left {
                PAGE_W_HALF + space_x
            } else {
                PAGE_W - 2.0 * space_x
            }
        } else {
            ctm_tx
        };
        let ctm_ty = if rotate_180 {
            PAGE_H - space_y
        } else {
            space_y
        };
        // let ctm_sx = scale;
        // let ctm_b = 0;
        // let ctm_c = 0;
        // let ctm_sy = scale;
        // let ctm_tx = if left { 0.0 } else { PAGE_W_HALF };
        // let ctm_ty = 0;
        let ctm = format!(
            "{} {} {} {} {} {}",
            ctm_sx, ctm_b, ctm_c, ctm_sy, ctm_tx, ctm_ty
        );

        // 提取资源字典（内部的 ID 引用完全不需要修改）
        let resources = src_page_dict
            .get(b"Resources")
            .unwrap_or(&Object::Dictionary(dictionary! {}))
            .clone();

        // 提取并解压内容流字节
        let contents_obj = src_page_dict.get(b"Contents").unwrap();
        let mut stream_bytes = Vec::new();

        match contents_obj {
            Object::Reference(id) => {
                if let Ok(obj) = dst_doc.get_object_mut(*id) {
                    if let Ok(stream) = obj.as_stream() {
                        // let _ = stream.decompress();
                        stream_bytes.extend_from_slice(&stream.decompressed_content().unwrap());
                    }
                }
            }
            Object::Array(arr) => {
                for item in arr {
                    if let Ok(id) = item.as_reference() {
                        if let Ok(obj) = dst_doc.get_object(id) {
                            if let Ok(stream) = obj.as_stream() {
                                // let _ = stream.decompress();
                                stream_bytes
                                    .extend_from_slice(&stream.decompressed_content().unwrap());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        // 包裹图形状态隔离，并应用缩放与平移矩阵
        let mut safe_stream_bytes = Vec::with_capacity(2000);
        safe_stream_bytes.extend_from_slice(b"q\n");
        // 应用缩放与平移矩阵
        safe_stream_bytes.extend_from_slice(format!("{} cm\n", ctm).as_bytes());
        // safe_stream_bytes
        //     .extend_from_slice(format!("{} 0 0 {} {} 0 cm\n", scale, scale, 0).as_bytes());
        // 设置裁切路径 (基于 TrimBox 的坐标)
        safe_stream_bytes.extend_from_slice(
            format!("{} {} {} {} re W n\n", trim_x, trim_y, trim_w, trim_h).as_bytes(),
        );
        safe_stream_bytes.append(&mut stream_bytes);
        safe_stream_bytes.extend_from_slice(b"\nQ\n");

        // 构造 Form XObject
        let form_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
            "Resources" => resources,
        };
        let mut form_stream = Stream::new(form_dict, safe_stream_bytes);
        let _ = form_stream.compress(); // 重新压缩以减小体积
        let form_xobject_id = dst_doc.add_object(Object::Stream(form_stream));

        // 构建目标页面的内容流
        let form_xobject_name = format!("bkb_SrcPage_{}", page_num);
        content_flow.extend(vec![
            Operation::new("q", vec![]),
            Operation::new("Do", vec![Object::Name(form_xobject_name.into())]), // 描边
            Operation::new("Q", vec![]),
        ]);
        return Some(form_xobject_id);
    } else {
        None
    }
}

type ContentFlow = Vec<Operation>;
type FormObj = Option<(ObjectId, ContentFlow, i32)>;

fn add_page_flow(
    dst_doc: &mut Document,
    dst_page_ids: &mut Vec<ObjectId>,
    left_form: FormObj,
    right_form: FormObj,
    booklet_num: i32,
    is_sheet_back: bool,
) {
    let mut merge_content_flow = Vec::with_capacity(60);
    let mut xobject_dict = dictionary!();
    merge_page_resources(&mut merge_content_flow, &mut xobject_dict, left_form);
    merge_page_resources(&mut merge_content_flow, &mut xobject_dict, right_form);

    const dot_wid: f32 = 1.5;
    const dot_space: f32 = 12.0f32 * 72.0 / 25.4; // 12mm
    const dot_num: f32 = (PAGE_H / dot_space).floor();
    const start_y: f32 = (PAGE_H - (dot_space + dot_wid) * dot_num) / 2.0;
    merge_content_flow.extend_from_slice(&[
        // --- 绘制正中间的灰色小圆点虚线 ---
        Operation::new("q", vec![]),     // 1. 保存图形状态
        Operation::new("0.2 G", vec![]), // 2. 设置描边颜色为 50% 灰色
        Operation::new(&format!("{} w", dot_wid), vec![]), // 3. 设置线宽为 0.5
        Operation::new("1 J", vec![]),   // 4. 设置线帽为圆头（Round Cap）
        Operation::new(
            "d",
            vec![
                // 5. 设置虚线样式 [1 3] 0 d (画1点，空3点)
                Object::Array(vec![0.into(), 34.into()]),
                0.into(),
            ],
        ),
        Operation::new("m", vec![PAGE_W_HALF.into(), start_y.into()]), // 6. 移动到 (421, 0)
        Operation::new("l", vec![PAGE_W_HALF.into(), PAGE_H.into()]),  // 7. 画线到 (421, 595)
        Operation::new("S", vec![]),                                   // 8. 描边
        Operation::new("Q", vec![]),                                   // 9. 恢复图形状态
    ]);
    if !is_sheet_back {
        merge_content_flow.extend_from_slice(&[
            // 在页面正中心绘制页码数字 ---
            Operation::new("q", vec![]),     // 保存图形状态
            Operation::new("BT", vec![]),    // Begin Text
            Operation::new("0.4 g", vec![]), // 设置文字颜色为同样的灰色
            Operation::new(
                "Tf",
                vec![
                    // 设置字体和大小 (使用内置的 Helvetica)
                    Object::Name("_bkb_F1".into()),
                    5.into(),
                ],
            ),
            Operation::new(
                "Tm",
                vec![
                    0.into(),
                    1.into(),
                    (-1).into(),
                    0.into(),
                    PAGE_W_HALF.into(),
                    (start_y + (dot_wid + dot_space) * dot_num / 2.0 + dot_space / 3.0).into(),
                ],
            ), // 移动到页面视觉正中心 (X: 418.5, Y: 291.5)
            Operation::new(
                "Tj",
                vec![Object::String(
                    format!("^_{}_^", booklet_num).into_bytes(),
                    lopdf::StringFormat::Literal,
                )],
            ), // 绘制数字
            Operation::new("ET", vec![]), // End Text
            Operation::new("Q", vec![]),  // 恢复图形状态
        ]);
    };
    let content = Content {
        operations: merge_content_flow,
    };
    let content_id = dst_doc.add_object(Object::Stream(Stream::new(
        dictionary! {},
        content.encode().unwrap(),
    )));
    let resources = dictionary! {
        "XObject" => xobject_dict,
        "Font" => dictionary! {
            "bkb_F1" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica", // PDF 内置标准字体，无需嵌入文件
            },
        },
    };
    let page_id = dst_doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Page",
        "Parent" => Object::Reference((0, 0)),
        "MediaBox" => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
        "Contents" => Object::Reference(content_id),
        "Resources" => resources,
    }));

    dst_page_ids.push(page_id);
}

fn add_pages_catalog(dst_doc: &mut Document, dst_page_ids: Vec<ObjectId>) {
    // 构建 Pages 和 Catalog 树
    let new_pages_id = dst_doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => dst_page_ids.iter().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
        "Count" => dst_page_ids.len() as u32,
    }));
    // let new_pages_id = dst_doc.new_object_id();
    // dst_doc.objects.insert(
    //     new_pages_id,
    //     Object::Dictionary(dictionary! {
    //         "Type" => "Pages",
    //         "Kids" => dst_page_ids.iter().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
    //         "Count" => dst_page_ids.len() as u32,
    //     }),
    // );

    let new_catalog_id = dst_doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(new_pages_id),
    }));
    // 修复所有目标页面的 Parent 引用
    for &page_id in &dst_page_ids {
        if let Some(Object::Dictionary(dict)) = dst_doc.objects.get_mut(&page_id) {
            dict.set("Parent", Object::Reference(new_pages_id));
        }
    }

    dst_doc
        .trailer
        .set("Root", Object::Reference(new_catalog_id));
    dst_doc.compress();
    dst_doc
        .catalog_mut()
        .unwrap()
        .as_hashmap_mut()
        .swap_remove(&"Outlines".to_string().into_bytes());
    let _ = dst_doc.save("260703.pdf").unwrap();
}

fn merge_page_resources(
    content_flow: &mut Vec<Operation>,
    xobject_dict: &mut Dictionary,
    form_obj: FormObj,
) {
    if let Some(form) = form_obj {
        let form_xobject_id = form.0;
        content_flow.extend(form.1);
        let form_xobject_name = format!("bkb_SrcPage_{}", form.2);
        xobject_dict.as_hashmap_mut().insert(
            form_xobject_name.into_bytes(),
            Object::Reference(form_xobject_id),
        );
        // xobject_dict.extend(&dictionary! {
        //     form_xobject_name => Object::Reference(form_xobject_id),
        // });
    }
}
