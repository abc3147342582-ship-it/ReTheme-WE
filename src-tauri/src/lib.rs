mod account;
mod api;
mod codex;
mod compatibility;
mod security_config;
mod theme;

use account::{AccountRuntime, AccountStatus, AccountSync, OAuthStart};
use codex::{CodexInstallation, SmokeTestReport, ThemePreviewReport, ThemeRuntime};
use compatibility::CompatibilityRepository;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    Manager, State,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;
use theme::{ThemeInstallReport, ThemeRepository, ThemeSummary};

const TRAY_OPEN_ID: &str = "open";
const TRAY_RESTORE_ID: &str = "restore";
const TRAY_QUIT_ID: &str = "quit";
const TRAY_ID: &str = "retheme";

struct TrayLabels {
    open: &'static str,
    restore: &'static str,
    quit: &'static str,
}

fn tray_labels(locale: &str) -> TrayLabels {
    if locale.starts_with("zh") {
        TrayLabels {
            open: "打开 ReTheme",
            restore: "恢复主题",
            quit: "退出 ReTheme",
        }
    } else {
        TrayLabels {
            open: "Open ReTheme",
            restore: "Restore Theme",
            quit: "Quit ReTheme",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEnvironment {
    app_version: String,
    theme_runtime_version: String,
    compatibility: Option<compatibility::CompatibilityStatus>,
}

#[derive(Debug, PartialEq, Eq)]
enum TrayAction {
    Open,
    Restore,
    Quit,
}

fn tray_action(menu_id: &str) -> Option<TrayAction> {
    match menu_id {
        TRAY_OPEN_ID => Some(TrayAction::Open),
        TRAY_RESTORE_ID => Some(TrayAction::Restore),
        TRAY_QUIT_ID => Some(TrayAction::Quit),
        _ => None,
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(window: &tauri::Window) {
    let _ = window.hide();
    #[cfg(target_os = "macos")]
    let _ = window.app_handle().set_dock_visibility(false);
}

fn should_hide_main_window(label: &str, close_requested: bool) -> bool {
    label == "main" && close_requested
}

fn restore_theme(app: &tauri::AppHandle, exit_after_restore: bool) {
    let app = app.clone();
    let runtime = app.state::<ThemeRuntime>().inner().clone();
    let account = app.state::<AccountRuntime>().inner().clone();
    tauri::async_runtime::spawn(async move {
        let _ =
            tauri::async_runtime::spawn_blocking(move || codex::stop_theme_preview(&runtime)).await;
        if exit_after_restore {
            account.deactivate().await;
            app.exit(0);
        }
    });
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let labels = tray_labels("zh-CN");
    let open = MenuItemBuilder::with_id(TRAY_OPEN_ID, labels.open).build(app)?;
    let restore = MenuItemBuilder::with_id(TRAY_RESTORE_ID, labels.restore).build(app)?;
    let quit = MenuItemBuilder::with_id(TRAY_QUIT_ID, labels.quit).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &restore, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("ReTheme")
        .menu(&menu)
        .on_menu_event(|app, event| match tray_action(event.id().as_ref()) {
            Some(TrayAction::Open) => show_main_window(app),
            Some(TrayAction::Restore) => restore_theme(app, false),
            Some(TrayAction::Quit) => restore_theme(app, true),
            None => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    #[cfg(target_os = "windows")]
    {
        builder = builder.icon(tauri::include_image!("icons/tray-windows.png"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[tauri::command]
fn sync_tray_locale(locale: String, app: tauri::AppHandle) -> Result<(), String> {
    api::set_language(&locale);
    let labels = tray_labels(&locale);
    let open = MenuItemBuilder::with_id(TRAY_OPEN_ID, labels.open)
        .build(&app)
        .map_err(|error| error.to_string())?;
    let restore = MenuItemBuilder::with_id(TRAY_RESTORE_ID, labels.restore)
        .build(&app)
        .map_err(|error| error.to_string())?;
    let quit = MenuItemBuilder::with_id(TRAY_QUIT_ID, labels.quit)
        .build(&app)
        .map_err(|error| error.to_string())?;
    let menu = MenuBuilder::new(&app)
        .items(&[&open, &restore, &quit])
        .build()
        .map_err(|error| error.to_string())?;
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "ReTheme 托盘尚未初始化".to_owned())?;
    tray.set_menu(Some(menu)).map_err(|error| error.to_string())
}

#[tauri::command]
fn detect_codex() -> Result<CodexInstallation, String> {
    codex::detect().map_err(|error| error.to_string())
}

#[tauri::command]
async fn run_cdp_smoke_test() -> Result<SmokeTestReport, String> {
    tauri::async_runtime::spawn_blocking(codex::run_smoke_test)
        .await
        .map_err(|error| format!("测试任务异常结束：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_themes(themes: State<'_, ThemeRepository>) -> Result<Vec<ThemeSummary>, String> {
    themes.list().map_err(|error| error.to_string())
}

#[tauri::command]
async fn uninstall_theme(
    theme_id: String,
    runtime: State<'_, ThemeRuntime>,
    themes: State<'_, ThemeRepository>,
) -> Result<bool, String> {
    let runtime = runtime.inner().clone();
    let themes = themes.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        if runtime.current_theme_id().as_deref() == Some(theme_id.as_str()) {
            codex::stop_theme_preview(&runtime).map_err(|error| error.to_string())?;
        }
        themes
            .uninstall(&theme_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("主题卸载任务异常结束：{error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_theme_preview(
    theme_id: String,
    locale: String,
    runtime: State<'_, ThemeRuntime>,
    themes: State<'_, ThemeRepository>,
    account: State<'_, AccountRuntime>,
    compatibility: State<'_, CompatibilityRepository>,
) -> Result<ThemePreviewReport, String> {
    let runtime = runtime.inner().clone();
    let themes = themes.inner().clone();
    let compatibility = compatibility.inner().clone();
    let has_pro = account.has_active_pro();
    let slug = themes
        .online_slug(&theme_id)
        .map_err(|error| error.to_string())?;
    let expires_at = account
        .authorize_theme(&slug)
        .await
        .map_err(|error| error.to_string())?
        .expires_at;
    if let Ok(installation) = codex::detect() {
        let _ = compatibility.refresh(installation.version()).await;
    }
    tauri::async_runtime::spawn_blocking(move || {
        codex::start_theme_preview_until(
            &runtime,
            &themes,
            &compatibility,
            &theme_id,
            expires_at,
            has_pro,
            &locale,
        )
    })
    .await
    .map_err(|error| format!("主题预览任务异常结束：{error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_local_theme_preview(
    theme_path: PathBuf,
    locale: String,
    runtime: State<'_, ThemeRuntime>,
    themes: State<'_, ThemeRepository>,
    account: State<'_, AccountRuntime>,
    compatibility: State<'_, CompatibilityRepository>,
) -> Result<ThemePreviewReport, String> {
    let runtime = runtime.inner().clone();
    let themes = themes.inner().clone();
    let compatibility = compatibility.inner().clone();
    let has_pro = account.has_active_pro();
    if let Ok(installation) = codex::detect() {
        let _ = compatibility.refresh(installation.version()).await;
    }
    tauri::async_runtime::spawn_blocking(move || {
        codex::start_development_theme_preview(
            &runtime,
            &themes,
            &compatibility,
            &theme_path,
            None,
            has_pro,
            &locale,
        )
    })
    .await
    .map_err(|error| format!("本地主题预览任务异常结束：{error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn sync_theme_locale(
    locale: String,
    runtime: State<'_, ThemeRuntime>,
) -> Result<bool, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || codex::sync_theme_locale(&runtime, &locale))
        .await
        .map_err(|error| format!("主题语言同步任务异常结束：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn account_status(account: State<'_, AccountRuntime>) -> AccountStatus {
    account.status()
}

#[tauri::command]
async fn account_sync(account: State<'_, AccountRuntime>) -> Result<AccountSync, String> {
    account.sync().await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_login(
    email: String,
    password: String,
    account: State<'_, AccountRuntime>,
) -> Result<AccountStatus, String> {
    account
        .login(email, password)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_oauth_start(
    provider: String,
    account: State<'_, AccountRuntime>,
) -> Result<OAuthStart, String> {
    account
        .start_oauth(provider)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_oauth_complete(
    code: String,
    state: String,
    account: State<'_, AccountRuntime>,
) -> Result<AccountStatus, String> {
    account
        .complete_oauth(code, state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_request_register_code(
    email: String,
    account: State<'_, AccountRuntime>,
) -> Result<Option<String>, String> {
    account
        .request_register_code(email)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_register(
    email: String,
    code: String,
    password: String,
    account: State<'_, AccountRuntime>,
) -> Result<AccountStatus, String> {
    account
        .register(email, code, password)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn account_logout(account: State<'_, AccountRuntime>) -> Result<AccountStatus, String> {
    account.logout().await;
    Ok(account.status())
}

#[tauri::command]
async fn account_redeem_cdk(
    code: String,
    account: State<'_, AccountRuntime>,
) -> Result<AccountStatus, String> {
    account
        .redeem_cdk(code)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn download_online_theme(
    slug: String,
    account: State<'_, AccountRuntime>,
    themes: State<'_, ThemeRepository>,
) -> Result<ThemeInstallReport, String> {
    account
        .download_theme(slug, themes.inner())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn stop_theme_preview(runtime: State<'_, ThemeRuntime>) -> Result<bool, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || codex::stop_theme_preview(&runtime))
        .await
        .map_err(|error| format!("恢复任务异常结束：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn theme_preview_status(
    runtime: State<'_, ThemeRuntime>,
) -> Result<Option<ThemePreviewReport>, String> {
    runtime.current_preview().map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime_environment(compatibility: State<'_, CompatibilityRepository>) -> RuntimeEnvironment {
    RuntimeEnvironment {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        theme_runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
        compatibility: codex::detect()
            .ok()
            .map(|installation| compatibility.status(installation.version())),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(ThemeRuntime::default())
        .setup(|app| {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            app.deep_link()
                .register_all()
                .map_err(|error| format!("无法注册 ReTheme 链接协议：{error}"))?;
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("无法定位应用数据目录：{error}"))?;
            let account = AccountRuntime::new(app_data_dir.join("account"))
                .map_err(|error| error.to_string())?;
            let repository = ThemeRepository::new_with_cache_key(
                app_data_dir.join("themes"),
                account
                    .theme_cache_key()
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("无法初始化主题仓库：{error}"))?;
            app.manage(repository);
            let compatibility = CompatibilityRepository::new(app_data_dir.join("compatibility"))
                .map_err(|error| error.to_string())?;
            app.manage(compatibility.clone());
            app.manage(account.clone());
            let handle = app.handle().clone();
            let lease_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(codex::PAGE_LEASE_RENEW_INTERVAL);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    let runtime = lease_handle.state::<ThemeRuntime>().inner().clone();
                    let _ =
                        tauri::async_runtime::spawn_blocking(move || runtime.renew_page_lease())
                            .await;
                }
            });
            tauri::async_runtime::spawn(async move {
                if let Ok(installation) = codex::detect() {
                    let _ = compatibility.refresh(installation.version()).await;
                }
                account.initialize().await;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    let _ = account.maintain().await;
                }
            });
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let close_requested = matches!(event, tauri::WindowEvent::CloseRequested { .. });
            if should_hide_main_window(window.label(), close_requested)
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                hide_main_window(window);
            }
        })
        .invoke_handler(tauri::generate_handler![
            detect_codex,
            run_cdp_smoke_test,
            list_themes,
            uninstall_theme,
            start_theme_preview,
            start_local_theme_preview,
            stop_theme_preview,
            theme_preview_status,
            runtime_environment,
            sync_tray_locale,
            sync_theme_locale,
            account_status,
            account_sync,
            account_login,
            account_oauth_start,
            account_oauth_complete,
            account_request_register_code,
            account_register,
            account_logout,
            account_redeem_cdk,
            download_online_theme
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ReTheme");
}

#[cfg(test)]
mod tests {
    use super::{TrayAction, should_hide_main_window, tray_action, tray_labels};

    #[test]
    fn parses_supported_tray_actions() {
        assert_eq!(tray_action("open"), Some(TrayAction::Open));
        assert_eq!(tray_action("restore"), Some(TrayAction::Restore));
        assert_eq!(tray_action("quit"), Some(TrayAction::Quit));
    }

    #[test]
    fn ignores_informational_and_unknown_tray_items() {
        assert_eq!(tray_action("unknown"), None);
    }

    #[test]
    fn localizes_short_tray_labels() {
        assert_eq!(tray_labels("zh-CN").restore, "恢复主题");
        assert_eq!(tray_labels("en").restore, "Restore Theme");
    }

    #[test]
    fn hides_only_the_main_window_on_close() {
        assert!(should_hide_main_window("main", true));
        assert!(!should_hide_main_window("main", false));
        assert!(!should_hide_main_window("account", true));
    }
}
