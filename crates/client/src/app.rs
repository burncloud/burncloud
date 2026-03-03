use dioxus::prelude::*;

use crate::components::{guest_layout::GuestLayout, layout::Layout};
use crate::pages::{
    api::AccessCredentialsPage,
    billing::BillingPage,
    connect::ConnectPage,
    dashboard::Dashboard,
    deploy::DeployConfig,
    home::{HomePage, Root},
    login::LoginPage,
    logs::LogPage,
    models::ChannelPage,
    monitor::ServiceMonitor,
    not_found::NotFoundPage,
    playground::PlaygroundPage,
    settings::SystemSettings,
    user::UserPage,
};
use burncloud_client_register::RegisterPage;
#[cfg(feature = "desktop")]
use burncloud_client_shared::DesktopMode;
#[cfg(all(feature = "desktop", target_os = "windows"))]
pub use burncloud_client_tray::{should_show_window, start_tray};

/// Load the window icon from embedded ICO bytes
/// This function creates a window icon from the embedded favicon.ico file
#[cfg(feature = "desktop")]
fn load_window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    use dioxus::desktop::tao::window::Icon;

    // Embed the favicon.ico file at compile time
    let icon_bytes = include_bytes!("../assets/favicon.ico");

    // Parse ICO file and extract the largest image
    // ICO files contain one or more images, we try to load the best one
    match parse_ico_icon(icon_bytes) {
        Some((rgba, width, height)) => {
            Icon::from_rgba(rgba, width, height).ok()
        }
        None => {
            eprintln!("Failed to parse ICO file for window icon");
            None
        }
    }
}

/// Parse an ICO file and extract the largest icon as RGBA pixels
/// Returns (rgba_bytes, width, height) or None if parsing fails
#[cfg(feature = "desktop")]
fn parse_ico_icon(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    // ICO file format:
    // - 6 bytes header: reserved (2), type (2=ICO), count (2)
    // - 16 bytes per entry: width, height, colors, reserved, planes, bpp, size, offset

    if data.len() < 6 {
        return None;
    }

    // Check ICO magic (type = 1)
    let icon_type = u16::from_le_bytes([data[2], data[3]]);
    if icon_type != 1 {
        return None;
    }

    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    if count == 0 || data.len() < 6 + count * 16 {
        return None;
    }

    // Find the largest icon (by width*height)
    let mut best_entry: Option<(usize, usize, usize)> = None; // (offset, size, area)

    for i in 0..count {
        let entry_start = 6 + i * 16;
        let width = data[entry_start] as usize;
        let height = data[entry_start + 1] as usize;
        let size = u32::from_le_bytes([
            data[entry_start + 8],
            data[entry_start + 9],
            data[entry_start + 10],
            data[entry_start + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            data[entry_start + 12],
            data[entry_start + 13],
            data[entry_start + 14],
            data[entry_start + 15],
        ]) as usize;

        // Width/Height of 0 means 256
        let w = if width == 0 { 256 } else { width };
        let h = if height == 0 { 256 } else { height };
        let area = w * h;

        if offset + size <= data.len() {
            match best_entry {
                None => best_entry = Some((offset, size, area)),
                Some((_, _, best_area)) if area > best_area => {
                    best_entry = Some((offset, size, area));
                }
                _ => {}
            }
        }
    }

    let (offset, size, _) = best_entry?;
    let icon_data = &data[offset..offset + size];

    // Check if it's a PNG (ICO can contain PNG data)
    // PNG signature: 89 50 4E 47 0D 0A 1A 0A
    if icon_data.len() >= 8 && icon_data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        // It's a PNG, we need to decode it
        decode_png(icon_data)
    } else {
        // It's a DIB (BMP without file header), decode it
        decode_dib(icon_data)
    }
}

/// Decode a PNG image to RGBA pixels
#[cfg(feature = "desktop")]
fn decode_png(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    use std::io::Cursor;

    let decoder = png::Decoder::new(Cursor::new(data));
    let mut reader = decoder.read_info().ok()?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;

    // Estimate buffer size based on output buffer size
    let buffer_size = reader.output_buffer_size();
    let mut buf = vec![0; buffer_size];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];

    // Convert to RGBA if necessary
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(bytes.len() / 3 * 4);
            for chunk in bytes.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255); // Alpha
            }
            rgba
        }
        png::ColorType::Grayscale => {
            let mut rgba = Vec::with_capacity(bytes.len() * 4);
            for &g in bytes {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
            rgba
        }
        png::ColorType::GrayscaleAlpha => {
            let mut rgba = Vec::with_capacity(bytes.len() * 2);
            for chunk in bytes.chunks(2) {
                rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            rgba
        }
        _ => return None,
    };

    Some((rgba, width, height))
}

/// Decode a DIB (device-independent bitmap) to RGBA pixels
/// DIB format: BITMAPINFOHEADER followed by pixel data
#[cfg(feature = "desktop")]
fn decode_dib(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    if data.len() < 40 {
        return None;
    }

    // BITMAPINFOHEADER
    let width = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let height = i32::from_le_bytes([data[8], data[9], data[10], data[11]]) / 2; // ICO height is doubled
    let bpp = u16::from_le_bytes([data[14], data[15]]) as usize;

    if width <= 0 || height <= 0 {
        return None;
    }

    let width = width as u32;
    let height = height as u32;

    // Calculate the size of the color table (if any)
    let compression = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    if compression != 0 {
        return None; // Only support uncompressed
    }

    let colors_used = u32::from_le_bytes([data[32], data[33], data[34], data[35]]) as usize;
    let color_table_size = if bpp <= 8 {
        let num_colors = if colors_used > 0 { colors_used } else { 1 << bpp };
        num_colors * 4
    } else {
        0
    };

    let pixel_offset = 40 + color_table_size;
    if data.len() < pixel_offset {
        return None;
    }

    let pixel_data = &data[pixel_offset..];

    match bpp {
        32 => {
            // BGRA format, need to convert to RGBA and flip vertically
            let row_size = (width * 4) as usize;
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);

            for y in (0..height).rev() {
                let row_start = (y as usize) * row_size;
                let row_end = row_start + row_size;
                if row_end > pixel_data.len() {
                    return None;
                }
                for chunk in pixel_data[row_start..row_end].chunks(4) {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(chunk[3]); // A
                }
            }
            Some((rgba, width, height))
        }
        24 => {
            // BGR format
            let row_stride = ((width * 3).div_ceil(4) * 4) as usize; // Rows are padded to 4-byte boundary
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);

            for y in (0..height).rev() {
                let row_start = (y as usize) * row_stride;
                for x in 0..width {
                    let pixel_start = row_start + (x as usize) * 3;
                    if pixel_start + 3 > pixel_data.len() {
                        return None;
                    }
                    rgba.push(pixel_data[pixel_start + 2]); // R
                    rgba.push(pixel_data[pixel_start + 1]); // G
                    rgba.push(pixel_data[pixel_start]); // B
                    rgba.push(255); // A
                }
            }
            Some((rgba, width, height))
        }
        _ => None, // Unsupported BPP
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Root {},
    #[layout(GuestLayout)]
    #[route("/home")]
    HomePage {},
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},
    #[end_layout]
    #[layout(Layout)]
    #[route("/console/dashboard")]
    Dashboard {},
    #[route("/console/deploy")]
    DeployConfig {},
    #[route("/console/monitor")]
    ServiceMonitor {},
    #[route("/console/access")]
    AccessCredentialsPage {},
    #[route("/console/models")]
    ChannelPage {},
    #[route("/console/users")]
    UserPage {},
    #[route("/console/settings")]
    SystemSettings {},
    #[route("/console/finance")]
    BillingPage {},
    #[route("/console/logs")]
    LogPage {},
    #[route("/console/connect")]
    ConnectPage {},
    #[route("/console/playground")]
    PlaygroundPage {},
    #[route("/console/:..segments")]
    NotFoundPage { segments: Vec<String> },
}

#[component]
pub fn App() -> Element {
    // Initialize i18n context
    burncloud_client_shared::i18n::use_init_i18n();
    // Initialize Toast
    burncloud_client_shared::use_init_toast();
    // Initialize Auth Context
    burncloud_client_shared::use_init_auth();

    rsx! {
        burncloud_client_shared::ToastContainer {}
        Router::<Route> {}
    }
}

#[cfg(feature = "desktop")]
pub fn launch_gui() {
    launch_gui_with_tray();
}

#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, WindowBuilder};

    // Load the window icon from embedded bytes
    let icon = load_window_icon();

    let mut window = WindowBuilder::new()
        .with_title("BurnCloud - AI Local Deployment Platform") // Changed to English/Bilingual
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true)
        .with_decorations(false);

    // Set window icon if available
    if let Some(icon) = icon {
        window = window.with_window_icon(Some(icon));
    }

    // Use a specific data directory in temp to avoid permission issues or path conflicts
    let data_dir = std::env::temp_dir().join("burncloud_webview_data");
    let config = Config::new()
        .with_window(window)
        .with_data_directory(data_dir);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(AppWithTray);
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
#[component]
fn AppWithTray() -> Element {
    use_context_provider(|| DesktopMode);
    let window = dioxus::desktop::use_window();

    let window_setup = window.clone();
    use_effect(move || {
        window_setup.set_maximized(true);

        // 启动托盘应用在后台线程
        std::thread::spawn(move || {
            if let Err(e) = start_tray() {
                eprintln!("Failed to start tray: {}", e);
            }
        });
    });

    // 轮询检查托盘操作
    use_effect(move || {
        let window_clone = window.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if should_show_window() {
                    // 强制显示窗口
                    window_clone.set_visible(false);
                    window_clone.set_visible(true);
                    window_clone.set_focus();
                }
            }
        });
    });

    rsx! { App {} }
}

#[cfg(all(feature = "desktop", not(target_os = "windows")))]
#[component]
fn AppWithTray() -> Element {
    use_context_provider(|| DesktopMode);
    let window = dioxus::desktop::use_window();

    use_effect(move || {
        window.set_maximized(true);
    });

    rsx! { App {} }
}
