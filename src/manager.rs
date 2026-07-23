use hidapi::HidApi;
use log::{info, warn};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::vec::Vec;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::controller::DeviceController;
use crate::devices::{DeviceInfo, RAZER_DEVICE_LIST};

static DID_DUMP_HID: AtomicBool = AtomicBool::new(false);

pub struct DeviceManager {
    api: HidApi,
    pub device_controllers: Arc<Mutex<Vec<DeviceController>>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            api: HidApi::new().unwrap(),
            device_controllers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn fetch_devices(&mut self) -> (Vec<u32>, Vec<u32>) {
        let old_ids: HashSet<u32> = self
            .device_controllers
            .lock()
            .iter()
            .map(|c| c.pid as u32)
            .collect();

        if let Err(err) = self.api.refresh_devices() {
            warn!("Failed to refresh HID devices: {:?}", err);
        }

        let new_controllers = self.get_connected_devices();
        let new_ids: HashSet<u32> = new_controllers.iter().map(|c| c.pid as u32).collect();

        let removed_devices: Vec<u32> = old_ids.difference(&new_ids).cloned().collect();
        let connected_devices: Vec<u32> = new_ids.difference(&old_ids).cloned().collect();

        *self.device_controllers.lock() = new_controllers;

        (removed_devices, connected_devices)
    }

    pub fn get_device_name(&self, id: u32) -> Option<String> {
        self.device_controllers
            .lock()
            .iter()
            .find(|c| c.pid as u32 == id)
            .map(|c| c.name.clone())
    }

    pub fn get_device_battery_level(&self, id: u32) -> Option<i32> {
        let controllers = self.device_controllers.lock();
        let controller = controllers.iter().find(|c| c.pid as u32 == id)?;

        match controller.get_battery_level() {
            Ok(level) => Some(level),
            Err(err) => {
                warn!("Failed to get battery level: {:?}", err);
                None
            }
        }
    }

    pub fn is_device_charging(&self, id: u32) -> Option<bool> {
        let controllers = self.device_controllers.lock();
        let controller = controllers.iter().find(|c| c.pid as u32 == id)?;

        match controller.get_charging_status() {
            Ok(status) => Some(status),
            Err(err) => {
                warn!("Failed to get charging status: {:?}", err);
                None
            }
        }
    }

    fn get_connected_devices(&self) -> Vec<DeviceController> {
        let razer_devices: HashMap<(u16, u16), &DeviceInfo> = RAZER_DEVICE_LIST
            .iter()
            .map(|d| ((d.vid, d.pid), d))
            .collect();

        self.api
            .device_list()
            .filter_map(|hid_device| {
                razer_devices
                    .get(&(hid_device.vendor_id(), hid_device.product_id()))
                    .and_then(|device| {
                        if hid_device.interface_number() != device.interface.into() {
                            return None;
                        }
                        if cfg!(target_os = "windows")
                            && (hid_device.usage_page() != device.usage_page
                                || hid_device.usage() != device.usage)
                        {
                            return None;
                        }
                        match hid_device.open_device(&self.api) {
                            Ok(handle) => DeviceController::new(device.name.to_owned(), device.pid, handle)
                                .map_err(|err| warn!("Failed to create device controller: {:?}", err))
                                .ok(),
                            Err(err) => {
                                warn!("Failed to open hid device: {:?}", err);
                                self.dump_hid_devices_once();
                                None
                            }
                        }
                    })
            })
            .collect()
    }

    fn dump_hid_devices_once(&self) {
        if DID_DUMP_HID
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        info!("Dumping HID device list (one-time) for diagnostics.");
        for d in self.api.device_list() {
            let matched = RAZER_DEVICE_LIST
                .iter()
                .any(|r| r.vid == d.vendor_id() && r.pid == d.product_id());

            info!(
                "HID vid={:04x} pid={:04x} iface={} usage_page={} usage={} matched_pid_list={} mfg={:?} prod={:?} serial={:?} path={}",
                d.vendor_id(),
                d.product_id(),
                d.interface_number(),
                d.usage_page(),
                d.usage(),
                matched,
                d.manufacturer_string(),
                d.product_string(),
                d.serial_number(),
                d.path().to_string_lossy()
            );
        }
    }
}
