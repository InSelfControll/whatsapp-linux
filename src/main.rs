//! WhatsApp Web Desktop Wrapper
//!
//! A minimal wry application that loads WhatsApp Web with a spoofed User-Agent.
//! Supports voice message recording, file viewing, downloads, and notifications.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::{WebContext, WebViewBuilder};

/// WhatsApp Desktop macOS User-Agent - mimics official Electron app
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) WhatsApp/2.24.6.80 Chrome/120.0.6099.291 Electron/28.2.5 Safari/537.36";

/// JavaScript to spoof navigator as macOS and add call feature flags
/// Minimal spoofing to avoid breaking WhatsApp loading
const SPOOF_SCRIPT: &str = r#"
(function() {
    'use strict';

    try {
        // ========== NAVIGATOR SPOOFING ==========
        Object.defineProperty(navigator, 'platform', {
            get: function() { return 'MacIntel'; },
            configurable: true
        });

        Object.defineProperty(navigator, 'vendor', {
            get: function() { return 'Google Inc.'; },
            configurable: true
        });

        // ========== CALL FEATURE FLAGS ==========
        // These flags tell WhatsApp to enable call UI
        window.mms_can_video_call = true;
        window.mms_can_voice_call = true;
        window.__ELECTRON__ = true;
        window.__WHATSAPP_DESKTOP__ = true;

        // Force desktop calling flag
        if (window.Debug && typeof window.Debug.setFeatureFlag === 'function') {
            try {
                window.Debug.setFeatureFlag('calling_desktop_enabled', true);
                console.log('[WhatsApp Desktop] Calling desktop enabled via Debug flag');
            } catch (e) {
                console.log('[WhatsApp Desktop] Debug.setFeatureFlag not available:', e);
            }
        }

        console.log('[WhatsApp Desktop] Spoof initialized');
        console.log('[WhatsApp Desktop] Platform:', navigator.platform);
        console.log('[WhatsApp Desktop] mms_can_video_call:', window.mms_can_video_call);

    } catch (e) {
        console.error('[WhatsApp Desktop] Spoof error:', e);
    }
})();
"#;

/// JavaScript to mute notifications while keeping media audio
const NOTIFICATION_MUTE_SCRIPT: &str = r#"
(function() {
    'use strict';

    // Mute state - stored in localStorage for persistence
    window.__notificationsMuted = localStorage.getItem('whatsapp_notifications_muted') === 'true';

    // Show mute status indicator
    function showMuteIndicator(muted) {
        // Remove existing indicator
        const existing = document.getElementById('mute-indicator');
        if (existing) existing.remove();

        const indicator = document.createElement('div');
        indicator.id = 'mute-indicator';
        indicator.style.cssText = 'position:fixed;top:10px;left:50%;transform:translateX(-50%);background:' +
            (muted ? '#ff6b6b' : '#25D366') + ';color:white;padding:8px 16px;border-radius:20px;z-index:999999;font-size:13px;box-shadow:0 2px 10px rgba(0,0,0,0.2);transition:opacity 0.3s;';
        indicator.textContent = muted ? 'Notifications Muted (Ctrl+Shift+M to unmute)' : 'Notifications Enabled';
        document.body.appendChild(indicator);

        // Fade out after 2 seconds
        setTimeout(() => {
            indicator.style.opacity = '0';
            setTimeout(() => indicator.remove(), 300);
        }, 2000);
    }

    // Toggle mute function
    window.toggleNotificationMute = function() {
        window.__notificationsMuted = !window.__notificationsMuted;
        localStorage.setItem('whatsapp_notifications_muted', window.__notificationsMuted);
        showMuteIndicator(window.__notificationsMuted);
        console.log('[Notifications] Muted:', window.__notificationsMuted);
    };

    // Override Notification API to block notifications when muted
    const OriginalNotification = window.Notification;

    class MutedNotification {
        constructor(title, options) {
            if (window.__notificationsMuted) {
                console.log('[Notifications] Blocked notification:', title);
                // Create a dummy notification that does nothing
                this.title = title;
                this.options = options;
                this.onclick = null;
                this.onclose = null;
                this.onerror = null;
                this.onshow = null;
            } else {
                // Create real notification
                return new OriginalNotification(title, options);
            }
        }

        close() {}

        static get permission() {
            return OriginalNotification.permission;
        }

        static requestPermission(callback) {
            return OriginalNotification.requestPermission(callback);
        }
    }

    // Replace Notification constructor
    window.Notification = MutedNotification;

    // Intercept notification sounds only - preserve voice messages
    const OriginalAudio = window.Audio;
    window.Audio = function(src) {
        const audio = new OriginalAudio(src);

        // Check if this might be a notification sound
        const originalPlay = audio.play.bind(audio);
        audio.play = function() {
            if (window.__notificationsMuted) {
                // Check current src as it might have changed since constructor
                const currentSrc = this.src || src;
                
                if (!currentSrc) return originalPlay();

                // More specific check for notification sounds
                // Removed blob: check as it was catching voice notes
                const isNotificationSound = 
                    (typeof currentSrc === 'string' && (
                        currentSrc.includes('notification') ||
                        currentSrc.includes('alert') ||
                        currentSrc.includes('ping')
                    ));

                // Additional check: if this audio is inside a message element, it's likely a voice message
                const audioElement = this;
                const isInMessage = audioElement.closest &&
                    (audioElement.closest('[data-id]') ||
                     audioElement.closest('[data-testid*="message"]') ||
                     audioElement.closest('[role="row"]') ||
                     audioElement.closest('.message-in, .message-out') ||
                     audioElement.closest('div[data-testid*="media"]'));

                if (isNotificationSound && !isInMessage) {
                    console.log('[Notifications] Blocked notification sound:', src);
                    return Promise.resolve();
                }
            }
            return originalPlay();
        };

        return audio;
    };
    window.Audio.prototype = OriginalAudio.prototype;

    // Monitor for dynamically created audio elements - only mute notifications
    const observer = new MutationObserver((mutations) => {
        if (!window.__notificationsMuted) return;

        mutations.forEach((mutation) => {
            mutation.addedNodes.forEach((node) => {
                if (node.nodeName === 'AUDIO') {
                    // Check if it's NOT inside a message (voice message/video/audio file)
                    const isInMessage = node.closest &&
                        (node.closest('[data-id]') ||
                         node.closest('[data-testid*="message"]') ||
                         node.closest('[role="row"]') ||
                         node.closest('.message-in, .message-out') ||
                         node.closest('div[data-testid*="media"]') ||
                         node.closest('audio') ||
                         node.closest('video'));

                    if (!isInMessage) {
                        // Only mute if it looks like a notification sound
                        const src = node.src || node.currentSrc;
                        
                        if (!src) return;

                        const isNotification = 
                             src.includes('notification') ||
                             src.includes('alert') ||
                             src.includes('ping');

                        if (isNotification) {
                            node.muted = true;
                            node.volume = 0;
                            console.log('[Notifications] Muted notification audio element');
                        }
                    }
                }
            });
        });
    });

    observer.observe(document.body, { childList: true, subtree: true });

    // Keyboard shortcut: Ctrl+Shift+M to toggle mute
    document.addEventListener('keydown', function(e) {
        if (e.ctrlKey && e.shiftKey && e.key === 'M') {
            e.preventDefault();
            window.toggleNotificationMute();
        }
    });

    // Show initial status if muted
    if (window.__notificationsMuted) {
        setTimeout(() => showMuteIndicator(true), 1000);
    }

    console.log('[WhatsApp Desktop] Notification mute system initialized. Muted:', window.__notificationsMuted);
})();
"#;

/// Supported browsers for opening PDFs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Browser {
    Brave,
    Firefox,
    Chrome,
    Chromium,
    System, // Use xdg-open
}

impl Browser {
    fn display_name(&self) -> &'static str {
        match self {
            Browser::Brave => "Brave",
            Browser::Firefox => "Firefox",
            Browser::Chrome => "Google Chrome",
            Browser::Chromium => "Chromium",
            Browser::System => "System Default",
        }
    }

    #[cfg(target_os = "linux")]
    fn command(&self) -> &'static str {
        match self {
            Browser::Brave => "brave-browser",
            Browser::Firefox => "firefox",
            Browser::Chrome => "google-chrome",
            Browser::Chromium => "chromium",
            Browser::System => "xdg-open",
        }
    }
}

/// Document opening preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocHandler {
    GoogleDocs,
    LocalApp,
}

impl DocHandler {
    fn display_name(&self) -> &'static str {
        match self {
            DocHandler::GoogleDocs => "Google Docs (Browser)",
            DocHandler::LocalApp => "Local Application",
        }
    }
}

/// User preferences configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub pdf_browser: Option<Browser>,
    pub doc_handler: Option<DocHandler>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pdf_browser: None,
            doc_handler: None,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whatsapp-desktop")
            .join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }
}

/// Detect which browsers are installed on the system
#[cfg(target_os = "linux")]
fn detect_installed_browsers() -> Vec<Browser> {
    let mut browsers = Vec::new();

    let browser_commands = [
        (Browser::Brave, "brave-browser"),
        (Browser::Firefox, "firefox"),
        (Browser::Chrome, "google-chrome"),
        (Browser::Chromium, "chromium"),
    ];

    for (browser, cmd) in browser_commands {
        if Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            browsers.push(browser);
        }
    }

    browsers
}

/// Show a dialog to let user pick from options using zenity
#[cfg(target_os = "linux")]
fn show_selection_dialog(title: &str, text: &str, options: &[&str]) -> Option<usize> {
    // Try zenity first
    let result = Command::new("zenity")
        .args([
            "--list",
            "--radiolist",
            "--title",
            title,
            "--text",
            text,
            "--column",
            "Select",
            "--column",
            "Option",
        ])
        .args(
            options
                .iter()
                .enumerate()
                .flat_map(|(i, opt)| {
                    if i == 0 {
                        vec!["TRUE", *opt]
                    } else {
                        vec!["FALSE", *opt]
                    }
                })
                .collect::<Vec<_>>(),
        )
        .output();

    if let Ok(output) = result {
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return options.iter().position(|&o| o == selected);
        }
    }

    // Fallback to kdialog
    let result = Command::new("kdialog")
        .args(["--menu", text])
        .args(
            options
                .iter()
                .flat_map(|opt| [opt, opt])
                .collect::<Vec<_>>(),
        )
        .output();

    if let Ok(output) = result {
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return options.iter().position(|&o| o == selected);
        }
    }

    None
}

/// Open a file with the system default application
fn open_with_system(path: &PathBuf) {
    eprintln!("[SYSTEM] Opening: {:?}", path);

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn();
    }
}

/// Get downloads directory based on platform
fn get_downloads_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| {
        #[cfg(target_os = "linux")]
        return PathBuf::from("~/Downloads");

        #[cfg(not(target_os = "linux"))]
        return PathBuf::from(".");
    })
}

/// Handle file opening based on user preferences
fn handle_file_open(path: &PathBuf, config: &mut Config) {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "pdf" => {
            let installed = detect_installed_browsers();
            let browsers: Vec<_> = installed
                .iter()
                .chain(std::iter::once(&Browser::System))
                .map(|b| b.display_name())
                .collect();

            if let Some(idx) =
                show_selection_dialog("Open PDF", "Select browser to open PDF files:", &browsers)
            {
                let selected = if idx < installed.len() {
                    installed[idx]
                } else {
                    Browser::System
                };

                config.pdf_browser = Some(selected);

                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new(selected.command()).arg(path).spawn();
                }

                #[cfg(not(target_os = "linux"))]
                {
                    open_with_system(path);
                }
            }
        }

        "odt" | "odp" | "ods" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => {
            let handlers = [DocHandler::GoogleDocs, DocHandler::LocalApp];
            let options: Vec<&str> = handlers.iter().map(|h| h.display_name()).collect();

            if let Some(idx) = show_selection_dialog(
                "Open Document",
                "How would you like to open this document?",
                &options,
            ) {
                if let Some(handler) = handlers.get(idx) {
                    config.doc_handler = Some(*handler);

                    if matches!(handler, DocHandler::GoogleDocs) {
                        let google_docs_url = "https://docs.google.com/document/upload";
                        let _ = Command::new("xdg-open").arg(google_docs_url).spawn();
                    } else {
                        open_with_system(path);
                    }
                }
            }
        }

        "mp4" | "webm" | "mov" | "mkv" | "avi" | "mp3" | "wav" | "ogg" | "flac" | "jpg"
        | "jpeg" | "png" | "gif" | "webp" | "bmp" | "zip" | "rar" | "7z" | "tar" | "gz" | "xz"
        | "txt" | "rtf" | "csv" => {
            open_with_system(path);
        }

        _ => {
            open_with_system(path);
        }
    }
}

/// Fix file extension based on content sniffing
fn fix_file_extension(path: &PathBuf) -> PathBuf {
    if !path.exists() {
        return path.clone();
    }

    let file = File::open(path);
    if file.is_err() {
        return path.clone();
    }
    let mut file = file.unwrap();

    let mut buffer = [0u8; 16];
    let bytes_read = file.read(&mut buffer);
    if bytes_read.is_err() {
        return path.clone();
    }
    let bytes_read = bytes_read.unwrap();

    let new_ext = match bytes_read {
        8 => match &buffer[0..8] {
            b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A" => Some("png"),
            _ => None,
        },
        4 => match &buffer[0..4] {
            b"\xFF\xD8\xFF\xE0" | b"\xFF\xD8\xFF\xE1" | b"\xFF\xD8\xFF\xE2"
            | b"\xFF\xD8\xFF\xE3" | b"\xFF\xD8\xFF\xE8" => Some("jpg"),
            b"RIFF" => Some("webp"),
            b"GIF8" => Some("gif"),
            _ => None,
        },
        _ => None,
    };

    if let Some(ext) = new_ext {
        let mut new_path = path.clone();
        new_path.set_extension(ext);
        if new_path != *path {
            eprintln!("[FILE] Fixing extension: {:?} -> {:?}", path, new_path);
            let _ = std::fs::rename(path, &new_path);
            return new_path;
        }
    }

    path.clone()
}

/// Load app icon from embedded data
fn load_icon() -> Option<Icon> {
    let icon_data = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_data).ok()?;
    let pixels = img.to_rgba8();
    let (width, height) = pixels.dimensions();
    Icon::from_rgba(pixels.into_raw(), width, height).ok()
}

#[cfg(target_os = "linux")]
fn setup_webview_for_calls(webview: &webkit2gtk::WebView) {
    use webkit2gtk::{
        CookieAcceptPolicy, CookieManagerExt, CookiePersistentStorage, PermissionRequestExt,
        SettingsExt, WebContextExt, WebViewExt,
    };

    if let Some(settings) = webview.settings() {
        // ========== HARDWARE ACCELERATION FOR PERFORMANCE ==========
        settings.set_enable_webgl(true);
        settings.set_enable_javascript(true);
        settings.set_javascript_can_open_windows_automatically(true);

        // ========== PERFORMANCE OPTIMIZATIONS ==========
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_write_console_messages_to_stdout(false); // Reduce console spam
        settings.set_enable_page_cache(true);
        settings.set_enable_back_forward_navigation_gestures(false); // Disable unused gestures

        // ========== MEDIA SETTINGS FOR CALLS ==========
        settings.set_enable_media_stream(true);
        settings.set_enable_webrtc(true);
        settings.set_enable_mediasource(true);
        settings.set_enable_webaudio(true);
        settings.set_enable_media(true);
        settings.set_enable_media_capabilities(true);
        settings.set_media_playback_requires_user_gesture(false);

        // Clipboard - CRITICAL for paste to work
        settings.set_javascript_can_access_clipboard(true);

        // ========== STORAGE FOR SESSION PERSISTENCE ==========
        settings.set_enable_html5_local_storage(true);
        settings.set_enable_html5_database(true);
        settings.set_enable_offline_web_application_cache(true);

        // ========== DEVELOPER TOOLS (for debugging, can disable for prod) ==========
        settings.set_enable_developer_extras(true);
        settings.set_allow_modal_dialogs(true);
        settings.set_enable_resizable_text_areas(true);
        settings.set_enable_fullscreen(true);

        eprintln!("[SETTINGS] Performance optimizations enabled");
    }

    // Set up persistent cookies
    if let Some(context) = webview.context() {
        let cookie_manager = context.cookie_manager().unwrap();

        let cookie_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whatsapp-desktop")
            .join("cookies.txt");

        if let Some(parent) = cookie_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        cookie_manager.set_persistent_storage(
            cookie_path.to_str().unwrap_or("cookies.txt"),
            CookiePersistentStorage::Text,
        );
        cookie_manager.set_accept_policy(CookieAcceptPolicy::Always);

        eprintln!("[COOKIES] Persistent storage at: {:?}", cookie_path);
    }

    webview.connect_permission_request(|_webview, permission_request| {
        eprintln!("[PERMISSION] Request received - auto-granting");
        permission_request.allow();
        true
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = std::sync::Arc::new(std::sync::Mutex::new(Config::load()));

    if let Ok(cfg) = config.lock() {
        cfg.save();
    }

    let config_clone = config.clone();

    let event_loop = EventLoop::new();

    let mut window_builder = WindowBuilder::new()
        .with_title("WhatsApp")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(400.0, 400.0));

    if let Some(icon) = load_icon() {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    let window = window_builder.build(&event_loop)?;

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whatsapp-desktop")
        .join("webdata");

    if let Err(e) = fs::create_dir_all(&data_dir) {
        eprintln!("[WARN] Failed to create data directory: {}", e);
    }

    eprintln!("[INFO] WebView data directory: {:?}", data_dir);

    let mut web_context = WebContext::new(Some(data_dir));

    let builder = WebViewBuilder::with_web_context(&mut web_context)
        .with_user_agent(USER_AGENT)
        .with_initialization_script(SPOOF_SCRIPT)
        .with_initialization_script(NOTIFICATION_MUTE_SCRIPT)
        .with_autoplay(true)
        .with_url("https://web.whatsapp.com")
        .with_navigation_handler(|url| {
            let dominated = url.starts_with("https://web.whatsapp.com")
                || url.starts_with("blob:")
                || url.starts_with("data:")
                || url.starts_with("https://mmg.whatsapp.net")
                || url.starts_with("https://static.whatsapp.net")
                || url.starts_with("https://pps.whatsapp.net");

            if !dominated {
                eprintln!("[NAV] URL: {}", url);
            }
            true
        })
        .with_new_window_req_handler(|url| {
            eprintln!("[EXTERNAL LINK] Opening: {}", url);

            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("xdg-open").arg(&url).spawn();
            }

            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(&url).spawn();
            }

            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("cmd").args(["/C", "start", "", &url]).spawn();
            }

            false
        })
        .with_download_started_handler(move |url, download_path| {
            let downloads_dir = get_downloads_dir();

            let filename = url
                .split('/')
                .last()
                .and_then(|s| s.split('?').next())
                .filter(|s| !s.is_empty() && s.len() < 255)
                .unwrap_or("whatsapp_download");

            let clean_filename: String = filename
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();

            let dest = downloads_dir.join(&clean_filename);
            eprintln!("[DOWNLOAD] Starting: {} -> {:?}", url, dest);

            *download_path = dest;
            true
        })
        .with_download_completed_handler(move |_url, path, success| {
            if success {
                if let Some(file_path) = path {
                    eprintln!("[DOWNLOAD] Completed: {:?}", file_path);

                    let fixed_path = fix_file_extension(&file_path);

                    if let Ok(mut cfg) = config_clone.lock() {
                        handle_file_open(&fixed_path, &mut cfg);
                    }
                }
            } else {
                eprintln!("[DOWNLOAD] Failed: {:?}", path);
            }
        })
        .with_devtools(true);

    #[cfg(target_os = "linux")]
    let _webview = {
        use gtk::prelude::*;
        use tao::platform::unix::WindowExtUnix;
        use webkit2gtk::WebViewExt as WebkitWebViewExt;
        use wry::{WebViewBuilderExtUnix, WebViewExtUnix};

        let vbox = window.default_vbox().unwrap();
        let webview = builder.build_gtk(vbox)?;

        let gtk_webview = webview.webview();
        setup_webview_for_calls(&gtk_webview);

        let webview_for_paste = gtk_webview.clone();

        gtk_webview.clone().connect_key_press_event(move |_wv, event| {
            let keyval = event.keyval();
            let state = event.state();

            // Check for Ctrl+Shift+M (toggle notifications)
            if state.contains(gdk::ModifierType::CONTROL_MASK)
                && state.contains(gdk::ModifierType::SHIFT_MASK)
                && (keyval == gdk::keys::constants::m || keyval == gdk::keys::constants::M)
            {
                let script = r#"
                    (function() {
                        if (!window.toggleNotificationMute) return;

                        // Show what action will be taken
                        const currentlyMuted = window.__notificationsMuted;
                        const actionText = currentlyMuted ? 'Enabling notifications...' : 'Muting notifications...';
                        const finalText = currentlyMuted ? 'Notifications Enabled' : 'Notifications Muted';
                        const backgroundColor = currentlyMuted ? '#25D366' : '#ff6b6b';

                        const indicator = document.createElement('div');
                        indicator.style.cssText = 'position:fixed;top:10px;left:50%;transform:translateX(-50%);background:' + backgroundColor +
                            ';color:white;padding:8px 16px;border-radius:20px;z-index:999999;font-size:13px;box-shadow:0 2px 10px rgba(0,0,0,0.2);';

                        indicator.textContent = actionText;
                        document.body.appendChild(indicator);

                        // Toggle after showing action
                        window.toggleNotificationMute();

                        // Update text after toggle
                        setTimeout(() => {
                            indicator.textContent = finalText;
                        }, 200);

                        // Fade out
                        setTimeout(() => {
                            indicator.style.opacity = '0';
                            setTimeout(() => indicator.remove(), 300);
                        }, 2000);
                    })();
                "#;
                gtk_webview.run_javascript(script, None::<&gio::Cancellable>, |_| {});
                return glib::Propagation::Stop;
            }

            // Check for Ctrl+V
            if state.contains(gdk::ModifierType::CONTROL_MASK)
                && (keyval == gdk::keys::constants::v || keyval == gdk::keys::constants::V)
            {
                let display = gdk::Display::default().unwrap();
                let clipboard = gtk::Clipboard::default(&display).unwrap();

                if clipboard.wait_is_image_available() {
                    eprintln!("[CLIPBOARD] Image found in clipboard");

                    if let Some(pixbuf) = clipboard.wait_for_image() {
                        eprintln!(
                            "[CLIPBOARD] Got pixbuf: {}x{}",
                            pixbuf.width(),
                            pixbuf.height()
                        );

                        if let Ok(png_data) = pixbuf.save_to_bufferv("png", &[]) {
                            use base64::Engine;
                            let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_data);

                            eprintln!("[CLIPBOARD] Encoded {} bytes to base64", png_data.len());

                            let script = format!(r##"
                                (async function() {{
                                    const base64Data = '{}';

                                    const indicator = document.createElement('div');
                                    indicator.style.cssText = 'position:fixed;top:10px;right:10px;background:#25D366;color:white;padding:10px 15px;border-radius:8px;z-index:999999;font-size:14px;box-shadow:0 2px 10px rgba(0,0,0,0.2);';
                                    indicator.textContent = 'Pasting image...';
                                    document.body.appendChild(indicator);

                                    try {{
                                        const byteChars = atob(base64Data);
                                        const byteNumbers = new Array(byteChars.length);
                                        for (let i = 0; i < byteChars.length; i++) {{
                                            byteNumbers[i] = byteChars.charCodeAt(i);
                                        }}
                                        const byteArray = new Uint8Array(byteNumbers);
                                        const blob = new Blob([byteArray], {{ type: 'image/png' }});
                                        const file = new File([blob], 'pasted-image.png', {{ type: 'image/png', lastModified: Date.now() }});

                                        console.log('[Clipboard] Created file:', file.size, 'bytes');
                                        indicator.textContent = 'Image: ' + Math.round(file.size/1024) + 'KB';

                                        const messageInput = document.querySelector('[contenteditable="true"][data-tab="10"]') ||
                                                            document.querySelector('[contenteditable="true"]') ||
                                                            document.querySelector('footer [contenteditable="true"]');

                                        if (messageInput) {{
                                            messageInput.focus();
                                        }}

                                        const dt = new DataTransfer();
                                        dt.items.add(file);

                                        const pasteEvent = new ClipboardEvent('paste', {{
                                            bubbles: true,
                                            cancelable: true,
                                            clipboardData: dt
                                        }});

                                        const target = messageInput || document.activeElement || document;
                                        const handled = !target.dispatchEvent(pasteEvent);
                                        console.log('[Clipboard] Paste event dispatched, handled:', handled);

                                        setTimeout(() => {{
                                            const previewCaption = document.querySelector('[data-testid="media-caption-input"]') ||
                                                                  document.querySelector('div[contenteditable="true"][role="textbox"][data-tab="10"]');

                                            if (previewCaption) {{
                                                indicator.textContent = 'Ready to add caption!';
                                            }} else {{
                                                console.log('[Clipboard] No preview, trying file input fallback...');
                                                const attachBtn = document.querySelector('[data-icon="attach-menu-plus"]') ||
                                                                  document.querySelector('[data-icon="plus"]');
                                                if (attachBtn) {{
                                                    const clickable = attachBtn.closest('button') || attachBtn.closest('div[role="button"]') || attachBtn;
                                                    clickable.click();

                                                    setTimeout(() => {{
                                                        const input = document.querySelector('input[type="file"][accept*="image"]');
                                                        if (input) {{
                                                            const dt = new DataTransfer();
                                                            dt.items.add(file);
                                                            input.files = dt.files;
                                                            input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                            indicator.textContent = 'Image attached (no caption)';
                                                        }}
                                                    }}, 400);
                                                }}
                                            }}
                                            setTimeout(() => indicator.remove(), 2000);
                                        }}, 500);

                                    }} catch (err) {{
                                        console.error('[Clipboard] Error:', err);
                                        indicator.textContent = 'Error: ' + err.message;
                                        indicator.style.background = '#ff6b6b';
                                        setTimeout(() => indicator.remove(), 2000);
                                    }}
                                }})();
                            "##, base64_data);

                            webview_for_paste.run_javascript(&script, None::<&gio::Cancellable>, |_| {});

                            return glib::Propagation::Stop;
                        }
                    }
                }

                return glib::Propagation::Proceed;
            }

            glib::Propagation::Proceed
        });

        webview
    };

    #[cfg(not(target_os = "linux"))]
    let _webview = builder.build(&window)?;

    eprintln!("[INFO] WhatsApp Desktop started");
    eprintln!("[INFO] Downloads saved to: {:?}", get_downloads_dir());
    eprintln!("[INFO] Config stored at: {:?}", Config::config_path());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            _ => {}
        }
    });
}
