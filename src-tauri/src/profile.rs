//! Automatic settings per PC. A small table of known GPU families decides the tier;
//! unknown GPUs fall back to video memory.

use crate::gpu::{best_gpu, SystemInfo};
use crate::settings::{EmuSettings, PerfSettings};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AutoProfile {
    /// "ultra" | "high" | "balanced" | "low" | "unsupported"
    pub tier: String,
    pub label: String,
    pub resolution: String,
    pub present_mode: String,
    pub gpu_index: i32,
    pub gpu_name: String,
    pub relaxed_shader_float: bool,
    pub ram_limit_gb: u32,
    pub amd_cpu_compat: bool,
    pub reasons: Vec<String>,
}

/// Rough performance class of known GPUs: 4 ultra, 3 high, 2 balanced, 1 low.
/// Checked in order; first match wins, so put specific names before generic ones.
const KNOWN: &[(&str, u8)] = &[
    // NVIDIA
    ("rtx 5090", 4), ("rtx 5080", 4), ("rtx 5070", 4), ("rtx 5060", 3), ("rtx 5050", 3),
    ("rtx 4090", 4), ("rtx 4080", 4), ("rtx 4070", 4), ("rtx 4060", 3), ("rtx 4050", 3),
    ("rtx 3090", 4), ("rtx 3080", 4), ("rtx 3070", 3), ("rtx 3060", 3), ("rtx 3050", 2),
    ("rtx 2080", 3), ("rtx 2070", 3), ("rtx 2060", 2),
    ("gtx 1660", 2), ("gtx 1650", 1), ("gtx 1080", 2), ("gtx 1070", 2), ("gtx 1060", 1), ("gtx 1050", 1),
    // AMD
    ("rx 9070", 4), ("rx 9060", 3),
    ("rx 7900", 4), ("rx 7800", 4), ("rx 7700", 3), ("rx 7600", 3),
    ("rx 6950", 4), ("rx 6900", 4), ("rx 6800", 4), ("rx 6750", 3), ("rx 6700", 3),
    ("rx 6650", 3), ("rx 6600", 3), ("rx 6500", 1), ("rx 6400", 1),
    ("rx 5700", 2), ("rx 5600", 2), ("rx 5500", 1),
    ("radeon 890m", 2), ("radeon 880m", 2), ("radeon 780m", 2), ("radeon 760m", 1), ("radeon 680m", 1),
    // Intel
    ("arc b580", 3), ("arc b570", 3), ("arc a770", 3), ("arc a750", 3), ("arc a580", 2), ("arc a380", 1),
    ("arc 140v", 2), ("arc 130v", 1),
];

pub fn known_class(name: &str) -> Option<u8> {
    let n = name.to_ascii_lowercase();
    KNOWN.iter().find(|(pat, _)| n.contains(pat)).map(|(_, c)| *c)
}

fn class_from_vram(kind: &str, vram_mb: u64) -> u8 {
    if kind != "discrete" {
        return if vram_mb >= 4096 { 2 } else { 1 };
    }
    match vram_mb {
        v if v >= 12 * 1024 => 4,
        v if v >= 6 * 1024 => 3,
        v if v >= 4 * 1024 => 2,
        _ => 1,
    }
}

/// Suggested RAM cap: leave ~4 GB for Windows and the launcher, stay within 8..16 GB.
pub fn suggested_ram_gb(total_mb: u64) -> u32 {
    let total_gb = (total_mb / 1024) as u32;
    total_gb.saturating_sub(4).clamp(4, 16).min(total_gb.max(4))
}

pub fn compute(sys: &SystemInfo) -> AutoProfile {
    let mut reasons = Vec::new();
    let ram = suggested_ram_gb(sys.ram_mb);
    let intel_cpu = sys.cpu_vendor.contains("Intel") || sys.cpu.to_ascii_lowercase().contains("intel");
    let base = |tier: &str, label: &str, res: &str, pm: &str, relaxed: bool| AutoProfile {
        tier: tier.into(),
        label: label.into(),
        resolution: res.into(),
        present_mode: pm.into(),
        gpu_index: -1,
        gpu_name: String::new(),
        relaxed_shader_float: relaxed,
        ram_limit_gb: ram,
        amd_cpu_compat: false,
        reasons: Vec::new(),
    };

    let Some(gpu) = best_gpu(&sys.gpus) else {
        let mut p = base("unsupported", "No Vulkan 1.3 GPU", "1280x720", "Mailbox", true);
        p.reasons.push(match &sys.vulkan_error {
            Some(e) => format!("Vulkan is not available: {e}. Update your graphics driver."),
            None => "No GPU with Vulkan 1.3 was found. Update your graphics driver.".into(),
        });
        return p;
    };

    let class = match known_class(&gpu.name) {
        Some(c) => {
            reasons.push(format!("{} is a known GPU (class {c}).", gpu.name));
            c
        }
        None => {
            let c = class_from_vram(&gpu.kind, gpu.vram_mb);
            reasons.push(format!("{} is not in the table; using {} MB of video memory (class {c}).", gpu.name, gpu.vram_mb));
            c
        }
    };

    let mut p = match class {
        4 => base("ultra", "Ultra · 1440p", "2560x1440", "Mailbox", false),
        3 => base("high", "High · 1080p", "1920x1080", "Mailbox", false),
        2 => base("balanced", "Balanced · 720p", "1280x720", "Mailbox", false),
        _ => base("low", "Low · 720p", "1280x720", "Immediate", true),
    };
    if p.tier == "low" {
        reasons.push("Low tier: relaxed shader math and uncapped presentation for extra speed.".into());
    }
    // Only pin a GPU when there is more than one; otherwise let the engine choose.
    if sys.gpus.len() > 1 {
        p.gpu_index = gpu.index;
        reasons.push(format!("Several GPUs found; pinned to #{} ({}).", gpu.index, gpu.name));
    }
    p.gpu_name = gpu.name.clone();
    if intel_cpu {
        p.amd_cpu_compat = true;
        reasons.push("Intel CPU detected: CPU compatibility patches enabled.".into());
    }
    reasons.push(format!("RAM cap suggestion: {} GB of {} GB.", p.ram_limit_gb, sys.ram_mb / 1024));
    p.reasons = reasons;
    p
}

pub fn apply(p: &AutoProfile, emu: &mut EmuSettings, perf: &mut PerfSettings) {
    emu.resolution = p.resolution.clone();
    emu.present_mode = p.present_mode.clone();
    emu.gpu_index = p.gpu_index;
    emu.relaxed_shader_float = p.relaxed_shader_float;
    emu.amd_cpu_compat = p.amd_cpu_compat;
    emu.shader_cache = true;
    // Expensive accuracy options start off; users turn them on per game when needed.
    emu.readback_linear = false;
    emu.readback_tiled = false;
    emu.readback_storage = false;
    emu.sync_raw_image_buffers = false;
    emu.occlusion_queries = false;
    perf.ram_limit_gb = p.ram_limit_gb;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuInfo;

    fn sys(gpus: Vec<(&str, &str, u64)>, cpu: &str, ram_gb: u64) -> SystemInfo {
        SystemInfo {
            os: "Windows 11".into(),
            cpu: cpu.into(),
            cpu_vendor: if cpu.contains("Intel") { "GenuineIntel".into() } else { "AuthenticAMD".into() },
            cores: 6,
            threads: 12,
            ram_mb: ram_gb * 1024,
            gpus: gpus
                .into_iter()
                .enumerate()
                .map(|(i, (n, k, v))| GpuInfo {
                    index: i as i32,
                    name: n.into(),
                    kind: k.into(),
                    vram_mb: v,
                    vulkan: "1.4.303".into(),
                    vulkan_ok: true,
                    vendor: "NVIDIA".into(),
                    driver: String::new(),
                })
                .collect(),
            vulkan_error: None,
        }
    }

    #[test]
    fn rtx_4050_laptop_gets_1080p() {
        let p = compute(&sys(
            vec![("Intel(R) UHD Graphics", "integrated", 1024), ("NVIDIA GeForce RTX 4050 Laptop GPU", "discrete", 6141)],
            "13th Gen Intel(R) Core(TM) i5-13420H",
            16,
        ));
        assert_eq!(p.tier, "high");
        assert_eq!(p.resolution, "1920x1080");
        assert_eq!(p.gpu_index, 1, "pinned to the RTX, not the iGPU");
        assert!(p.amd_cpu_compat);
    }

    #[test]
    fn rtx_3050_gets_720p_even_with_8gb() {
        let p = compute(&sys(vec![("NVIDIA GeForce RTX 3050", "discrete", 8192)], "AMD Ryzen 5 5600", 16));
        assert_eq!(p.tier, "balanced");
        assert_eq!(p.resolution, "1280x720");
        assert_eq!(p.gpu_index, -1);
        assert!(!p.amd_cpu_compat);
    }

    #[test]
    fn unknown_gpu_uses_vram() {
        let p = compute(&sys(vec![("Mystery GPU 9000", "discrete", 16384)], "AMD", 32));
        assert_eq!(p.tier, "ultra");
        let p = compute(&sys(vec![("Some iGPU", "integrated", 512)], "AMD", 8));
        assert_eq!(p.tier, "low");
        assert!(p.relaxed_shader_float);
    }

    #[test]
    fn no_vulkan_is_unsupported() {
        let mut s = sys(vec![], "AMD", 16);
        s.vulkan_error = Some("loader missing".into());
        assert_eq!(compute(&s).tier, "unsupported");
    }

    #[test]
    fn ram_suggestion() {
        assert_eq!(suggested_ram_gb(8 * 1024), 4);
        assert_eq!(suggested_ram_gb(16 * 1024), 12);
        assert_eq!(suggested_ram_gb(64 * 1024), 16);
    }
}
