//! ZCAD原生文件格式（.zcad）
//!
//! 基于 MessagePack + Zstd 的紧凑二进制格式：
//! - 体积小：MessagePack 比 JSON 小 30-50%，Zstd 再压缩 60-80%
//! - 速度快：无需 SQL 解析，直接序列化/反序列化
//! - 简单可靠：无外部数据库依赖

use crate::document::{Document, DocumentMetadata, SavedView};
use crate::error::FileError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use zcad_core::entity::Entity;
use zcad_core::layer::Layer;

/// 文件魔数 "ZCAD"
const MAGIC: &[u8; 4] = b"ZCAD";

/// 当前文件格式版本
const FORMAT_VERSION: u32 = 2;

/// Zstd 压缩级别（1-22，3 是默认值，平衡速度和压缩比）
const COMPRESSION_LEVEL: i32 = 3;

/// 文件头（16 字节）
#[derive(Debug)]
struct FileHeader {
    /// 魔数 "ZCAD"
    magic: [u8; 4],
    /// 格式版本
    version: u32,
    /// 标志位（预留）
    flags: u32,
    /// 压缩后数据长度
    compressed_size: u32,
}

impl FileHeader {
    fn new(compressed_size: u32) -> Self {
        Self {
            magic: *MAGIC,
            version: FORMAT_VERSION,
            flags: 0,
            compressed_size,
        }
    }

    fn write(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.flags.to_le_bytes())?;
        writer.write_all(&self.compressed_size.to_le_bytes())?;
        Ok(())
    }

    fn read(reader: &mut impl Read) -> Result<Self, FileError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if &magic != MAGIC {
            return Err(FileError::InvalidFormat(
                "Invalid magic number, not a ZCAD file".to_string(),
            ));
        }

        let mut buf = [0u8; 4];

        reader.read_exact(&mut buf)?;
        let version = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let flags = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let compressed_size = u32::from_le_bytes(buf);

        Ok(Self {
            magic,
            version,
            flags,
            compressed_size,
        })
    }
}

/// 可序列化的文件内容
#[derive(Debug, Serialize, Deserialize)]
struct FileContent {
    /// 文档元数据
    metadata: DocumentMetadata,
    /// 所有图层
    layers: Vec<Layer>,
    /// 所有实体
    entities: Vec<Entity>,
    /// 保存的视图
    views: Vec<SavedView>,
}

/// 保存文档到文件
pub fn save(document: &Document, path: &Path) -> Result<(), FileError> {
    // 收集文件内容
    let content = FileContent {
        metadata: document.metadata.clone(),
        layers: document.layers.all_layers().iter().cloned().collect(),
        entities: document.all_entities().cloned().collect(),
        views: document.views.clone(),
    };

    // 序列化为 MessagePack
    let msgpack_data = rmp_serde::to_vec(&content)?;

    // 使用 Zstd 压缩
    let compressed_data = zstd::encode_all(msgpack_data.as_slice(), COMPRESSION_LEVEL)?;

    // 写入文件
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // 写入文件头
    let header = FileHeader::new(compressed_data.len() as u32);
    header.write(&mut writer)?;

    // 写入压缩数据
    writer.write_all(&compressed_data)?;
    writer.flush()?;

    tracing::info!(
        "Saved {} entities, {} layers to {} ({} bytes compressed)",
        content.entities.len(),
        content.layers.len(),
        path.display(),
        compressed_data.len()
    );

    Ok(())
}

/// 从文件加载文档
pub fn load(path: &Path) -> Result<Document, FileError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // 读取文件头
    let header = FileHeader::read(&mut reader)?;

    // 版本检查
    if header.version > FORMAT_VERSION {
        return Err(FileError::UnsupportedVersion(format!(
            "File version {} is newer than supported version {}",
            header.version, FORMAT_VERSION
        )));
    }

    // 读取压缩数据
    let mut compressed_data = vec![0u8; header.compressed_size as usize];
    reader.read_exact(&mut compressed_data)?;

    // 解压缩
    let msgpack_data = zstd::decode_all(compressed_data.as_slice())?;

    // 反序列化
    let content: FileContent = rmp_serde::from_slice(&msgpack_data)?;

    // 重建文档
    let mut document = Document::new();
    document.metadata = content.metadata;

    // 重建图层管理器
    document.layers = zcad_core::layer::LayerManager::new();
    for layer in content.layers.into_iter().skip(1) {
        // 跳过默认图层0
        document.layers.add_layer(layer);
    }

    // 加载实体
    for entity in content.entities {
        document.entities_mut().insert(entity.id, entity);
    }

    // 加载视图
    document.views = content.views;

    // 重建空间索引
    document.rebuild_spatial_index();

    tracing::info!(
        "Loaded {} entities, {} layers from {}",
        document.entity_count(),
        document.layers.count(),
        path.display()
    );

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zcad_core::geometry::{Geometry, Line};
    use zcad_core::math::Point2;

    #[test]
    fn test_save_load_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_document.zcad");

        // 创建文档
        let mut doc = Document::new();
        doc.metadata.title = "Test Document".to_string();

        let line = Line::new(Point2::new(0.0, 0.0), Point2::new(100.0, 100.0));
        let entity = Entity::new(Geometry::Line(line));
        doc.add_entity(entity);

        // 保存
        save(&doc, &file_path).expect("Failed to save");

        // 验证文件头
        let file = File::open(&file_path).expect("Failed to open");
        let mut reader = BufReader::new(file);
        let header = FileHeader::read(&mut reader).expect("Failed to read header");
        assert_eq!(&header.magic, MAGIC);
        assert_eq!(header.version, FORMAT_VERSION);

        // 加载
        let loaded = load(&file_path).expect("Failed to load");

        assert_eq!(loaded.metadata.title, "Test Document");
        assert_eq!(loaded.entity_count(), 1);

        // 清理
        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_invalid_magic() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_invalid.zcad");

        // 写入无效的魔数
        let mut file = File::create(&file_path).expect("Failed to create");
        file.write_all(b"XXXX").expect("Failed to write");
        file.write_all(&[0u8; 12]).expect("Failed to write padding");

        // 尝试加载应该失败
        let result = load(&file_path);
        assert!(result.is_err());

        // 清理
        std::fs::remove_file(&file_path).ok();
    }
}
