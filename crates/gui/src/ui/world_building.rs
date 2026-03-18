use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass, EguiUserTextures};

use crate::state::AppState;

pub struct WorldBuildingPlugin;

impl Plugin for WorldBuildingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetGenState>()
            // Egui UI must run in EguiPrimaryContextPass (bevy_egui 0.39 multi-pass API).
            .add_systems(
                EguiPrimaryContextPass,
                render_world_building_ui.run_if(in_state(AppState::WorldBuilding)),
            )
            // Polling systems don't touch egui; Update is fine.
            .add_systems(
                Update,
                (poll_stability_results, poll_meshy_results)
                    .run_if(in_state(AppState::WorldBuilding)),
            );
    }
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Portraits,
    Meshes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GenCharacter {
    #[default]
    Researcher,
    Orin,
    Doss,
    Kaleo,
}

impl GenCharacter {
    fn label(self) -> &'static str {
        match self {
            GenCharacter::Researcher => "Researcher",
            GenCharacter::Orin => "Orin",
            GenCharacter::Doss => "Doss",
            GenCharacter::Kaleo => "Kaleo",
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            GenCharacter::Researcher => "researcher",
            GenCharacter::Orin => "orin",
            GenCharacter::Doss => "doss",
            GenCharacter::Kaleo => "kaleo",
        }
    }

    fn default_prompt(self) -> &'static str {
        match self {
            GenCharacter::Researcher => {
                "portrait of young researcher in temporal sci-fi gear, teal accent lighting"
            }
            GenCharacter::Orin => {
                "portrait of composed woman, dark professional attire, cool blue-purple palette"
            }
            GenCharacter::Doss => {
                "portrait of weathered soldier, heavy armor, warm amber tones, scarred"
            }
            GenCharacter::Kaleo => {
                "portrait of androgynous AI construct, geometric facial markings, electric blue glows"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Emotion {
    #[default]
    Neutral,
    Concerned,
    Determined,
    Hurt,
    Enigmatic,
}

impl Emotion {
    fn label(self) -> &'static str {
        match self {
            Emotion::Neutral => "Neutral",
            Emotion::Concerned => "Concerned",
            Emotion::Determined => "Determined",
            Emotion::Hurt => "Hurt",
            Emotion::Enigmatic => "Enigmatic",
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Emotion::Neutral => "neutral",
            Emotion::Concerned => "concerned",
            Emotion::Determined => "determined",
            Emotion::Hurt => "hurt",
            Emotion::Enigmatic => "enigmatic",
        }
    }
}

const PROMPT_SUFFIX: &str = "vibrant neon sci-fi colors, expressive anime face, detailed linework, high contrast, masterpiece, best quality, upper body portrait";

fn build_prompt(character: GenCharacter, emotion: Emotion) -> String {
    format!(
        "{}, expression: {}, {}",
        character.default_prompt(),
        emotion.label().to_lowercase(),
        PROMPT_SUFFIX
    )
}

// ── Shared state for background threads ───────────────────────────────────────

#[derive(Default)]
pub struct StabilityPending {
    pub images: Vec<Vec<u8>>,
    pub error: Option<String>,
    pub done: bool,
}

#[derive(Default)]
pub struct MeshyPending {
    pub job: Option<MeshyJob>,
    pub error: Option<String>,
    pub done: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MeshyJob {
    pub task_id: String,
    pub status: String,
    pub progress: u32,
    pub model_url: Option<String>,
}

pub struct GenerationResult {
    pub image_bytes: Vec<u8>,
    pub _handle: Option<Handle<Image>>,
    pub texture: Option<egui::TextureId>,
}

fn bytes_to_egui_texture(
    bytes: &[u8],
    images: &mut Assets<Image>,
    egui_user_textures: &mut EguiUserTextures,
) -> Option<(Handle<Image>, egui::TextureId)> {
    let dyn_image = image::load_from_memory(bytes).ok()?;
    let rgba = dyn_image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let bevy_image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(bevy_image);
    let tex_id =
        egui_user_textures.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
    Some((handle, tex_id))
}

// ── Main resource ──────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct AssetGenState {
    pub tab: Tab,

    // Portraits tab
    pub character: GenCharacter,
    pub emotion: Emotion,
    pub prompt: String,
    pub generating: bool,
    pub stability_pending: Arc<Mutex<StabilityPending>>,
    pub results: Vec<GenerationResult>,
    pub selected_result: Option<usize>,
    pub stability_error: Option<String>,
    pub save_status: Option<String>,
    pub base_portrait: Option<Vec<u8>>,
    pub base_portrait_texture: Option<egui::TextureId>,
    pub base_portrait_dirty: bool,
    pub img2img_strength: f32,

    // Meshes tab
    pub mesh_character: GenCharacter,
    pub reference_path: String,
    pub mesh_generating: bool,
    pub meshy_pending: Arc<Mutex<MeshyPending>>,
    pub active_job: Option<MeshyJob>,
    pub meshy_error: Option<String>,
    pub mesh_save_status: Option<String>,
}

impl Default for AssetGenState {
    fn default() -> Self {
        let character = GenCharacter::default();
        let emotion = Emotion::default();
        Self {
            tab: Tab::default(),
            character,
            emotion,
            prompt: build_prompt(character, emotion),
            generating: false,
            stability_pending: Arc::new(Mutex::new(StabilityPending::default())),
            results: Vec::new(),
            selected_result: None,
            stability_error: None,
            save_status: None,
            base_portrait: None,
            base_portrait_texture: None,
            base_portrait_dirty: false,
            img2img_strength: 0.65,
            mesh_character: GenCharacter::default(),
            reference_path: String::new(),
            mesh_generating: false,
            meshy_pending: Arc::new(Mutex::new(MeshyPending::default())),
            active_job: None,
            meshy_error: None,
            mesh_save_status: None,
        }
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn render_world_building_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<AssetGenState>,
    mut next_app_state: ResMut<NextState<AppState>>,
) -> Result {
    egui::CentralPanel::default().show(contexts.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            ui.heading("World Building");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Exit").clicked() {
                    next_app_state.set(AppState::MainMenu);
                }
            });
        });
        ui.separator();

        // Tab selector
        ui.horizontal(|ui| {
            if ui
                .selectable_label(state.tab == Tab::Portraits, "Portraits")
                .clicked()
            {
                state.tab = Tab::Portraits;
            }
            if ui
                .selectable_label(state.tab == Tab::Meshes, "3D Meshes")
                .clicked()
            {
                state.tab = Tab::Meshes;
            }
        });
        ui.separator();

        match state.tab {
            Tab::Portraits => render_portraits_tab(ui, &mut state),
            Tab::Meshes => render_meshes_tab(ui, &mut state),
        }
    });
    Ok(())
}

fn render_portraits_tab(ui: &mut egui::Ui, state: &mut AssetGenState) {
    // Character selector
    ui.horizontal(|ui| {
        ui.label("Character:");
        for ch in [
            GenCharacter::Researcher,
            GenCharacter::Orin,
            GenCharacter::Doss,
            GenCharacter::Kaleo,
        ] {
            if ui
                .selectable_label(state.character == ch, ch.label())
                .clicked()
                && state.character != ch
            {
                state.character = ch;
                state.prompt = build_prompt(ch, state.emotion);
            }
        }
    });

    // Emotion selector — non-Neutral locked until base is set
    let has_base = state.base_portrait.is_some();
    if !has_base {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Generate Neutral first, then Set as Base to unlock other emotions.",
        );
    }
    ui.horizontal(|ui| {
        ui.label("Emotion:   ");
        for em in [
            Emotion::Neutral,
            Emotion::Concerned,
            Emotion::Determined,
            Emotion::Hurt,
            Emotion::Enigmatic,
        ] {
            let enabled = has_base || em == Emotion::Neutral;
            if ui
                .add_enabled(enabled, egui::Button::selectable(state.emotion == em, em.label()))
                .clicked()
                && state.emotion != em
            {
                state.emotion = em;
                state.prompt = build_prompt(state.character, em);
            }
        }
    });

    // Prompt
    ui.label("Prompt:");
    ui.add(
        egui::TextEdit::multiline(&mut state.prompt)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    ui.separator();

    // Generate button
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.generating, egui::Button::new("Generate"))
            .clicked()
        {
            trigger_stability_generation(state);
        }
        if state.base_portrait.is_some() {
            ui.colored_label(egui::Color32::from_rgb(100, 180, 255), "[img2img]");
        }
        if state.generating {
            ui.spinner();
            ui.label("Generating…");
        }
        if let Some(err) = &state.stability_error.clone() {
            ui.colored_label(egui::Color32::RED, err);
        }
    });
    ui.separator();

    // Results
    if !state.results.is_empty() {
        ui.label("Results (click to select):");
        let mut new_selected = state.selected_result;
        egui::Grid::new("portrait_results")
            .num_columns(4)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                for (i, result) in state.results.iter().enumerate() {
                    if i > 0 && i % 2 == 0 {
                        ui.end_row();
                    }
                    if let Some(tex_id) = result.texture {
                        let is_selected = state.selected_result == Some(i);
                        let sized = egui::load::SizedTexture::new(tex_id, egui::vec2(256.0, 256.0));
                        let response =
                            ui.add(egui::Image::from_texture(sized).sense(egui::Sense::click()));
                        if response.clicked() {
                            new_selected = Some(i);
                        }
                        if is_selected {
                            ui.painter().rect_stroke(
                                response.rect,
                                0.0,
                                egui::Stroke::new(3.0, egui::Color32::YELLOW),
                                egui::StrokeKind::Outside,
                            );
                        }
                    } else {
                        ui.label(format!("Image {}", i + 1));
                    }
                }
            });
        state.selected_result = new_selected;

        ui.separator();
        if let Some(selected) = state.selected_result {
            ui.horizontal(|ui| {
                if ui.button("Set as Base").clicked() {
                    if let Some(result) = state.results.get(selected) {
                        state.base_portrait = Some(result.image_bytes.clone());
                        state.base_portrait_dirty = true;
                        state.save_status = Some("Base portrait set.".to_string());
                    }
                }
                let label = format!(
                    "Save → portraits/{}_{}.png",
                    state.character.file_name(),
                    state.emotion.file_name()
                );
                if ui.button(label).clicked() {
                    if let Some(result) = state.results.get(selected) {
                        state.save_status = Some(
                            match save_portrait(&result.image_bytes, state.character, state.emotion)
                            {
                                Ok(path) => format!("Saved: {path}"),
                                Err(e) => format!("Error: {e}"),
                            },
                        );
                    }
                }
                if let Some(status) = &state.save_status {
                    ui.label(status);
                }
            });
        }

        // Base portrait row
        if let Some(tex_id) = state.base_portrait_texture {
            ui.horizontal(|ui| {
                ui.label("Base:");
                let sized = egui::load::SizedTexture::new(tex_id, egui::vec2(128.0, 128.0));
                ui.add(egui::Image::from_texture(sized));
                if ui.button("Clear Base").clicked() {
                    state.base_portrait = None;
                    state.base_portrait_texture = None;
                    state.emotion = Emotion::Neutral;
                    state.prompt = build_prompt(state.character, Emotion::Neutral);
                }
            });
        }

        // Strength slider — always shown, disabled when no base
        ui.horizontal(|ui| {
            let has_base = state.base_portrait.is_some();
            ui.add_enabled(
                has_base,
                egui::Slider::new(&mut state.img2img_strength, 0.0..=1.0).text("Strength"),
            );
        });
    }
}

fn render_meshes_tab(ui: &mut egui::Ui, state: &mut AssetGenState) {
    // Character selector
    ui.horizontal(|ui| {
        ui.label("Character:");
        for ch in [
            GenCharacter::Researcher,
            GenCharacter::Orin,
            GenCharacter::Doss,
            GenCharacter::Kaleo,
        ] {
            if ui
                .selectable_label(state.mesh_character == ch, ch.label())
                .clicked()
            {
                state.mesh_character = ch;
            }
        }
    });

    // Reference image path
    ui.horizontal(|ui| {
        ui.label("Reference image:");
        ui.add(egui::TextEdit::singleline(&mut state.reference_path).desired_width(400.0));
        if ui.button("Use Selected Portrait").clicked() {
            state.reference_path = format!(
                "assets/portraits/{}_{}.png",
                state.character.file_name(),
                state.emotion.file_name()
            );
        }
    });
    ui.separator();

    // Generate 3D button
    let can_generate = !state.mesh_generating && !state.reference_path.is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_generate, egui::Button::new("Generate 3D"))
            .clicked()
        {
            trigger_meshy_generation(state);
        }
        if state.mesh_generating {
            ui.spinner();
            ui.label("Generating 3D model…");
        }
        if let Some(err) = &state.meshy_error.clone() {
            ui.colored_label(egui::Color32::RED, err);
        }
    });
    ui.separator();

    // Job status
    if let Some(job) = &state.active_job.clone() {
        ui.label(format!("Task ID: {}", job.task_id));
        ui.label(format!("Status: {} ({}%)", job.status, job.progress));

        if job.status == "SUCCEEDED" {
            if job.model_url.is_some() {
                ui.horizontal(|ui| {
                    if ui.button("Download & Save GLB").clicked() {
                        state.mesh_save_status =
                            Some(match download_and_save_glb(job, state.mesh_character) {
                                Ok(path) => format!("Saved: {path}"),
                                Err(e) => format!("Error: {e}"),
                            });
                    }
                    if let Some(status) = &state.mesh_save_status {
                        ui.label(status);
                    }
                });
            }
        } else if job.status == "FAILED" {
            ui.colored_label(egui::Color32::RED, "Generation failed.");
        }
    }
}

// ── Background thread triggers ─────────────────────────────────────────────────

fn trigger_stability_generation(state: &mut AssetGenState) {
    let api_key = match std::env::var("STABILITY_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            state.stability_error = Some("STABILITY_API_KEY env var not set".to_string());
            return;
        }
    };

    let prompt = state.prompt.clone();
    let base_image = state.base_portrait.clone();
    let strength = state.img2img_strength;
    let pending = Arc::clone(&state.stability_pending);

    {
        let mut p = pending.lock().unwrap();
        p.done = false;
        p.error = None;
        p.images.clear();
    }

    std::thread::spawn(move || {
        let result = run_stability_request(&api_key, &prompt, base_image.as_deref(), strength);
        let mut p = pending.lock().unwrap();
        match result {
            Ok(bytes) => p.images.push(bytes),
            Err(e) => p.error = Some(e),
        }
        p.done = true;
    });

    state.generating = true;
    state.stability_error = None;
    state.results.clear();
    state.selected_result = None;
    state.save_status = None;
}

fn run_stability_request(
    api_key: &str,
    prompt: &str,
    base_image: Option<&[u8]>,
    strength: f32,
) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::new();

    let (url, form) = if let Some(img_bytes) = base_image {
        let img_part = reqwest::blocking::multipart::Part::bytes(img_bytes.to_vec())
            .file_name("base.png")
            .mime_str("image/png")
            .map_err(|e| format!("MIME error: {e}"))?;
        let form = reqwest::blocking::multipart::Form::new()
            .text("prompt", prompt.to_string())
            .text("mode", "image-to-image")
            .part("image", img_part)
            .text("strength", format!("{:.2}", strength))
            .text("output_format", "png");
        ("https://api.stability.ai/v2beta/stable-image/generate/sd3", form)
    } else {
        let form = reqwest::blocking::multipart::Form::new()
            .text("prompt", prompt.to_string())
            .text("output_format", "png");
        ("https://api.stability.ai/v2beta/stable-image/generate/core", form)
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "image/*")
        .multipart(form)
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }

    response
        .bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read response bytes: {e}"))
}

fn trigger_meshy_generation(state: &mut AssetGenState) {
    let api_key = match std::env::var("MESHY_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            state.meshy_error = Some("MESHY_API_KEY env var not set".to_string());
            return;
        }
    };

    let reference_path = state.reference_path.clone();
    let pending = Arc::clone(&state.meshy_pending);

    {
        let mut p = pending.lock().unwrap();
        p.done = false;
        p.error = None;
        p.job = None;
    }

    std::thread::spawn(move || {
        let image_bytes = match std::fs::read(&reference_path) {
            Ok(b) => b,
            Err(e) => {
                let mut p = pending.lock().unwrap();
                p.error = Some(format!("Failed to read reference image: {e}"));
                p.done = true;
                return;
            }
        };
        if let Err(e) = run_meshy_workflow(&api_key, &image_bytes, &pending) {
            let mut p = pending.lock().unwrap();
            if p.error.is_none() {
                p.error = Some(e);
            }
            p.done = true;
        }
    });

    state.mesh_generating = true;
    state.meshy_error = None;
    state.active_job = None;
    state.mesh_save_status = None;
}

fn run_meshy_workflow(
    api_key: &str,
    image_bytes: &[u8],
    pending: &Arc<Mutex<MeshyPending>>,
) -> Result<(), String> {
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, image_bytes);
    let data_uri = format!("data:image/png;base64,{b64}");

    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({ "image_url": data_uri, "enable_pbr": true });

    let create_resp = client
        .post("https://api.meshy.ai/openapi/v1/image-to-3d")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .map_err(|e| format!("Meshy create request failed: {e}"))?;

    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let text = create_resp.text().unwrap_or_default();
        return Err(format!("Meshy API error {status}: {text}"));
    }

    let create_data: serde_json::Value = create_resp
        .json()
        .map_err(|e| format!("Failed to parse Meshy create response: {e}"))?;

    let task_id = create_data["result"]
        .as_str()
        .ok_or_else(|| "No task id in Meshy response".to_string())?
        .to_string();

    // Update pending with initial job state
    {
        let mut p = pending.lock().unwrap();
        p.job = Some(MeshyJob {
            task_id: task_id.clone(),
            status: "PENDING".to_string(),
            progress: 0,
            model_url: None,
        });
    }

    // Poll until done
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let poll_resp = client
            .get(format!(
                "https://api.meshy.ai/openapi/v1/image-to-3d/{task_id}"
            ))
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .map_err(|e| format!("Meshy poll failed: {e}"))?;

        let data: serde_json::Value = poll_resp
            .json()
            .map_err(|e| format!("Failed to parse Meshy poll response: {e}"))?;

        let status = data["status"].as_str().unwrap_or("UNKNOWN").to_string();
        let progress = data["progress"].as_u64().unwrap_or(0) as u32;
        let model_url = data["model_urls"]["glb"].as_str().map(str::to_string);

        let job = MeshyJob {
            task_id: task_id.clone(),
            status: status.clone(),
            progress,
            model_url,
        };

        let finished = status == "SUCCEEDED" || status == "FAILED";
        {
            let mut p = pending.lock().unwrap();
            p.job = Some(job);
            if finished {
                p.done = true;
            }
        }
        if finished {
            break;
        }
    }

    Ok(())
}

// ── Polling systems ────────────────────────────────────────────────────────────

fn poll_stability_results(
    mut state: ResMut<AssetGenState>,
    mut images: ResMut<Assets<Image>>,
    mut egui_user_textures: ResMut<EguiUserTextures>,
) {
    // Register base portrait texture if dirty
    if state.base_portrait_dirty {
        if let Some(bytes) = state.base_portrait.clone() {
            if let Some((_handle, tex_id)) =
                bytes_to_egui_texture(&bytes, &mut images, &mut egui_user_textures)
            {
                state.base_portrait_texture = Some(tex_id);
            }
        }
        state.base_portrait_dirty = false;
    }

    let done = state.stability_pending.lock().unwrap().done;
    if !done {
        return;
    }

    let (image_list, error) = {
        let mut p = state.stability_pending.lock().unwrap();
        let imgs = p.images.drain(..).collect::<Vec<_>>();
        let err = p.error.take();
        p.done = false;
        (imgs, err)
    };

    let new_results: Vec<GenerationResult> = image_list
        .into_iter()
        .filter_map(|bytes| {
            let (handle, tex_id) =
                bytes_to_egui_texture(&bytes, &mut images, &mut egui_user_textures)?;
            Some(GenerationResult {
                image_bytes: bytes,
                _handle: Some(handle),
                texture: Some(tex_id),
            })
        })
        .collect();

    state.results.extend(new_results);
    state.generating = false;
    if error.is_some() {
        state.stability_error = error;
    }
}

fn poll_meshy_results(mut state: ResMut<AssetGenState>) {
    let done = state.meshy_pending.lock().unwrap().done;
    if !done {
        return;
    }

    let (job, error) = {
        let mut p = state.meshy_pending.lock().unwrap();
        let job = p.job.take();
        let err = p.error.take();
        p.done = false;
        (job, err)
    };

    state.active_job = job;
    state.mesh_generating = false;
    if error.is_some() {
        state.meshy_error = error;
    }
}

// ── File save helpers ──────────────────────────────────────────────────────────

fn save_portrait(
    bytes: &[u8],
    character: GenCharacter,
    emotion: Emotion,
) -> Result<String, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base.join(format!(
        "assets/portraits/{}_{}.png",
        character.file_name(),
        emotion.file_name()
    ));
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn download_and_save_glb(job: &MeshyJob, character: GenCharacter) -> Result<String, String> {
    let url = job.model_url.as_deref().ok_or("No model URL available")?;
    let response = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base.join(format!("assets/characters/{}.glb", character.file_name()));
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
