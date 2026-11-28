//! The colours and font the desktop draws a window's frame with, so the
//! page's tab bar and toolbar can match it.
//!
//! Only Linux has any.  The page takes the GTK menu bar's colours for its
//! tab bar, menu bar and toolbar (src/hooks/useSystemChrome.ts).  Where the
//! page draws the title bar it also takes the title bar's colours, the
//! desktop's button layout and which desktop it is, for the buttons' look.
//! macOS and Windows keep the app's own colours.

use serde::Serialize;

/// Sent to a window when the desktop's theme changes
#[cfg(target_os = "linux")]
pub const CHANGED_EVENT: &str = "system-chrome-changed";

/// How the desktop draws a window's menu bar, as CSS
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemChrome {
    /// The menu bar's background while the window is active
    pub background: String,
    /// Its background while another window is active
    pub backdrop_background: String,
    /// The theme's border colour, if it names one
    pub border: Option<String>,
    /// Whether the background is dark, so a dark page can use it and a light one can't
    pub dark: bool,
    /// The desktop's interface font family, such as "Noto Sans"
    pub font_family: Option<String>,
    /// Where the page draws the title bar: its background while the window
    /// is active, as the window manager draws other windows' title bars
    pub title_background: String,
    /// The title bar's background while another window is active
    pub title_backdrop_background: String,
    /// Which buttons the title bar has, and on which side, as GTK's
    /// `gtk-decoration-layout`: "icon:minimize,maximize,close"
    pub decoration_layout: Option<String>,
    /// The desktop, for the window buttons' look: "kde", "gnome" or "other"
    pub desktop: &'static str,
}

/// The desktop from `XDG_CURRENT_DESKTOP`, such as "KDE" or "ubuntu:GNOME"
#[cfg(any(target_os = "linux", test))]
pub fn desktop_kind(current_desktop: &str) -> &'static str {
    let names: Vec<String> = current_desktop
        .split(':')
        .map(|n| n.trim().to_ascii_lowercase())
        .collect();
    if names.iter().any(|n| n == "kde") {
        "kde"
    } else if names.iter().any(|n| n == "gnome") {
        "gnome"
    } else {
        "other"
    }
}

/// KWin's title bar colours from kdeglobals: the `[WM]` section's
/// `activeBackground` and `inactiveBackground`, as "r,g,b"
#[cfg(any(target_os = "linux", test))]
pub fn kde_title_colours(kdeglobals: &str) -> Option<(Rgba, Rgba)> {
    let mut in_wm = false;
    let (mut active, mut inactive) = (None, None);
    for line in kdeglobals.lines().map(str::trim) {
        if line.starts_with('[') {
            in_wm = line == "[WM]";
            continue;
        }
        if !in_wm {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "activeBackground" => active = Rgba::from_kde(value),
                "inactiveBackground" => inactive = Rgba::from_kde(value),
                _ => {}
            }
        }
    }
    let active = active?;
    Some((active, inactive.unwrap_or(active)))
}

/// A colour with channels from 0 to 1, as GTK gives it
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[cfg(any(target_os = "linux", test))]
impl Rgba {
    /// A colour as KDE writes it: "227,229,231", with an optional alpha
    pub fn from_kde(value: &str) -> Option<Self> {
        let parts: Vec<f64> = value
            .split(',')
            .map(|p| p.trim().parse::<f64>().ok().map(|n| n / 255.0))
            .collect::<Option<_>>()?;
        match parts[..] {
            [red, green, blue] => Some(Rgba {
                red,
                green,
                blue,
                alpha: 1.0,
            }),
            [red, green, blue, alpha] => Some(Rgba {
                red,
                green,
                blue,
                alpha,
            }),
            _ => None,
        }
    }

    /// As CSS: `#rrggbb` when opaque, else `rgba(r, g, b, a)`
    pub fn css(self) -> String {
        let byte = |c: f64| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        let (r, g, b) = (byte(self.red), byte(self.green), byte(self.blue));
        if self.alpha >= 1.0 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("rgba({r}, {g}, {b}, {:.3})", self.alpha.clamp(0.0, 1.0))
        }
    }

    /// WCAG's relative luminance
    pub fn luminance(self) -> f64 {
        let linear = |c: f64| {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.red) + 0.7152 * linear(self.green) + 0.0722 * linear(self.blue)
    }

    /// Dark enough that black text on it would read worse than white
    pub fn is_dark(self) -> bool {
        // Black and white have equal contrast on a luminance of about 0.179
        self.luminance() < 0.179
    }

    /// Whether any of it shows: a theme that paints the menu bar with an
    /// image leaves its background colour transparent
    pub fn is_visible(self) -> bool {
        self.alpha > 0.0
    }
}

/// The family from a Pango font description, such as "Noto Sans,  10"
#[cfg(any(target_os = "linux", test))]
pub fn font_family(description: &str) -> Option<String> {
    let family = description.split(',').next()?.trim();
    // A description with no comma ends with its size: "Cantarell 11"
    let family = if description.contains(',') {
        family
    } else {
        match family.rsplit_once(' ') {
            Some((name, size)) if size.parse::<f64>().is_ok() => name.trim(),
            _ => family,
        }
    };
    (!family.is_empty()).then(|| family.to_string())
}

/// The desktop's chrome for `window`, if it has one to match
#[tauri::command]
pub fn get_system_chrome(window: tauri::WebviewWindow) -> Option<SystemChrome> {
    #[cfg(target_os = "linux")]
    {
        gtk_theme::read(&window)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        None
    }
}

/// Tell `window` whenever the desktop's theme changes how its menu bar looks
pub fn watch<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    #[cfg(target_os = "linux")]
    gtk_theme::watch(window);
    #[cfg(not(target_os = "linux"))]
    let _ = window;
}

#[cfg(target_os = "linux")]
mod gtk_theme {
    use super::{desktop_kind, font_family, kde_title_colours, Rgba, SystemChrome, CHANGED_EVENT};
    use gtk::prelude::*;
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use tauri::{Emitter, Runtime, WebviewWindow};

    /// KDE's settings file, where KWin's colours are
    fn kdeglobals() -> Option<String> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        std::fs::read_to_string(config.join("kdeglobals")).ok()
    }

    fn rgba(c: gtk::gdk::RGBA) -> Rgba {
        Rgba {
            red: c.red(),
            green: c.green(),
            blue: c.blue(),
            alpha: c.alpha(),
        }
    }

    /// Read the colours from a menu bar the window never shows, packed where
    /// the real one goes, so the theme's rules for a window's menu bar apply
    /// to it.  A window has no menu bar until the page sends its menu.
    /// GTK runs on the main thread, as sync commands do.
    pub fn read<R: Runtime>(window: &WebviewWindow<R>) -> Option<SystemChrome> {
        let vbox = window.default_vbox().ok()?;
        let probe = gtk::MenuBar::new();
        vbox.pack_start(&probe, false, false, 0);
        let style = probe.style_context();
        let background_in = |state: gtk::StateFlags| {
            style
                .style_property_for_state("background-color", state)
                .get::<gtk::gdk::RGBA>()
                .ok()
                .map(rgba)
                .filter(|c| c.is_visible())
        };
        let window_background = style.lookup_color("theme_bg_color").map(rgba);
        let background = background_in(gtk::StateFlags::NORMAL).or(window_background);
        let backdrop = background_in(gtk::StateFlags::BACKDROP)
            .or_else(|| style.lookup_color("theme_unfocused_bg_color").map(rgba))
            .or(background);
        let border = style.lookup_color("borders").map(rgba);
        vbox.remove(&probe);

        let background = background?;
        let backdrop = backdrop.unwrap_or(background);
        let settings = gtk::Settings::default();
        let font = settings
            .as_ref()
            .and_then(|s| s.gtk_font_name())
            .and_then(|name| font_family(&name));
        let layout = settings
            .as_ref()
            .and_then(|s| s.gtk_decoration_layout())
            .map(|l| l.to_string());
        let desktop = desktop_kind(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default());
        // KWin draws other windows' title bars in KDE's own colours, which
        // GTK's theme only approximates; elsewhere GTK's menu bar colours
        // are the nearest to the title bar's
        let (title, title_backdrop) = match desktop {
            "kde" => kdeglobals()
                .and_then(|text| kde_title_colours(&text))
                .filter(|(active, _)| active.is_dark() == background.is_dark())
                .unwrap_or((background, backdrop)),
            _ => (background, backdrop),
        };
        Some(SystemChrome {
            background: background.css(),
            backdrop_background: backdrop.css(),
            border: border.map(Rgba::css),
            dark: background.is_dark(),
            font_family: font,
            title_background: title.css(),
            title_backdrop_background: title_backdrop.css(),
            decoration_layout: layout,
            desktop,
        })
    }

    /// GTK restyles the window when the theme, its colours or the font
    /// change, and also when the window gains or loses focus; only a change
    /// in what `read` finds is sent.
    pub fn watch<R: Runtime>(window: &WebviewWindow<R>) {
        let Ok(gtk_window) = window.gtk_window() else {
            return;
        };
        let window = window.clone();
        let last = RefCell::new(read(&window));
        let reading = Cell::new(false);
        gtk_window.connect_style_updated(move |_| {
            // Reading packs a menu bar into the window; never re-enter
            if reading.replace(true) {
                return;
            }
            let now = read(&window);
            reading.set(false);
            if now != *last.borrow() {
                if let Some(chrome) = &now {
                    let _ = window.emit_to(window.label(), CHANGED_EVENT, chrome);
                }
                *last.borrow_mut() = now;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba {
            red: f64::from(r) / 255.0,
            green: f64::from(g) / 255.0,
            blue: f64::from(b) / 255.0,
            alpha: 1.0,
        }
    }

    #[test]
    fn opaque_colours_are_hex() {
        assert_eq!(rgb(222, 224, 226).css(), "#dee0e2");
        assert_eq!(rgb(0, 0, 0).css(), "#000000");
    }

    #[test]
    fn translucent_colours_keep_their_alpha() {
        let c = Rgba {
            alpha: 0.5,
            ..rgb(255, 0, 16)
        };
        assert_eq!(c.css(), "rgba(255, 0, 16, 0.500)");
        assert!(c.is_visible());
        assert!(!Rgba { alpha: 0.0, ..c }.is_visible());
    }

    #[test]
    fn breeze_and_adwaita_are_light_and_their_dark_themes_dark() {
        assert!(!rgb(222, 224, 226).is_dark()); // Breeze header
        assert!(!rgb(246, 245, 244).is_dark()); // Adwaita window
        assert!(rgb(41, 44, 48).is_dark()); // Breeze Dark header
        assert!(rgb(53, 53, 53).is_dark()); // Adwaita dark window
    }

    #[test]
    fn luminance_runs_from_black_to_white() {
        assert!(rgb(0, 0, 0).luminance().abs() < 1e-9);
        assert!((rgb(255, 255, 255).luminance() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn desktops_are_named_from_xdg_current_desktop() {
        assert_eq!(desktop_kind("KDE"), "kde");
        assert_eq!(desktop_kind("ubuntu:GNOME"), "gnome");
        assert_eq!(desktop_kind("GNOME-Classic:GNOME"), "gnome");
        assert_eq!(desktop_kind("XFCE"), "other");
        assert_eq!(desktop_kind(""), "other");
    }

    #[test]
    fn kwin_title_colours_come_from_the_wm_section() {
        let kdeglobals = "[Colors:Header]\nBackgroundNormal=222,224,226\n\n\
                          [WM]\nactiveBackground=227,229,231\nactiveForeground=35,38,41\n\
                          inactiveBackground=239,240,241\n";
        let (active, inactive) = kde_title_colours(kdeglobals).unwrap();
        assert_eq!(active.css(), "#e3e5e7");
        assert_eq!(inactive.css(), "#eff0f1");
    }

    #[test]
    fn kwin_title_colours_need_an_active_background() {
        assert_eq!(kde_title_colours("[WM]\ninactiveBackground=1,2,3\n"), None);
        let (active, inactive) = kde_title_colours("[WM]\nactiveBackground=1,2,3\n").unwrap();
        assert_eq!(active, inactive);
        assert_eq!(
            kde_title_colours("[General]\nactiveBackground=1,2,3\n"),
            None
        );
    }

    #[test]
    fn kde_colours_take_an_optional_alpha() {
        assert_eq!(Rgba::from_kde("255,0,0").unwrap().css(), "#ff0000");
        assert_eq!(
            Rgba::from_kde("255,0,0,127.5").unwrap().css(),
            "rgba(255, 0, 0, 0.500)"
        );
        assert_eq!(Rgba::from_kde("255,0"), None);
        assert_eq!(Rgba::from_kde("red"), None);
    }

    #[test]
    fn font_family_drops_the_size() {
        assert_eq!(font_family("Noto Sans,  10").as_deref(), Some("Noto Sans"));
        assert_eq!(font_family("Cantarell 11").as_deref(), Some("Cantarell"));
        assert_eq!(font_family("Ubuntu").as_deref(), Some("Ubuntu"));
        assert_eq!(
            font_family("Source Sans 3 10").as_deref(),
            Some("Source Sans 3")
        );
        assert_eq!(font_family(""), None);
    }
}
