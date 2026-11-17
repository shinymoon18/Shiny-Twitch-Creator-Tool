use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, process::Command, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc::{self, Receiver}}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use crate::media::Result;

pub const REPOSITORY: &str = "https://github.com/shinymoon18/Shiny-Twitch-Creator-Tool";
const API: &str = "https://api.github.com/repos/shinymoon18/Shiny-Twitch-Creator-Tool/releases/latest";
const MAX_ASSET: u64 = 2 * 1024 * 1024 * 1024;
#[derive(Clone, Copy)]
pub enum Platform { Windows, Linux }
impl Platform {
    pub fn current() -> Self { if cfg!(windows) { Self::Windows } else { Self::Linux } }
    fn filename(self, version: &str) -> String {
        match self {
            Self::Windows => format!("Shiny-Twitch-Creator-Tool-Portable-{version}.exe"),
            Self::Linux => format!("Shiny-Twitch-Creator-Tool-{version}-x86_64.AppImage"),
        }
    }
}
#[derive(Deserialize)]
struct Release { tag_name: String, html_url: String, #[serde(default)] body: String, draft: bool, prerelease: bool, assets: Vec<Asset> }
#[derive(Deserialize)]
struct Asset { name: String, browser_download_url: String, size: u64, digest: Option<String> }
#[derive(Clone, Debug)]
pub struct Update { pub version: String, pub page: String, pub notes: String, pub filename: String, pub url: String, pub size: u64, pub hash: String }

fn trusted_url(value: &str, prefix: &str) -> bool {
    url::Url::parse(value).ok().is_some_and(|url| url.scheme() == "https" && url.host_str() == Some("github.com") && url.port_or_known_default() == Some(443) && url.username().is_empty() && url.password().is_none() && url.fragment().is_none() && value.starts_with(prefix))
}
fn hash_text(value: &str) -> Result<String> {
    let hash = value.strip_prefix("sha256:").ok_or(tr!("The release does not provide a SHA256 digest."))?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(tr!("The release SHA256 digest is invalid.").into()); }
    Ok(hash.to_ascii_lowercase())
}
fn select_update(json: &[u8], current: &str, platform: Platform) -> Result<Option<Update>> {
    let release: Release = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    if release.draft || release.prerelease { return Ok(None); }
    let version = semver::Version::parse(release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name)).map_err(|e| e.to_string())?;
    if !version.pre.is_empty() || !version.cmp_precedence(&semver::Version::parse(current).map_err(|e| e.to_string())?).is_gt() { return Ok(None); }
    if !trusted_url(&release.html_url, &format!("{REPOSITORY}/releases/tag/")) { return Err(tr!("Unexpected release address.").into()); }
    let filename = platform.filename(&version.to_string());
    let mut assets = release.assets.into_iter().filter(|a| a.name == filename);
    let asset = assets.next().ok_or(tr!("This release has no compatible package for your system."))?;
    if assets.next().is_some() || asset.size == 0 || asset.size > MAX_ASSET || !trusted_url(&asset.browser_download_url, &format!("{REPOSITORY}/releases/download/{}/", release.tag_name)) {
        return Err(tr!("The release package is invalid.").into());
    }
    let hash = hash_text(asset.digest.as_deref().unwrap_or(""))?;
    Ok(Some(Update { version: version.to_string(), page: release.html_url, notes: release.body, filename, url: asset.browser_download_url, size: asset.size, hash }))
}
fn agent(timeout: u64) -> ureq::Agent {
    ureq::Agent::config_builder().https_only(true).max_redirects(5)
        .timeout_global(Some(Duration::from_secs(timeout))).timeout_connect(Some(Duration::from_secs(15))).build().into()
}
pub fn check() -> Result<Option<Update>> { check_version(env!("CARGO_PKG_VERSION")) }
fn check_version(current: &str) -> Result<Option<Update>> {
    let mut response = agent(30).get(API).header("User-Agent", concat!("Shiny-Twitch-Creator-Tool/", env!("CARGO_PKG_VERSION"))).header("Accept", "application/vnd.github+json").call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new(); response.body_mut().as_reader().take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 { return Err(tr!("The release response is too large.").into()); }
    select_update(&bytes, current, Platform::current())
}
pub fn installed_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let image = std::env::var_os("APPIMAGE").ok_or(tr!("Automatic installation on Linux requires the AppImage package."))?;
        return fs::canonicalize(image).map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "linux"))]
    fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn stream_package(mut input: impl Read, output: &mut fs::File, update: &Update, progress: &AtomicU64, cancel: &AtomicBool) -> Result<()> {
    let mut digest = Sha256::new(); let mut total = 0; let mut chunk = [0u8; 128 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) { return Err(tr!("Update canceled.").into()); }
        let count = input.read(&mut chunk).map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Relaxed) { return Err(tr!("Update canceled.").into()); }
        if count == 0 { break; }
        total += count as u64;
        if total > update.size { return Err(tr!("The downloaded package has an unexpected size.").into()); }
        output.write_all(&chunk[..count]).map_err(|e| e.to_string())?; digest.update(&chunk[..count]); progress.store(total, Ordering::Relaxed);
    }
    if total != update.size || format!("{:x}", digest.finalize()) != update.hash { return Err(tr!("Update verification failed. The existing program was not changed.").into()); }
    output.sync_all().map_err(|e| e.to_string())
}
fn download(update: &Update, target: &Path, progress: &AtomicU64, cancel: &AtomicBool) -> Result<tempfile::TempPath> {
    let parent = target.parent().ok_or(tr!("The application folder is unavailable."))?;
    let mut file = tempfile::Builder::new().prefix(".shiny-update-").tempfile_in(parent).map_err(|e| format!("{} {e}", tr!("The application folder must be writable to install an update.")))?;
    let mut response = agent(1800).get(&update.url).header("User-Agent", concat!("Shiny-Twitch-Creator-Tool/", env!("CARGO_PKG_VERSION"))).call().map_err(|e| e.to_string())?;
    stream_package(response.body_mut().as_reader(), file.as_file_mut(), update, progress, cancel)?;
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(file.path(), fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?; }
    Ok(file.into_temp_path())
}
fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?; let mut hash = Sha256::new(); let mut buf = [0u8; 128 * 1024];
    loop { let count = file.read(&mut buf).map_err(|e| e.to_string())?; if count == 0 { break; } hash.update(&buf[..count]); }
    Ok(format!("{:x}", hash.finalize()))
}
// Preserve a recoverable original and restore it if replacement or relaunch fails.
fn replace_and_launch(target: &Path, payload: &Path, expected: &str, launch: impl FnOnce(&Path) -> Result<()>) -> Result<PathBuf> {
    if target.parent() != payload.parent() || target == payload || file_hash(payload)? != expected { return Err(tr!("The staged update is invalid. The existing program was not changed.").into()); }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let backup = target.with_file_name(format!("{}.previous-{stamp}", target.file_name().unwrap_or_default().to_string_lossy()));
    if backup.exists() { return Err(tr!("The update backup already exists.").into()); }
    #[cfg(windows)] fs::rename(target, &backup).map_err(|e| e.to_string())?;
    #[cfg(not(windows))] { fs::copy(target, &backup).map_err(|e| e.to_string())?; fs::File::open(&backup).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?; }
    if let Err(error) = fs::rename(payload, target).map_err(|e| e.to_string()).and_then(|_| launch(target)) {
        #[cfg(windows)] { if target.exists() { fs::remove_file(target).map_err(|e| format!("{error}; rollback: {e}; backup: {}", backup.display()))?; } }
        fs::rename(&backup, target).map_err(|e| format!("{error}; rollback: {e}; backup: {}", backup.display()))?;
        return Err(format!("{} {error}", tr!("The previous program was restored.")));
    }
    Ok(backup)
}
fn relaunch(path: &Path) -> Result<()> {
    relaunch_with_args(path, &[])
}
fn relaunch_with_args(path: &Path, args: &[String]) -> Result<()> {
    let mut cmd = Command::new(path); cmd.args(args).current_dir(path.parent().unwrap_or(Path::new(".")));
    for key in ["APPIMAGE", "APPDIR", "ARGV0", "LD_LIBRARY_PATH"] { cmd.env_remove(key); }
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}
#[cfg(windows)]
#[derive(serde::Serialize, Deserialize)]
struct InstallPlan { target: PathBuf, payload: PathBuf, hash: String, parent_pid: u32, result: PathBuf, #[serde(default)] restart_args: Vec<String> }
#[cfg(windows)]
fn wait_for_parent(pid: u32) -> Result<()> {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" { fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void; fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32; fn CloseHandle(handle: *mut c_void) -> i32; fn GetLastError() -> u32; }
    // Only a synchronization handle to our own previous process is requested.
    unsafe {
        let handle = OpenProcess(0x00100000, 0, pid);
        if handle.is_null() { return if GetLastError() == 87 { Ok(()) } else { Err("Could not wait for the previous application.".into()) }; }
        let status = WaitForSingleObject(handle, 120_000); CloseHandle(handle);
        if status == 0 { Ok(()) } else { Err("The previous application did not close in time.".into()) }
    }
}
pub fn helper_mode() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|arg| arg != "--apply-update") { return None; }
    #[cfg(windows)] {
        let result = (|| -> Result<()> {
            if args.len() != 3 { return Err("Invalid update installer arguments.".into()); }
            let path = PathBuf::from(&args[2]); let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let plan: InstallPlan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let outcome = wait_for_parent(plan.parent_pid).and_then(|_| replace_and_launch(&plan.target, &plan.payload, &plan.hash, |path| relaunch_with_args(path, &plan.restart_args)));
            let message = match &outcome { Ok(backup) => format!("OK\n{}", backup.display()), Err(error) => format!("ERROR\n{error}") };
            let _ = fs::write(&plan.result, message);
            if outcome.is_err() { let _ = relaunch(&plan.target); }
            let _ = fs::remove_file(path);
            outcome.map(|_| ())
        })();
        return Some(if result.is_ok() { 0 } else { 1 });
    }
    #[cfg(not(windows))] Some(1)
}
fn install(target: PathBuf, payload: PathBuf, hash: String, data: PathBuf) -> Result<()> {
    #[cfg(windows)] {
        let cache = data.join("updates"); fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        let mut helper = tempfile::Builder::new().prefix("shiny-update-helper-").suffix(".exe").tempfile_in(&cache).map_err(|e| e.to_string())?;
        let mut source = fs::File::open(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::io::copy(&mut source, &mut helper).map_err(|e| e.to_string())?; helper.flush().map_err(|e| e.to_string())?;
        let helper = helper.into_temp_path().keep().map_err(|e| e.to_string())?;
        let mut plan = tempfile::Builder::new().prefix("install-").suffix(".json").tempfile_in(&cache).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut plan, &InstallPlan { target, payload, hash, parent_pid: std::process::id(), result: cache.join("result.txt"), restart_args: Vec::new() }).map_err(|e| e.to_string())?;
        plan.flush().map_err(|e| e.to_string())?;
        let plan = plan.into_temp_path().keep().map_err(|e| e.to_string())?;
        use std::os::windows::process::CommandExt;
        Command::new(helper).arg("--apply-update").arg(plan).creation_flags(0x08000000).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))] {
        let outcome = replace_and_launch(&target, &payload, &hash, relaunch);
        let cache = data.join("updates"); let _ = fs::create_dir_all(&cache);
        let _ = fs::write(cache.join("result.txt"), match &outcome { Ok(backup) => format!("OK\n{}", backup.display()), Err(e) => format!("ERROR\n{e}") });
        outcome.map(|_| ())
    }
}

pub enum Stage { Idle, Checking, Current, Available, Downloading, Ready, Installing, Error(String) }
enum Event { Checked(Result<Option<Update>>), Downloaded(Result<tempfile::TempPath>), Installed(Result<()>) }
pub struct Manager {
    pub stage: Stage, pub update: Option<Update>, pub show: bool, pub restart: bool,
    pub progress: Arc<AtomicU64>, cancel: Arc<AtomicBool>, receiver: Option<Receiver<Event>>,
    staged: Option<PathBuf>, target: Option<PathBuf>, next_check: Instant,
}
impl Default for Manager {
    fn default() -> Self { Self { stage: Stage::Idle, update: None, show: false, restart: false, progress: Arc::new(AtomicU64::new(0)), cancel: Arc::new(AtomicBool::new(false)), receiver: None, staged: None, target: None, next_check: Instant::now() } }
}
impl Manager {
    pub fn new(data: &Path) -> Self {
        let mut manager = Self::default();
        let cache = data.join("updates");
        if let Ok(result) = fs::read_to_string(cache.join("result.txt")) {
            if let Some(error) = result.strip_prefix("ERROR\n") {
                manager.stage = Stage::Error(error.into()); manager.show = true;
                manager.next_check = Instant::now() + Duration::from_secs(6 * 60 * 60);
            }
            let _ = fs::remove_file(cache.join("result.txt"));
        }
        if let Ok(entries) = fs::read_dir(cache) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("shiny-update-helper-") && entry.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age > Duration::from_secs(24 * 60 * 60)) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        manager
    }
    pub fn busy(&self) -> bool { matches!(self.stage, Stage::Checking | Stage::Downloading | Stage::Installing) }
    pub fn downloading(&self) -> bool { matches!(self.stage, Stage::Downloading) }
    pub fn installing(&self) -> bool { matches!(self.stage, Stage::Installing) }
    pub fn cancel(&self) { self.cancel.store(true, Ordering::Relaxed); }
    pub fn cancel_download(&mut self) {
        if self.downloading() { self.cancel(); self.receiver = None; self.stage = Stage::Available; }
    }
    pub fn check(&mut self, ctx: &eframe::egui::Context) {
        if self.busy() || self.staged.is_some() { return; }
        self.next_check = Instant::now() + Duration::from_secs(6 * 60 * 60); self.stage = Stage::Checking;
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone();
        std::thread::spawn(move || { let _ = tx.send(Event::Checked(check())); ctx.request_repaint(); });
    }
    pub fn download(&mut self, ctx: &eframe::egui::Context) {
        let Some(update) = self.update.clone() else { return; }; if self.busy() { return; }
        let target = match installed_path() { Ok(path) => path, Err(e) => { self.stage = Stage::Error(e); return; } };
        self.target = Some(target.clone()); self.cancel = Arc::new(AtomicBool::new(false)); self.progress.store(0, Ordering::Relaxed); self.stage = Stage::Downloading;
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone(); let progress = self.progress.clone(); let cancel = self.cancel.clone();
        std::thread::spawn(move || { let _ = tx.send(Event::Downloaded(download(&update, &target, &progress, &cancel))); ctx.request_repaint(); });
    }
    pub fn install(&mut self, data: &Path, ctx: &eframe::egui::Context) {
        if !matches!(self.stage, Stage::Ready) { return; }
        let (Some(target), Some(payload), Some(update)) = (self.target.clone(), self.staged.clone(), self.update.clone()) else { return; };
        self.stage = Stage::Installing;
        let data = data.to_path_buf(); let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone();
        std::thread::spawn(move || { let _ = tx.send(Event::Installed(install(target, payload, update.hash, data))); ctx.request_repaint(); });
    }
    pub fn poll(&mut self, automatic: bool, ctx: &eframe::egui::Context) {
        let event = self.receiver.as_ref().and_then(|receiver| match receiver.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Event::Checked(Err(tr!("The update worker stopped unexpectedly.").into()))),
        });
        if let Some(event) = event {
            self.receiver = None;
            match event {
                Event::Checked(Ok(Some(update))) => { self.update = Some(update); self.stage = Stage::Available; if automatic { self.show = true; } }
                Event::Checked(Ok(None)) => { self.update = None; self.stage = Stage::Current; }
                Event::Downloaded(Ok(path)) => match path.keep() {
                    Ok(path) => { self.staged = Some(path); self.stage = Stage::Ready; }
                    Err(error) => self.stage = Stage::Error(error.to_string()),
                },
                Event::Installed(Ok(())) => { self.restart = true; }
                Event::Installed(Err(e)) => { if let Some(path) = self.staged.take() { let _ = fs::remove_file(path); } self.stage = Stage::Error(e); }
                Event::Checked(Err(e)) | Event::Downloaded(Err(e)) => self.stage = Stage::Error(e),
            }
        }
        if !self.restart && automatic && !self.busy() && self.staged.is_none() && Instant::now() >= self.next_check { self.check(ctx); }
        if self.busy() { ctx.request_repaint_after(Duration::from_millis(100)); }
        else if automatic { ctx.request_repaint_after(self.next_check.saturating_duration_since(Instant::now()).min(Duration::from_secs(60))); }
    }
}
impl Drop for Manager {
    fn drop(&mut self) { self.cancel(); if !self.restart { if let Some(path) = &self.staged { let _ = fs::remove_file(path); } } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires the generated AppImage; run explicitly in the Linux release workflow"]
    fn linux_appimage_installer_probe() {
        let image = PathBuf::from(std::env::var_os("SHINY_TEST_APPIMAGE").expect("AppImage path required"));
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("current.AppImage"); let payload = dir.path().join("replacement.AppImage");
        fs::write(&target, b"previous application").unwrap();
        let mut previous_handle = fs::File::open(&target).unwrap();
        fs::copy(image, &payload).unwrap();
        let expected = file_hash(&payload).unwrap();
        let backup = replace_and_launch(&target, &payload, &expected, |path| {
            let output = Command::new(path).arg("--update-check").env("APPIMAGE_EXTRACT_AND_RUN", "1")
                .env_remove("APPIMAGE").env_remove("APPDIR").env_remove("ARGV0").env_remove("LD_LIBRARY_PATH").output().map_err(|e| e.to_string())?;
            if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).into_owned()); }
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
            if report["current"] != env!("CARGO_PKG_VERSION") { return Err("Restarted package version mismatch".into()); }
            Ok(())
        }).unwrap();
        assert_eq!(file_hash(&target).unwrap(), expected);
        assert_eq!(fs::read(backup).unwrap(), b"previous application");
        let mut original = Vec::new(); previous_handle.read_to_end(&mut original).unwrap();
        assert_eq!(original, b"previous application");
    }
    #[test]
    #[ignore = "downloads the latest published package from GitHub; run explicitly"]
    fn live_github_release_and_package_verification() {
        let update = check_version("0.0.0").unwrap().expect("a stable public release is required");
        let dir = tempfile::tempdir().unwrap();
        let path = download(&update, &dir.path().join("application"), &AtomicU64::new(0), &AtomicBool::new(false)).unwrap();
        assert_eq!(file_hash(&path).unwrap(), update.hash);
        assert_eq!(fs::metadata(&path).unwrap().len(), update.size);
    }
    fn json(version: &str, platform: Platform, digest: &str) -> Vec<u8> {
        let name = platform.filename(version);
        serde_json::to_vec(&serde_json::json!({"tag_name":format!("v{version}"),"html_url":format!("{REPOSITORY}/releases/tag/v{version}"),"body":"changes","draft":false,"prerelease":false,"assets":[{"name":name,"browser_download_url":format!("{REPOSITORY}/releases/download/v{version}/{name}"),"size":4,"digest":digest}]})).unwrap()
    }
    #[test] fn chooses_correct_platform_and_semantic_version_without_downgrades() {
        let hash = format!("sha256:{}", "a".repeat(64));
        for platform in [Platform::Windows, Platform::Linux] {
            let bytes = json("1.10.0", platform, &hash);
            assert_eq!(select_update(&bytes, "1.9.0", platform).unwrap().unwrap().version, "1.10.0");
            assert!(select_update(&bytes, "1.10.0", platform).unwrap().is_none());
            assert!(select_update(&bytes, "2.0.0", platform).unwrap().is_none());
        }
    }
    #[test] fn rejects_missing_checksums_external_urls_and_prereleases() {
        assert!(select_update(&json("2.0.0", Platform::Windows, ""), "1.0.0", Platform::Windows).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&json("2.0.0", Platform::Windows, &format!("sha256:{}", "a".repeat(64)))).unwrap();
        value["assets"][0]["browser_download_url"] = "https://example.com/file.exe".into();
        assert!(select_update(&serde_json::to_vec(&value).unwrap(), "1.0.0", Platform::Windows).is_err());
        value["prerelease"] = true.into();
        assert!(select_update(&serde_json::to_vec(&value).unwrap(), "1.0.0", Platform::Windows).unwrap().is_none());
    }
    #[test] fn verifies_download_size_hash_and_cancellation() {
        let dir = tempfile::tempdir().unwrap(); let mut out = fs::File::create(dir.path().join("download")).unwrap();
        let mut update = select_update(&json("2.0.0", Platform::Windows, &format!("sha256:{:x}", Sha256::digest(b"test"))), "1.0.0", Platform::Windows).unwrap().unwrap();
        let progress = AtomicU64::new(0); let cancel = AtomicBool::new(false);
        stream_package(&b"test"[..], &mut out, &update, &progress, &cancel).unwrap(); assert_eq!(progress.load(Ordering::Relaxed), 4);
        assert!(stream_package(&b"bad!"[..], &mut out, &update, &progress, &cancel).is_err());
        update.size = 3; assert!(stream_package(&b"test"[..], &mut out, &update, &progress, &cancel).is_err());
        cancel.store(true, Ordering::Relaxed); assert!(stream_package(&b"test"[..], &mut out, &update, &progress, &cancel).is_err());
    }
    #[test] fn installation_preserves_backup_and_rolls_back_a_failed_restart() {
        let dir = tempfile::tempdir().unwrap(); let target = dir.path().join("app.exe"); let payload = dir.path().join("staged");
        fs::write(&target, b"old").unwrap(); fs::write(&payload, b"new").unwrap();
        let hash = file_hash(&payload).unwrap();
        assert!(replace_and_launch(&target, &payload, &"a".repeat(64), |_| Ok(())).is_err()); assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(replace_and_launch(&target, &payload, &hash, |_| Err("launch failed".into())).is_err()); assert_eq!(fs::read(&target).unwrap(), b"old");
        fs::write(&payload, b"new").unwrap();
        let backup = replace_and_launch(&target, &payload, &hash, |_| Ok(())).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new"); assert_eq!(fs::read(backup).unwrap(), b"old");
    }
    #[test] fn invalid_executable_is_restored_after_real_launch_failure() {
        let dir = tempfile::tempdir().unwrap(); let target = dir.path().join("app.exe"); let payload = dir.path().join("staged");
        fs::write(&target, b"original executable").unwrap(); fs::write(&payload, b"invalid executable").unwrap();
        let hash = file_hash(&payload).unwrap();
        assert!(replace_and_launch(&target, &payload, &hash, relaunch).is_err());
        assert_eq!(fs::read(target).unwrap(), b"original executable");
    }
    #[cfg(windows)]
    #[test] fn windows_installer_waits_until_the_previous_process_exits() {
        use std::os::windows::process::CommandExt;
        let mut child = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Milliseconds 200"]).creation_flags(0x08000000).spawn().unwrap();
        wait_for_parent(child.id()).unwrap();
        assert!(child.try_wait().unwrap().is_some());
    }
}
