#[cfg(target_os = "windows")]
mod desktop {
    use std::fmt;
    use std::process;
    use std::sync::atomic::{AtomicBool, Ordering};
    use systray::Application;

    // 嵌入图标数据
    static ICON_DATA: &[u8] = include_bytes!("../res/burncloud.ico");

    #[derive(Debug)]
    pub struct SimpleError(String);

    impl fmt::Display for SimpleError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for SimpleError {}

    // 简单的原子标志
    static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);
    static SHOULD_HIDE: AtomicBool = AtomicBool::new(false);

    pub fn should_show_window() -> bool {
        SHOULD_SHOW.swap(false, Ordering::Relaxed)
    }

    pub fn should_hide_window() -> bool {
        SHOULD_HIDE.swap(false, Ordering::Relaxed)
    }

    pub fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = Application::new()?;

        // 创建临时文件来设置图标
        let temp_dir = std::env::temp_dir();
        let temp_icon_path = temp_dir.join("burncloud_temp.ico");

        // 将嵌入的图标数据写入临时文件
        match std::fs::write(&temp_icon_path, ICON_DATA) {
            Ok(_) => {
                // 尝试使用临时文件设置图标
                match app.set_icon_from_file(&temp_icon_path.to_string_lossy()) {
                    Ok(_) => {
                        println!("Icon set successfully");
                        // 设置成功后清理临时文件
                        let _ = std::fs::remove_file(&temp_icon_path);
                    }
                    Err(e) => {
                        println!("Warning: Failed to set icon: {:?}", e);
                        // 清理临时文件
                        let _ = std::fs::remove_file(&temp_icon_path);
                    }
                }
            }
            Err(e) => {
                println!("Warning: Failed to create temporary icon file: {:?}", e);
            }
        }

        // 添加启动界面菜单项
        app.add_menu_item("显示界面", move |_| -> Result<(), SimpleError> {
            SHOULD_SHOW.store(true, Ordering::Relaxed);
            Ok(())
        })?;

        // 添加分隔符
        app.add_menu_separator()?;

        // 添加退出菜单项
        app.add_menu_item("退出程序", |_| -> Result<(), SimpleError> {
            process::exit(0);
        })?;

        // 等待
        app.wait_for_message()?;

        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use desktop::*;

#[cfg(not(target_os = "windows"))]
pub fn should_show_window() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn should_hide_window() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
