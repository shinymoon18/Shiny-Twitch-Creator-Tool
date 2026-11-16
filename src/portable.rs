use std::{fs::{self, File}, io::{Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, sync::OnceLock};
use serde::Deserialize;

const MAGIC: &[u8; 16] = b"ATCUT_BUNDLE_001";
const TRAILER_SIZE: u64 = 56;
static TOOLS: OnceLock<PathBuf> = OnceLock::new();
pub fn tools_dir() -> Option<&'static Path> { TOOLS.get().map(PathBuf::as_path) }

#[derive(Deserialize)]
struct Manifest { files: Vec<Entry> }
#[derive(Deserialize)]
struct Entry { name: String, size: u64, packed: u64 }

fn extract(executable: &Path, data: &Path) -> Result<Option<PathBuf>, String> {
    let mut file = File::open(executable).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    if length < TRAILER_SIZE { return Ok(None); }
    file.seek(SeekFrom::End(-(TRAILER_SIZE as i64))).map_err(|e| e.to_string())?;
    let mut trailer = [0u8; 56]; file.read_exact(&mut trailer).map_err(|e| e.to_string())?;
    if &trailer[40..] != MAGIC { return Ok(None); }
    let packed = u64::from_le_bytes(trailer[..8].try_into().unwrap());
    if packed < 8 || packed > length - TRAILER_SIZE { return Err(tr!("The portable bundle is incomplete. Copy the executable again.").into()); }
    let offset = length - TRAILER_SIZE - packed;
    let version = trailer[8..24].iter().map(|b| format!("{b:02x}")).collect::<String>();
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut bytes = [0u8; 8]; file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let manifest_size = u64::from_le_bytes(bytes);
    if manifest_size > 65536 || manifest_size > packed - 8 { return Err(tr!("The portable bundle index is invalid.").into()); }
    let mut bytes = vec![0; manifest_size as usize]; file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| tf!("Invalid portable bundle: {e}"))?;
    if manifest.files.is_empty() || manifest.files.len() > 32 { return Err(tr!("The bundle does not contain valid tools.").into()); }
    let mut total = 8 + manifest_size;
    for entry in &manifest.files {
        if entry.name.is_empty() || entry.name == "." || entry.name == ".." || entry.name.contains(['/', '\\', ':']) || entry.size > 1024 * 1024 * 1024 || entry.packed == 0 {
            return Err(tr!("The bundle contains an invalid path or size.").into());
        }
        total = total.checked_add(entry.packed).ok_or(tr!("Invalid bundle size."))?;
    }
    if total != packed { return Err(tr!("The portable bundle is truncated.").into()); }
    let cache = data.join("herramientas").join(version);
    fs::create_dir_all(&cache).map_err(|e| tf!("Could not prepare {}: {e}. Place the executable in a writable folder.", cache.display()))?;
    for entry in manifest.files {
        let output = cache.join(&entry.name);
        let position = file.stream_position().map_err(|e| e.to_string())?;
        if fs::metadata(&output).is_ok_and(|m| m.is_file() && m.len() == entry.size) {
            file.seek(SeekFrom::Start(position + entry.packed)).map_err(|e| e.to_string())?; continue;
        }
        let mut temp = tempfile::NamedTempFile::new_in(&cache).map_err(|e| e.to_string())?;
        let source = std::io::Read::by_ref(&mut file).take(entry.packed);
        let mut decoder = flate2::read::GzDecoder::new(source).take(entry.size + 1);
        let actual = std::io::copy(&mut decoder, &mut temp).map_err(|e| tf!("Could not prepare {}: {e}", entry.name))?;
        if actual != entry.size { return Err(tf!("The {} tool is incomplete.", entry.name)); }
        temp.flush().map_err(|e| e.to_string())?;
        if let Err(error) = temp.persist(&output) {
            // Another simultaneous launch may have finished the same atomic extraction.
            if !fs::metadata(&output).is_ok_and(|m| m.len() == entry.size) { return Err(error.to_string()); }
        }
        file.seek(SeekFrom::Start(position + entry.packed)).map_err(|e| e.to_string())?;
    }
    Ok(Some(cache))
}

pub fn initialize() -> Result<Option<PathBuf>, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if let Some(bin) = executable.parent() {
        // An AppImage mounts its AppDir read-only. Use bundled Linux executables,
        // but keep settings/history in the normal writable XDG data directory.
        if ["ffmpeg", "ffprobe", "ffplay", "yt-dlp", "deno"].iter().all(|name| bin.join(name).is_file()) {
            let _ = TOOLS.set(bin.to_path_buf());
            return Ok(None);
        }
    }
    let data = executable.parent().ok_or(tr!("Could not find the executable's folder."))?.join("Audio Twitch Cut Datos");
    if let Some(cache) = extract(&executable, &data)? {
        let _ = TOOLS.set(cache);
        // Preserve this computer's previous preferences without overwriting portable data.
        if let Some(old) = directories::ProjectDirs::from("com", "AudioTwitchCut", "Audio Twitch Cut") {
            for name in ["historial.json", "apariencia.json"] {
                let source = old.data_local_dir().join(name); let destination = data.join(name);
                if !destination.exists() && source.is_file() { let _ = fs::copy(source, destination); }
            }
        }
        Ok(Some(data))
    } else { Ok(None) }
}

pub fn verify(data: &Path) -> Result<(), String> {
    let cache = tools_dir().ok_or(tr!("This executable does not contain the embedded bundle."))?;
    let mut results = Vec::new();
    for name in ["ffmpeg", "ffprobe", "ffplay", "yt-dlp", "deno"] {
        let resolved = crate::media::tool(name);
        if resolved.parent() != Some(cache) { return Err(tf!("{name} is not from the embedded bundle.")); }
        let bytes = crate::media::run(crate::media::command(name).arg(if name.starts_with("ff") { "-version" } else { "--version" }), 30)?;
        results.push(serde_json::json!({"tool": name, "bundled": true, "version": String::from_utf8_lossy(&bytes).lines().next().unwrap_or("")}));
    }
    let fixture = tempfile::tempdir_in(data).map_err(|e| e.to_string())?;
    let source = fixture.path().join("prueba.wav");
    crate::media::run(crate::media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=880:duration=12"]).arg(&source), 30)?;
    let audio = crate::media::load(source)?;
    let output = fixture.path().join("corte.wav");
    crate::media::export_gain(&audio, 0.5, 9.0, &output, -6.0)?;
    let check = crate::media::load(output)?;
    if (check.duration - 9.0).abs() > 0.001 { return Err(tr!("The portable trim check failed.").into()); }
    let beside_exe = std::env::current_exe().ok().is_some_and(|exe| exe.parent() == data.parent());
    let report = serde_json::json!({"single_executable": cfg!(windows) || std::env::var_os("APPIMAGE").is_some(), "tools": results, "cut_seconds": check.duration, "data_next_to_exe": beside_exe});
    fs::write(data.join("comprobacion-portable.json"), serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bundle(dir: &Path, name: &str, contents: &[u8], corrupt: bool) -> PathBuf {
        use flate2::{Compression, write::GzEncoder};
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default()); encoder.write_all(contents).unwrap();
        let mut compressed = encoder.finish().unwrap();
        if corrupt { let end = compressed.len() - 8; compressed[end] ^= 0xff; }
        let manifest = serde_json::to_vec(&serde_json::json!({"files":[{"name":name,"size":contents.len(),"packed":compressed.len()}]})).unwrap();
        let mut bytes = b"EXE-STUB".to_vec(); let size = 8 + manifest.len() + compressed.len();
        bytes.extend_from_slice(&(manifest.len() as u64).to_le_bytes()); bytes.extend(manifest); bytes.extend(compressed);
        bytes.extend_from_slice(&(size as u64).to_le_bytes()); bytes.extend_from_slice(&[1u8;32]); bytes.extend_from_slice(MAGIC);
        let path = dir.join("portable.exe"); fs::write(&path, bytes).unwrap(); path
    }
    #[test] fn bundled_tools_extract_and_reuse_cache() {
        let dir = tempfile::tempdir().unwrap(); let exe = bundle(dir.path(), "ffmpeg.exe", b"tool-bytes", false); let data = dir.path().join("datos");
        let cache = extract(&exe, &data).unwrap().unwrap(); assert_eq!(fs::read(cache.join("ffmpeg.exe")).unwrap(), b"tool-bytes");
        let modified = fs::metadata(cache.join("ffmpeg.exe")).unwrap().modified().unwrap();
        assert_eq!(extract(&exe, &data).unwrap(), Some(cache.clone())); assert_eq!(fs::metadata(cache.join("ffmpeg.exe")).unwrap().modified().unwrap(), modified);
    }
    #[test] fn rejects_escaping_paths_and_corrupted_payload() {
        let dir = tempfile::tempdir().unwrap(); let data = dir.path().join("datos");
        let exe = bundle(dir.path(), "../escape.exe", b"bad", false); assert!(extract(&exe, &data).is_err()); assert!(!dir.path().join("escape.exe").exists());
        let exe = bundle(dir.path(), "ffmpeg.exe", b"tool", true); assert!(extract(&exe, &data).is_err());
    }
}
