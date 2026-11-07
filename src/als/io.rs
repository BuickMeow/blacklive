use std::io::{Cursor, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use flate2::{Compression, GzBuilder};
use xmltree::{Element, EmitterConfig};

/// Read an `.als` file and return the decompressed XML bytes.
///
/// Both gzip-compressed `.als` and plain XML are accepted, which makes debugging
/// a hand-extracted template easier.
pub fn read_als_xml(path: &Path) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("读取 .als 失败: {}", path.display()))?;
    read_als_bytes(&raw, &path.display().to_string())
}

/// Decompress an `.als` byte slice. Plain XML is accepted for debugging.
pub fn read_als_bytes(raw: &[u8], label: &str) -> Result<Vec<u8>> {
    if raw.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = MultiGzDecoder::new(raw);
        let mut xml = Vec::new();
        decoder
            .read_to_end(&mut xml)
            .with_context(|| format!("解压 gzip 失败: {label}"))?;
        if xml.is_empty() {
            bail!(".als 解压后是空文件: {label}");
        }
        Ok(xml)
    } else {
        if raw.is_empty() {
            bail!(".als 是空文件: {label}");
        }
        Ok(raw.to_vec())
    }
}

/// Write the decompressed XML next to the user for manual inspection.
pub fn dump_template_xml(template: &Path, output: &Path) -> Result<()> {
    let xml = read_als_xml(template)?;
    write_xml_file(output, &xml)
}

pub fn dump_als_bytes_xml(label: &str, raw: &[u8], output: &Path) -> Result<()> {
    let xml = read_als_bytes(raw, label)?;
    write_xml_file(output, &xml)
}

pub(crate) fn write_xml_file(output: &Path, xml: &[u8]) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, xml).with_context(|| format!("写入 XML 失败: {}", output.display()))?;
    Ok(())
}
#[inline]
pub(crate) fn parse_xml(xml: &[u8]) -> Result<Element> {
    Element::parse(Cursor::new(xml)).context("模板 XML 解析失败：请确认它是有效的 Ableton .als")
}

pub(crate) fn serialize_xml(root: &Element) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(1024 * 1024);
    out.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    root.write_with_config(
        &mut out,
        EmitterConfig::new().write_document_declaration(false),
    )
    .context("序列化 Ableton XML 失败")?;
    Ok(out)
}

/// Compress XML and atomically replace the destination `.als`.
pub(crate) fn write_als_atomic(output: &Path, xml: &[u8]) -> Result<()> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    encoder.write_all(xml).context("gzip 压缩 XML 失败")?;
    let gz = encoder.finish().context("完成 gzip 压缩失败")?;

    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("创建输出目录失败: {}", parent.display()))?;
    let file_name = output
        .file_name()
        .context("输出路径没有文件名")?
        .to_string_lossy();
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));

    std::fs::write(&tmp, &gz).with_context(|| format!("写入临时文件失败: {}", tmp.display()))?;
    if let Err(err) = std::fs::rename(&tmp, output) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err).with_context(|| format!("替换输出文件失败: {}", output.display()));
    }
    Ok(())
}
