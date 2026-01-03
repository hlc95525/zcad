# ZCAD - 下一代开源 CAD 系统

<div align="center">

![ZCAD Logo](docs/assets/logo.svg)

**快速 • 开放 • 现代**

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.83+-orange.svg)](https://www.rust-lang.org/)

</div>

## 🎯 愿景

ZCAD 致力于成为传统 CAD 软件的现代替代品，解决以下痛点：

- **性能问题**：传统 CAD 在大图形下卡顿严重，即使是高端硬件也无法流畅操作
- **封闭生态**：专有格式、昂贵授权、功能受限
- **糟糕的对象处理**：如 xclip 爆炸产生大量无用对象

## ✨ 核心特性

### 🚀 极致性能

- **GPU 加速渲染**：基于 WebGPU/wgpu，充分利用现代 GPU
- **多线程架构**：几何计算、渲染、UI 完全并行
- **智能 LOD**：远距离自动简化，保持流畅
- **增量更新**：只重绘变化的部分

### 📐 精确的几何内核

- 自研 2D/3D 几何引擎
- 精确的布尔运算
- 智能的对象爆炸和分解
- 约束求解器

### 📁 开放的文件格式

- `.zcad` 原生格式（基于 SQLite）
- 完整的 DXF 读写支持
- 可扩展的插件系统

### 🎨 现代 UI

- 深色主题，护眼设计
- 自定义工具栏和快捷键
- 命令行界面（类 AutoCAD）
- 多视口支持

## 🛠️ 技术栈

| 组件 | 技术                 | 说明                 |
| ---- | -------------------- | -------------------- |
| 语言 | Rust                 | 安全、高性能         |
| 渲染 | wgpu                 | 跨平台 GPU 加速      |
| UI   | egui                 | 即时模式 GUI         |
| 几何 | nalgebra + parry     | 向量/矩阵 + 碰撞检测 |
| 存储 | SQLite + FlatBuffers | 高效的数据持久化     |

## 📦 项目结构

```
zcad/
├── crates/
│   ├── zcad-core/       # 核心几何引擎
│   ├── zcad-renderer/   # GPU渲染器
│   ├── zcad-file/       # 文件格式处理
│   ├── zcad-ui/         # 用户界面组件
│   └── zcad-app/        # 主应用程序
├── docs/                 # 文档
└── examples/            # 示例文件
```

## 🚀 快速开始

### 下载预编译版本

前往 [Releases 页面](https://github.com/zcad/zcad/releases) 下载适合您系统的版本：

- **Windows**: `zcad-windows-x64.zip`
- **macOS (Intel)**: `zcad-macos-x64.tar.gz`
- **macOS (Apple Silicon)**: `zcad-macos-arm64.tar.gz`
- **Linux**: `zcad-linux-x64.tar.gz`

#### Windows

1. 解压 zip 文件
2. 双击 `zcad.exe` 运行
3. **系统要求**: Windows 10 (1809+) 或 Windows 11，支持 DirectX 12

#### macOS

```bash
# 解压
tar -xzf zcad-macos-*.tar.gz
cd zcad

# 运行
./zcad
```

#### Linux

```bash
# 解压
tar -xzf zcad-linux-x64.tar.gz
cd zcad

# 可能需要安装依赖
sudo apt install libxcb-render0 libxcb-shape0 libxcb-xfixes0 libxkbcommon0

# 运行
./zcad
```

### 从源码构建

#### 前置要求

- Rust 1.83+
- 支持 Vulkan/Metal/DX12 的 GPU

#### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/zcad/zcad.git
cd zcad

# 构建并运行
cargo run --release

# 或仅构建
cargo build --release
# 可执行文件位于: target/release/zcad (或 zcad.exe)
```

#### 发布打包

```bash
# 本地平台
./scripts/build-native.sh

# Windows 交叉编译（需要 mingw-w64）
./scripts/build-windows.sh
```

### ⌨️ 快捷键

#### 文件操作

- `Ctrl+N` - 新建文档
- `Ctrl+O` - 打开文件 (.zcad / .dxf)
- `Ctrl+S` - 保存
- `Ctrl+Shift+S` - 另存为

#### 绘图工具

- `L` - 直线
- `C` - 圆
- `R` - 矩形
- `Space` - 选择工具

#### 视图操作

- `Z` - 缩放至全部
- `G` - 切换网格显示
- `F8` - 切换正交模式
- `鼠标滚轮` - 缩放视图
- `鼠标中键拖动` - 平移视图

#### 编辑操作

- `Del` - 删除选中对象
- `Esc` - 取消当前操作

## 🗺️ 路线图

### Phase 1: 基础框架 (当前)

- [ ] 核心几何库（点、线、圆、弧、多段线）
- [ ] GPU 渲染管线
- [ ] 基础 UI 框架
- [ ] 文件格式规范

### Phase 2: 基础功能

- [ ] 绘图命令（LINE, CIRCLE, ARC, PLINE 等）
- [ ] 编辑命令（MOVE, COPY, ROTATE, SCALE 等）
- [ ] 图层管理
- [ ] 捕捉和追踪

### Phase 3: 高级功能

- [ ] 块和外部参照
- [ ] 标注系统
- [ ] 打印/导出
- [ ] DXF 互操作

### Phase 4: 生态建设

- [ ] 插件系统
- [ ] 脚本支持（Lua/Python）
- [ ] 在线协作

## 📄 许可证

双重许可：MIT 或 Apache-2.0，由您选择。

## 🤝 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。
