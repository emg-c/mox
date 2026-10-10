use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::ExportError;

fn options(mode: u32) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::DEFAULT) // 같은 입력이면 같은 zip
        .unix_permissions(mode)
}

/// root 아래 전체를 zip으로 묶는다. 항목은 이름순, 폴더 0755, 파일 0644, is_executable이 참이면 0755.
pub fn zip_dir(root: &Path, is_executable: impl Fn(&str) -> bool) -> Result<Vec<u8>, ExportError> {
    let mut entries = Vec::new();
    collect(root, root, &mut entries)?;
    entries.sort();
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (rel, is_dir) in entries {
        if is_dir {
            zip.add_directory(format!("{rel}/"), options(0o755))?;
        } else {
            let mode = if is_executable(&rel) { 0o755 } else { 0o644 };
            zip.start_file(rel.as_str(), options(mode))?;
            zip.write_all(&fs::read(root.join(&rel))?)?;
        }
    }
    Ok(zip.finish()?.into_inner())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, bool)>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let rel = path
            .strip_prefix(root)
            .expect("root 아래 경로")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if path.is_dir() {
            out.push((rel, true));
            collect(root, &path, out)?;
        } else {
            out.push((rel, false));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("한글.app/MacOS")).unwrap();
        std::fs::write(dir.path().join("한글.app/b.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("한글.app/MacOS/run"), b"#!").unwrap();
        dir
    }

    #[test]
    fn zips_sorted_entries_with_permissions() {
        let dir = tree();
        let bytes = zip_dir(dir.path(), |rel| rel.ends_with("/MacOS/run")).unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut got = Vec::new();
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).unwrap();
            let mut body = Vec::new();
            f.read_to_end(&mut body).unwrap();
            got.push((
                f.name().unwrap().into_owned(),
                f.is_dir(),
                f.unix_mode().unwrap_or(0) & 0o777,
                body,
            ));
        }
        assert_eq!(
            got,
            vec![
                ("한글.app/".into(), true, 0o755, vec![]),
                ("한글.app/MacOS/".into(), true, 0o755, vec![]),
                ("한글.app/MacOS/run".into(), false, 0o755, b"#!".to_vec()),
                ("한글.app/b.txt".into(), false, 0o644, b"hello".to_vec()),
            ]
        );
    }

    #[test]
    fn output_is_deterministic() {
        let dir = tree();
        let a = zip_dir(dir.path(), |_| false).unwrap();
        let b = zip_dir(dir.path(), |_| false).unwrap();
        assert_eq!(a, b);
    }
}
