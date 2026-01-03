# ZCAD 文件格式规范

## 版本

当前版本：2

## 概述

ZCAD 使用基于 **MessagePack + Zstd** 的紧凑二进制格式（`.zcad`），这种设计带来以下优势：

- **体积小**：MessagePack 比 JSON 小 30-50%，Zstd 再压缩 60-80%
- **速度快**：无需 SQL 解析，直接序列化/反序列化
- **单文件**：易于分发和管理
- **简单可靠**：无外部数据库依赖
- **跨平台**：Rust 原生实现，所有平台表现一致

## 文件结构

```
┌─────────────────────────────────────┐
│           文件头 (16 bytes)          │
├─────────────────────────────────────┤
│ Magic: "ZCAD" (4 bytes)             │
│ Version: u32 LE (4 bytes)           │
│ Flags: u32 LE (4 bytes, 预留)        │
│ Compressed Size: u32 LE (4 bytes)   │
├─────────────────────────────────────┤
│     Zstd 压缩的 MessagePack 数据     │
│ ┌─────────────────────────────────┐ │
│ │ metadata: DocumentMetadata      │ │
│ │ layers: Vec<Layer>              │ │
│ │ entities: Vec<Entity>           │ │
│ │ views: Vec<SavedView>           │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### 文件头

| 偏移 | 大小 | 类型 | 描述 |
|------|------|------|------|
| 0 | 4 | bytes | 魔数 "ZCAD" |
| 4 | 4 | u32 LE | 格式版本号 |
| 8 | 4 | u32 LE | 标志位（预留） |
| 12 | 4 | u32 LE | 压缩后数据长度 |

### 压缩参数

- 压缩算法：Zstd
- 压缩级别：3（平衡速度和压缩比）

## 数据结构

### DocumentMetadata

```rust
struct DocumentMetadata {
    id: Uuid,                              // 文档唯一标识
    title: String,                         // 文档标题
    author: String,                        // 作者
    created_at: DateTime<Utc>,             // 创建时间
    modified_at: DateTime<Utc>,            // 最后修改时间
    format_version: u32,                   // 文件格式版本
    units: String,                         // 单位 (mm, cm, m, inch, feet)
    custom_properties: HashMap<String, String>,  // 自定义属性
}
```

### Layer

```rust
struct Layer {
    id: EntityId,                          // 图层ID
    name: String,                          // 图层名称
    color: Color,                          // 颜色
    line_type: LineType,                   // 线型
    line_weight: LineWeight,               // 线宽
    visible: bool,                         // 可见性
    locked: bool,                          // 锁定状态
    frozen: bool,                          // 冻结状态
    plottable: bool,                       // 可打印
    description: String,                   // 描述
}
```

### Entity

```rust
struct Entity {
    id: EntityId,                          // 实体ID
    geometry: Geometry,                    // 几何数据
    properties: Properties,                // 属性
    layer_id: EntityId,                    // 所属图层
    visible: bool,                         // 可见性
    locked: bool,                          // 锁定状态
}
```

### Geometry 类型

| 类型 | 描述 |
|------|------|
| Point | 点 |
| Line | 线段 |
| Circle | 圆 |
| Arc | 圆弧 |
| Polyline | 多段线 |
| Ellipse | 椭圆 |
| Spline | 样条曲线 |
| Text | 文字 |
| Hatch | 填充 |

### SavedView

```rust
struct SavedView {
    name: String,                          // 视图名称
    center_x: f64,                         // 中心X坐标
    center_y: f64,                         // 中心Y坐标
    zoom: f64,                             // 缩放级别
}
```

## 文件操作

### 保存文件

1. 收集文档数据到 `FileContent` 结构
2. 使用 MessagePack 序列化
3. 使用 Zstd 压缩
4. 写入文件头
5. 写入压缩数据

### 加载文件

1. 读取并验证文件头魔数
2. 检查版本兼容性
3. 读取压缩数据
4. 使用 Zstd 解压缩
5. 使用 MessagePack 反序列化
6. 重建图层管理器
7. 加载实体
8. 重建空间索引

## 版本兼容性

- 低版本软件无法打开高版本文件
- 高版本软件可以打开低版本文件（向后兼容）
- 版本升级时可能需要数据迁移

## 与 DXF 的互操作

ZCAD 支持导入和导出 DXF 格式。

### 导入映射

| DXF实体 | ZCAD几何 |
|---------|----------|
| LINE | Line |
| CIRCLE | Circle |
| ARC | Arc |
| LWPOLYLINE | Polyline |
| POLYLINE | Polyline |
| POINT | Point |

### 导出映射

所有 ZCAD 几何都可以导出为对应的 DXF 实体。

### 颜色映射

使用 AutoCAD 颜色索引(ACI)与 RGB 之间的标准映射。

| ACI | 颜色 | RGB |
|-----|------|-----|
| 1 | Red | #FF0000 |
| 2 | Yellow | #FFFF00 |
| 3 | Green | #00FF00 |
| 4 | Cyan | #00FFFF |
| 5 | Blue | #0000FF |
| 6 | Magenta | #FF00FF |
| 7 | White | #FFFFFF |
| 256 | ByLayer | - |
| 0 | ByBlock | - |
