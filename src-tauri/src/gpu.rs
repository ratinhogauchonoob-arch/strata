//! Hardware detection. GPUs come from Vulkan, in the same order the engine uses for
//! its GPU index, so "gpu_index" here is exactly what the engine expects.

use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct GpuInfo {
    pub index: i32,
    pub name: String,
    /// "discrete" | "integrated" | "virtual" | "cpu" | "other"
    pub kind: String,
    pub vram_mb: u64,
    pub vulkan: String,
    pub vulkan_ok: bool, // Vulkan 1.3 or newer
    pub vendor: String,
    pub driver: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub cpu: String,
    pub cpu_vendor: String,
    pub cores: usize,
    pub threads: usize,
    pub ram_mb: u64,
    pub gpus: Vec<GpuInfo>,
    pub vulkan_error: Option<String>,
}

fn vendor_name(id: u32) -> &'static str {
    match id {
        0x10DE => "NVIDIA",
        0x1002 => "AMD",
        0x8086 => "Intel",
        0x13B5 => "ARM",
        0x5143 => "Qualcomm",
        0x106B => "Apple",
        _ => "Unknown",
    }
}

fn nvidia_driver(raw: u32) -> String {
    format!("{}.{:02}", (raw >> 22) & 0x3ff, (raw >> 14) & 0xff)
}

pub fn detect_gpus() -> Result<Vec<GpuInfo>, String> {
    use ash::vk;
    // SAFETY: loads the system Vulkan loader (vulkan-1.dll / libvulkan.so.1) at runtime.
    let entry = unsafe { ash::Entry::load() }.map_err(|e| format!("Vulkan loader not found: {e}"))?;
    let app_info = vk::ApplicationInfo::default().api_version(vk::make_api_version(0, 1, 3, 0));
    let create = vk::InstanceCreateInfo::default().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&create, None) }.map_err(|e| format!("vkCreateInstance failed: {e}"))?;
    let mut out = Vec::new();
    let devices = unsafe { instance.enumerate_physical_devices() }.unwrap_or_default();
    for (i, dev) in devices.iter().enumerate() {
        let props = unsafe { instance.get_physical_device_properties(*dev) };
        let mem = unsafe { instance.get_physical_device_memory_properties(*dev) };
        let name = props
            .device_name_as_c_str()
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "Unknown GPU".into());
        let vram: u64 = mem.memory_heaps[..mem.memory_heap_count as usize]
            .iter()
            .filter(|h| h.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
            .map(|h| h.size)
            .max()
            .unwrap_or(0);
        let kind = match props.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => "discrete",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual",
            vk::PhysicalDeviceType::CPU => "cpu",
            _ => "other",
        };
        let api = props.api_version;
        let (maj, min, pat) = (vk::api_version_major(api), vk::api_version_minor(api), vk::api_version_patch(api));
        let driver = if props.vendor_id == 0x10DE {
            nvidia_driver(props.driver_version)
        } else {
            props.driver_version.to_string()
        };
        out.push(GpuInfo {
            index: i as i32,
            name,
            kind: kind.into(),
            vram_mb: vram / (1024 * 1024),
            vulkan: format!("{maj}.{min}.{pat}"),
            vulkan_ok: maj > 1 || (maj == 1 && min >= 3),
            vendor: vendor_name(props.vendor_id).into(),
            driver,
        });
    }
    unsafe { instance.destroy_instance(None) };
    Ok(out)
}

pub fn system_info() -> SystemInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let cpus = sys.cpus();
    let cpu = cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let cpu_vendor = cpus.first().map(|c| c.vendor_id().to_string()).unwrap_or_default();
    let (gpus, vulkan_error) = match detect_gpus() {
        Ok(g) => (g, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    SystemInfo {
        os: format!(
            "{} {}",
            sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.into()),
            sysinfo::System::os_version().unwrap_or_default()
        ),
        cpu,
        cpu_vendor,
        cores: sysinfo::System::physical_core_count().unwrap_or(cpus.len()),
        threads: cpus.len(),
        ram_mb: sys.total_memory() / (1024 * 1024),
        gpus,
        vulkan_error,
    }
}

/// The GPU the engine would most likely pick: best discrete, then best of the rest.
pub fn best_gpu(gpus: &[GpuInfo]) -> Option<&GpuInfo> {
    gpus.iter().filter(|g| g.vulkan_ok).max_by_key(|g| {
        let k = match g.kind.as_str() {
            "discrete" => 3,
            "integrated" => 2,
            "virtual" => 1,
            _ => 0,
        };
        (k, g.vram_mb)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(i: i32, name: &str, kind: &str, vram: u64) -> GpuInfo {
        GpuInfo {
            index: i,
            name: name.into(),
            kind: kind.into(),
            vram_mb: vram,
            vulkan: "1.3.0".into(),
            vulkan_ok: true,
            vendor: "x".into(),
            driver: String::new(),
        }
    }

    #[test]
    fn prefers_discrete() {
        let list = vec![g(0, "Intel UHD 630", "integrated", 2048), g(1, "RTX 4050", "discrete", 6144)];
        assert_eq!(best_gpu(&list).unwrap().index, 1);
    }

    #[test]
    fn nvidia_driver_format() {
        // 566.36 encoded the NVIDIA way.
        let raw = (566u32 << 22) | (36u32 << 14);
        assert_eq!(nvidia_driver(raw), "566.36");
    }

    #[test]
    fn detect_does_not_panic() {
        // CI machines usually have no Vulkan driver; it must fail gracefully.
        let _ = detect_gpus();
        let s = system_info();
        assert!(s.ram_mb > 0);
    }
}
