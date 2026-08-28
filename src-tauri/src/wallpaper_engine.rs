use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use turbojpeg::{Compressor, Image as JpegImage, PixelFormat, Subsamp};
#[cfg(target_os = "windows")]
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
#[cfg(target_os = "windows")]
use windows_capture::frame::Frame;
#[cfg(target_os = "windows")]
use windows_capture::graphics_capture_api::InternalCaptureControl;
#[cfg(target_os = "windows")]
use windows_capture::settings::{
    ColorFormat as CaptureColorFormat, CursorCaptureSettings, DirtyRegionSettings,
    DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
#[cfg(target_os = "windows")]
use windows_capture::window::Window as CaptureWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, RECT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, FindWindowW, GetClassNameW, GetWindowRect, GetWindowTextW,
    IsWindow, IsWindowVisible, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
};

const JPEG_QUALITY: i32 = 90;
const FRAME_INTERVAL: Duration = Duration::from_millis(100);
const ENGINE_READY_TIMEOUT: Duration = Duration::from_secs(20);
const FIRST_FRAME_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct FrameCache {
    captured_at: Option<Instant>,
    source_sequence: u64,
    sequence: u64,
    jpeg: Option<Arc<[u8]>>,
}

#[derive(Clone)]
pub(crate) struct SceneFrame {
    pub(crate) sequence: u64,
    pub(crate) jpeg: Arc<[u8]>,
}

#[derive(Default)]
pub(crate) struct SceneCaptureWorker {
    #[cfg(target_os = "windows")]
    compressor: Option<Compressor>,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct GraphicsCaptureFrame {
    width: u32,
    height: u32,
    sequence: u64,
    bgra: Arc<[u8]>,
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct GraphicsCaptureState {
    latest: Mutex<Option<GraphicsCaptureFrame>>,
    closed: AtomicBool,
}

#[cfg(target_os = "windows")]
struct GraphicsCaptureHandler {
    state: Arc<GraphicsCaptureState>,
    scratch: Vec<u8>,
}

#[cfg(target_os = "windows")]
impl GraphicsCaptureApiHandler for GraphicsCaptureHandler {
    type Flags = Arc<GraphicsCaptureState>;
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            state: ctx.flags,
            scratch: Vec::new(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();
        let buffer = frame
            .buffer()
            .map_err(|error| format!("无法读取 Windows Graphics Capture 帧：{error}"))?;
        let bgra: Arc<[u8]> = Arc::from(buffer.as_nopadding_buffer(&mut self.scratch));
        let mut latest = self
            .state
            .latest
            .lock()
            .map_err(|_| "Windows Graphics Capture 帧缓存已损坏".to_owned())?;
        let sequence = latest
            .as_ref()
            .map_or(1, |previous| previous.sequence.wrapping_add(1).max(1));
        *latest = Some(GraphicsCaptureFrame {
            width,
            height,
            sequence,
            bgra,
        });
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.state.closed.store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
type GraphicsCaptureControl = CaptureControl<GraphicsCaptureHandler, String>;

#[cfg(target_os = "windows")]
fn start_graphics_capture(
    hwnd: HWND,
) -> Result<(Arc<GraphicsCaptureState>, GraphicsCaptureControl), String> {
    let state = Arc::new(GraphicsCaptureState::default());
    let window = CaptureWindow::from_raw_hwnd(hwnd.cast());
    let settings = Settings::new(
        window,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Exclude,
        MinimumUpdateIntervalSettings::Custom(FRAME_INTERVAL),
        DirtyRegionSettings::Default,
        CaptureColorFormat::Bgra8,
        state.clone(),
    );
    let control = GraphicsCaptureHandler::start_free_threaded(settings)
        .map_err(|error| format!("无法启动 Windows Graphics Capture：{error}"))?;
    Ok((state, control))
}

#[cfg(target_os = "windows")]
fn bgra_frame_is_effectively_black(bgra: &[u8]) -> bool {
    let pixels = bgra.len() / 4;
    if pixels == 0 {
        return true;
    }
    let sample_step = (pixels / 4_096).max(1);
    let mut visible_samples = 0_usize;
    for pixel_index in (0..pixels).step_by(sample_step) {
        let offset = pixel_index * 4;
        if bgra[offset..offset + 3]
            .iter()
            .copied()
            .max()
            .unwrap_or_default()
            > 3
        {
            visible_samples += 1;
            if visible_samples >= 8 {
                return false;
            }
        }
    }
    true
}

pub(crate) struct SceneWallpaperCapture {
    engine: PathBuf,
    mode: SceneCaptureMode,
    window_name: Option<String>,
    #[cfg(target_os = "windows")]
    hwnd: isize,
    #[cfg(target_os = "windows")]
    graphics_state: Option<Arc<GraphicsCaptureState>>,
    #[cfg(target_os = "windows")]
    graphics_control: Option<GraphicsCaptureControl>,
    frame: Mutex<FrameCache>,
    paused: AtomicBool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SceneCaptureMode {
    DesktopSynchronized,
}

impl SceneCaptureMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DesktopSynchronized => "desktop-synchronized",
        }
    }
}

impl fmt::Debug for SceneWallpaperCapture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SceneWallpaperCapture")
            .field("engine", &self.engine)
            .field("mode", &self.mode)
            .field("window_name", &self.window_name)
            .field("paused", &self.paused.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SceneWallpaperCapture {
    #[cfg(target_os = "windows")]
    pub(crate) fn start(workshop_root: &Path, project_json: &Path) -> Result<Self, String> {
        let engine = wallpaper_engine_executable(workshop_root)?;
        let project_json = project_json
            .canonicalize()
            .map_err(|error| format!("无法解析 Scene project.json：{error}"))?;

        // 官方控制接口要求 Wallpaper Engine 主进程先运行。已有实例时直接复用，
        // 避免每次应用主题都再次触发启动和安全恢复界面。
        if !wallpaper_engine_is_running() {
            hidden_command(&engine)
                .spawn()
                .map_err(|error| format!("无法启动 Wallpaper Engine：{error}"))?;
        }
        wait_for_wallpaper_engine_ready()?;
        let desktop_hwnd = open_desktop_wallpaper(&engine, &project_json)?;
        let mut desktop_bounds = empty_rect();
        if unsafe { GetWindowRect(desktop_hwnd, &mut desktop_bounds) } == 0 {
            return Err("无法读取 Wallpaper Engine 桌面渲染尺寸".into());
        }
        let width = desktop_bounds.right - desktop_bounds.left;
        let height = desktop_bounds.bottom - desktop_bounds.top;
        if width <= 0 || height <= 0 || width > 4096 || height > 2160 {
            return Err(format!(
                "Wallpaper Engine 桌面渲染尺寸异常：{width}x{height}"
            ));
        }
        let (window_name, hwnd) =
            open_independent_wallpaper(&engine, &project_json, width, height)?;
        let (graphics_state, graphics_control) = match start_graphics_capture(hwnd) {
            Ok(capture) => capture,
            Err(error) => {
                close_wallpaper_window(&engine, &window_name);
                return Err(error);
            }
        };
        if !wait_for_non_black_graphics_frame(&graphics_state, FIRST_FRAME_ATTEMPT_TIMEOUT) {
            let _ = graphics_control.stop();
            close_wallpaper_window(&engine, &window_name);
            return Err(
                "Wallpaper Engine playInWindow 在 10 秒内没有产生可用画面；若出现“安全启动”，请先在 Wallpaper Engine 中确认恢复。ReTheme 没有关闭“防止崩溃”保护。"
                    .into(),
            );
        }

        let capture = Self {
            engine,
            mode: SceneCaptureMode::DesktopSynchronized,
            window_name: Some(window_name),
            hwnd: hwnd as isize,
            graphics_state: Some(graphics_state),
            graphics_control: Some(graphics_control),
            frame: Mutex::new(FrameCache {
                captured_at: None,
                source_sequence: 0,
                sequence: 0,
                jpeg: None,
            }),
            paused: AtomicBool::new(false),
        };
        Ok(capture)
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn start(_workshop_root: &Path, _project_json: &Path) -> Result<Self, String> {
        Err("Wallpaper Engine Scene 渲染仅支持 Windows".into())
    }

    pub(crate) fn frame_interval() -> Duration {
        FRAME_INTERVAL
    }

    pub(crate) fn mode(&self) -> SceneCaptureMode {
        self.mode
    }

    pub(crate) fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    pub(crate) fn capture_worker() -> SceneCaptureWorker {
        SceneCaptureWorker::default()
    }

    fn cached_frame_or_error(&self, message: &str) -> Result<SceneFrame, String> {
        let cache = self
            .frame
            .lock()
            .map_err(|_| "Scene 帧缓存已损坏".to_owned())?;
        if let Some(jpeg) = &cache.jpeg {
            return Ok(SceneFrame {
                sequence: cache.sequence,
                jpeg: jpeg.clone(),
            });
        }
        Err(message.to_owned())
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn capture_jpeg(
        &self,
        worker: &mut SceneCaptureWorker,
    ) -> Result<SceneFrame, String> {
        {
            let cache = self
                .frame
                .lock()
                .map_err(|_| "Scene 帧缓存已损坏".to_owned())?;
            if let Some(jpeg) = &cache.jpeg
                && (self.is_paused()
                    || cache
                        .captured_at
                        .is_some_and(|captured_at| captured_at.elapsed() < FRAME_INTERVAL))
            {
                return Ok(SceneFrame {
                    sequence: cache.sequence,
                    jpeg: jpeg.clone(),
                });
            }
        }

        // SAFETY: hwnd 在启动时解析，并在每次抓取前检查是否仍然有效。
        let hwnd = self.hwnd as HWND;
        if unsafe { IsWindow(hwnd) } == 0 {
            return Err("Wallpaper Engine Scene 渲染窗口已关闭".into());
        }
        let graphics_frame = if let Some(graphics_state) = self.graphics_state.as_ref() {
            if graphics_state.closed.load(Ordering::Acquire) {
                return Err("Windows Graphics Capture 会话已关闭".into());
            }
            let graphics_frame = graphics_state
                .latest
                .lock()
                .map_err(|_| "Windows Graphics Capture 帧缓存已损坏".to_owned())?
                .clone()
                .ok_or_else(|| "Windows Graphics Capture 尚未产生画面".to_owned())?;
            if graphics_frame.width == 0
                || graphics_frame.height == 0
                || graphics_frame.width > 4096
                || graphics_frame.height > 2160
            {
                return Err(format!(
                    "Windows Graphics Capture 帧尺寸异常：{}x{}",
                    graphics_frame.width, graphics_frame.height
                ));
            }
            {
                let cache = self
                    .frame
                    .lock()
                    .map_err(|_| "Scene 帧缓存已损坏".to_owned())?;
                if cache.source_sequence == graphics_frame.sequence
                    && let Some(jpeg) = &cache.jpeg
                {
                    return Ok(SceneFrame {
                        sequence: cache.sequence,
                        jpeg: jpeg.clone(),
                    });
                }
            }
            if bgra_frame_is_effectively_black(&graphics_frame.bgra) {
                return self.cached_frame_or_error("Wallpaper Engine playInWindow 当前输出纯黑帧");
            }
            graphics_frame
        } else {
            return Err("Windows Graphics Capture 未初始化".into());
        };
        if worker.compressor.is_none() {
            let mut compressor =
                Compressor::new().map_err(|error| format!("无法初始化 libjpeg-turbo：{error}"))?;
            compressor
                .set_quality(JPEG_QUALITY)
                .map_err(|error| format!("无法设置 JPEG 质量：{error}"))?;
            compressor
                .set_subsamp(Subsamp::Sub2x2)
                .map_err(|error| format!("无法设置 JPEG 色度采样：{error}"))?;
            worker.compressor = Some(compressor);
        }
        let jpeg = worker
            .compressor
            .as_mut()
            .expect("JPEG compressor")
            .compress_to_vec(JpegImage {
                pixels: graphics_frame.bgra.as_ref(),
                width: graphics_frame.width as usize,
                pitch: graphics_frame.width as usize * 4,
                height: graphics_frame.height as usize,
                format: PixelFormat::BGRA,
            })
            .map_err(|error| format!("Scene 帧 JPEG 编码失败：{error}"))?;
        let jpeg: Arc<[u8]> = Arc::from(jpeg);
        let mut cache = self
            .frame
            .lock()
            .map_err(|_| "Scene 帧缓存已损坏".to_owned())?;
        cache.captured_at = Some(Instant::now());
        cache.source_sequence = graphics_frame.sequence;
        cache.sequence = cache.sequence.wrapping_add(1).max(1);
        cache.jpeg = Some(jpeg.clone());
        Ok(SceneFrame {
            sequence: cache.sequence,
            jpeg,
        })
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn capture_jpeg(
        &self,
        _worker: &mut SceneCaptureWorker,
    ) -> Result<SceneFrame, String> {
        Err("Wallpaper Engine Scene 渲染仅支持 Windows".into())
    }
}

impl Drop for SceneWallpaperCapture {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(control) = self.graphics_control.take() {
            let _ = control.stop();
        }
        if let Some(window_name) = self.window_name.as_deref() {
            close_wallpaper_window(&self.engine, window_name);
        }
    }
}

#[cfg(target_os = "windows")]
fn open_desktop_wallpaper(engine: &Path, project_json: &Path) -> Result<HWND, String> {
    let project_json = project_json
        .to_str()
        .ok_or_else(|| "Scene 路径不是有效的 Unicode".to_owned())?;
    open_wallpaper_on_desktop(engine, project_json)?;
    resume_wallpaper_engine(engine)?;

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(hwnd) = find_desktop_wallpaper_window() {
            return Ok(hwnd);
        }
        if Instant::now() >= deadline {
            return Err("Wallpaper Engine 桌面渲染窗口在 20 秒内没有就绪".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
fn open_wallpaper_on_desktop(engine: &Path, project_json: &str) -> Result<(), String> {
    let status = hidden_command(engine)
        .args([
            "-control",
            "openWallpaper",
            "-file",
            project_json,
            "-monitor",
            "0",
        ])
        .status()
        .map_err(|error| format!("无法把 Scene 同步到桌面：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Wallpaper Engine 桌面切换命令返回 {}",
            status.code().unwrap_or(-1)
        ))
    }
}

#[cfg(target_os = "windows")]
fn resume_wallpaper_engine(engine: &Path) -> Result<(), String> {
    let status = hidden_command(engine)
        .args(["-control", "play"])
        .status()
        .map_err(|error| format!("无法恢复 Wallpaper Engine 播放：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Wallpaper Engine 播放命令返回 {}",
            status.code().unwrap_or(-1)
        ))
    }
}

#[cfg(target_os = "windows")]
fn wait_for_non_black_graphics_frame(state: &GraphicsCaptureState, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if state.closed.load(Ordering::Acquire) {
            return false;
        }
        if let Ok(latest) = state.latest.lock()
            && let Some(frame) = latest.as_ref()
            && !bgra_frame_is_effectively_black(&frame.bgra)
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
fn open_independent_wallpaper(
    engine: &Path,
    project_json: &Path,
    width: i32,
    height: i32,
) -> Result<(String, HWND), String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let window_name = format!("ReThemeWEScene-{}-{nonce}", std::process::id());
    let project_json = project_json
        .to_str()
        .ok_or_else(|| "Scene 路径不是有效的 Unicode".to_owned())?;
    let width_arg = width.to_string();
    let height_arg = height.to_string();
    hidden_command(engine)
        .args([
            "-control",
            "openWallpaper",
            "-file",
            project_json,
            "-playInWindow",
            &window_name,
            "-width",
            &width_arg,
            "-height",
            &height_arg,
            "-x",
            "0",
            "-y",
            "0",
            "-borderless",
        ])
        .spawn()
        .map_err(|error| format!("无法通过 Wallpaper Engine 打开独立 Scene：{error}"))?;

    let encoded_name = wide_null(&window_name);
    let deadline = Instant::now() + Duration::from_secs(20);
    let hwnd = loop {
        // SAFETY: encoded_name 以 NUL 结尾，并在调用期间保持有效。
        let hwnd = unsafe { FindWindowW(std::ptr::null(), encoded_name.as_ptr()) };
        if !hwnd.is_null() {
            break hwnd;
        }
        if Instant::now() >= deadline {
            close_wallpaper_window(engine, &window_name);
            return Err("Wallpaper Engine 独立 Scene 渲染窗口在 20 秒内没有就绪".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    // 保留窗口的渲染状态，但移出可见桌面，避免遮挡用户。
    // SAFETY: 尺寸已在桌面同步阶段验证，标志位禁止激活和改变 Z 顺序。
    unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            -32_000,
            -32_000,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
    };
    Ok((window_name, hwnd))
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct DesktopWindowCandidate {
    hwnd: isize,
    rect: RECT,
}

#[cfg(target_os = "windows")]
fn find_desktop_wallpaper_window() -> Option<HWND> {
    let mut candidates = Vec::<DesktopWindowCandidate>::new();
    // SAFETY: lparam 在枚举调用期间始终指向有效 Vec，回调只在当前线程同步执行。
    unsafe {
        EnumWindows(
            Some(collect_desktop_wallpaper_roots),
            (&mut candidates as *mut Vec<DesktopWindowCandidate>) as isize,
        );
    }
    candidates
        .into_iter()
        .max_by_key(|candidate| {
            let contains_origin = candidate.rect.left <= 0
                && candidate.rect.right > 0
                && candidate.rect.top <= 0
                && candidate.rect.bottom > 0;
            let width = (candidate.rect.right - candidate.rect.left).max(0) as i64;
            let height = (candidate.rect.bottom - candidate.rect.top).max(0) as i64;
            (contains_origin, width.saturating_mul(height))
        })
        .map(|candidate| candidate.hwnd as HWND)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn collect_desktop_wallpaper_roots(hwnd: HWND, lparam: isize) -> i32 {
    unsafe {
        collect_desktop_wallpaper_window(hwnd, lparam);
        EnumChildWindows(hwnd, Some(collect_desktop_wallpaper_window), lparam);
    }
    1
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn collect_desktop_wallpaper_window(hwnd: HWND, lparam: isize) -> i32 {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let class_name = window_text(hwnd, true);
    let title = window_text(hwnd, false);
    if !class_name.starts_with("WPEDesktop")
        || !class_name.ends_with("Window")
        || title != "WPELiveWallpaper"
    {
        return 1;
    }
    let mut rect = empty_rect();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0
        || rect.right <= rect.left
        || rect.bottom <= rect.top
    {
        return 1;
    }
    // SAFETY: lparam 由 find_desktop_wallpaper_window 传入，指向同步存活的 Vec。
    let candidates = unsafe { &mut *(lparam as *mut Vec<DesktopWindowCandidate>) };
    candidates.push(DesktopWindowCandidate {
        hwnd: hwnd as isize,
        rect,
    });
    1
}

#[cfg(target_os = "windows")]
fn window_text(hwnd: HWND, class_name: bool) -> String {
    let mut buffer = [0_u16; 256];
    // SAFETY: buffer 可写且长度与传入容量一致，hwnd 仅在同步枚举期间使用。
    let length = unsafe {
        if class_name {
            GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32)
        } else {
            GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32)
        }
    };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

#[cfg(target_os = "windows")]
fn wallpaper_engine_is_running() -> bool {
    ["WPETrayWindow", "WPEEventWindow"]
        .iter()
        .any(|class_name| {
            let class_name = wide_null(class_name);
            !unsafe { FindWindowW(class_name.as_ptr(), std::ptr::null()) }.is_null()
        })
}

#[cfg(target_os = "windows")]
fn wait_for_wallpaper_engine_ready() -> Result<(), String> {
    let deadline = Instant::now() + ENGINE_READY_TIMEOUT;
    loop {
        if wallpaper_engine_is_running() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("Wallpaper Engine 主进程在 20 秒内没有就绪".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn wallpaper_engine_executable(workshop_root: &Path) -> Result<PathBuf, String> {
    let steamapps = workshop_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| "无法从 Workshop 路径定位 Steam steamapps 目录".to_owned())?;
    let directory = steamapps.join("common").join("wallpaper_engine");
    for name in ["wallpaper64.exe", "wallpaper32.exe"] {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return candidate
                .canonicalize()
                .map_err(|error| format!("无法解析 Wallpaper Engine 程序路径：{error}"));
        }
    }
    Err(format!(
        "找不到 Wallpaper Engine 官方程序：{}",
        directory.display()
    ))
}

fn hidden_command(engine: &Path) -> Command {
    let mut command = Command::new(engine);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn close_wallpaper_window(engine: &Path, window_name: &str) {
    let _ = hidden_command(engine)
        .args(["-control", "closeWallpaper", "-location", window_name])
        .spawn();
}

#[cfg(target_os = "windows")]
fn empty_rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn detects_effectively_black_bgra_frames() {
        assert!(bgra_frame_is_effectively_black(&vec![0; 300 * 300 * 4]));
        assert!(bgra_frame_is_effectively_black(&vec![3; 300 * 300 * 4]));

        let mut visible = vec![0; 300 * 300 * 4];
        for pixel in visible.chunks_exact_mut(4).take(2_000) {
            pixel.copy_from_slice(&[12, 24, 48, 255]);
        }
        assert!(!bgra_frame_is_effectively_black(&visible));
    }

    #[test]
    #[ignore = "launches Wallpaper Engine; set RETHEME_TEST_WALLPAPER_SCENE_PROJECT"]
    #[cfg(target_os = "windows")]
    fn measures_scene_capture_cost() {
        let project = std::env::var_os("RETHEME_TEST_WALLPAPER_SCENE_PROJECT")
            .map(PathBuf::from)
            .expect("RETHEME_TEST_WALLPAPER_SCENE_PROJECT");
        let workshop_root = project.parent().expect("Wallpaper Engine workshop root");
        let capture = SceneWallpaperCapture::start(workshop_root, &project.join("project.json"))
            .expect("start Scene capture");
        let mut worker = SceneWallpaperCapture::capture_worker();
        let mut durations = Vec::new();
        let mut sizes = Vec::new();
        for _ in 0..30 {
            std::thread::sleep(SceneWallpaperCapture::frame_interval());
            let started = Instant::now();
            let frame = capture
                .capture_jpeg(&mut worker)
                .expect("capture Scene frame");
            if durations.is_empty()
                && let Some(output) = std::env::var_os("RETHEME_TEST_SCENE_FRAME_OUTPUT")
            {
                std::fs::write(output, frame.jpeg.as_ref()).expect("write Scene frame");
            }
            durations.push(started.elapsed().as_secs_f64() * 1000.0);
            sizes.push(frame.jpeg.len());
        }
        durations.sort_by(f64::total_cmp);
        sizes.sort_unstable();
        let average = durations.iter().sum::<f64>() / durations.len() as f64;
        let p95 = durations[(durations.len() * 95 / 100).min(durations.len() - 1)];
        let average_size = sizes.iter().sum::<usize>() / sizes.len();
        eprintln!(
            "Scene capture: avg={average:.1} ms p95={p95:.1} ms max={:.1} ms avg_size={} KiB",
            durations.last().copied().unwrap_or_default(),
            average_size / 1024
        );
    }
}
