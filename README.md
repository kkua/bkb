# BKB - Booklet Builder

一个用于将大型PDF文件拆分为多个小册子（booklet）基于Rust的工具。
注意打印时需要使用双面模式，并选择长边翻转
## 功能

- 将大型PDF文件按指定纸张数量拆分为多个小册子
- 自动计算每册的最佳页数分配
- 支持智能页数对齐（自动对齐到4的倍数）
- 保留原始PDF的页面内容
- 按小册子模式重新排版成PDF文件，适配中间装订（线装或胶装）、两边装订（仅适用于胶装，必须裁开）。
- 添加中缝装订线
- 手动双面打印功能，缓解使用70g打印纸时自动双面容易出现卡纸的问题

## 实现中的功能
- 自动设置页码
- 图形操作界面
- 批量处理多个PDF

## 规划中的功能（低优先级）
- 调用打印机打印，自动设置打印参数

## 工作原理
本工具的原理是将PDF渲染为图片，然后将图片并排在一页A4纸中，生成小册子。由于是转换成图片，因此生成的小册子文件会比原始PDF文件大很多，且打印出来的文字会出现一定的模糊。计划使用mupdf库通过对象嵌入方式解决，如果有朋友知道更好的解决方案，欢迎提issue或PR。


## 构建流程

### 1. 安装Rust环境

确保已安装Rust环境，然后克隆项目：

```bash
git clone https://codeberg.org/Kaay/bcfbh
cd bcfbh
```

### 2. 准备Pdfium动态链接库

本项目使用 [pdfium-render](https://crates.io/crates/pdfium-render) 库进行PDF渲染，需要Pdfium动态链接库支持。

#### 下载Pdfium库

从 [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) 下载对应平台的预编译库：

- **Windows**: 下载 `pdfium-windows-x64.tgz` 或 `pdfium-windows-x86.tgz`
- **Linux**: 下载 `pdfium-linux-x64.tgz` 或 `pdfium-linux-arm64.tgz`
- **macOS**: 下载 `pdfium-mac-x64.tgz` 或 `pdfium-mac-arm64.tgz`

#### 放置库文件

在项目根目录创建 `lib` 文件夹，将下载的Pdfium库文件放入其中：

```
bdfb/
├── lib/
│   ├── pdfium.dll          # Windows
│   ├── libpdfium.so        # Linux
│   └── libpdfium.dylib     # macOS
├── src/
└── ...
```

> **注意**: 程序运行时会从 `./lib/` 目录加载Pdfium动态链接库。

### 3. 构建项目

```bash
cargo build --release
```

## 使用方法

编辑 `src/main.rs` 中的配置参数：

```rust
    let pdfium = pdf_render::init_pdfium();
    let filename = "input.pdf";
    let input_path = PathBuf::from(filename);
    let booklet_config = booklet::BindingRule{
        input_path: PathBuf::from("input.pdf"),      // 输入PDF文件路径
        output_dir: PathBuf::from("out"),              // 输出目录
        sheets_per_booklet: 10,                        // 每个小册子的A4纸张数量（默认10张，即40页）
    };
    let src_pdf = pdf_render::PdfDocumentHolder::new(&pdfium, &input_path, None);
    dbg!(src_pdf.get_page_count());
    booklet::create_booklet(&src_pdf, &binding_rule);
```

然后运行：

```bash
cargo run --release
```

## 配置参数

### BindingRule 结构体

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `input_path` | `PathBuf` | - | 输入PDF文件的完整路径 |
| `output_dir` | `PathBuf` | 源文件所在目录下的`out`文件夹 | 输出目录路径 |
| `sheets_per_booklet` | `usize` | 10 | 每个小册子包含的A4纸张数量，每张纸可打印4页（双面打印，每面2页） |
| `binding_at_middle` | `bool` | `true` | 装订方式，`true`为中间装订，`false`为两边装订 |
| `auto_double_side` | `bool` | `true` | 是否自动双面。若为否则使用手动双面模式，打印时需要注意先打印偶数页后奇数页（本程序已经将奇数偶数页拆分到不同的文件，在文件名中标记了打印顺序，print order先po1后po2） |

## 输出文件

程序将生成多个PDF文件，命名格式为 `${src_filename}_XX.pdf`，其中 `XX` 为两位数序号（如 `input_01.pdf`, `input_02.pdf` 等）。

## 算法说明

拆分算法会智能处理以下情况：

1. **页数对齐**：自动将总页数对齐到4的倍数（因为每张A4纸可打印4页）
2. **均匀分配**：当剩余页数适中时，会将页数均匀分配到各册
3. **增量分配**：当剩余页数较少时，去除最后一册，前几册会多分配1张纸

## 项目结构

```
bdfb/
├── Cargo.toml          # 项目配置
├── src/
│   ├── main.rs         # 程序入口
│   ├── booklet.rs      # 小册子拆分逻辑和配置结构体
│   ├── pdf_creator.rs  # PDF小册子页面创建
│   └── pdf_render.rs   # PDF渲染和页面图像提取
└── README.md           # 本文件
```

## 许可证

MIT License
