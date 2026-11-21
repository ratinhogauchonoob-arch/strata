//! Developer self-tests: quick checks that explain why a game might not start.

use crate::{engine, games, gpu, profile, settings::Settings};
use serde::Serialize;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: String, // pass | warn | fail
    pub detail: String,
    pub ms: u128,
}

fn run(out: &mut Vec<Check>, name: &str, f: impl FnOnce() -> (&'static str, String)) {
    let t = Instant::now();
    let (status, detail) = f();
    out.push(Check { name: name.into(), status: status.into(), detail, ms: t.elapsed().as_millis() });
}

pub fn run_all(s: &Settings, data: &Path) -> Vec<Check> {
    let mut out = Vec::new();

    run(&mut out, "Data folder", || {
        let probe = data.join(".selftest");
        match std::fs::write(&probe, b"ok") {
            Ok(_) => {
                let _ = std::fs::remove_file(probe);
                ("pass", format!("{} is writable", data.display()))
            }
            Err(e) => ("fail", format!("{} is not writable: {e}", data.display())),
        }
    });

    run(&mut out, "Free disk space", || {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let best = disks
            .list()
            .iter()
            .filter(|d| data.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len());
        match best {
            Some(d) => {
                let gb = d.available_space() / 1_073_741_824;
                if gb < 5 {
                    ("warn", format!("Only {gb} GB free on {} — shader cache and logs need space", d.mount_point().display()))
                } else {
                    ("pass", format!("{gb} GB free on {}", d.mount_point().display()))
                }
            }
            None => ("warn", "Could not read disk information".into()),
        }
    });

    let installed: Vec<&str> = crate::engines::ENGINES
        .iter()
        .filter(|d| crate::engines::find(d, &s.engine_path(d.id), data).is_some())
        .map(|d| d.name)
        .collect();
    run(&mut out, "Engines installed", || {
        if installed.is_empty() {
            ("fail", "No engine yet. Settings → Engines → Download.".into())
        } else {
            ("pass", installed.join(", "))
        }
    });
    let has_ps4 = installed.contains(&"shadPS4") || installed.contains(&"SharpEmu");
    run(&mut out, "PS4 support", || {
        if has_ps4 {
            ("pass", "a PS4 engine is installed".into())
        } else {
            ("warn", "no PS4 engine (only needed for PS4 games)".into())
        }
    });

    let engine_path = crate::engines::find(crate::engines::def("kyty").unwrap(), &s.engine_path("kyty"), data);
    run(&mut out, "KytyPS5 responds", || match &engine_path {
        Some(p) => match engine::engine_version(p) {
            Ok(v) => ("pass", v),
            Err(e) => ("fail", e),
        },
        None => ("warn", "skipped (KytyPS5 not installed)".into()),
    });

    let sys = gpu::system_info();
    run(&mut out, "Vulkan loader", || match &sys.vulkan_error {
        None => ("pass", format!("{} GPU(s) visible", sys.gpus.len())),
        Some(e) => ("fail", format!("{e}. Install or update the GPU driver.")),
    });

    run(&mut out, "Vulkan 1.3 GPU", || match gpu::best_gpu(&sys.gpus) {
        Some(g) => ("pass", format!("#{} {} · Vulkan {} · {} MB", g.index, g.name, g.vulkan, g.vram_mb)),
        None if sys.gpus.is_empty() => ("fail", "No GPU found".into()),
        None => ("fail", format!("{} found, but none supports Vulkan 1.3. Update the driver.", sys.gpus[0].name)),
    });

    run(&mut out, "Selected GPU", || {
        let idx = s.emu.gpu_index;
        if idx < 0 {
            ("pass", "Automatic".into())
        } else if sys.gpus.iter().any(|g| g.index == idx) {
            ("pass", format!("#{idx}"))
        } else {
            ("warn", format!("GPU #{idx} does not exist on this PC; the engine will pick one"))
        }
    });

    run(&mut out, "Auto profile", || {
        let p = profile::compute(&sys);
        let st = if p.tier == "unsupported" { "fail" } else { "pass" };
        (st, format!("{} ({})", p.label, p.gpu_name))
    });

    run(&mut out, "Memory", || {
        let total = sys.ram_mb / 1024;
        if s.perf.ram_limit_enabled && s.perf.ram_limit_gb > total as u32 {
            ("warn", format!("RAM limit {} GB is above installed {} GB", s.perf.ram_limit_gb, total))
        } else if s.perf.ram_limit_enabled && s.perf.ram_limit_gb < 8 {
            ("warn", format!("RAM limit {} GB is low; many games need 8 GB or more", s.perf.ram_limit_gb))
        } else if total < 12 {
            ("warn", format!("{total} GB installed; 16 GB is recommended"))
        } else {
            ("pass", format!("{total} GB installed"))
        }
    });

    run(&mut out, "Library folders", || {
        if s.library_folders.is_empty() {
            return ("warn", "No folders yet".into());
        }
        let missing: Vec<_> = s.library_folders.iter().filter(|f| !Path::new(f).is_dir()).cloned().collect();
        if missing.is_empty() {
            ("pass", format!("{} folder(s) reachable", s.library_folders.len()))
        } else {
            ("warn", format!("Not reachable: {}", missing.join(", ")))
        }
    });

    run(&mut out, "Games", || {
        let list = games::scan(&s.library_folders, false);
        let no_meta = list.iter().filter(|g| !g.archive && g.title_id.is_empty()).count();
        if list.is_empty() {
            ("warn", "No games found (folders need eboot.bin, or .zar files)".into())
        } else if no_meta > 0 {
            ("warn", format!("{} found, {no_meta} without sce_sys/param.json", list.len()))
        } else {
            ("pass", format!("{} found", list.len()))
        }
    });

    run(&mut out, "Command line", || {
        let a = engine::build_args(&s.emu, &s.dev, "test/eboot.bin", Path::new("logs"));
        if a.iter().any(|x| x.is_empty()) {
            ("fail", "An empty argument would be passed to the engine".into())
        } else {
            ("pass", format!("{} arguments", a.len()))
        }
    });

    out
}
