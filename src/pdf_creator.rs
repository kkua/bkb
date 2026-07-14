use crate::{booklet::BindingRule, cache};
use std::{path::Path, str::FromStr};

use lopdf::{
    Dictionary, Document,
    Object::{self, Reference},
    ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};

/// 创建册子
///
/// # 参数
/// * `src_pdf` - 源PDF文档容器
/// * `binding_rule` - 装订规则
/// * `booklet_num` - 册子编号
/// * `is_last_booklet` - 是否是最后一册
/// * `booklet_start_page` - 小册子开始页索引(包含)
/// * `booklet_end_page` - 小册子结束页索引(不包含)
pub fn do_create_booklet(
    src_pdf: &Document,
    binding_rule: &BindingRule,
    booklet_num: i32,
    is_last_booklet: bool,
    booklet_start_page: i32,
    booklet_end_page: i32,
) {
    let is_first_booklet = booklet_num == 1;

    if is_first_booklet {
        cache::clear();
    }
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
const MM_2_PT: f32 = 1.0 * 72.0 / 25.4;
const MARGIN_X: f32 = MM_2_PT * 2.0;
const PRINT_BOX_W: f32 = PAGE_W_HALF - MARGIN_X * 2.0;
const PRINT_BOX_H: f32 = PAGE_H * PRINT_BOX_W / PAGE_W_HALF;
const MARGIN_Y: f32 = (PAGE_H - PRINT_BOX_H) / 2.0;
// const L_BOX_X: f32 = MM_2_PT;
// const L_BOX_Y: f32 = PAGE_H - PADDING_TB;
// const R_BOX_X: f32 = PAGE_W_HALF + PADDING_LR;
// const R_BOX_Y: f32 = L_BOX_Y;

pub fn add_page(
    src_doc: &Document,
    dst_doc: &mut Document,
    dst_page_ids: &mut Vec<ObjectId>,
    booklet_num: i32,
    page_pair: (i32, i32),
    binding_at_middle: bool,
    is_sheet_back: bool,
) {
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
        return None;
    }
    dbg!(page_num + 1);
    let page_id = pages.get(&((page_num + 1) as u32)).unwrap();
    if let Ok(page_obj) = src_doc.get_object(*page_id) {
        let src_page_dict = page_obj.as_dict().unwrap();

        // let media_box_obj = src_page_dict.get(b"MediaBox").unwrap();

        // let media_box = match media_box_obj {
        //     Reference(ref_id) => src_doc.get_object(*ref_id).unwrap().as_array().unwrap(),
        //     Object::Array(_arr) => media_box_obj.as_array().unwrap(),
        //     _ => panic!("不支持的对象类型,{:?}", media_box_obj),
        // };
        // 提取尺寸并计算缩放因子（假设源页面等比缩放放入 A4 的一半）
        // let media_box = src_page_dict.get(b"MediaBox").unwrap().as_array().unwrap();
        // let src_width = media_box[2].as_float().unwrap();
        // let src_height = media_box[3].as_float().unwrap();
        let media_box = get_page_box(src_page_dict, src_doc, "MediaBox").unwrap();
        let src_width = *media_box.get(2).unwrap();
        let src_height = *media_box.get(3).unwrap();
        let scale = (PRINT_BOX_W / src_width).min(PRINT_BOX_H / src_height);

        // 提取 TrimBox，如果没有则回退到 MediaBox
        // let trim_box = match src_page_dict.get(b"TrimBox").and_then(|v| v.as_array()) {
        //     Ok(arr) => arr,
        //     Err(_) => media_box, // 如果源文档没有 TrimBox，则使用 MediaBox 作为裁切边界
        // };
        let trim_box = get_page_box(src_page_dict, src_doc, "TrimBox").unwrap_or(media_box);
        // let trim_x = trim_box[0].as_float().unwrap_or(0.0);
        // let trim_y = trim_box[1].as_float().unwrap_or(0.0);
        // let trim_w = trim_box[2].as_float().unwrap_or(src_width) - trim_x;
        // let trim_h = trim_box[3].as_float().unwrap_or(src_height) - trim_y;
        let trim_x = *trim_box.get(0).unwrap_or(&0.0);
        let trim_y = *trim_box.get(1).unwrap_or(&0.0);
        let trim_w = *trim_box.get(2).unwrap_or(&src_width) - trim_x;
        let trim_h = *trim_box.get(3).unwrap_or(&src_height) - trim_y;

        let space_y = 0f32.max((PRINT_BOX_H - src_height * scale) / 2.0);
        let space_x = 0f32.max((PRINT_BOX_W - src_width * scale) / 2.0);

        // cos(θ) sin(θ) -sin(θ) cos(θ) tx ty cm
        // a = sx, b , c , d = sy, e = tx, f = ty
        let ctm_sx = if rotate_180 { -scale } else { scale };
        let ctm_b = "0";
        let ctm_c = "0";
        let ctm_sy = if rotate_180 { -scale } else { scale };
        let ctm_tx = if left {
            space_x + MARGIN_X
        } else {
            PAGE_W_HALF + MARGIN_X + space_x
        };
        let ctm_tx = if rotate_180 {
            if left {
                PRINT_BOX_W + MARGIN_X + space_x
            } else {
                PRINT_BOX_W + PAGE_W_HALF + MARGIN_X + space_x
            }
        } else {
            ctm_tx
        };
        let ctm_ty = if rotate_180 {
            PAGE_H - MARGIN_Y - space_y
        } else {
            MARGIN_Y + space_y
        };

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
        let form_xobject_name = format!("_bkb_op_{}", page_num);
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

fn parse_page_box_size_vec(str: String) -> Vec<f32> {
    str.split(",")
        .map(f32::from_str)
        .map(|i| i.unwrap_or(0.0))
        .collect()
}

fn page_size_to_str(box_size: &Vec<f32>) -> String {
    box_size
        .iter()
        .map(f32::to_string)
        .collect::<Vec<String>>()
        .join(",")
}

fn get_page_box(page_dict: &Dictionary, doc: &Document, box_key: &str) -> Option<Vec<f32>> {
    fn get_box(dict: &Dictionary, doc: &Document, box_key: &str) -> Option<Vec<f32>> {
        let obj = dict.get(box_key.as_bytes());
        if obj.is_err() {
            return None;
        }
        let obj = obj.unwrap();
        let box_obj = match obj {
            Reference(ref_id) => Some(doc.get_object(*ref_id).unwrap().as_array().unwrap()),
            Object::Array(_arr) => Some(obj.as_array().unwrap()),
            _ => None,
        };
        if let Some(rect) = box_obj {
            return Some(vec![
                rect[0].as_float().unwrap(),
                rect[1].as_float().unwrap(),
                rect[2].as_float().unwrap(),
                rect[3].as_float().unwrap(),
            ]);
        }
        return None;
    }

    if let Some(obj) = get_box(page_dict, doc, box_key) {
        // let media_box = match obj {
        //     Reference(ref_id) => doc.get_object(*ref_id).unwrap().as_array().unwrap(),
        //     Object::Array(_arr) => obj.as_array().unwrap(),
        //     _ => panic!("不支持的对象类型,{:?}", obj),
        // };
        // media_box
        Some(obj)
    } else {
        // 尝试从父级 Pages 节点继承
        if let Some(box_rect_str) = cache::get_data::<String>(box_key) {
            if box_rect_str.is_empty() {
                return None;
            }
            return Some(parse_page_box_size_vec(box_rect_str));
        }

        fn get_box_from_parent(
            dict: &Dictionary,
            doc: &Document,
            box_key: &str,
        ) -> Option<Vec<f32>> {
            if let Ok(parent_ref) = dict.get(b"Parent") {
                if let Ok(parent_dict) = doc.get_object(parent_ref.as_reference().unwrap()) {
                    let dict = parent_dict.as_dict().unwrap();
                    if let Some(box_size) = get_box(dict, doc, box_key) {
                        cache::add_data(box_key, page_size_to_str(box_size.as_ref()).as_str());
                        return Some(box_size);
                    } else {
                        return get_box_from_parent(dict, doc, box_key);
                    }
                } else {
                    cache::add_data(box_key, "");
                    None
                }
            } else {
                cache::add_data(box_key, "");
                None
            }
        }

        return get_box_from_parent(page_dict, doc, box_key);
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

    const DOT_WID: f32 = 1.5;
    const DOT_SPACE: f32 = 12.0 * MM_2_PT; // 12mm
    const DOT_NUM: f32 = (PAGE_H / DOT_SPACE).floor();
    const START_Y: f32 = (PAGE_H - (DOT_SPACE + DOT_WID) * DOT_NUM) / 2.0;
    merge_content_flow.extend_from_slice(&[
        // --- 绘制正中间的灰色小圆点虚线 ---
        Operation::new("q", vec![]),                       // 保存图形状态
        Operation::new("0.8 G", vec![]), // 设置描边灰度为0.8，取值范围0~1.0数字越大越浅
        Operation::new(&format!("{} w", DOT_WID), vec![]), // 设置线宽
        Operation::new("1 J", vec![]),   // 设置线帽为圆头（Round Cap）
        Operation::new(
            "d",
            vec![
                // 设置虚线样式 [0 34] 0 d (画1点，空34点)
                Object::Array(vec![0.into(), 34.into()]),
                0.into(),
            ],
        ),
        Operation::new("m", vec![PAGE_W_HALF.into(), START_Y.into()]), // 移动到 (421, 0)
        Operation::new("l", vec![PAGE_W_HALF.into(), PAGE_H.into()]),  // 画线到 (421, 595)
        Operation::new("S", vec![]),                                   // 描边
        Operation::new("Q", vec![]),                                   // 恢复图形状态
    ]);
    if !is_sheet_back {
        merge_content_flow.extend_from_slice(&[
            // 在页面正中心绘制页码数字 ---
            Operation::new("q", vec![]),     // 保存图形状态
            Operation::new("BT", vec![]),    // Begin Text
            Operation::new("0.7 g", vec![]), // 设置文字颜色为浅灰色
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
                    (START_Y + (DOT_WID + DOT_SPACE) * DOT_NUM / 2.0 + DOT_SPACE / 3.0).into(),
                ],
            ), // 移动到页面中缝虚线空白处
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
            "_bkb_F1" => dictionary! {
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
}

fn merge_page_resources(
    content_flow: &mut Vec<Operation>,
    xobject_dict: &mut Dictionary,
    form_obj: FormObj,
) {
    if let Some(form) = form_obj {
        let form_xobject_id = form.0;
        content_flow.extend(form.1);
        let form_xobject_name = format!("_bkb_op_{}", form.2);
        // xobject_dict.as_hashmap_mut().insert(
        //     form_xobject_name.into_bytes(),
        //     Object::Reference(form_xobject_id),
        // );
        xobject_dict.extend(&dictionary! {
            form_xobject_name => Object::Reference(form_xobject_id),
        });
    }
}
