use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    console::DebugConsole, manager::DeviceManager, notify::Notify, settings::ThemeSetting,
};
use log::{error, info, trace, warn};
use parking_lot::Mutex;
use tao::event_loop::{EventLoopBuilder, EventLoopProxy};
use tray_icon::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    TrayIcon, TrayIconBuilder,
};

const BATTERY_UPDATE_INTERVAL: u64 = 300; // 5 min
const DEVICE_FETCH_INTERVAL: Duration = Duration::from_secs(5);

const BATTERY_CRITICAL_LEVEL: i32 = 5;
const BATTERY_LOW_LEVEL: i32 = 15;

#[derive(Debug)]
pub struct MemoryDevice {
    pub name: String,
    #[allow(unused)]
    pub pid: u32,
    pub battery_level: i32,
    pub old_battery_level: i32,
    pub is_charging: bool,
}

impl MemoryDevice {
    fn new(name: String, pid: u32) -> Self {
        Self {
            name,
            pid,
            battery_level: -1,
            old_battery_level: 50,
            is_charging: false,
        }
    }
}

/// Handles to the tray menu items we need to react to or update at runtime.
struct MenuHandles {
    show_console: MenuItem,
    theme_auto: CheckMenuItem,
    theme_light: CheckMenuItem,
    theme_dark: CheckMenuItem,
    quit: MenuItem,
}

impl MenuHandles {
    fn new(theme: ThemeSetting) -> Self {
        Self {
            show_console: MenuItem::new("Show Log Window", true, None),
            theme_auto: CheckMenuItem::new("Auto", true, theme == ThemeSetting::Auto, None),
            theme_light: CheckMenuItem::new(
                "Light taskbar",
                true,
                theme == ThemeSetting::Light,
                None,
            ),
            theme_dark: CheckMenuItem::new("Dark taskbar", true, theme == ThemeSetting::Dark, None),
            quit: MenuItem::new("Exit", true, None),
        }
    }

    /// Reflect the active theme setting in the radio-style check marks.
    fn sync_theme_checks(&self, theme: ThemeSetting) {
        self.theme_auto.set_checked(theme == ThemeSetting::Auto);
        self.theme_light.set_checked(theme == ThemeSetting::Light);
        self.theme_dark.set_checked(theme == ThemeSetting::Dark);
    }
}

pub struct TrayInner {
    tray_icon: Rc<Mutex<Option<TrayIcon>>>,
    menu: Rc<MenuHandles>,
    debug_console: Rc<DebugConsole>,
    /// The user's theme preference (persisted).
    theme_setting: Rc<Mutex<ThemeSetting>>,
    /// The `(battery_level, is_charging)` currently shown, so a theme switch can
    /// re-render the same state. `-1` means the unknown placeholder.
    current_state: Rc<Mutex<(i32, bool)>>,
}

impl TrayInner {
    fn new(debug_console: Rc<DebugConsole>, theme: ThemeSetting) -> Self {
        Self {
            tray_icon: Rc::new(Mutex::new(None)),
            menu: Rc::new(MenuHandles::new(theme)),
            debug_console,
            theme_setting: Rc::new(Mutex::new(theme)),
            current_state: Rc::new(Mutex::new((-1, false))),
        }
    }

    fn create_menu(&self) -> Menu {
        let tray_menu = Menu::new();
        let m = &self.menu;

        let theme_menu = Submenu::new("Icon Theme", true);
        if let Err(e) =
            theme_menu.append_items(&[&m.theme_auto, &m.theme_light, &m.theme_dark])
        {
            warn!("Failed to build theme submenu: {}", e);
        }

        let separator = PredefinedMenuItem::separator();
        let items: [&dyn IsMenuItem; 4] =
            [&m.show_console, &theme_menu, &separator, &m.quit];
        if let Err(e) = tray_menu.append_items(&items) {
            warn!("Failed to append menu items: {}", e);
        }
        tray_menu
    }

    fn build_tray(
        tray_icon: &Rc<Mutex<Option<TrayIcon>>>,
        tray_menu: &Menu,
        icon: tray_icon::Icon,
    ) {
        let tray_builder = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.clone()))
            .with_tooltip("Search for devices")
            .with_icon(icon)
            .build();

        match tray_builder {
            Ok(tray) => *tray_icon.lock() = Some(tray),
            Err(err) => error!("Failed to create tray icon: {}", err),
        }
    }
}

pub struct TrayApp {
    device_manager: Arc<Mutex<DeviceManager>>,
    devices: Arc<Mutex<HashMap<u32, MemoryDevice>>>,
    tray_inner: TrayInner,
    notify: Arc<Notify>,
}

#[derive(Debug)]
enum TrayEvent {
    DeviceUpdate(Vec<u32>),
    MenuEvent(MenuEvent),
}

impl TrayApp {
    pub fn new(debug_console: DebugConsole) -> Self {
        let theme = crate::settings::load();
        Self {
            device_manager: Arc::new(Mutex::new(DeviceManager::new())),
            devices: Arc::new(Mutex::new(HashMap::new())),
            tray_inner: TrayInner::new(Rc::new(debug_console), theme),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn run(&self) {
        let theme = self.tray_inner.theme_setting.lock().resolve();
        let icon = match crate::icon::render_unknown_icon(theme) {
            Ok(icon) => icon,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };
        let event_loop = EventLoopBuilder::with_user_event().build();
        let tray_menu = self.tray_inner.create_menu();

        let proxy = event_loop.create_proxy();

        self.spawn_device_fetch_thread(proxy.clone());

        self.run_event_loop(event_loop, icon, tray_menu, proxy);
    }

    fn spawn_device_fetch_thread(&self, proxy: EventLoopProxy<TrayEvent>) {
        let devices = Arc::clone(&self.devices);
        let device_manager = Arc::clone(&self.device_manager);
        let notify = Arc::clone(&self.notify);

        thread::spawn(move || {
            let mut last_devices = HashSet::new();
            let mut battery_update_counter = 0;
            loop {
                let (removed_devices, connected_devices) = {
                    let mut manager = device_manager.lock();
                    manager.fetch_devices()
                };

                let mut devices_lock = devices.lock();
                for id in removed_devices {
                    if let Some(device) = devices_lock.remove(&id) {
                        info!("Device removed: {}", device.name);
                        let _ = notify.device_disconnecred(&device.name);
                    }
                }

                for &id in &connected_devices {
                    if let std::collections::hash_map::Entry::Vacant(e) = devices_lock.entry(id) {
                        if let Some(name) = device_manager.lock().get_device_name(id) {
                            e.insert(MemoryDevice::new(name.clone(), id));
                            info!("New device: {}", name);
                            let _ = notify.device_connected(&name);
                        } else {
                            error!("Failed to get device name for id: {}", id);
                        }
                    }
                }

                let current_devices: HashSet<_> = connected_devices.iter().cloned().collect();
                if current_devices != last_devices {
                    let _ = proxy.send_event(TrayEvent::DeviceUpdate(connected_devices));
                    last_devices = current_devices;
                }

                if battery_update_counter == 0 {
                    let device_ids: Vec<u32> = devices_lock.keys().cloned().collect();
                    let _ = proxy.send_event(TrayEvent::DeviceUpdate(device_ids));
                }

                battery_update_counter = (battery_update_counter + 1)
                    % (BATTERY_UPDATE_INTERVAL / DEVICE_FETCH_INTERVAL.as_secs());

                thread::sleep(DEVICE_FETCH_INTERVAL);
            }
        });
    }

    fn run_event_loop(
        &self,
        event_loop: tao::event_loop::EventLoop<TrayEvent>,
        icon: tray_icon::Icon,
        tray_menu: Menu,
        proxy: EventLoopProxy<TrayEvent>,
    ) {
        let devices = Arc::clone(&self.devices);
        let device_manager = Arc::clone(&self.device_manager);
        let tray_icon = Rc::clone(&self.tray_inner.tray_icon);
        let debug_console = Rc::clone(&self.tray_inner.debug_console);
        let menu = Rc::clone(&self.tray_inner.menu);
        let theme_setting = Rc::clone(&self.tray_inner.theme_setting);
        let current_state = Rc::clone(&self.tray_inner.current_state);
        let notify = Arc::clone(&self.notify);

        let menu_channel = MenuEvent::receiver();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = tao::event_loop::ControlFlow::Wait;

            match event {
                tao::event::Event::NewEvents(tao::event::StartCause::Init) => {
                    TrayInner::build_tray(&tray_icon, &tray_menu, icon.clone());
                }
                tao::event::Event::UserEvent(TrayEvent::DeviceUpdate(device_ids)) => {
                    Self::update(
                        &devices,
                        &device_manager,
                        &device_ids,
                        &tray_icon,
                        &notify,
                        &theme_setting,
                        &current_state,
                    );
                }
                tao::event::Event::UserEvent(TrayEvent::MenuEvent(event)) => {
                    if event.id == menu.show_console.id() {
                        debug_console.toggle_visibility();
                        let visible = debug_console.is_visible();
                        menu.show_console.set_text(if visible {
                            "Hide Log Window"
                        } else {
                            "Show Log Window"
                        });
                        trace!("{} log window", if visible { "showing" } else { "hiding" });
                    } else if event.id == menu.theme_auto.id() {
                        Self::set_theme(&menu, &theme_setting, &current_state, &tray_icon, ThemeSetting::Auto);
                    } else if event.id == menu.theme_light.id() {
                        Self::set_theme(&menu, &theme_setting, &current_state, &tray_icon, ThemeSetting::Light);
                    } else if event.id == menu.theme_dark.id() {
                        Self::set_theme(&menu, &theme_setting, &current_state, &tray_icon, ThemeSetting::Dark);
                    } else if event.id == menu.quit.id() {
                        *control_flow = tao::event_loop::ControlFlow::Exit;
                    }
                }
                _ => (),
            }

            if let Ok(event) = menu_channel.try_recv() {
                let _ = proxy.send_event(TrayEvent::MenuEvent(event));
            }
        });
    }

    /// Apply a new theme preference: persist it, update the check marks, and
    /// re-render the current icon so the change is visible immediately.
    fn set_theme(
        menu: &Rc<MenuHandles>,
        theme_setting: &Rc<Mutex<ThemeSetting>>,
        current_state: &Rc<Mutex<(i32, bool)>>,
        tray_icon: &Rc<Mutex<Option<TrayIcon>>>,
        new_setting: ThemeSetting,
    ) {
        *theme_setting.lock() = new_setting;
        crate::settings::save(new_setting);
        menu.sync_theme_checks(new_setting);
        trace!("Theme set to {:?}", new_setting);

        let (level, is_charging) = *current_state.lock();
        Self::set_tray_icon(tray_icon, level, is_charging, new_setting.resolve());
    }

    /// Render `(level, is_charging)` with the given palette and push it to the tray.
    fn set_tray_icon(
        tray_icon: &Rc<Mutex<Option<TrayIcon>>>,
        level: i32,
        is_charging: bool,
        theme: crate::icon::Theme,
    ) {
        let icon = if level < 0 {
            crate::icon::render_unknown_icon(theme)
        } else {
            crate::icon::render_battery_icon(
                level,
                is_charging,
                BATTERY_CRITICAL_LEVEL,
                BATTERY_LOW_LEVEL,
                theme,
            )
        };

        match icon {
            Ok(icon) => {
                if let Some(tray_icon) = tray_icon.lock().as_mut() {
                    if let Err(e) = tray_icon.set_icon(Some(icon)) {
                        warn!("Failed to update tray icon: {}", e);
                    }
                }
            }
            Err(e) => warn!("Failed to render tray icon: {}", e),
        }
    }

    fn update(
        devices: &Arc<Mutex<HashMap<u32, MemoryDevice>>>,
        manager: &Arc<Mutex<DeviceManager>>,
        device_ids: &[u32],
        tray_icon: &Rc<Mutex<Option<TrayIcon>>>,
        notify: &Arc<Notify>,
        theme_setting: &Rc<Mutex<ThemeSetting>>,
        current_state: &Rc<Mutex<(i32, bool)>>,
    ) {
        let mut devices = devices.lock();
        let manager = manager.lock();

        for &id in device_ids {
            if let Some(device) = devices.get_mut(&id) {
                if let (Some(battery_level), Some(is_charging)) = (
                    manager.get_device_battery_level(id),
                    manager.is_device_charging(id),
                ) {
                    info!("{}  battery level: {}%", device.name, battery_level);
                    info!("{}  charging status: {}", device.name, is_charging);

                    device.old_battery_level = device.battery_level;
                    device.battery_level = battery_level;
                    device.is_charging = is_charging;

                    Self::check_notify(device, notify);

                    // Remember what's on screen so a theme switch can re-render it.
                    *current_state.lock() = (battery_level, is_charging);

                    if device.old_battery_level != battery_level
                        || device.is_charging != is_charging
                    {
                        let theme = theme_setting.lock().resolve();
                        Self::set_tray_icon(tray_icon, battery_level, is_charging, theme);
                    }

                    if let Some(tray_icon) = tray_icon.lock().as_mut() {
                        let _ = tray_icon
                            .set_tooltip(Some(format!("{}: {}%", device.name, battery_level)));
                    }
                }
            }
        }
    }

    fn check_notify(device: &MemoryDevice, notify: &Notify) {
        if device.battery_level == -1 {
            return;
        }

        if !device.is_charging
            && (device.battery_level <= BATTERY_CRITICAL_LEVEL
                || (device.old_battery_level > BATTERY_LOW_LEVEL
                    && device.battery_level <= BATTERY_LOW_LEVEL))
        {
            info!("{}: Battery low ({}%)", device.name, device.battery_level);
            let _ = notify.battery_low(&device.name, device.battery_level);
        } else if device.old_battery_level <= 99
            && device.battery_level == 100
            && device.is_charging
        {
            info!(
                "{}: Battery fully charged ({}%)",
                device.name, device.battery_level
            );
            let _ = notify.battery_full(&device.name);
        }
    }
}
