use lopdf::Document;

use crate::pdf_creator;
use std::path::PathBuf;

#[derive(Debug)]
pub struct BindingRule {
    /// 输入PDF文件路径
    pub input_path: PathBuf,
    /// 输出目录（默认源文件所在目录下的out文件夹）
    pub output_dir: PathBuf,
    /// 每个小册子的A4纸数量（默认10张，即40页）
    pub sheets_per_booklet: usize,
    /// 是否自动双面。若为否则使用手动双面模式，打印时需要注意先偶数页后奇数页（输出文件名已标记好打印顺序，print order先po1后po2）
    pub auto_double_side: bool,
    /// 装订方式（默认为true:在中间装订）
    pub binding_at_middle: bool,
    // 是否有封面封底（第一页和最后一页）
    pub has_cover: bool,
    // 是否仅打印正文（不保留封面封底）
    pub keep_cover: bool,
    // /// 是否在首页前添加空白页作为封面
    // pub add_blank_cover: bool,
    // /// 是否添加页码
    // pub add_page_numbers: bool,
    // /// 页码格式
    // pub page_number_format: PageNumberFormat,
    // /// 页码位置
    // pub page_number_position: PageNumberPosition,
}

impl Default for BindingRule {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            output_dir: PathBuf::new(),
            sheets_per_booklet: 10,
            auto_double_side: true,
            binding_at_middle: true,
            has_cover: false,
            keep_cover: false,
        }
    }
}

#[allow(dead_code)]
impl BindingRule {
    pub fn new(input_path: &PathBuf) -> Self {
        Self {
            input_path: input_path.clone(),
            output_dir: input_path.parent().unwrap().join("out"),
            ..Default::default()
        }
    }

    pub fn set_output_path(mut self, out_path: &Option<PathBuf>) -> Self {
        if let Some(path) = out_path {
            self.output_dir = path.clone();
        }
        self
    }
}

pub struct BookletConfig {
    pub booklet_sheets: u32,
    // 加1张纸的册子数量
    pub add_sheet_booklet_count: u32,
    /// 最后一册的填充页数
    pub tail_pad_page: u32,
    // /// 源文件有封面封底
    // has_cover: bool,
    // /// 保留封面封底
    // keep_cover: bool,
}

/// 计算每册的纸张数量
fn calc_booklet_sheets(
    page_count: u32,
    sheets_per_booklet: u32,
    has_cover: bool,
    keep_cover: bool,
) -> BookletConfig {
    // let last_add = page_count % 4;
    // 对齐到4的倍数
    // let mut keep_cover = keep_cover;
    let page_count = if has_cover {
        if keep_cover {
            // 封面和封底背面各增加一张空白页
            page_count + 2
        } else {
            page_count - 2
        }
    } else {
        // keep_cover = false;
        page_count
    };

    let total = ((page_count + 3) / 4) * 4;
    let last_add = total - page_count;
    // println!(
    //     "末尾添加{}页空白页。若在其他位置插入请先自行修改源PDF",
    //     last_add
    // );
    // 每册对应的页数
    let pages_per_booklet = sheets_per_booklet * 4;
    // 获取册数
    let mut booklet_count = total / pages_per_booklet;
    // 最后一册的页数
    let last_booklet_sheets = total % pages_per_booklet;
    let mut booklet_sheets = sheets_per_booklet;
    // 重新分配每册页数
    if last_booklet_sheets / 4 <= booklet_count {
        // 最后一册全部分给前几册，每册多分1张纸
        let res = BookletConfig {
            booklet_sheets,
            add_sheet_booklet_count: last_booklet_sheets / 4,
            tail_pad_page: last_add,
            // has_cover,
            // keep_cover,
        };
        // booklet_sheets += 1;
        return res;
    } else if last_booklet_sheets * 4 < pages_per_booklet * 3 {
        // 最后一册纸张数小于期望页数的3/4，册数不变，页数均分
        booklet_count += 1;
        // booklet_sheets 一定会小于 paper_count_per_booklet
        booklet_sheets = total / booklet_count / 4;
        // remain_booklet_sheets 一定会小于 booklet_sheets
        let remain_booklet_sheets = (total - booklet_sheets * 4 * booklet_count) / 4;
        let booklet_config = BookletConfig {
            booklet_sheets,
            add_sheet_booklet_count: remain_booklet_sheets,
            tail_pad_page: last_add,
            // has_cover,
            // keep_cover,
        };
        return booklet_config;
    } else {
        BookletConfig {
            booklet_sheets,
            add_sheet_booklet_count: 0,
            tail_pad_page: last_add,
            // has_cover,
            // keep_cover,
        }
    }
}

pub fn create_booklet(src_pdf: &Document, binding_rule: &BindingRule) {
    let has_cover = binding_rule.has_cover;
    let keep_cover = binding_rule.keep_cover;
    let src_page_cnt = src_pdf.get_pages().len() as i32;
    let (mut page_idx, page_count) = if has_cover && !keep_cover {
        (1, src_page_cnt - 2)
    } else {
        (0, src_page_cnt)
    };
    let booklet_config = calc_booklet_sheets(
        page_count as u32,
        binding_rule.sheets_per_booklet as u32,
        has_cover,
        keep_cover,
    );
    let mut booklet_idx = 0;

    let pages_per_booklet = (booklet_config.booklet_sheets * 4) as i32;
    while page_idx < page_count {
        let booklet_start_page = page_idx;
        let mut booklet_end_page = booklet_start_page + pages_per_booklet;
        if (booklet_idx as u32) < booklet_config.add_sheet_booklet_count {
            booklet_end_page += 4;
        }
        let is_last_booklet = booklet_end_page >= page_count;
        if booklet_end_page > page_count {
            booklet_end_page = page_count + booklet_config.tail_pad_page as i32;
        }
        if has_cover && keep_cover {
            if booklet_idx == 0 {
                booklet_end_page -= 1;
            } else if is_last_booklet {
                // 封面背面的空白页已经包含，只需要加1
                booklet_end_page += 1;
            }
        }
        booklet_idx += 1;
        pdf_creator::do_create_booklet(
            src_pdf,
            binding_rule,
            booklet_idx,
            is_last_booklet,
            booklet_start_page,
            booklet_end_page,
        );
        page_idx = booklet_end_page;
    }
}
