#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use image::RgbaImage;
use rand::SeedableRng;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, State};
use tauri_plugin_autostart::ManagerExt;

use crate::config::PetConfig;
use crate::pet::{sprite_row, DesktopLayout, MonitorRect, Pet, StdPetRng};

pub mod config;
pub mod pet;
pub mod petdex;
pub mod sprite;

const DEFAULT_SCALE: f64 = 0.7;
const TICK_MS: u64 = 16;
/// Refresh display geometry every N ticks (~2s at 60fps).
const LAYOUT_REFRESH_TICKS: u64 = 120;

fn desktop_layout_from_app(app: &tauri::AppHandle) -> DesktopLayout {
    let primary_xy: Option<(i32, i32)> = app.primary_monitor().ok().flatten().map(|m| {
        let p = m.position();
        (p.x, p.y)
    });

    let mut monitors: Vec<MonitorRect> = app
        .available_monitors()
        .unwrap_or_else(|_| Vec::new())
        .into_iter()
        .map(|m| {
            let p = m.position();
            let s = m.size();
            MonitorRect::new(p.x as f64, p.y as f64, s.width as f64, s.height as f64)
        })
        .collect();

    if monitors.is_empty() {
        if let Some(overlay) = app.get_webview_window("overlay") {
            if let Ok(Some(m)) = overlay.current_monitor() {
                let p = m.position();
                let s = m.size();
                monitors.push(MonitorRect::new(
                    p.x as f64,
                    p.y as f64,
                    s.width as f64,
                    s.height as f64,
                ));
            }
        }
    }

    if let Some((px, py)) = primary_xy {
        monitors.sort_by_key(|m| {
            if (m.x as i32) == px && (m.y as i32) == py {
                0
            } else {
                1
            }
        });
    }

    DesktopLayout::from_monitors(monitors)
}

#[derive(Serialize, Clone)]
struct FrameSet {
    normal: Vec<String>,
    mirror: Vec<String>,
}

#[derive(Serialize, Clone, Default)]
pub struct SpriteCache {
    frames: HashMap<String, FrameSet>,
}

impl SpriteCache {
    fn frame_count(&self, row: &str) -> usize {
        self.frames.get(row).map(|fs| fs.normal.len()).unwrap_or(0)
    }
}

fn encode_data_url(img: &RgbaImage) -> String {
    let mut buf = Cursor::new(Vec::new());
    let dyn_img = image::DynamicImage::ImageRgba8(img.clone());
    dyn_img
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("png encode");
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    format!("data:image/png;base64,{b64}")
}

fn load_sprite_cache(
    sheet_path: &Path,
    scale: f32,
) -> Result<SpriteCache, Box<dyn std::error::Error>> {
    let sheet = image::open(sheet_path)?.to_rgba8();
    let mut cache = SpriteCache::default();
    for row in sprite::STATE_ROWS.iter() {
        let frames = sprite::load_frames_from_sheet(&sheet, row, scale);
        if frames.is_empty() {
            continue;
        }
        let normal: Vec<String> = frames.iter().map(encode_data_url).collect();
        let mirror: Vec<String> = frames
            .iter()
            .map(|f| {
                let flipped = image::imageops::flip_horizontal(f);
                encode_data_url(&flipped)
            })
            .collect();
        cache
            .frames
            .insert((*row).to_string(), FrameSet { normal, mirror });
    }
    Ok(cache)
}

struct PetRuntime {
    pet: Pet,
    rng: StdPetRng,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Serialize)]
struct PetFrameEvent {
    row: &'static str,
    frame_idx: usize,
    mirror: bool,
    pos_x: f64,
    pos_y: f64,
}

#[derive(Serialize, Clone)]
struct PetInfo {
    slug: String,
    name: String,
    source: String,
    downloaded: bool,
}

fn hermes_pets_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("hermes").join("pets"))
            .unwrap_or_else(|_| PathBuf::from(".").join("hermes").join("pets"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .ok()
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("hermes").join("pets")
    }
}

fn find_spritesheet(
    slug: &str,
    hermes_dir: &Path,
    app_data: &Path,
    resource_dir: &Path,
) -> Option<PathBuf> {
    let candidates = [
        hermes_dir.join(slug).join("spritesheet.webp"),
        app_data
            .join("petdex_cache")
            .join(slug)
            .join("spritesheet.webp"),
        resource_dir
            .join("resources")
            .join(slug)
            .join("spritesheet.webp"),
        resource_dir.join(slug).join("spritesheet.webp"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub struct AppState {
    sprites: Arc<RwLock<SpriteCache>>,
    runtime: Mutex<PetRuntime>,
    avatar_cache: Mutex<HashMap<String, String>>,
    config: Mutex<PetConfig>,
    config_path: PathBuf,
    hermes_dir: PathBuf,
    app_data_dir: PathBuf,
    resource_dir: PathBuf,
    autostart_item: Mutex<Option<MenuItem<tauri::Wry>>>,
}

// ── Overlay commands ──────────────────────────────────────

#[tauri::command]
fn get_frames(state: State<'_, AppState>) -> HashMap<String, FrameSet> {
    state.sprites.read().unwrap().frames.clone()
}

#[tauri::command]
#[allow(clippy::explicit_auto_deref)]
fn pet_drag(
    stage: &str,
    x: f64,
    y: f64,
    drag_ox: f64,
    drag_oy: f64,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) {
    let layout = desktop_layout_from_app(&app);
    let mut rt = state.runtime.lock().unwrap();
    let rt: &mut PetRuntime = &mut *rt;
    rt.pet.set_layout(layout);
    let now = now_secs();
    match stage {
        "press" => {
            rt.pet.on_press(x, y);
            rt.pet.reset_anim(now);
        }
        "drag" => rt.pet.on_drag(x, y, drag_ox, drag_oy),
        "release" => rt.pet.on_release(now, &mut rt.rng),
        _ => {}
    }
}

#[tauri::command]
fn show_settings(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// ── Settings commands ─────────────────────────────────────

#[tauri::command]
fn list_local_pets(state: State<'_, AppState>) -> Vec<PetInfo> {
    petdex::get_installed_pets(&state.hermes_dir, &state.app_data_dir)
        .into_iter()
        .map(|p| PetInfo {
            slug: p.slug,
            name: p.display_name,
            source: p.source.unwrap_or_else(|| "local".into()),
            downloaded: true,
        })
        .collect()
}

#[tauri::command]
fn list_all_pets(state: State<'_, AppState>) -> Vec<PetInfo> {
    let local = petdex::get_installed_pets(&state.hermes_dir, &state.app_data_dir);
    let mut seen: std::collections::HashSet<String> =
        local.iter().map(|p| p.slug.clone()).collect();
    let mut out: Vec<PetInfo> = local
        .into_iter()
        .map(|p| PetInfo {
            slug: p.slug,
            name: p.display_name,
            source: p.source.unwrap_or_else(|| "local".into()),
            downloaded: true,
        })
        .collect();
    let manifest = petdex::fetch_manifest(&state.app_data_dir, false);
    for p in manifest {
        if seen.insert(p.slug.clone()) {
            out.push(PetInfo {
                slug: p.slug,
                name: p.display_name,
                source: "petdex".into(),
                downloaded: false,
            });
        }
    }
    out
}

#[tauri::command]
fn get_avatar(slug: String, state: State<'_, AppState>) -> Option<String> {
    {
        let cache = state.avatar_cache.lock().unwrap();
        if let Some(url) = cache.get(&slug) {
            return Some(url.clone());
        }
    }
    let sheet = find_spritesheet(
        &slug,
        &state.hermes_dir,
        &state.app_data_dir,
        &state.resource_dir,
    )?;
    let frames = sprite::load_frames("idle", 0.25, &sheet).ok()?;
    if frames.is_empty() {
        return None;
    }
    let url = encode_data_url(&frames[0]);
    state.avatar_cache.lock().unwrap().insert(slug, url.clone());
    Some(url)
}

#[tauri::command]
fn download_pet_cmd(slug: String, state: State<'_, AppState>) -> Result<bool, String> {
    let manifest = petdex::fetch_manifest(&state.app_data_dir, false);
    let info = manifest
        .into_iter()
        .find(|p| p.slug == slug)
        .ok_or_else(|| format!("pet {slug} not in manifest"))?;
    let (_, ok) = petdex::download_pet(&state.app_data_dir, &info).map_err(|e| e.to_string())?;
    Ok(ok)
}

#[tauri::command]
fn delete_pet_cmd(slug: String, state: State<'_, AppState>) -> Result<(), String> {
    let paths = [
        state.app_data_dir.join("petdex_cache").join(&slug),
        state.hermes_dir.join(&slug),
    ];
    for p in paths {
        if p.exists() {
            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
        }
    }
    state.avatar_cache.lock().unwrap().remove(&slug);
    Ok(())
}

#[tauri::command]
#[allow(clippy::explicit_auto_deref)]
fn apply_config(
    slug: String,
    scale: f64,
    enabled: bool,
    run_enabled: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let new_cfg = PetConfig {
        slug: slug.clone(),
        scale,
        enabled,
        run_enabled,
    };
    new_cfg
        .save(&state.config_path)
        .map_err(|e| e.to_string())?;

    let old_cfg = state.config.lock().unwrap().clone();
    let scale_changed = (old_cfg.scale - scale).abs() > 1e-9;
    let slug_changed = old_cfg.slug != slug;
    *state.config.lock().unwrap() = new_cfg;

    if slug_changed || scale_changed {
        if let Some(sheet) = find_spritesheet(
            &slug,
            &state.hermes_dir,
            &state.app_data_dir,
            &state.resource_dir,
        ) {
            let scale_f32 = scale as f32;
            if let Ok(new_cache) = load_sprite_cache(&sheet, scale_f32) {
                *state.sprites.write().unwrap() = new_cache;
            }
            let (fw, fh) = sprite::get_frame_size(scale as f32);
            let (fw, fh) = (fw as f64, fh as f64);
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_size(PhysicalSize::new(fw as u32, fh as u32));
                let _ = overlay.emit(
                    "sprites-reloaded",
                    &serde_json::json!({ "fw": fw, "fh": fh }),
                );
            }
            let layout = desktop_layout_from_app(&app);
            let mut rt = state.runtime.lock().unwrap();
            let rt: &mut PetRuntime = &mut *rt;
            rt.pet.fw = fw;
            rt.pet.fh = fh;
            rt.pet.set_layout(layout);
            rt.pet.reset_spawn(now_secs());
        }
    }
    if let Some(overlay) = app.get_webview_window("overlay") {
        if enabled {
            let _ = overlay.show();
        } else {
            let _ = overlay.hide();
        }
    }
    {
        let mut rt = state.runtime.lock().unwrap();
        let rt: &mut PetRuntime = &mut *rt;
        rt.pet.run_enabled = run_enabled;
    }
    Ok(())
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> PetConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())
    } else {
        mgr.disable().map_err(|e| e.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_frames,
            pet_drag,
            show_settings,
            list_local_pets,
            list_all_pets,
            get_avatar,
            download_pet_cmd,
            delete_pet_cmd,
            apply_config,
            get_config,
            get_autostart,
            set_autostart,
        ])
        .setup(|app| {
            let resource_path = app
                .path()
                .resolve(
                    "resources/zero-2/spritesheet.webp",
                    tauri::path::BaseDirectory::Resource,
                )
                .or_else(|_| {
                    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("resources/zero-2/spritesheet.webp");
                    if p.exists() {
                        Ok(p)
                    } else {
                        Err(tauri::Error::AssetNotFound("spritesheet".into()))
                    }
                })?;

            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir).ok();
            let hermes_dir = hermes_pets_dir();
            let config_path = app_data_dir.join("pet_config.json");

            let scale_f32 = DEFAULT_SCALE as f32;
            let sprites = Arc::new(RwLock::new(
                load_sprite_cache(&resource_path, scale_f32).expect("load sprites"),
            ));

            let overlay = app.get_webview_window("overlay").expect("overlay window");
            let layout = desktop_layout_from_app(app.handle());
            let (fw, fh) = sprite::get_frame_size(DEFAULT_SCALE as f32);
            let (fw, fh) = (fw as f64, fh as f64);
            overlay
                .set_size(PhysicalSize::new(fw as u32, fh as u32))
                .ok();
            let _ = overlay.set_shadow(false);

            let now = now_secs();
            let mut config = PetConfig::load(&config_path);
            if config.slug.is_empty() {
                config = PetConfig::default();
            }
            if !config.enabled {
                let _ = overlay.hide();
            }
            let pet = Pet::with_layout(layout, fw, fh, config.run_enabled, now);
            let rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));

            let app_state = AppState {
                sprites: sprites.clone(),
                runtime: Mutex::new(PetRuntime { pet, rng }),
                avatar_cache: Mutex::new(HashMap::new()),
                config: Mutex::new(config),
                config_path,
                hermes_dir,
                app_data_dir,
                resource_dir,
                autostart_item: Mutex::new(None),
            };
            app.manage(app_state);

            let app_handle = app.handle().clone();
            let sprites_ref = sprites.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
                let overlay = app_handle.get_webview_window("overlay").expect("overlay");
                let mut tick_n: u64 = 0;
                let mut last_pos: Option<(i32, i32)> = None;
                loop {
                    interval.tick().await;
                    tick_n = tick_n.wrapping_add(1);
                    let now = now_secs();
                    if tick_n.is_multiple_of(LAYOUT_REFRESH_TICKS) {
                        let layout = desktop_layout_from_app(&app_handle);
                        let app_state = app_handle.state::<AppState>();
                        let mut guard = app_state.runtime.lock().unwrap();
                        guard.pet.set_layout(layout);
                    }
                    let event = {
                        let app_state = app_handle.state::<AppState>();
                        let mut guard = app_state.runtime.lock().unwrap();
                        #[allow(clippy::explicit_auto_deref)]
                        let rt: &mut PetRuntime = &mut *guard;
                        rt.pet.tick(now, &mut rt.rng);
                        let row = sprite_row(rt.pet.state);
                        let count = sprites_ref.read().unwrap().frame_count(row);
                        rt.pet.advance_anim(now, count);
                        let frame_idx = rt.pet.anim_idx.min(count.saturating_sub(1));
                        let mirror = rt.pet.mirror;
                        let pos_x = rt.pet.pos_x;
                        let pos_y = rt.pet.pos_y;
                        PetFrameEvent {
                            row,
                            frame_idx,
                            mirror,
                            pos_x,
                            pos_y,
                        }
                    };
                    let target_pos = (event.pos_x.round() as i32, event.pos_y.round() as i32);
                    if last_pos != Some(target_pos) {
                        let _ = overlay.set_position(PhysicalPosition::new(target_pos.0, target_pos.1));
                        last_pos = Some(target_pos);
                    }
                    let _ = overlay.emit("pet-frame", &event);
                }
            });

            // Tray
            let show_i = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_text = if cfg!(target_os = "windows") {
                if autostart_enabled {
                    "✓ Start with Windows"
                } else {
                    "Start with Windows"
                }
            } else if autostart_enabled {
                "✓ Start on Boot"
            } else {
                "Start on Boot"
            };
            let autostart_i =
                MenuItem::with_id(app, "autostart", autostart_text, true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &autostart_i, &quit_i])?;

            // Store autostart menu item handle for text updates on toggle
            {
                let state = app.state::<AppState>();
                *state.autostart_item.lock().unwrap() = Some(autostart_i);
            }

            let mut tray_builder = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("Desktop Pet");

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "autostart" => {
                        let mgr = app.autolaunch();
                        let new_state = !mgr.is_enabled().unwrap_or(false);
                        let _ = if new_state {
                            mgr.enable()
                        } else {
                            mgr.disable()
                        };
                        let state = app.state::<AppState>();
                        let guard = state.autostart_item.lock().unwrap();
                        if let Some(item) = guard.as_ref() {
                            let text = if cfg!(target_os = "windows") {
                                if new_state {
                                    "✓ Start with Windows"
                                } else {
                                    "Start with Windows"
                                }
                            } else if new_state {
                                "✓ Start on Boot"
                            } else {
                                "Start on Boot"
                            };
                            let _ = item.set_text(text);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running deskbuddy");
}
