//! Reproducible release benchmark for scene preparation and window presentation.

use std::collections::BTreeMap;
use std::f32::consts::TAU;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

use crate::backend::wgpu::backend::WgpuRenderSystem;
use crate::math::{Quat, Vec3};
use crate::platform::{
    PlatformAdapter, PlatformError, PlatformEvent, WindowDescriptor, WindowSize,
};
use crate::render::system::{RenderError, RenderSystem};
use crate::resources::library::BuiltinMesh;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::{Mesh, SubMesh, Vertex};
use crate::scene::api::SceneApi;
use crate::scene::error::SceneError;
use crate::scene::manager::SceneManager;
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

const WINDOW_TITLE: &str = "BFGE stress benchmark";
const MEMORY_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const KIB: u64 = 1024;
const PROC_STATUS: &str = "/proc/self/status";
const PROC_CPUINFO: &str = "/proc/cpuinfo";
const RSS_FIELD: &str = "VmRSS:";
const PEAK_RSS_FIELD: &str = "VmHWM:";
const CPU_MODEL_FIELD: &str = "model name";
const CUBE_GRID_SPACING: f32 = 2.0;
const CUBE_GRID_CAMERA_HORIZONTAL: f32 = 2.8;
const CUBE_GRID_CAMERA_VERTICAL: f32 = 2.0;
const CUBE_GRID_ROTATION_RADIANS_PER_SECOND: f32 = 0.35;
const FRAME_WINDOW_INTERVAL: Duration = Duration::from_secs(1);

/// Failure to configure, run, or save a stress benchmark.
#[derive(Debug, thiserror::Error)]
pub enum StressError {
    /// Benchmark must be compiled with optimizations.
    #[error("stress benchmark must be built with --release")]
    DebugBuild,
    /// The input configuration cannot be read.
    #[error("failed to read stress configuration {path}: {source}")]
    ReadConfig {
        /// Input path.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The input configuration is malformed.
    #[error("failed to parse or serialize stress JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A configuration value is invalid.
    #[error("invalid stress configuration: {0}")]
    InvalidConfig(String),
    /// Current time cannot be represented as a Unix timestamp.
    #[error("failed to read benchmark start time: {0}")]
    Clock(#[from] SystemTimeError),
    /// The platform event loop could not start.
    #[error("failed to create window event loop: {0}")]
    EventLoop(#[from] EventLoopError),
    /// The window platform failed.
    #[error("benchmark window failed: {0}")]
    Platform(#[from] PlatformError),
    /// GPU rendering failed.
    #[error("benchmark rendering failed: {0}")]
    Render(#[from] RenderError),
    /// Scene construction failed.
    #[error("benchmark scene failed: {0}")]
    Scene(#[from] SceneError),
    /// Process memory metrics could not be read.
    #[error("failed to read process memory: {0}")]
    Memory(#[from] io::Error),
    /// A window or rendered frame did not match the configured scenario.
    #[error("benchmark window or frame is invalid: {0}")]
    Window(String),
    /// The result file cannot be written.
    #[error("failed to write stress report {path}: {source}")]
    WriteReport {
        /// Output path.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
}

/// Available benchmark execution modes.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkMode {
    /// Measures scene frame construction without a window or GPU.
    SceneOnly,
    /// Measures scene frame construction plus GPU render and presentation.
    Window,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SyncMode {
    Vsync,
    NoVsync,
}

impl SyncMode {
    fn present_mode(self) -> wgpu::PresentMode {
        match self {
            Self::Vsync => wgpu::PresentMode::Fifo,
            Self::NoVsync => wgpu::PresentMode::Immediate,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SceneKind {
    #[default]
    Synthetic,
    CubeGrid,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    width: u32,
    height: u32,
    sync_mode: SyncMode,
    warmup_seconds: u64,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    name: String,
    #[serde(default)]
    scene_kind: SceneKind,
    objects: usize,
    nodes: usize,
    depth: usize,
    triangles_per_mesh: usize,
    meshes: usize,
    duration_seconds: u64,
}

#[derive(Debug, Serialize)]
struct Summary {
    median_ns: u64,
    p95_ns: u64,
    p99_ns: u64,
    max_ns: u64,
}

#[derive(Debug, Serialize)]
struct MemorySample {
    elapsed_seconds: f64,
    rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct FrameWindow {
    elapsed_seconds: f64,
    duration_seconds: f64,
    frames: usize,
    fps: f64,
    p95_full_pass_ns: u64,
}

#[derive(Debug, Serialize)]
struct ScenarioResult<'a> {
    scenario: &'a Scenario,
    measured_seconds: f64,
    measured_frames: usize,
    fps: Option<f64>,
    frame_preparation_ns: Summary,
    full_pass_ns: Option<Summary>,
    frame_preparation_histogram_ns: BTreeMap<u64, usize>,
    full_pass_histogram_ns: Option<BTreeMap<u64, usize>>,
    frame_windows: Option<Vec<FrameWindow>>,
    memory: Vec<MemorySample>,
}

#[derive(Debug, Serialize)]
struct Hardware {
    cpu_model: Option<String>,
    gpu: Option<String>,
    gpu_backend: Option<String>,
    gpu_driver: Option<String>,
    gpu_driver_info: Option<String>,
    gpu_vendor: Option<u32>,
    gpu_device: Option<u32>,
    operating_system: String,
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    started_unix_seconds: u64,
    package_version: &'static str,
    build_profile: &'static str,
    mode: BenchmarkMode,
    config: &'a Config,
    hardware: Hardware,
    memory_metric: &'static str,
    frame_preparation_definition: &'static str,
    full_pass_definition: &'static str,
    results: Vec<ScenarioResult<'a>>,
}

#[derive(Default)]
struct Samples {
    counts: BTreeMap<u64, usize>,
    total: usize,
}

impl Samples {
    fn record(&mut self, nanos: u64) {
        *self.counts.entry(nanos).or_default() += 1;
        self.total += 1;
    }

    fn percentile(&self, percentile: usize) -> u64 {
        if self.total == 0 {
            return 0;
        }
        let rank = (self.total * percentile).div_ceil(100).saturating_sub(1);
        let mut cumulative = 0;
        for (nanos, count) in &self.counts {
            cumulative += count;
            if cumulative > rank {
                return *nanos;
            }
        }
        0
    }

    fn summary(&self) -> Summary {
        Summary {
            median_ns: self.percentile(50),
            p95_ns: self.percentile(95),
            p99_ns: self.percentile(99),
            max_ns: self.counts.last_key_value().map_or(0, |(nanos, _)| *nanos),
        }
    }
}

/// Runs the configured benchmark and writes its measurements as JSON.
///
/// `SceneOnly` runs without a graphics session and omits FPS and full pass
/// measurements. `Window` requires a working display and presentation device.
/// Invalid settings, scene or graphics failures, and I/O failures are returned.
pub fn run(mode: BenchmarkMode, config_path: &Path, output_path: &Path) -> Result<(), StressError> {
    if cfg!(debug_assertions) {
        return Err(StressError::DebugBuild);
    }
    let started_unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let input = fs::read(config_path).map_err(|source| StressError::ReadConfig {
        path: config_path.to_owned(),
        source,
    })?;
    let config: Config = serde_json::from_slice(&input)?;
    validate(&config)?;
    let mut event_loop = if matches!(mode, BenchmarkMode::Window) {
        Some(EventLoop::new()?)
    } else {
        None
    };
    let mut results = Vec::with_capacity(config.scenarios.len());
    let mut adapter_info: Option<wgpu::AdapterInfo> = None;

    for scenario in &config.scenarios {
        println!(
            "Benchmark scenario {}: {} objects",
            scenario.name, scenario.objects
        );
        let (mut scene, resources) = build_scene(scenario)?;
        let (result, adapter) = match mode {
            BenchmarkMode::SceneOnly => (measure_scene_only(&scene, &config, scenario)?, None),
            BenchmarkMode::Window => {
                let loop_ref = event_loop
                    .as_mut()
                    .ok_or_else(|| StressError::Window("missing window event loop".to_owned()))?;
                let (result, adapter) =
                    measure_window(&mut scene, &resources, &config, scenario, loop_ref)?;
                (result, Some(adapter))
            }
        };
        if let Some(adapter) = adapter {
            adapter_info = Some(adapter);
        }
        if let (Some(fps), Some(summary)) = (result.fps, result.full_pass_ns.as_ref()) {
            println!(
                "{} objects: {fps:.1} FPS, p95 {:.2} ms",
                scenario.objects,
                summary.p95_ns as f64 / 1_000_000.0
            );
        }
        results.push(result);
    }

    let report = Report {
        started_unix_seconds,
        package_version: env!("CARGO_PKG_VERSION"),
        build_profile: "release",
        mode,
        config: &config,
        hardware: Hardware {
            cpu_model: cpu_model(),
            gpu: adapter_info.as_ref().map(|adapter| adapter.name.clone()),
            gpu_backend: adapter_info
                .as_ref()
                .map(|adapter| format!("{:?}", adapter.backend)),
            gpu_driver: adapter_info.as_ref().map(|adapter| adapter.driver.clone()),
            gpu_driver_info: adapter_info
                .as_ref()
                .map(|adapter| adapter.driver_info.clone()),
            gpu_vendor: adapter_info.as_ref().map(|adapter| adapter.vendor),
            gpu_device: adapter_info.as_ref().map(|adapter| adapter.device),
            operating_system: std::env::consts::OS.to_owned(),
        },
        memory_metric: "Linux /proc/self/status VmRSS and VmHWM; process RAM only, not GPU VRAM",
        frame_preparation_definition: "SceneManager::build_render_frame wall time",
        full_pass_definition: "nonblocking window event pump + optional scene animation + build_render_frame + render + present wall time; GPU completion is not awaited",
        results,
    };
    fs::write(output_path, serde_json::to_vec_pretty(&report)?).map_err(|source| {
        StressError::WriteReport {
            path: output_path.to_owned(),
            source,
        }
    })?;
    Ok(())
}

fn validate(config: &Config) -> Result<(), StressError> {
    if config.width == 0 || config.height == 0 || config.scenarios.is_empty() {
        return Err(StressError::InvalidConfig(
            "window dimensions and scenario list must be nonzero".to_owned(),
        ));
    }
    for scenario in &config.scenarios {
        if scenario.name.is_empty()
            || scenario.objects == 0
            || scenario.nodes == 0
            || scenario.depth >= scenario.nodes
            || (scenario.depth == 0 && scenario.nodes != 1)
            || scenario.triangles_per_mesh == 0
            || scenario.meshes == 0
            || scenario.meshes > scenario.objects
            || scenario.duration_seconds == 0
            || scenario
                .triangles_per_mesh
                .checked_mul(3)
                .is_none_or(|n| n > u32::MAX as usize)
            || scenario.nodes > u32::MAX as usize
            || scenario.objects > u32::MAX as usize
            || scenario
                .objects
                .checked_mul(scenario.triangles_per_mesh)
                .is_none_or(|n| n > u32::MAX as usize)
            || (scenario.scene_kind == SceneKind::CubeGrid
                && (scenario.objects.checked_add(1) != Some(scenario.nodes)
                    || scenario.depth != 1
                    || scenario.meshes != 1
                    || scenario.triangles_per_mesh != 12))
        {
            return Err(StressError::InvalidConfig(format!(
                "invalid scenario: {}",
                scenario.name
            )));
        }
    }
    Ok(())
}

fn build_scene(scenario: &Scenario) -> Result<(SceneManager, ResourceManager), StressError> {
    match scenario.scene_kind {
        SceneKind::Synthetic => build_synthetic_scene(scenario),
        SceneKind::CubeGrid => build_cube_grid(scenario),
    }
}

fn build_cube_grid(scenario: &Scenario) -> Result<(SceneManager, ResourceManager), StressError> {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let cube = resources.builtin(BuiltinMesh::Cube);
    let side = (scenario.objects as f64).cbrt().ceil() as usize;
    let half_extent = (side as f32 - 1.0) * CUBE_GRID_SPACING * 0.5;
    let camera_position = Vec3::new(
        side as f32 * CUBE_GRID_CAMERA_HORIZONTAL,
        side as f32 * CUBE_GRID_CAMERA_VERTICAL,
        side as f32 * CUBE_GRID_CAMERA_HORIZONTAL,
    );
    scene.set_viewpoint(ViewpointState {
        position: camera_position,
        direction: -camera_position.normalize(),
        fov_y: 60.0_f32.to_radians(),
    })?;
    let root = scene.root_node();
    for index in 0..scenario.objects {
        let column = index % side;
        let row = (index / side) % side;
        let layer = index / (side * side);
        let node = scene.create_node(root)?;
        scene.set_node_transform(
            node,
            Transform::from_translation(Vec3::new(
                column as f32 * CUBE_GRID_SPACING - half_extent,
                row as f32 * CUBE_GRID_SPACING - half_extent,
                layer as f32 * CUBE_GRID_SPACING - half_extent,
            )),
        )?;
        scene.create_object(cube, node)?;
    }
    Ok((scene, resources))
}

fn animate_cube_grid(scene: &mut SceneManager, elapsed_seconds: f32) -> Result<(), StressError> {
    let root = scene.root_node();
    scene.set_node_transform(
        root,
        Transform::from_rotation(Quat::from_rotation_y(
            elapsed_seconds * CUBE_GRID_ROTATION_RADIANS_PER_SECOND,
        )),
    )?;
    Ok(())
}

fn build_synthetic_scene(
    scenario: &Scenario,
) -> Result<(SceneManager, ResourceManager), StressError> {
    let mut scene = SceneManager::new();
    let mut resources = ResourceManager::new();
    let mut nodes = Vec::with_capacity(scenario.nodes);
    nodes.push(scene.root_node());
    for index in 1..scenario.nodes {
        let parent_index = if index <= scenario.depth {
            index - 1
        } else {
            (index - scenario.depth - 1) % scenario.depth
        };
        let node = scene.create_node(nodes[parent_index])?;
        let column = (index % 16) as f32;
        let row = ((index / 16) % 16) as f32;
        scene.set_node_transform(
            node,
            Transform::from_translation(Vec3::new((column - 7.5) * 0.1, (row - 7.5) * 0.1, 0.0)),
        )?;
        nodes.push(node);
    }
    let handles = (0..scenario.meshes)
        .map(|index| resources.add_mesh(make_mesh(scenario.triangles_per_mesh, index)))
        .collect::<Vec<_>>();
    for index in 0..scenario.objects {
        let node = if scenario.nodes == 1 {
            nodes[0]
        } else {
            nodes[1 + index % (scenario.nodes - 1)]
        };
        scene.create_object(handles[index % handles.len()], node)?;
    }
    Ok((scene, resources))
}

fn make_mesh(triangles: usize, mesh_index: usize) -> Mesh {
    let phase = mesh_index as f32 * 0.013;
    let vertices = (0..triangles)
        .flat_map(|index| {
            let angle = TAU * index as f32 / triangles as f32 + phase;
            let next = angle + (TAU / triangles.max(3) as f32).min(1.0);
            [
                Vertex {
                    position: Vec3::ZERO,
                },
                Vertex {
                    position: Vec3::new(angle.cos() * 0.5, angle.sin() * 0.5, 0.0),
                },
                Vertex {
                    position: Vec3::new(next.cos() * 0.5, next.sin() * 0.5, 0.0),
                },
            ]
        })
        .collect::<Vec<_>>();
    let indices = (0..vertices.len() as u32).collect();
    Mesh {
        submeshes: vec![SubMesh { vertices, indices }],
        color: [0.5 + (mesh_index % 5) as f32 * 0.1, 0.7, 0.8, 1.0],
    }
}

fn measure_scene_only<'a>(
    scene: &SceneManager,
    config: &Config,
    scenario: &'a Scenario,
) -> Result<ScenarioResult<'a>, StressError> {
    let size = WindowSize::new(config.width, config.height);
    let warmup_end = Instant::now() + Duration::from_secs(config.warmup_seconds);
    while Instant::now() < warmup_end {
        std::hint::black_box(scene.build_render_frame(size));
    }
    let start = Instant::now();
    let end = start + Duration::from_secs(scenario.duration_seconds);
    let mut samples = Samples::default();
    let mut memory = vec![sample_memory(0.0)?];
    let mut next_memory = start + MEMORY_SAMPLE_INTERVAL;
    while Instant::now() < end {
        let frame_start = Instant::now();
        std::hint::black_box(scene.build_render_frame(size));
        samples.record(frame_start.elapsed().as_nanos() as u64);
        if Instant::now() >= next_memory {
            memory.push(sample_memory(start.elapsed().as_secs_f64())?);
            next_memory += MEMORY_SAMPLE_INTERVAL;
        }
    }
    let measured_seconds = start.elapsed().as_secs_f64();
    memory.push(sample_memory(measured_seconds)?);
    Ok(ScenarioResult {
        scenario,
        measured_seconds,
        measured_frames: samples.total,
        fps: None,
        frame_preparation_ns: samples.summary(),
        full_pass_ns: None,
        frame_preparation_histogram_ns: samples.counts,
        full_pass_histogram_ns: None,
        frame_windows: None,
        memory,
    })
}

fn measure_window<'a>(
    scene: &mut SceneManager,
    resources: &ResourceManager,
    config: &Config,
    scenario: &'a Scenario,
    event_loop: &mut EventLoop<()>,
) -> Result<(ScenarioResult<'a>, wgpu::AdapterInfo), StressError> {
    let size = WindowSize::new(config.width, config.height);
    let title = format!("{WINDOW_TITLE}: {} objects", scenario.objects);
    let platform = PlatformAdapter::new(
        WindowDescriptor::new(&title, config.width, config.height),
        event_loop,
    )?;
    if platform.window().size() != size {
        return Err(StressError::Window(format!(
            "window size differs from requested {size:?}"
        )));
    }
    let mut backend = WgpuRenderSystem::with_present_mode(config.sync_mode.present_mode());
    backend.initialize(platform.window())?;
    let adapter = backend
        .adapter_info()
        .ok_or_else(|| StressError::Window("missing GPU adapter".to_owned()))?;
    let expected_triangles = (scenario.objects * scenario.triangles_per_mesh) as u32;
    let expected_draw_calls = scenario.objects as u32;
    let animation_start = Instant::now();
    let warmup_end = animation_start + Duration::from_secs(config.warmup_seconds);
    while Instant::now() < warmup_end {
        pump_window(&platform, event_loop, size)?;
        if scenario.scene_kind == SceneKind::CubeGrid {
            animate_cube_grid(scene, animation_start.elapsed().as_secs_f32())?;
        }
        let frame = scene.build_render_frame(size);
        let stats = backend.render(&frame, resources)?;
        if stats.draw_calls != expected_draw_calls || stats.triangles != expected_triangles {
            return Err(StressError::Window(format!(
                "incomplete warmup frame: {stats:?}"
            )));
        }
    }
    let start = Instant::now();
    let end = start + Duration::from_secs(scenario.duration_seconds);
    let mut preparation_samples = Samples::default();
    let mut pass_samples = Samples::default();
    let mut window_samples = Samples::default();
    let mut frame_windows = Vec::new();
    let mut window_start = start;
    let mut memory = vec![sample_memory(0.0)?];
    let mut next_memory = start + MEMORY_SAMPLE_INTERVAL;
    while Instant::now() < end {
        let pass_start = Instant::now();
        pump_window(&platform, event_loop, size)?;
        if scenario.scene_kind == SceneKind::CubeGrid {
            animate_cube_grid(scene, animation_start.elapsed().as_secs_f32())?;
        }
        let preparation_start = Instant::now();
        let frame = scene.build_render_frame(size);
        let preparation_ns = preparation_start.elapsed().as_nanos() as u64;
        let stats = backend.render(&frame, resources)?;
        let pass_ns = pass_start.elapsed().as_nanos() as u64;
        preparation_samples.record(preparation_ns);
        pass_samples.record(pass_ns);
        window_samples.record(pass_ns);
        if stats.draw_calls != expected_draw_calls || stats.triangles != expected_triangles {
            return Err(StressError::Window(format!(
                "incomplete measured frame: {stats:?}"
            )));
        }
        if Instant::now() >= next_memory {
            memory.push(sample_memory(start.elapsed().as_secs_f64())?);
            next_memory += MEMORY_SAMPLE_INTERVAL;
        }
        let now = Instant::now();
        if now.duration_since(window_start) >= FRAME_WINDOW_INTERVAL {
            frame_windows.push(frame_window(start, window_start, now, &window_samples));
            window_samples = Samples::default();
            window_start = now;
        }
    }
    let finish = Instant::now();
    if window_samples.total > 0 {
        frame_windows.push(frame_window(start, window_start, finish, &window_samples));
    }
    let measured_seconds = finish.duration_since(start).as_secs_f64();
    memory.push(sample_memory(measured_seconds)?);
    backend.shutdown();
    Ok((
        ScenarioResult {
            scenario,
            measured_seconds,
            measured_frames: pass_samples.total,
            fps: Some(pass_samples.total as f64 / measured_seconds),
            frame_preparation_ns: preparation_samples.summary(),
            full_pass_ns: Some(pass_samples.summary()),
            frame_preparation_histogram_ns: preparation_samples.counts,
            full_pass_histogram_ns: Some(pass_samples.counts),
            frame_windows: Some(frame_windows),
            memory,
        },
        adapter,
    ))
}

fn frame_window(
    start: Instant,
    window_start: Instant,
    end: Instant,
    samples: &Samples,
) -> FrameWindow {
    let duration_seconds = end.duration_since(window_start).as_secs_f64();
    FrameWindow {
        elapsed_seconds: end.duration_since(start).as_secs_f64(),
        duration_seconds,
        frames: samples.total,
        fps: samples.total as f64 / duration_seconds,
        p95_full_pass_ns: samples.percentile(95),
    }
}

fn pump_window(
    platform: &PlatformAdapter,
    event_loop: &mut EventLoop<()>,
    size: WindowSize,
) -> Result<(), StressError> {
    for event in platform.pump_events(event_loop, Some(Duration::ZERO)) {
        match event {
            PlatformEvent::CloseRequested => {
                return Err(StressError::Window(
                    "benchmark window was closed".to_owned(),
                ));
            }
            PlatformEvent::Resized(actual) if actual != size => {
                return Err(StressError::Window(format!(
                    "window size changed to {actual:?}"
                )));
            }
            PlatformEvent::Resized(_) => {}
        }
    }
    Ok(())
}

fn sample_memory(elapsed_seconds: f64) -> io::Result<MemorySample> {
    #[cfg(target_os = "linux")]
    let status = Some(fs::read_to_string(PROC_STATUS)?);
    #[cfg(not(target_os = "linux"))]
    let status: Option<String> = None;
    let field = |name: &str| {
        status.as_ref().and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix(name)
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
                    .and_then(|value| value.checked_mul(KIB))
            })
        })
    };
    let rss_bytes = field(RSS_FIELD);
    let peak_rss_bytes = field(PEAK_RSS_FIELD);
    #[cfg(target_os = "linux")]
    if rss_bytes.is_none() || peak_rss_bytes.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "VmRSS or VmHWM is missing from process status",
        ));
    }
    Ok(MemorySample {
        elapsed_seconds,
        rss_bytes,
        peak_rss_bytes,
    })
}

fn cpu_model() -> Option<String> {
    fs::read_to_string(PROC_CPUINFO).ok().and_then(|info| {
        info.lines().find_map(|line| {
            line.strip_prefix(CPU_MODEL_FIELD)
                .and_then(|line| line.split_once(':'))
                .map(|(_, model)| model.trim().to_owned())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_generation_preserves_counts_and_shared_handles() {
        let scenario = Scenario {
            name: "shared".to_owned(),
            scene_kind: SceneKind::Synthetic,
            objects: 4,
            nodes: 5,
            depth: 2,
            triangles_per_mesh: 12,
            meshes: 1,
            duration_seconds: 1,
        };
        let (scene, resources) = build_scene(&scenario).expect("valid fixture must build");
        let frame = scene.build_render_frame(WindowSize::new(800, 600));
        assert_eq!(scene.nodes().len(), 5);
        assert_eq!(frame.items.len(), 4);
        assert_eq!(resources.meshes().len(), 1);
        assert!(
            frame
                .items
                .iter()
                .all(|item| item.mesh == frame.items[0].mesh)
        );
    }

    #[test]
    fn distinct_mesh_scenario_uses_distinct_handles() {
        let scenario = Scenario {
            name: "distinct".to_owned(),
            scene_kind: SceneKind::Synthetic,
            objects: 3,
            nodes: 4,
            depth: 1,
            triangles_per_mesh: 12,
            meshes: 3,
            duration_seconds: 1,
        };
        let (scene, resources) = build_scene(&scenario).expect("valid fixture must build");
        let frame = scene.build_render_frame(WindowSize::new(800, 600));
        assert_eq!(resources.meshes().len(), 3);
        assert_ne!(frame.items[0].mesh, frame.items[1].mesh);
        assert_ne!(frame.items[1].mesh, frame.items[2].mesh);
        assert_ne!(frame.items[0].mesh, frame.items[2].mesh);
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let mut samples = Samples::default();
        for value in [1, 2, 3, 4, 5] {
            samples.record(value);
        }
        assert_eq!(samples.percentile(50), 3);
        assert_eq!(samples.percentile(95), 5);
        let mut pair = Samples::default();
        pair.record(1);
        pair.record(2);
        assert_eq!(pair.percentile(50), 1);
    }

    #[test]
    fn frame_window_reports_elapsed_fps_and_tail_latency() {
        let start = Instant::now();
        let mut samples = Samples::default();
        samples.record(10);
        samples.record(20);

        let window = frame_window(start, start, start + Duration::from_secs(1), &samples);

        assert_eq!(window.elapsed_seconds, 1.0);
        assert_eq!(window.duration_seconds, 1.0);
        assert_eq!(window.frames, 2);
        assert_eq!(window.fps, 2.0);
        assert_eq!(window.p95_full_pass_ns, 20);
    }

    #[test]
    fn cube_grid_uses_one_cube_mesh_and_distinct_positions() {
        let scenario = Scenario {
            name: "cube_grid".to_owned(),
            scene_kind: SceneKind::CubeGrid,
            objects: 8,
            nodes: 9,
            depth: 1,
            triangles_per_mesh: 12,
            meshes: 1,
            duration_seconds: 1,
        };
        let (scene, resources) = build_scene(&scenario).expect("valid cube grid must build");
        let frame = scene.build_render_frame(WindowSize::new(800, 600));

        assert_eq!(scene.nodes().len(), 9);
        assert_eq!(resources.meshes().len(), 1);
        assert_eq!(frame.items.len(), 8);
        assert!(
            frame
                .items
                .iter()
                .all(|item| item.mesh == frame.items[0].mesh)
        );
        assert_eq!(resources.meshes()[0].submeshes[0].indices.len(), 36);
        assert_eq!(
            frame.items[0].world_transform.transform_point3(Vec3::ZERO),
            Vec3::new(-1.0, -1.0, -1.0)
        );
        assert_eq!(
            frame.items[7].world_transform.transform_point3(Vec3::ZERO),
            Vec3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn cube_grid_rejects_inconsistent_geometry() {
        let config = Config {
            width: 800,
            height: 600,
            sync_mode: SyncMode::Vsync,
            warmup_seconds: 1,
            scenarios: vec![Scenario {
                name: "invalid_cube_grid".to_owned(),
                scene_kind: SceneKind::CubeGrid,
                objects: 8,
                nodes: 9,
                depth: 1,
                triangles_per_mesh: 120,
                meshes: 1,
                duration_seconds: 1,
            }],
        };

        assert!(matches!(
            validate(&config),
            Err(StressError::InvalidConfig(_))
        ));
    }

    #[test]
    fn cube_grid_animation_moves_objects_between_frames() {
        let scenario = Scenario {
            name: "animated_cube_grid".to_owned(),
            scene_kind: SceneKind::CubeGrid,
            objects: 8,
            nodes: 9,
            depth: 1,
            triangles_per_mesh: 12,
            meshes: 1,
            duration_seconds: 1,
        };
        let (mut scene, _) = build_scene(&scenario).expect("valid cube grid must build");
        let before = scene.build_render_frame(WindowSize::new(800, 600)).items[0]
            .world_transform
            .transform_point3(Vec3::ZERO);

        animate_cube_grid(&mut scene, 1.0).expect("a valid rotation updates the grid");
        let after = scene.build_render_frame(WindowSize::new(800, 600)).items[0]
            .world_transform
            .transform_point3(Vec3::ZERO);

        assert_ne!(before, after);
    }

    #[test]
    fn existing_stress_config_keeps_synthetic_scenes() {
        let config: Config = serde_json::from_str(include_str!("../docs/stress-config.json"))
            .expect("the existing benchmark configuration is valid JSON");

        assert!(
            config
                .scenarios
                .iter()
                .all(|scenario| scenario.scene_kind == SceneKind::Synthetic)
        );
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn baseline_config_builds_the_full_cube_grid() {
        let config: Config = serde_json::from_str(include_str!("../docs/stress-baseline.json"))
            .expect("the bundled baseline configuration is valid JSON");
        validate(&config).expect("the baseline values match the cube grid requirements");
        let (scene, resources) =
            build_scene(&config.scenarios[0]).expect("the baseline scene must build");

        assert_eq!(scene.nodes().len(), 1001);
        assert_eq!(scene.objects().len(), 1000);
        assert_eq!(resources.meshes().len(), 1);
    }

    #[test]
    fn scaling_config_has_increasing_valid_cube_grids() {
        let config: Config = serde_json::from_str(include_str!("../docs/stress-scaling.json"))
            .expect("the bundled scaling configuration is valid JSON");
        validate(&config).expect("all scaling scenarios use valid cube grids");

        assert!(
            config
                .scenarios
                .iter()
                .all(|scenario| scenario.scene_kind == SceneKind::CubeGrid)
        );
        assert!(
            config
                .scenarios
                .windows(2)
                .all(|pair| pair[0].objects < pair[1].objects)
        );
        assert_eq!(
            config.scenarios.first().map(|scenario| scenario.objects),
            Some(250)
        );
        assert_eq!(
            config.scenarios.last().map(|scenario| scenario.objects),
            Some(12000)
        );
    }
}
