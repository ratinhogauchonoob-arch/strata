// Mirrors EmuSettings::default() in src-tauri/src/settings.rs (used by "Reset this page").
export const EMU_DEFAULTS = {
  resolution: "1280x720", present_mode: "Mailbox", gpu_index: -1, fullscreen: false, stretch: false, hdr: false,
  hide_cursor: true, vblank_frequency: 60, shader_cache: true, relaxed_shader_float: false, readback_linear: false,
  readback_tiled: false, readback_storage: false, sync_raw_image_buffers: false, flush_draws: 0, tessellation: false,
  occlusion_queries: false, native_rt: false, pro_mode: false, upscaler: "Linear", dlss_folder: "", fsr_folder: "",
  perf_overlay: false,
};
