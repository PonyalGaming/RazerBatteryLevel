## Context

`razer-battery-report` is a Windows-only Rust tray app (`tao` + `tray-icon`). The tray icon is currently one of three static PNGs (`mouse_white/yellow/red.png`) chosen by battery threshold, loaded with the `image` crate and handed to `tray-icon` as RGBA (`tray.rs::get_battery_icon` / `create_icon`). The exact percentage only appears in the tooltip.

The `tray-icon` crate requires raw **RGBA pixel data** (`tray_icon::Icon::from_rgba(rgba, w, h)`); it cannot consume SVG or PNG directly. So any SVG approach needs a rasterization step producing RGBA.

Relevant existing constants/behavior we must stay consistent with:
- `BATTERY_CRITICAL_LEVEL = 5`, `BATTERY_LOW_LEVEL = 15` (used for both notifications and icon color).
- Battery level `-1` is the "unknown / not yet read" sentinel (`MemoryDevice::new`).
- The icon is only refreshed when `old_battery_level != battery_level || is_charging changed` — we keep this guard.
- The last device to update drives the single tray icon (multi-device shows the most recently updated device's level). We preserve this.

## Goals / Non-Goals

**Goals:**
- Tray icon shows the numeric battery percentage (0–100), colored by the existing thresholds, with a distinct charging appearance and an unknown/searching placeholder.
- Icons authored as SVG (100 files, one per percent 1–100) and rasterized to RGBA at runtime.
- No regression to notifications, tooltip, menu, or device polling.

**Non-Goals:**
- Per-device icons / multiple simultaneous tray icons (still one icon = most recently updated device).
- Cross-platform support (remains Windows-only).
- User-configurable icon themes, fonts, or colors.
- Animated / progress-ring artwork (a number is sufficient; a subtle ring is optional polish, not required).

## Decisions

### Decision 1: Rasterize SVG with `resvg` (+ `usvg` + `tiny-skia`)
`resvg` is the de-facto Rust SVG renderer and bundles `usvg` (parse) and `tiny-skia` (raster). At runtime we parse the selected SVG, render to a `tiny-skia::Pixmap` at the target tray size (e.g. 32×32, matching the current icons' expectations), and pass `pixmap.data()` (already RGBA8, premultiplied — un-premultiply if edges look wrong) to `tray_icon::Icon::from_rgba`.
- *Alternative considered:* pre-rasterize all icons to PNG at build time and keep using `image`. Rejected because the user explicitly wants SVG icons, and runtime rendering keeps the assets vector/editable and avoids shipping ~300 PNGs.
- *Alternative considered:* `nsvg`/`librsvg`. Rejected — `librsvg` needs a C/GTK toolchain (bad on Windows); `resvg` is pure Rust.

### Decision 2: Text is pre-converted to vector paths in the SVG assets (no runtime fonts)
Rendering `<text>` with `resvg` requires a font database (`usvg::fontdb`) and a bundled font, which adds weight and a font-license concern. Instead, the 100 SVGs embed each digit as `<path>` geometry, so runtime rendering is **font-free and deterministic**. Digit path data comes from one permissively licensed numeric font, converted to paths once during asset generation.
- *Alternative considered:* bundle a font and load it into `fontdb` at runtime. Rejected to avoid font-licensing and layout variability; kept as a fallback if path generation proves impractical.

### Decision 3: One physical SVG per percentage (1–100) with a color placeholder token, not per-color files
Authoring 100 × 3-colors × charging = hundreds of files is unmaintainable. Each of the 100 SVGs contains a placeholder token (e.g. `__FG__` / `__BG__`, or `fill="currentColor"`) for the themeable colors. At runtime the tray code:
1. selects the SVG for the clamped level,
2. substitutes the color token(s) based on threshold (normal / low / critical) and charging state,
3. rasterizes.
This yields exactly the **100 SVG files** requested while color/charging are applied at render time. `0%`, unknown/searching, and the charging accent are handled as additional small SVG assets / token values.
- *Alternative considered:* a single parametric template rendered with the digits drawn programmatically. Rejected because the user asked specifically for a set of SVG icon files (inspectable/editable assets).

### Decision 4: Asset generation via a small generator script committed alongside the assets
The 100 SVGs are produced by a generator (a standalone script or a `xtask`/build helper) that stamps each number into a shared template and writes `assets/icons/<NN>.svg`. Committing both the generator and the output makes regeneration reproducible without forcing a build-time dependency into the shipped binary.

### Decision 5: Embed icons at compile time
SVGs are embedded with `include_str!` (via a generated `mod`/array mapping level → SVG string) so the released `.exe` stays a single self-contained file, matching how PNGs are embedded today (`include_bytes!`). The runtime never reads `assets/` from disk.

### Decision 6: Refactor icon logic into a dedicated path
Replace `get_battery_icon`/`create_icon` internals with an `icon` module (or private fns in `tray.rs`) exposing something like `render_tray_icon(level: i32, is_charging: bool, size: u32) -> Result<tray_icon::Icon, String>`, keeping the call sites in `update()` and startup unchanged in shape. Keep the change guard in `update()`.

## Risks / Trade-offs

- **[resvg rasterization cost per update]** → Battery updates happen at most every few seconds and typically every 5 min; rendering a 32×32 SVG is sub-millisecond. Guarded by the existing "only on change" check, so cost is negligible.
- **[Premultiplied alpha / color fringing from tiny-skia]** → `Pixmap` is premultiplied RGBA; if edges look dark, un-premultiply before `from_rgba`. Verify visually against the tray at build time.
- **[Legibility of 3 digits at 16–32 px]** → "100" is wide; the template must shrink/condense for 3-digit values (special-case width for `100`, and generally scale the number to fit). Covered by a "fits the tray dimensions" scenario in the spec.
- **[Windows DPI scaling]** → Render at a size appropriate for the tray (32×32 is safe; Windows downscales). If blurry on high-DPI, render larger and let the shell scale, or query the tray size.
- **[New dependencies increase build time/binary size]** → `resvg` pulls `tiny-skia`/`usvg`; acceptable, and we can drop `image` if it's no longer used elsewhere.
- **[Digit-to-path generation complexity]** → If converting glyphs to paths is impractical during apply, fall back to Decision 2's alternative (bundle a numeric font + `fontdb`), which is still self-contained.

## Open Questions

- Target render size — reuse whatever the current PNGs are (likely 32×32) or query the actual tray icon size? Default to 32×32 unless the current assets differ.
- Should the old `mouse_*.png` assets be removed or kept as a documented fallback? Default: keep the files but stop referencing them, remove in a later cleanup.
- Exact visual style (number only vs. number + battery outline/ring). Default: number-only with color coding + a small charging bolt accent; refine during apply if it reads poorly.
