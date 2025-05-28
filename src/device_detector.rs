//! Storage device detection and optimization for ferrocp
//!
//! This module provides functionality to detect storage device types and
//! optimize I/O operations based on device characteristics.

use crate::error::{Error, Result};
use std::path::Path;
use tracing::{debug, warn};

/// Storage device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Solid State Drive (SSD)
    SSD,
    /// Hard Disk Drive (HDD)
    HDD,
    /// NVMe SSD
    NVMe,
    /// Network storage
    Network,
    /// Unknown device type
    Unknown,
}

impl DeviceType {
    /// Get the recommended thread count for this device type
    pub fn recommended_thread_count(&self) -> usize {
        match self {
            DeviceType::NVMe => num_cpus::get() * 2,      // NVMe can handle high concurrency
            DeviceType::SSD => num_cpus::get(),           // SSD benefits from moderate concurrency
            DeviceType::HDD => 2,                         // HDD prefers sequential access
            DeviceType::Network => num_cpus::get() / 2,   // Network depends on bandwidth
            DeviceType::Unknown => num_cpus::get(),       // Default to CPU count
        }
    }

    /// Get the recommended buffer size for this device type
    pub fn recommended_buffer_size(&self) -> usize {
        match self {
            DeviceType::NVMe => 16 * 1024 * 1024,    // 16MB for NVMe
            DeviceType::SSD => 8 * 1024 * 1024,      // 8MB for SSD
            DeviceType::HDD => 4 * 1024 * 1024,      // 4MB for HDD (larger sequential reads)
            DeviceType::Network => 1 * 1024 * 1024,  // 1MB for network (smaller chunks)
            DeviceType::Unknown => 8 * 1024 * 1024,  // Default 8MB
        }
    }

    /// Get the recommended batch size for small files
    pub fn recommended_batch_size(&self) -> usize {
        match self {
            DeviceType::NVMe => 100,     // NVMe can handle many small files
            DeviceType::SSD => 50,       // SSD handles moderate batches well
            DeviceType::HDD => 10,       // HDD prefers smaller batches
            DeviceType::Network => 20,   // Network depends on latency
            DeviceType::Unknown => 50,   // Default batch size
        }
    }

    /// Check if this device type benefits from zero-copy operations
    pub fn supports_zerocopy(&self) -> bool {
        match self {
            DeviceType::NVMe | DeviceType::SSD => true,
            DeviceType::HDD => false,  // HDD may not benefit from zero-copy
            DeviceType::Network => false,  // Network doesn't support local zero-copy
            DeviceType::Unknown => true,   // Default to enabled
        }
    }
}

/// Device detection and optimization configuration
#[derive(Debug, Clone)]
pub struct DeviceDetector {
    /// Cache of detected device types by path
    device_cache: std::collections::HashMap<String, DeviceType>,
}

impl DeviceDetector {
    /// Create a new device detector
    pub fn new() -> Self {
        Self {
            device_cache: std::collections::HashMap::new(),
        }
    }

    /// Detect the device type for a given path
    pub async fn detect_device_type<P: AsRef<Path>>(&mut self, path: P) -> Result<DeviceType> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Check cache first
        if let Some(&device_type) = self.device_cache.get(&path_str) {
            return Ok(device_type);
        }

        let device_type = self.detect_device_type_impl(path).await?;
        
        // Cache the result
        self.device_cache.insert(path_str, device_type);
        
        debug!("Detected device type for {:?}: {:?}", path, device_type);
        Ok(device_type)
    }

    /// Internal implementation for device detection
    async fn detect_device_type_impl<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        let path = path.as_ref();

        // Check if it's a network path
        if self.is_network_path(path) {
            return Ok(DeviceType::Network);
        }

        // Platform-specific device detection
        #[cfg(target_os = "windows")]
        {
            self.detect_windows_device_type(path).await
        }

        #[cfg(target_os = "linux")]
        {
            self.detect_linux_device_type(path).await
        }

        #[cfg(target_os = "macos")]
        {
            self.detect_macos_device_type(path).await
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            warn!("Device detection not supported on this platform");
            Ok(DeviceType::Unknown)
        }
    }

    /// Check if the path is a network path
    fn is_network_path<P: AsRef<Path>>(&self, path: P) -> bool {
        let path_str = path.as_ref().to_string_lossy();
        
        // Windows UNC paths
        if path_str.starts_with("\\\\") {
            return true;
        }
        
        // SMB/CIFS paths
        if path_str.starts_with("smb://") || path_str.starts_with("cifs://") {
            return true;
        }
        
        // NFS paths
        if path_str.contains(":/") && !path_str.starts_with("/") {
            return true;
        }
        
        false
    }

    /// Windows-specific device detection
    #[cfg(target_os = "windows")]
    async fn detect_windows_device_type<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let path = path.as_ref();
        
        // Get the drive letter
        let drive = if let Some(drive_letter) = path.to_string_lossy().chars().next() {
            format!("{}:", drive_letter)
        } else {
            return Ok(DeviceType::Unknown);
        };

        // Try to detect using WMI or system calls
        // For now, use a simple heuristic based on drive type
        match self.get_windows_drive_type(&drive).await {
            Ok(drive_type) => Ok(drive_type),
            Err(_) => {
                warn!("Failed to detect Windows drive type for {}", drive);
                Ok(DeviceType::Unknown)
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_windows_drive_type(&self, drive: &str) -> Result<DeviceType> {
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        use windows::core::HSTRING;

        // Convert drive letter to wide string
        let drive_path = if drive.ends_with('\\') {
            drive.to_string()
        } else {
            format!("{}\\", drive)
        };

        let wide_path = HSTRING::from(&drive_path);

        // Get basic drive type first
        let drive_type = unsafe { GetDriveTypeW(&wide_path) };

        match drive_type {
            4 => return Ok(DeviceType::Network),      // DRIVE_REMOTE
            2 | 5 => return Ok(DeviceType::Unknown),  // DRIVE_REMOVABLE | DRIVE_CDROM
            6 => return Ok(DeviceType::SSD),          // DRIVE_RAMDISK - RAM disk is fast like SSD
            3 => {                                    // DRIVE_FIXED
                // For fixed drives, try to detect SSD vs HDD using WMI
                if let Ok(device_type) = self.detect_windows_storage_type(drive).await {
                    return Ok(device_type);
                }
                // Fallback to registry-based detection
                self.detect_windows_storage_from_registry(drive).await
            }
            _ => Ok(DeviceType::Unknown),
        }
    }

    /// Linux-specific device detection
    #[cfg(target_os = "linux")]
    async fn detect_linux_device_type<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        use tokio::fs;

        let path = path.as_ref();
        
        // Get the mount point and device
        let mount_info = self.get_linux_mount_info(path).await?;
        
        // Check if it's an NVMe device
        if mount_info.device.contains("nvme") {
            return Ok(DeviceType::NVMe);
        }
        
        // Check /sys/block for rotational flag
        let device_name = mount_info.device.trim_start_matches("/dev/");
        let rotational_path = format!("/sys/block/{}/queue/rotational", device_name);
        
        match fs::read_to_string(&rotational_path).await {
            Ok(content) => {
                if content.trim() == "0" {
                    Ok(DeviceType::SSD)
                } else {
                    Ok(DeviceType::HDD)
                }
            }
            Err(_) => {
                warn!("Could not read rotational flag for {}", device_name);
                Ok(DeviceType::Unknown)
            }
        }
    }

    #[cfg(target_os = "linux")]
    async fn get_linux_mount_info<P: AsRef<Path>>(&self, _path: P) -> Result<MountInfo> {
        // TODO: Parse /proc/mounts to get device information
        // For now, return a placeholder
        Ok(MountInfo {
            device: "/dev/sda1".to_string(),
            mount_point: "/".to_string(),
            fs_type: "ext4".to_string(),
        })
    }

    /// macOS-specific device detection
    #[cfg(target_os = "macos")]
    async fn detect_macos_device_type<P: AsRef<Path>>(&self, _path: P) -> Result<DeviceType> {
        // TODO: Implement macOS-specific detection using system_profiler or diskutil
        // For now, return SSD as a reasonable default for modern Macs
        Ok(DeviceType::SSD)
    }

    /// Get optimized I/O configuration for a device type
    pub fn get_io_config(&self, device_type: DeviceType) -> IOOptimizationConfig {
        IOOptimizationConfig {
            device_type,
            thread_count: device_type.recommended_thread_count(),
            buffer_size: device_type.recommended_buffer_size(),
            batch_size: device_type.recommended_batch_size(),
            enable_zerocopy: device_type.supports_zerocopy(),
            read_ahead: match device_type {
                DeviceType::HDD => true,     // HDD benefits from read-ahead
                DeviceType::Network => false, // Network may not benefit
                _ => false,                   // SSD/NVMe don't need read-ahead
            },
        }
    }
}

/// Mount information for Linux systems
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
struct MountInfo {
    device: String,
    mount_point: String,
    fs_type: String,
}

/// I/O optimization configuration based on device type
#[derive(Debug, Clone)]
pub struct IOOptimizationConfig {
    /// Detected device type
    pub device_type: DeviceType,
    /// Recommended thread count
    pub thread_count: usize,
    /// Recommended buffer size
    pub buffer_size: usize,
    /// Recommended batch size for small files
    pub batch_size: usize,
    /// Whether to enable zero-copy operations
    pub enable_zerocopy: bool,
    /// Whether to enable read-ahead
    pub read_ahead: bool,
}

impl Default for DeviceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IOOptimizationConfig {
    fn default() -> Self {
        Self {
            device_type: DeviceType::Unknown,
            thread_count: num_cpus::get(),
            buffer_size: 8 * 1024 * 1024,
            batch_size: 50,
            enable_zerocopy: true,
            read_ahead: false,
        }
    }
}

#[cfg(target_os = "windows")]
impl DeviceDetector {
    /// Detect storage type using WMI
    async fn detect_windows_storage_type(&self, drive: &str) -> Result<DeviceType> {
        use wmi::{COMLibrary, WMIConnection, Variant};
        use std::collections::HashMap;

        // Initialize COM library
        let com_con = COMLibrary::new().map_err(|e| {
            warn!("Failed to initialize COM library: {}", e);
            Error::device_detection("COM initialization failed")
        })?;

        let wmi_con = WMIConnection::new(com_con.into()).map_err(|e| {
            warn!("Failed to connect to WMI: {}", e);
            Error::device_detection("WMI connection failed")
        })?;

        // Get drive letter without backslash
        let drive_letter = drive.trim_end_matches('\\').trim_end_matches(':');

        // Query Win32_LogicalDisk to get the DeviceID
        let query = format!("SELECT * FROM Win32_LogicalDisk WHERE DeviceID = '{}:'", drive_letter);
        let results: Vec<HashMap<String, Variant>> = wmi_con.raw_query(&query).map_err(|e| {
            warn!("WMI query failed: {}", e);
            Error::device_detection("WMI query failed")
        })?;

        if results.is_empty() {
            return Err(Error::device_detection("Drive not found"));
        }

        // Query Win32_DiskDrive for physical disk information
        let disk_query = "SELECT * FROM Win32_DiskDrive".to_string();
        let disk_results: Vec<HashMap<String, Variant>> = wmi_con.raw_query(&disk_query).map_err(|e| {
            warn!("Disk WMI query failed: {}", e);
            Error::device_detection("Disk WMI query failed")
        })?;

        // Look for SSD indicators
        for disk in disk_results {
            if let Some(Variant::String(model)) = disk.get("Model") {
                let model_lower = model.to_lowercase();

                // Check for NVMe
                if model_lower.contains("nvme") {
                    debug!("Detected NVMe drive: {}", model);
                    return Ok(DeviceType::NVMe);
                }

                // Check for SSD indicators
                if model_lower.contains("ssd") ||
                   model_lower.contains("solid state") ||
                   model_lower.contains("flash") {
                    debug!("Detected SSD drive: {}", model);
                    return Ok(DeviceType::SSD);
                }
            }

            // Check MediaType if available
            if let Some(Variant::String(media_type)) = disk.get("MediaType") {
                if media_type.to_lowercase().contains("solid state") {
                    debug!("Detected SSD via MediaType: {}", media_type);
                    return Ok(DeviceType::SSD);
                }
            }
        }

        // Default to HDD if no SSD indicators found
        debug!("No SSD indicators found, assuming HDD");
        Ok(DeviceType::HDD)
    }

    /// Fallback detection using Windows Registry
    async fn detect_windows_storage_from_registry(&self, _drive: &str) -> Result<DeviceType> {
        use windows::Win32::System::Registry::{
            RegOpenKeyExW, RegCloseKey, HKEY_LOCAL_MACHINE, KEY_READ, HKEY
        };
        use windows::Win32::Foundation::ERROR_SUCCESS;
        use windows::core::HSTRING;

        // Try to read from registry for storage device information
        // This is a simplified approach - in practice, you'd need to map drive letters to physical devices

        let registry_path = HSTRING::from("SYSTEM\\CurrentControlSet\\Services\\disk\\Enum");
        let mut key_handle = HKEY::default();

        let result = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                &registry_path,
                0,
                KEY_READ,
                &mut key_handle,
            )
        };

        if result == ERROR_SUCCESS {
            unsafe { RegCloseKey(key_handle) };
            // For now, return SSD as a reasonable default for modern systems
            debug!("Registry access successful, defaulting to SSD");
            Ok(DeviceType::SSD)
        } else {
            warn!("Registry access failed, defaulting to Unknown");
            Ok(DeviceType::Unknown)
        }
    }
}
