//! Загрузка и температура GPU (NVML), загрузка CPU (sysinfo). Если данных нет — поля None, не выдумываем.
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::Nvml;
use serde::Serialize;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Telemetry {
    pub cpu_percent: Option<f32>,
    pub gpu_percent: Option<u32>,
    pub gpu_temp_c: Option<u32>,
    pub gpu_clock_mhz: Option<u32>,
    pub vram_used_mb: Option<u64>,
    pub vram_total_mb: Option<u64>,
    pub gpu_name: Option<String>,
    pub gpu_note: Option<String>,
}

pub struct Sampler { sys: System, nvml: Option<Nvml>, nvml_err: Option<String>, name: Option<String> }

impl Sampler {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()));
        let (nvml, nvml_err) = match Nvml::init() { Ok(n) => (Some(n), None), Err(e) => (None, Some(format!("NVML недоступен: {e}"))) };
        let name = nvml.as_ref().and_then(|n| n.device_by_index(0).ok()).and_then(|d| d.name().ok());
        Sampler { sys, nvml, nvml_err, name }
    }
    pub fn sample(&mut self) -> Telemetry {
        self.sys.refresh_cpu_usage();
        let mut t = Telemetry { cpu_percent: Some(self.sys.global_cpu_usage()), gpu_name: self.name.clone(), gpu_note: self.nvml_err.clone(), ..Default::default() };
        if let Some(dev) = self.nvml.as_ref().and_then(|n| n.device_by_index(0).ok()) {
            t.gpu_percent = dev.utilization_rates().ok().map(|u| u.gpu);
            t.gpu_temp_c = dev.temperature(TemperatureSensor::Gpu).ok();
            t.gpu_clock_mhz = dev.clock_info(Clock::Graphics).ok();
            if let Ok(m) = dev.memory_info() { t.vram_used_mb = Some(m.used / 1048576); t.vram_total_mb = Some(m.total / 1048576); }
        }
        t
    }
}
