//! Renders the system-tray battery icon.
//!
//! Battery percentages are authored as font-free SVG assets (see
//! `tools/generate_icons.py`) embedded via `icon_data`. Each SVG carries the
//! literal token `__FG__` for its fill color; here we pick the color for the
//! current battery threshold / charging state, substitute it, and rasterize the
//! SVG to RGBA with `resvg` so it can be handed to `tray-icon`.

use resvg::{tiny_skia, usvg};

use crate::icon_data::{LEVEL_SVGS, UNKNOWN_SVG};

/// Pixel size the icon is rasterized at. Windows downscales to the tray size.
const ICON_SIZE: u32 = 32;

/// Which color palette to render with, chosen to contrast the taskbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Bright digits, for a dark taskbar.
    Dark,
    /// Dark digits, for a light taskbar.
    Light,
}

/// Foreground colors keyed to battery state.
struct Palette {
    normal: &'static str,
    low: &'static str,
    critical: &'static str,
    charging: &'static str,
    unknown: &'static str,
}

const DARK: Palette = Palette {
    normal: "#F2F2F2",
    low: "#FFC400",
    critical: "#FF453A",
    charging: "#34C759",
    unknown: "#C8CDD2",
};

const LIGHT: Palette = Palette {
    normal: "#202124",
    low: "#9A6400",
    critical: "#C62828",
    charging: "#1B7A34",
    unknown: "#5F6368",
};

fn palette(theme: Theme) -> &'static Palette {
    match theme {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

/// Render the tray icon for the given battery `level`.
///
/// A negative `level` (the unknown/not-yet-read sentinel) yields the searching
/// placeholder. Otherwise the level is clamped to `0..=100` and colored by the
/// `critical` / `low` thresholds, or the charging color, using the palette for
/// the given `theme`.
pub fn render_battery_icon(
    level: i32,
    is_charging: bool,
    critical: i32,
    low: i32,
    theme: Theme,
) -> Result<tray_icon::Icon, String> {
    let p = palette(theme);

    if level < 0 {
        return render_svg(UNKNOWN_SVG, p.unknown);
    }

    let lvl = level.clamp(0, 100) as usize;
    let color = if is_charging {
        p.charging
    } else if level <= critical {
        p.critical
    } else if level <= low {
        p.low
    } else {
        p.normal
    };

    render_svg(LEVEL_SVGS[lvl], color)
}

/// Convenience wrapper for the startup / unknown placeholder icon.
pub fn render_unknown_icon(theme: Theme) -> Result<tray_icon::Icon, String> {
    render_svg(UNKNOWN_SVG, palette(theme).unknown)
}

fn render_svg(template: &str, color: &str) -> Result<tray_icon::Icon, String> {
    let svg = template.replace("__FG__", color);

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg, &options)
        .map_err(|e| format!("Failed to parse SVG icon: {}", e))?;

    let mut pixmap = tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE)
        .ok_or_else(|| "Failed to allocate icon pixmap".to_string())?;

    // viewBox and pixmap are both ICON_SIZE, so the mapping is 1:1.
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    // tiny-skia stores premultiplied RGBA; tray-icon expects straight alpha.
    let rgba = unpremultiply(pixmap.data());

    tray_icon::Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)
        .map_err(|e| format!("Failed to create tray icon: {}", e))
}

/// Convert premultiplied RGBA bytes to straight (non-premultiplied) alpha.
fn unpremultiply(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for px in data.chunks_exact(4) {
        let a = px[3];
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let un = |c: u8| (((c as u16) * 255 + (a as u16) / 2) / a as u16).min(255) as u8;
            out.extend_from_slice(&[un(px[0]), un(px[1]), un(px[2]), a]);
        }
    }
    out
}
