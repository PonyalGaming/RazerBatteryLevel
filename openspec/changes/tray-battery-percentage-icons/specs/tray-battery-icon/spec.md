## ADDED Requirements

### Requirement: Numeric percentage tray icon

The system SHALL render the system-tray icon so that it displays the current battery percentage of the active device as a number. The number displayed SHALL equal the last reported battery level, clamped to the range `0`–`100`.

#### Scenario: Known battery level is displayed as a number

- **WHEN** a device reports a battery level of `N` percent (with `0 <= N <= 100`)
- **THEN** the tray icon shows the number `N`

#### Scenario: Full charge

- **WHEN** a device reports a battery level of `100` percent
- **THEN** the tray icon shows `100` in a legible form sized to fit the tray icon dimensions

#### Scenario: Zero charge

- **WHEN** a device reports a battery level of `0` percent
- **THEN** the tray icon shows `0`

### Requirement: Icon color reflects battery threshold

The system SHALL color the tray icon according to the same battery thresholds already used for notifications: a critical appearance at or below the critical level, a low appearance at or below the low level, and a normal appearance otherwise. These thresholds SHALL remain consistent with the existing `BATTERY_CRITICAL_LEVEL` and `BATTERY_LOW_LEVEL` constants.

#### Scenario: Critical battery

- **WHEN** the battery level is at or below the critical level and the device is not charging
- **THEN** the tray icon uses the critical (red) color scheme

#### Scenario: Low battery

- **WHEN** the battery level is above the critical level, at or below the low level, and the device is not charging
- **THEN** the tray icon uses the low (yellow) color scheme

#### Scenario: Normal battery

- **WHEN** the battery level is above the low level and the device is not charging
- **THEN** the tray icon uses the normal color scheme

### Requirement: Charging state is visually distinct

The system SHALL render a visually distinct icon when the active device is charging, so the charging state is distinguishable from an equivalent non-charging level.

#### Scenario: Charging indicator shown

- **WHEN** the device reports it is charging
- **THEN** the tray icon shows the percentage together with a distinct charging appearance (e.g. a charging accent/color) that differs from the non-charging icon at the same level

### Requirement: Unknown or searching state

The system SHALL display a distinct placeholder icon when no valid battery level is available, such as at startup before the first successful read or when the level is unknown (represented internally as `-1`).

#### Scenario: No level available yet

- **WHEN** the battery level is unknown (no successful reading has occurred)
- **THEN** the tray icon shows a placeholder/searching appearance rather than a numeric percentage

### Requirement: SVG source with runtime rasterization

The percentage icons SHALL be authored as SVG assets, one per whole-number percentage from `1` to `100`, and the system SHALL rasterize the selected SVG to RGBA pixels at runtime before handing it to the tray so the vector artwork drives the displayed icon.

#### Scenario: SVG set is complete

- **WHEN** the icon assets are built
- **THEN** there exists one SVG icon for every integer percentage value from `1` to `100` inclusive, plus the `0%`, unknown, and charging variants required by the other requirements

#### Scenario: Selected SVG is rasterized for the tray

- **WHEN** the tray icon is set for a given battery level
- **THEN** the SVG matching that level is rasterized to RGBA and applied as the tray icon without requiring an external image file at runtime

### Requirement: Icon updates on battery change

The system SHALL update the tray icon whenever the battery level or charging state of the active device changes, so the displayed number stays current.

#### Scenario: Level changes

- **WHEN** a subsequent reading reports a different battery level or charging state than the currently displayed icon
- **THEN** the tray icon is replaced with the icon matching the new level and charging state

#### Scenario: No change

- **WHEN** a subsequent reading reports the same battery level and charging state as the currently displayed icon
- **THEN** the tray icon is not needlessly re-rendered
