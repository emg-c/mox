use std::fs;
use std::path::{Path, PathBuf};

use mox_payload::{Payload, PayloadError};

use crate::{safe_file_name, zip_dir, ExportError};

pub const EXECUTABLE: &str = "mox-player";

/// 이름마다 다른 번들 ID. 같은 이름이면 다시 추출해도 같다.
pub fn bundle_id(name: &str) -> String {
    format!("com.mox.player.m{:08x}", crc32fast::hash(name.as_bytes()))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn info_plist(name: &str, bundle_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>ko</string>
  <key>CFBundleExecutable</key>
  <string>{exe}</string>
  <key>CFBundleIdentifier</key>
  <string>{id}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>{name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>{ver}</string>
  <key>CFBundleVersion</key>
  <string>{ver}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
"#,
        exe = EXECUTABLE,
        id = xml_escape(bundle_id),
        name = xml_escape(name),
        ver = env!("CARGO_PKG_VERSION"),
    )
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(()) // Windows에는 실행 비트가 없다. zip에는 zip_dir가 0755를 직접 기록한다.
}

/// dir 아래에 서명 전 `<이름>.app`을 만든다.
pub fn write_bundle(dir: &Path, stub: &[u8], payload: &Payload) -> Result<PathBuf, ExportError> {
    let name = &payload.mox.meta.name;
    let app = dir.join(format!("{}.app", safe_file_name(name)));
    let contents = app.join("Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(contents.join("Resources"))?;
    fs::write(
        contents.join("Info.plist"),
        info_plist(name, &bundle_id(name)),
    )?;
    let exe = contents.join("MacOS").join(EXECUTABLE);
    fs::write(&exe, stub)?;
    set_executable(&exe)?;
    let mox_text = mox_format::to_string(&payload.mox).map_err(PayloadError::from)?;
    fs::write(contents.join("Resources").join("macro.mox"), mox_text)?;
    fs::write(
        contents.join("Resources").join("export.json"),
        payload.settings.to_json(),
    )?;
    Ok(app)
}

fn sign(unsigned: &Path, signed: &Path) -> Result<(), ExportError> {
    let signer = apple_codesign::BundleSigner::new_from_path(unsigned)
        .map_err(|e| ExportError::Sign(e.to_string()))?;
    // 서명 키를 지정하지 않으면 ad-hoc 서명이 된다
    let settings = apple_codesign::SigningSettings::default();
    signer
        .write_signed_bundle(signed, &settings)
        .map_err(|e| ExportError::Sign(e.to_string()))?;
    Ok(())
}

/// 번들을 조립하고 ad-hoc 서명한 뒤 zip으로 묶는다. work는 비어 있는 임시 폴더.
pub fn export_macos_zip(
    stub: &[u8],
    payload: &Payload,
    work: &Path,
) -> Result<Vec<u8>, ExportError> {
    let unsigned = write_bundle(&work.join("unsigned"), stub, payload)?;
    let signed_root = work.join("signed");
    fs::create_dir_all(&signed_root)?;
    let app_name = unsigned.file_name().expect("번들 경로에는 이름이 있다");
    sign(&unsigned, &signed_root.join(app_name))?;
    let exe_suffix = format!("/Contents/MacOS/{EXECUTABLE}");
    zip_dir(&signed_root, |rel| rel.ends_with(&exe_suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::payload;

    #[test]
    fn bundle_id_is_stable_and_valid() {
        let id = bundle_id("주간보고 정리");
        assert_eq!(id, bundle_id("주간보고 정리"));
        assert_ne!(id, bundle_id("다른 매크로"));
        assert!(id.starts_with("com.mox.player.m"));
        assert!(id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'));
        assert_eq!(id.len(), "com.mox.player.m".len() + 8);
    }

    #[test]
    fn info_plist_escapes_name_and_sets_required_keys() {
        let plist = info_plist("A&B <c>", "com.mox.player.m00000000");
        assert!(plist.contains("<string>A&amp;B &lt;c&gt;</string>"));
        for key in [
            "CFBundleExecutable",
            "CFBundleIdentifier",
            "CFBundlePackageType",
            "LSMinimumSystemVersion",
        ] {
            assert!(plist.contains(&format!("<key>{key}</key>")), "{key}");
        }
        assert!(plist.contains("<string>mox-player</string>"));
        assert!(plist.contains("<string>12.0</string>"));
    }

    #[test]
    fn writes_bundle_layout() {
        let dir = tempfile::tempdir().unwrap();
        let p = payload("주간/보고");
        let app = write_bundle(dir.path(), b"stub", &p).unwrap();
        assert_eq!(app.file_name().unwrap(), "주간_보고.app");
        let c = app.join("Contents");
        assert_eq!(std::fs::read(c.join("MacOS/mox-player")).unwrap(), b"stub");
        assert_eq!(
            mox_format::load(&c.join("Resources/macro.mox")).unwrap(),
            p.mox
        );
        assert_eq!(
            std::fs::read_to_string(c.join("Resources/export.json")).unwrap(),
            p.settings.to_json()
        );
        assert!(std::fs::read_to_string(c.join("Info.plist"))
            .unwrap()
            .contains(&bundle_id("주간/보고")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(c.join("MacOS/mox-player"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    /// 실제 universal 재생기로 서명까지 확인한다. Task 5에서 MOX_MACOS_STUB를 지정해 실행한다.
    #[test]
    #[ignore]
    fn signs_real_universal_stub() {
        let stub = std::fs::read(std::env::var("MOX_MACOS_STUB").expect("MOX_MACOS_STUB")).unwrap();
        let work = tempfile::tempdir().unwrap();
        let zip_bytes = export_macos_zip(&stub, &payload("검증용 매크로"), work.path()).unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
        let mut names = Vec::new();
        for i in 0..archive.len() {
            let f = archive.by_index(i).unwrap();
            names.push((
                f.name().unwrap().into_owned(),
                f.unix_mode().unwrap_or(0) & 0o777,
            ));
        }
        assert!(
            names.contains(&("검증용 매크로.app/Contents/MacOS/mox-player".into(), 0o755)),
            "{names:?}"
        );
        assert!(
            names
                .iter()
                .any(|(n, _)| n == "검증용 매크로.app/Contents/_CodeSignature/CodeResources"),
            "{names:?}"
        );
    }
}
