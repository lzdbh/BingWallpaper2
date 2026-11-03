use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{PlatformCapabilities, WallpaperStyle};
use crate::domain::traits::WallpaperBackend;

const SPI_SETDESKWALLPAPER: u32 = 0x0014;
const SPIF_UPDATEINIFILE: u32 = 0x0001;
const SPIF_SENDCHANGE: u32 = 0x0002;

#[link(name = "user32")]
extern "system" {
    fn SystemParametersInfoW(action: u32, param: u32, data: *mut c_void, win_ini: u32) -> i32;
}

// ---------------------------------------------------------------------------
// IDesktopWallpaper (Windows 8+) — modern COM API that applies the wallpaper to
// every monitor with an explicit position. Hand-rolled FFI keeps the project
// free of the heavy `windows` crate; the vtable order below MUST match
// shobjidl.h. Only a subset of methods is ever called.
// ---------------------------------------------------------------------------

/// 128-bit COM GUID layout expected by `CoCreateInstance`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

/// `{C2CF3110-460E-4fc1-B9D0-8A1C0C9CC4BD}`
const CLSID_DESKTOP_WALLPAPER: Guid = Guid {
    data1: 0xC2CF_3110,
    data2: 0x460E,
    data3: 0x4FC1,
    data4: [0xB9, 0xD0, 0x8A, 0x1C, 0x0C, 0x9C, 0xC4, 0xBD],
};

/// `{B92B56A9-8B55-4E14-9A89-0199BBB6F93B}`
const IID_IDESKTOP_WALLPAPER: Guid = Guid {
    data1: 0xB92B_56A9,
    data2: 0x8B55,
    data3: 0x4E14,
    data4: [0x9A, 0x89, 0x01, 0x99, 0xBB, 0xB6, 0xF9, 0x3B],
};

const COINIT_APARTMENTTHREADED: u32 = 0x2;
const CLSCTX_LOCAL_SERVER: u32 = 0x4;
const S_OK: i32 = 0;
const S_FALSE: i32 = 1;

// DESKTOP_WALLPAPER_POSITION
const DWPOS_CENTER: u32 = 0;
const DWPOS_TILE: u32 = 1;
const DWPOS_STRETCH: u32 = 2;
const DWPOS_FIT: u32 = 3;
const DWPOS_FILL: u32 = 4;
const DWPOS_SPAN: u32 = 5;

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pv_reserved: *const c_void, dw_co_init: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const Guid,
        p_unk_outer: *mut c_void,
        dw_cls_context: u32,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> i32;
}

/// `RECT` for the (uncalled) `GetMonitorRECT` slot; kept for vtable fidelity.
#[repr(C)]
#[allow(dead_code)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// Full `IDesktopWallpaper` vtable. Slot order matches shobjidl.h exactly.
#[repr(C)]
#[allow(dead_code)]
struct DesktopWallpaperVtbl {
    // IUnknown
    query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IDesktopWallpaper
    set_wallpaper: unsafe extern "system" fn(*mut c_void, *const u16, *const u16) -> i32,
    get_wallpaper: unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut u16) -> i32,
    get_monitor_device_path_at: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u16) -> i32,
    get_monitor_device_path_count: unsafe extern "system" fn(*mut c_void) -> u32,
    get_monitor_rect: unsafe extern "system" fn(*mut c_void, *const u16, *mut Rect) -> i32,
    get_background_color: unsafe extern "system" fn(*mut c_void, *mut u32),
    set_background_color: unsafe extern "system" fn(*mut c_void, u32),
    set_position: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    get_position: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    set_slideshow: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    get_slideshow: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    advance_slideshow: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> i32,
    get_slideshow_options: unsafe extern "system" fn(*mut c_void, *mut u32, *mut u32) -> i32,
    set_slideshow_options: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
}

#[repr(C)]
struct DesktopWallpaper {
    vtbl: *const DesktopWallpaperVtbl,
}

/// RAII guard that balances `CoInitializeEx` with `CoUninitialize`.
struct ComGuard {
    uninit: bool,
}

impl ComGuard {
    fn init() -> Self {
        let hr = unsafe { CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED) };
        // S_OK / S_FALSE both require a matching CoUninitialize. A failure such
        // as RPC_E_CHANGED_MODE means COM is already up in another mode and must
        // not be torn down here.
        ComGuard {
            uninit: hr == S_OK || hr == S_FALSE,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.uninit {
            unsafe { CoUninitialize() };
        }
    }
}

/// Owned `IDesktopWallpaper` pointer; released on drop. Only ever lives for the
/// duration of a single synchronous call on one thread.
struct Wallpaper {
    this: *mut DesktopWallpaper,
}

impl Wallpaper {
    fn create() -> AppResult<Self> {
        let mut raw: *mut c_void = ptr::null_mut();
        let hr = unsafe {
            CoCreateInstance(
                &CLSID_DESKTOP_WALLPAPER,
                ptr::null_mut(),
                CLSCTX_LOCAL_SERVER,
                &IID_IDESKTOP_WALLPAPER,
                &mut raw,
            )
        };
        if hr != S_OK || raw.is_null() {
            return Err(AppError::Io(format!(
                "创建 IDesktopWallpaper 失败（HRESULT 0x{:08X}）",
                hr as u32
            )));
        }
        Ok(Self {
            this: raw as *mut DesktopWallpaper,
        })
    }

    fn vtbl(&self) -> &DesktopWallpaperVtbl {
        unsafe { &*(*self.this).vtbl }
    }

    fn set_position(&self, position: u32) -> AppResult<()> {
        let hr = unsafe { (self.vtbl().set_position)(self.this as *mut c_void, position) };
        if hr != S_OK {
            return Err(AppError::Io(format!(
                "IDesktopWallpaper::SetPosition 失败（0x{:08X}）",
                hr as u32
            )));
        }
        Ok(())
    }

    /// A null `monitorID` applies the wallpaper to every attached monitor.
    fn set_wallpaper_all(&self, path_wide: &[u16]) -> AppResult<()> {
        let hr =
            unsafe { (self.vtbl().set_wallpaper)(self.this as *mut c_void, ptr::null(), path_wide.as_ptr()) };
        if hr != S_OK {
            return Err(AppError::Io(format!(
                "IDesktopWallpaper::SetWallpaper 失败（0x{:08X}）",
                hr as u32
            )));
        }
        Ok(())
    }

    fn monitor_count(&self) -> u32 {
        unsafe { (self.vtbl().get_monitor_device_path_count)(self.this as *mut c_void) }
    }
}

impl Drop for Wallpaper {
    fn drop(&mut self) {
        unsafe {
            (self.vtbl().release)(self.this as *mut c_void);
        }
    }
}

pub struct WindowsBackend;

impl WallpaperBackend for WindowsBackend {
    fn set_wallpaper(&self, path: &Path, style: WallpaperStyle) -> AppResult<()> {
        if !path.is_file() {
            return Err(AppError::Io(format!("壁纸文件不存在：{}", path.display())));
        }

        // Write the registry style first so the SPI fallback and the shell agree
        // with the COM position we are about to set.
        apply_style(style)?;

        // Prefer IDesktopWallpaper (correct multi-monitor + position); fall back
        // to SystemParametersInfoW when COM is unavailable.
        match set_via_com(path, style) {
            Ok(()) => Ok(()),
            Err(error) => {
                log::warn!("IDesktopWallpaper 不可用，回退 SystemParametersInfoW：{error}");
                apply_desktop(path)
            }
        }
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            os: "windows".to_string(),
            arch: std::env::consts::ARCH.to_string(),
            styles: WallpaperStyle::ALL.to_vec(),
            multi_monitor: com_multi_monitor_supported(),
            desktop_environment: None,
        }
    }
}

/// Map a style to the `WallpaperStyle` / `TileWallpaper` registry values.
/// Documented by Microsoft: 10 = fill, 6 = fit, 2 = stretch, 0 = center,
/// 22 = span; tile is `WallpaperStyle = 0` + `TileWallpaper = 1`.
fn style_registry_values(style: WallpaperStyle) -> (&'static str, &'static str) {
    match style {
        WallpaperStyle::Fill => ("10", "0"),
        WallpaperStyle::Fit => ("6", "0"),
        WallpaperStyle::Stretch => ("2", "0"),
        WallpaperStyle::Center => ("0", "0"),
        WallpaperStyle::Tile => ("0", "1"),
        WallpaperStyle::Span => ("22", "0"),
    }
}

/// Map a style to a `DESKTOP_WALLPAPER_POSITION` value.
fn position_for(style: WallpaperStyle) -> u32 {
    match style {
        WallpaperStyle::Center => DWPOS_CENTER,
        WallpaperStyle::Tile => DWPOS_TILE,
        WallpaperStyle::Stretch => DWPOS_STRETCH,
        WallpaperStyle::Fit => DWPOS_FIT,
        WallpaperStyle::Fill => DWPOS_FILL,
        WallpaperStyle::Span => DWPOS_SPAN,
    }
}

fn apply_style(style: WallpaperStyle) -> AppResult<()> {
    let (wallpaper_style, tile) = style_registry_values(style);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let desktop = hkcu
        .open_subkey_with_flags("Control Panel\\Desktop", KEY_SET_VALUE)
        .map_err(|error| AppError::Io(format!("无法打开桌面注册表项：{error}")))?;
    desktop
        .set_value("WallpaperStyle", &wallpaper_style)
        .map_err(|error| AppError::Io(format!("无法写入 WallpaperStyle：{error}")))?;
    desktop
        .set_value("TileWallpaper", &tile)
        .map_err(|error| AppError::Io(format!("无法写入 TileWallpaper：{error}")))?;
    Ok(())
}

fn to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

/// Apply the wallpaper through `IDesktopWallpaper`: set the position, then push
/// the image to all monitors.
fn set_via_com(path: &Path, style: WallpaperStyle) -> AppResult<()> {
    let _com = ComGuard::init();
    let wallpaper = Wallpaper::create()?;
    wallpaper.set_position(position_for(style))?;
    wallpaper.set_wallpaper_all(&to_wide(path))?;
    log::debug!(
        "IDesktopWallpaper 应用成功（检测到 {} 个显示器）",
        wallpaper.monitor_count()
    );
    Ok(())
}

/// Whether the modern COM API is usable (Windows 8+). Drives `multi_monitor`.
fn com_multi_monitor_supported() -> bool {
    let _com = ComGuard::init();
    match Wallpaper::create() {
        Ok(wallpaper) => wallpaper.monitor_count() >= 1,
        Err(_) => false,
    }
}

fn apply_desktop(path: &Path) -> AppResult<()> {
    let wide = to_wide(path);
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut c_void,
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };

    if ok == 0 {
        return Err(AppError::Io(format!(
            "SystemParametersInfoW 调用失败（错误码 {}）",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_style_to_registry_values() {
        assert_eq!(style_registry_values(WallpaperStyle::Fill), ("10", "0"));
        assert_eq!(style_registry_values(WallpaperStyle::Fit), ("6", "0"));
        assert_eq!(style_registry_values(WallpaperStyle::Stretch), ("2", "0"));
        assert_eq!(style_registry_values(WallpaperStyle::Center), ("0", "0"));
        assert_eq!(style_registry_values(WallpaperStyle::Tile), ("0", "1"));
        assert_eq!(style_registry_values(WallpaperStyle::Span), ("22", "0"));
    }

    #[test]
    fn maps_style_to_com_position() {
        assert_eq!(position_for(WallpaperStyle::Center), DWPOS_CENTER);
        assert_eq!(position_for(WallpaperStyle::Tile), DWPOS_TILE);
        assert_eq!(position_for(WallpaperStyle::Stretch), DWPOS_STRETCH);
        assert_eq!(position_for(WallpaperStyle::Fit), DWPOS_FIT);
        assert_eq!(position_for(WallpaperStyle::Fill), DWPOS_FILL);
        assert_eq!(position_for(WallpaperStyle::Span), DWPOS_SPAN);
    }
}
