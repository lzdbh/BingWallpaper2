use std::path::Path;
use std::process::Command;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{PlatformCapabilities, WallpaperStyle};
use crate::domain::traits::WallpaperBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Desktop {
    Gnome,
    Kde,
    Xfce,
    Sway,
    Hyprland,
    Feh,
}

pub struct LinuxBackend {
    desktop: Desktop,
    raw_desktop: Option<String>,
}

impl LinuxBackend {
    pub fn detect() -> Self {
        let raw = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .ok();
        let value = raw
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();

        let desktop = if value.contains("gnome")
            || value.contains("unity")
            || value.contains("pantheon")
            || value.contains("budgie")
            || value.contains("cinnamon")
            || value.contains("pop")
        {
            Desktop::Gnome
        } else if value.contains("kde") || value.contains("plasma") {
            Desktop::Kde
        } else if value.contains("xfce") {
            Desktop::Xfce
        } else if value.contains("sway") {
            Desktop::Sway
        } else if value.contains("hyprland") {
            Desktop::Hyprland
        } else {
            Desktop::Feh
        };

        Self {
            desktop,
            raw_desktop: raw,
        }
    }

    fn styles(&self) -> Vec<WallpaperStyle> {
        match self.desktop {
            Desktop::Gnome | Desktop::Xfce | Desktop::Feh => WallpaperStyle::ALL.to_vec(),
            Desktop::Sway => vec![
                WallpaperStyle::Fill,
                WallpaperStyle::Fit,
                WallpaperStyle::Stretch,
                WallpaperStyle::Tile,
                WallpaperStyle::Center,
            ],
            Desktop::Kde | Desktop::Hyprland => vec![WallpaperStyle::Fill],
        }
    }
}

impl WallpaperBackend for LinuxBackend {
    fn set_wallpaper(&self, path: &Path, style: WallpaperStyle) -> AppResult<()> {
        if !path.is_file() {
            return Err(AppError::Io(format!("壁纸文件不存在：{}", path.display())));
        }
        if !self.styles().contains(&style) {
            return Err(AppError::UnsupportedStyle);
        }

        let path_str = path.to_string_lossy().to_string();
        let uri = url::Url::from_file_path(path)
            .map(|url| url.to_string())
            .unwrap_or_else(|_| format!("file://{path_str}"));

        match self.desktop {
            Desktop::Gnome => set_gnome(&uri, style),
            Desktop::Kde => run("plasma-apply-wallpaperimage", &[&path_str]),
            Desktop::Xfce => set_xfce(&path_str, style),
            Desktop::Sway => {
                let mode = sway_mode(style);
                run("swaymsg", &["output", "*", "bg", &path_str, mode])
            }
            Desktop::Hyprland => {
                let _ = run("hyprctl", &["hyprpaper", "unload", "all"]);
                run("hyprctl", &["hyprpaper", "preload", &path_str])?;
                run(
                    "hyprctl",
                    &["hyprpaper", "wallpaper", &format!(",{path_str},{}", hypr_mode(style))],
                )
            }
            Desktop::Feh => run("feh", &[feh_flag(style), &path_str]),
        }
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            os: "linux".to_string(),
            arch: std::env::consts::ARCH.to_string(),
            styles: self.styles(),
            multi_monitor: !matches!(self.desktop, Desktop::Hyprland),
            desktop_environment: self.raw_desktop.clone(),
        }
    }
}

fn set_gnome(uri: &str, style: WallpaperStyle) -> AppResult<()> {
    let option = match style {
        WallpaperStyle::Fill => "zoom",
        WallpaperStyle::Fit => "scaled",
        WallpaperStyle::Stretch => "stretched",
        WallpaperStyle::Tile => "wallpaper",
        WallpaperStyle::Center => "centered",
        WallpaperStyle::Span => "spanned",
    };
    run(
        "gsettings",
        &["set", "org.gnome.desktop.background", "picture-uri", uri],
    )?;
    // Dark mode background. Not all versions expose the key; ignore failures.
    let _ = run(
        "gsettings",
        &["set", "org.gnome.desktop.background", "picture-uri-dark", uri],
    );
    run(
        "gsettings",
        &["set", "org.gnome.desktop.background", "picture-options", option],
    )
}

fn set_xfce(path: &str, style: WallpaperStyle) -> AppResult<()> {
    let style_value = match style {
        WallpaperStyle::Center => "1",
        WallpaperStyle::Tile => "2",
        WallpaperStyle::Stretch => "3",
        WallpaperStyle::Fit => "4",
        WallpaperStyle::Fill | WallpaperStyle::Span => "5",
    };

    let listing = Command::new("xfconf-query")
        .args(["-c", "xfce4-desktop", "-l"])
        .output()
        .map_err(|error| AppError::Io(format!("无法执行 xfconf-query：{error}")))?;

    if !listing.status.success() {
        return Err(AppError::Io(
            String::from_utf8_lossy(&listing.stderr).trim().to_string(),
        ));
    }

    let properties = String::from_utf8_lossy(&listing.stdout);
    let mut applied = 0;
    for property in properties
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with("/last-image"))
    {
        run(
            "xfconf-query",
            &["-c", "xfce4-desktop", "-p", property, "-s", path],
        )?;
        if let Some(prefix) = property.strip_suffix("/last-image") {
            let style_property = format!("{prefix}/image-style");
            let _ = run(
                "xfconf-query",
                &["-c", "xfce4-desktop", "-p", &style_property, "-s", style_value],
            );
        }
        applied += 1;
    }

    if applied == 0 {
        return Err(AppError::Io(
            "未找到 XFCE 桌面背景属性，请先在桌面设置中配置一次壁纸".to_string(),
        ));
    }
    Ok(())
}

fn sway_mode(style: WallpaperStyle) -> &'static str {
    match style {
        WallpaperStyle::Fill | WallpaperStyle::Span => "fill",
        WallpaperStyle::Fit => "fit",
        WallpaperStyle::Stretch => "stretch",
        WallpaperStyle::Tile => "tile",
        WallpaperStyle::Center => "center",
    }
}

fn hypr_mode(style: WallpaperStyle) -> &'static str {
    match style {
        WallpaperStyle::Fit => "contain",
        WallpaperStyle::Tile => "tile",
        _ => "cover",
    }
}

fn feh_flag(style: WallpaperStyle) -> &'static str {
    match style {
        WallpaperStyle::Fill | WallpaperStyle::Span => "--bg-fill",
        WallpaperStyle::Fit => "--bg-max",
        WallpaperStyle::Stretch => "--bg-scale",
        WallpaperStyle::Tile => "--bg-tile",
        WallpaperStyle::Center => "--bg-center",
    }
}

fn run(program: &str, args: &[&str]) -> AppResult<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| AppError::Io(format!("无法执行 {program}：{error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Io(format!(
            "{program} 执行失败：{}",
            stderr.trim()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sway_modes() {
        assert_eq!(sway_mode(WallpaperStyle::Fill), "fill");
        assert_eq!(sway_mode(WallpaperStyle::Span), "fill");
        assert_eq!(sway_mode(WallpaperStyle::Fit), "fit");
        assert_eq!(sway_mode(WallpaperStyle::Stretch), "stretch");
        assert_eq!(sway_mode(WallpaperStyle::Tile), "tile");
        assert_eq!(sway_mode(WallpaperStyle::Center), "center");
    }

    #[test]
    fn maps_hyprland_modes() {
        assert_eq!(hypr_mode(WallpaperStyle::Fit), "contain");
        assert_eq!(hypr_mode(WallpaperStyle::Tile), "tile");
        assert_eq!(hypr_mode(WallpaperStyle::Fill), "cover");
        assert_eq!(hypr_mode(WallpaperStyle::Stretch), "cover");
        assert_eq!(hypr_mode(WallpaperStyle::Center), "cover");
        assert_eq!(hypr_mode(WallpaperStyle::Span), "cover");
    }

    #[test]
    fn maps_feh_flags() {
        assert_eq!(feh_flag(WallpaperStyle::Fill), "--bg-fill");
        assert_eq!(feh_flag(WallpaperStyle::Span), "--bg-fill");
        assert_eq!(feh_flag(WallpaperStyle::Fit), "--bg-max");
        assert_eq!(feh_flag(WallpaperStyle::Stretch), "--bg-scale");
        assert_eq!(feh_flag(WallpaperStyle::Tile), "--bg-tile");
        assert_eq!(feh_flag(WallpaperStyle::Center), "--bg-center");
    }
}
