## Why

Today the tray icon is a static mouse glyph that only conveys battery state through three colors (white / yellow / red). Users must hover to read the exact percentage in the tooltip. Showing the numeric battery percentage directly on the tray icon lets users read the charge at a glance without hovering, which is the whole point of a battery indicator that lives in the tray.

## What Changes

- Add a set of **100 SVG icons** (one per battery percentage `1`–`100`) that render the numeric percentage as the tray glyph, plus edge-case icons for `0%`, unknown/searching state, and a charging overlay/variant.
- Modify the tray code to **select and render the SVG matching the current battery level** instead of the fixed color PNGs, updating the icon whenever the battery level changes.
- Add an **SVG-to-RGBA rasterization step** so the vector icons can be handed to the `tray-icon` crate (which requires raw RGBA pixels).
- Preserve the existing **color semantics** (red critical / yellow low / normal) by coloring the number/background according to the battery thresholds, and keep a distinct **charging** appearance.
- Keep the tooltip and low/full-battery notification behavior unchanged.

## Capabilities

### New Capabilities
- `tray-battery-icon`: Rendering and selection of the system-tray icon so that it visually displays the current battery percentage as a number, colored by battery threshold and charging state.

### Modified Capabilities
<!-- No existing specs in openspec/specs/; nothing to modify. -->

## Impact

- **Code**: `src/tray.rs` (icon creation/selection/update logic in `TrayApp`), `src/main.rs` (only if module wiring is needed).
- **Assets**: new `assets/icons/` directory containing 100+ generated SVG files; existing `mouse_*.png` may be retained as fallback or removed.
- **Dependencies (`Cargo.toml`)**: add an SVG rasterization stack (e.g. `resvg` / `usvg` / `tiny-skia`) to convert SVG to RGBA at runtime; the `image` crate dependency for icon loading may be reduced or removed.
- **Behavior**: tray icon appearance changes for all users; no changes to device communication, notifications, or the menu.
