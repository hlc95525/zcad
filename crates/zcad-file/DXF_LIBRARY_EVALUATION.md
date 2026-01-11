# DXF 库评估报告

## 概述

本文档评估是否需要创建 `libdxfrw` 的 Rust 绑定来替换当前使用的 `dxf` crate。

## 当前实现分析

### 使用的 `dxf` crate (v0.6)

**已支持的实体类型：**
- ✅ Line
- ✅ Circle
- ✅ Arc
- ✅ LwPolyline / Polyline
- ✅ Point
- ✅ Text / MText
- ✅ RotatedDimension (线性/对齐标注)
- ✅ RadialDimension (半径标注)
- ✅ DiameterDimension (直径标注)

**缺失的实体类型：**
- ❌ Ellipse (椭圆)
- ❌ Spline (样条曲线)
- ❌ Hatch (填充)
- ❌ Leader (引线)
- ❌ 3DFace
- ❌ Solid (实心填充)
- ❌ Ray / XLine (射线/构造线)
- ❌ Insert (块参照)
- ❌ Image
- ❌ Viewport
- ❌ Tolerance (形位公差)
- ❌ Angular Dimension (角度标注)
- ❌ Ordinate Dimension (坐标标注)

### LibreCAD 的 libdxfrw

**支持的实体类型：**（来自 drw_entities.h）
- 3DFACE, ARC, BLOCK, CIRCLE, DIMENSION (6种), ELLIPSE, HATCH, IMAGE, INSERT, LEADER, LINE, LWPOLYLINE, MTEXT, POINT, POLYLINE, RAY, SOLID, SPLINE, TEXT, TOLERANCE, TRACE, UNDERLAY, VERTEX, VIEWPORT, XLINE

**额外功能：**
- DWG 格式支持 (AutoCAD 二进制格式)
- 更多 DXF 版本兼容性
- 完整的块(Block)支持
- 外部参照(XRef)支持

## 评估结论

### 推荐：暂时不创建 libdxfrw 绑定

**理由：**

1. **当前功能足够基础使用**
   - `dxf` crate 已支持最常用的实体类型
   - 标注支持已经比较完整

2. **创建 FFI 绑定成本高**
   - libdxfrw 是 C++ 库，需要 `cxx` 或 `bindgen`
   - 需要维护构建脚本和跨平台编译
   - 内存管理和生命周期处理复杂

3. **替代方案**
   - 可以直接扩展 `dxf` crate 的功能
   - 或者提交 PR 给 `dxf` crate

### 建议的改进路径

**短期 (v0.2):**
1. 参考 libdxfrw 的 `drw_entities.cpp` 实现，为 `dxf` crate 补充：
   - Ellipse 支持
   - Angular Dimension 支持
   
2. 完善颜色转换（支持更多 ACI 颜色）

**中期 (v0.3):**
1. 添加 Spline 支持
2. 添加 Hatch 支持
3. 添加 Leader 支持

**长期 (v1.0):**
如果需要 DWG 格式支持，考虑：
1. 创建 libdxfrw 的 Rust 绑定
2. 或使用 ODA (Open Design Alliance) SDK

## 参考文件

### 如需扩展 DXF 支持，参考以下 libdxfrw 文件：

```
../librecad/libraries/libdxfrw/src/drw_entities.cpp  - 实体解析
../librecad/libraries/libdxfrw/src/drw_entities.h    - 实体定义
../librecad/libraries/libdxfrw/src/dxfreader.cpp     - DXF 读取
../librecad/libraries/libdxfrw/src/dxfwriter.cpp     - DXF 写入
../librecad/libraries/libdxfrw/src/intern/dwgreader*.cpp - DWG 读取
```

### 椭圆实现参考

libdxfrw 中椭圆的关键字段：
- center: 中心点
- majorAxisEndPoint: 长轴端点（相对于中心）
- ratio: 短轴/长轴比例
- startAngle, endAngle: 起止角度

### Spline 实现参考

libdxfrw 中样条的关键字段：
- degree: 阶数
- knotslist: 节点向量
- controllist: 控制点
- fitlist: 拟合点
- tgStart, tgEnd: 起止切向量

## 结论

**当前建议：不创建 libdxfrw 绑定**

原因：
1. 现有 `dxf` crate 满足基础需求
2. FFI 绑定维护成本高
3. 可通过扩展现有 crate 或贡献上游来解决

只有在以下情况下考虑创建绑定：
- 需要 DWG 格式支持
- 需要完整的 DXF 版本兼容性
- 需要复杂的块/外部参照支持
