//! Debug helper: rasterize battery SVGs in both palettes onto representative
//! taskbar backgrounds so we can eyeball contrast.
//! Run: cargo run --example render_check --release

use resvg::{tiny_skia, usvg};

const SIZE: u32 = 32;

fn render(svg_tpl: &str, fg: &str, bg: tiny_skia::Color, out: &str) {
    let svg = svg_tpl.replace("__FG__", fg);
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg, &opt).expect("parse svg");

    let mut pixmap = tiny_skia::Pixmap::new(SIZE, SIZE).unwrap();
    pixmap.fill(bg); // simulate the taskbar background
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.save_png(out).expect("save png");
    println!("wrote {out}");
}

fn main() {
    let lvl42 = include_str!("../assets/icons/042.svg");
    let unknown = include_str!("../assets/icons/unknown.svg");

    // Dark taskbar palette (bright digits) on a dark background.
    let dark_bg = tiny_skia::Color::from_rgba8(32, 32, 32, 255);
    render(lvl42, "#F2F2F2", dark_bg, "check_dark_42.png");

    // Light taskbar palette (dark digits) on a light background.
    let light_bg = tiny_skia::Color::from_rgba8(243, 243, 243, 255);
    render(lvl42, "#202124", light_bg, "check_light_42.png");
    render(unknown, "#5F6368", light_bg, "check_light_unknown.png");
    println!("done");
}
