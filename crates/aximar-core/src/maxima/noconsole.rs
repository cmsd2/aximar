/// On Windows, configure a command to not create a visible console window.
/// This is a no-op on other platforms.
#[cfg(target_os = "windows")]
pub fn hide_console_window(cmd: &mut tokio::process::Command) {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(target_os = "windows")]
pub fn hide_console_window_sync(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
pub fn hide_console_window(_cmd: &mut tokio::process::Command) {}

#[cfg(not(target_os = "windows"))]
pub fn hide_console_window_sync(_cmd: &mut std::process::Command) {}
