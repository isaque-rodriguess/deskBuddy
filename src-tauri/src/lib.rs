#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use image::RgbaImage;
use rand::SeedableRng;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, State};

use crate::config::PetConfig;
use crate::pet::{sprite_row, Pet, StdPetRng};

pub mod config;
pub mod pet;
pub mod petdex;
pub mod sprite;

const SCALE: f64 = 0.7;
const TICK_MS: u64 = 16;

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
    sheet_path: &std::path::Path,
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

#[tauri::command]
fn get_frames(state: State<'_, AppState>) -> HashMap<String, FrameSet> {
    state.sprites.frames.clone()
}

#[tauri::command]
#[allow(clippy::explicit_auto_deref)]
fn pet_drag(stage: &str, x: f64, y: f64, drag_ox: f64, drag_oy: f64, state: State<'_, AppState>) {
    let mut rt = state.runtime.lock().unwrap();
    let rt: &mut PetRuntime = &mut *rt;
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

pub struct AppState {
    sprites: Arc<SpriteCache>,
    runtime: Mutex<PetRuntime>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_frames,
            pet_drag,
            show_settings
        ])
        .setup(|app| {
            let resource_path = app
                .path()
                .resolve(
                    "resources/zero-2/spritesheet.webp",
                    tauri::path::BaseDirectory::Resource,
                )
                .or_else(|_| {
                    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("resources/zero-2/spritesheet.webp");
                    if p.exists() {
                        Ok(p)
                    } else {
                        Err(tauri::Error::AssetNotFound("spritesheet".into()))
                    }
                })?;

            let scale_f32 = SCALE as f32;
            let sprites =
                Arc::new(load_sprite_cache(&resource_path, scale_f32).expect("load sprites"));

            let overlay = app.get_webview_window("overlay").expect("overlay window");
            let monitor = overlay.current_monitor()?.expect("monitor");
            let (sw, sh) = (monitor.size().width as f64, monitor.size().height as f64);
            let (fw, fh) = sprite::get_frame_size(SCALE as f32);
            let (fw, fh) = (fw as f64, fh as f64);
            overlay
                .set_size(PhysicalSize::new(fw as u32, fh as u32))
                .ok();
            let _ = overlay.set_shadow(false);

            let now = now_secs();
            let config = PetConfig::default();
            let pet = Pet::new(sw, sh, fw, fh, config.run_enabled, now);
            let rng = StdPetRng(rand::rngs::StdRng::seed_from_u64(42));

            let app_state = AppState {
                sprites: sprites.clone(),
                runtime: Mutex::new(PetRuntime { pet, rng }),
            };
            app.manage(app_state);

            let app_handle = app.handle().clone();
            let sprites_ref = sprites.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
                let overlay = app_handle.get_webview_window("overlay").expect("overlay");
                loop {
                    interval.tick().await;
                    let now = now_secs();
                    let event = {
                        let app_state = app_handle.state::<AppState>();
                        let mut guard = app_state.runtime.lock().unwrap();
                        #[allow(clippy::explicit_auto_deref)]
                        let rt: &mut PetRuntime = &mut *guard;
                        rt.pet.tick(now, &mut rt.rng);
                        let row = sprite_row(rt.pet.state);
                        let count = sprites_ref.frame_count(row);
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
                    let _ = overlay.set_position(PhysicalPosition::new(
                        event.pos_x as i32,
                        event.pos_y as i32,
                    ));
                    let _ = overlay.emit("pet-frame", &event);
                }
            });

            // Tray (minimal for Fase 2 — full lifecycle in Fase 4)
            let show_i = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("Desktop Pet")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
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
