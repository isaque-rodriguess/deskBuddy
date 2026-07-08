use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MANIFEST_URL: &str = "https://petdex.dev/api/manifest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pet {
    pub slug: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub info: Option<serde_json::Value>,
    #[serde(default)]
    pub spritesheet_url: Option<String>,
    #[serde(default)]
    pub pet_json_url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

impl Pet {
    fn from_manifest_entry(v: &serde_json::Value) -> Self {
        Pet {
            slug: v
                .get("slug")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: v
                .get("displayName")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            source: None,
            info: Some(v.clone()),
            spritesheet_url: v
                .get("spritesheetUrl")
                .and_then(|s| s.as_str())
                .map(String::from),
            pet_json_url: v
                .get("petJsonUrl")
                .and_then(|s| s.as_str())
                .map(String::from),
            path: None,
        }
    }
}

pub fn cache_dir(app_data: &Path) -> PathBuf {
    app_data.join("petdex_cache")
}

pub fn manifest_cache_path(app_data: &Path) -> PathBuf {
    cache_dir(app_data).join("manifest.json")
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("DesktopPet/1.0")
        .build()
        .expect("reqwest client")
}

pub fn fetch_manifest(app_data: &Path, force: bool) -> Vec<Pet> {
    fetch_manifest_with_url(app_data, MANIFEST_URL, force)
}

pub fn fetch_manifest_with_url(app_data: &Path, manifest_url: &str, force: bool) -> Vec<Pet> {
    let cache = cache_dir(app_data);
    fs::create_dir_all(&cache).ok();
    let manifest_cache = manifest_cache_path(app_data);

    if !force && manifest_cache.exists() {
        if let Ok(data) = fs::read_to_string(&manifest_cache) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(pets) = parsed.get("pets").and_then(|p| p.as_array()) {
                    if pets.len() > 100 {
                        return pets.iter().map(Pet::from_manifest_entry).collect();
                    }
                }
            }
        }
    }

    let client = http_client();
    match client.get(manifest_url).send() {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>() {
                let _ = fs::write(
                    &manifest_cache,
                    serde_json::to_string(&data).unwrap_or_default(),
                );
                let pets = data.get("pets").and_then(|p| p.as_array());
                pets.map(|arr| arr.iter().map(Pet::from_manifest_entry).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch manifest: {e}");
            Vec::new()
        }
    }
}

pub fn download_pet(app_data: &Path, pet: &Pet) -> Result<(PathBuf, bool), String> {
    let pet_dir = cache_dir(app_data).join(&pet.slug);
    fs::create_dir_all(&pet_dir).map_err(|e| e.to_string())?;

    let ss_path = pet_dir.join("spritesheet.webp");
    let pj_path = pet_dir.join("pet.json");
    let client = http_client();

    if let Some(url) = &pet.spritesheet_url {
        if !ss_path.exists() {
            let bytes = client
                .get(url)
                .send()
                .map_err(|e| e.to_string())?
                .bytes()
                .map_err(|e| e.to_string())?;
            fs::write(&ss_path, &bytes).map_err(|e| e.to_string())?;
        }
    }

    if let Some(url) = &pet.pet_json_url {
        if !pj_path.exists() {
            let bytes = client
                .get(url)
                .send()
                .map_err(|e| e.to_string())?
                .bytes()
                .map_err(|e| e.to_string())?;
            fs::write(&pj_path, &bytes).map_err(|e| e.to_string())?;
        }
    }

    if !pj_path.exists() {
        let fallback = serde_json::json!({
            "id": pet.slug,
            "displayName": if pet.display_name.is_empty() { pet.slug.clone() } else { pet.display_name.clone() },
            "description": "Downloaded from petdex.dev",
            "spritesheetPath": "spritesheet.webp"
        });
        fs::write(&pj_path, serde_json::to_string_pretty(&fallback).unwrap())
            .map_err(|e| e.to_string())?;
    }

    Ok((pet_dir, true))
}

pub fn get_downloaded_pets(app_data: &Path) -> Vec<Pet> {
    let cache = cache_dir(app_data);
    let mut pets = Vec::new();
    let Ok(entries) = fs::read_dir(&cache) else {
        return pets;
    };
    for entry in entries.flatten() {
        let d = entry.path();
        if d.is_dir() && d.join("spritesheet.webp").exists() {
            let slug = d
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let name = fs::read_to_string(d.join("pet.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| {
                    v.get("displayName")
                        .and_then(|s| s.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| slug.clone());
            pets.push(Pet {
                slug,
                display_name: name,
                source: Some("petdex".into()),
                info: None,
                spritesheet_url: None,
                pet_json_url: None,
                path: Some(d.to_string_lossy().into_owned()),
            });
        }
    }
    pets
}

pub fn get_installed_pets(hermes_pets_dir: &Path, app_data: &Path) -> Vec<Pet> {
    let mut pets = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if hermes_pets_dir.exists() {
        if let Ok(entries) = fs::read_dir(hermes_pets_dir) {
            for entry in entries.flatten() {
                let d = entry.path();
                if d.is_dir()
                    && !d
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.starts_with('.'))
                        .unwrap_or(true)
                    && d.join("spritesheet.webp").exists()
                {
                    let slug = d
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    let name = fs::read_to_string(d.join("pet.json"))
                        .ok()
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                        .and_then(|v| {
                            v.get("displayName")
                                .and_then(|s| s.as_str())
                                .map(String::from)
                        })
                        .unwrap_or_else(|| slug.clone());
                    pets.push(Pet {
                        slug: slug.clone(),
                        display_name: name,
                        source: Some("hermes".into()),
                        info: None,
                        spritesheet_url: None,
                        pet_json_url: None,
                        path: Some(d.to_string_lossy().into_owned()),
                    });
                    seen.insert(slug);
                }
            }
        }
    }

    for p in get_downloaded_pets(app_data) {
        if !seen.contains(&p.slug) {
            let mut p = p;
            p.source = Some("petdex".into());
            pets.push(p);
        }
    }
    pets
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tmpdir")
    }

    fn fake_manifest() -> String {
        let pets: Vec<serde_json::Value> = (0..150)
            .map(|i| {
                serde_json::json!({
                    "slug": format!("pet-{i}"),
                    "displayName": format!("Pet {i}"),
                    "spritesheetUrl": format!("https://x/{i}.webp")
                })
            })
            .collect();
        serde_json::json!({ "pets": pets }).to_string()
    }

    #[test]
    fn fetch_manifest_uses_cache_when_large() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/api/manifest");
            then.status(200).body(fake_manifest());
        });

        let dir = tmp();
        let app_data = dir.path();

        let first = fetch_manifest_with_url(app_data, &server.url("/api/manifest"), false);
        assert_eq!(first.len(), 150);
        assert_eq!(first[0].slug, "pet-0");
        assert_eq!(first[0].display_name, "Pet 0");
        assert_eq!(first[1].slug, "pet-1");
        m.assert_hits(1);

        let second = fetch_manifest_with_url(app_data, &server.url("/api/manifest"), false);
        assert_eq!(second.len(), 150);
        m.assert_hits(1);
    }

    #[test]
    fn fetch_manifest_force_refetches() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/api/manifest");
            then.status(200).body(fake_manifest());
        });

        let dir = tmp();
        let app_data = dir.path();
        let _ = fetch_manifest_with_url(app_data, &server.url("/api/manifest"), false);
        let _ = fetch_manifest_with_url(app_data, &server.url("/api/manifest"), true);
        m.assert_hits(2);
    }

    #[test]
    fn download_pet_writes_files() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/sprite.webp");
            then.status(200).body(&b"\x00\x01\x02"[..]);
        });
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/pet.json");
            then.status(200)
                .body(r#"{"id":"zero-2","displayName":"Zero"}"#);
        });

        let dir = tmp();
        let app_data = dir.path();
        let pet = Pet {
            slug: "zero-2".into(),
            display_name: "Zero".into(),
            source: None,
            info: None,
            spritesheet_url: Some(server.url("/sprite.webp")),
            pet_json_url: Some(server.url("/pet.json")),
            path: None,
        };
        let (pet_dir, ok) = download_pet(app_data, &pet).expect("download");
        assert!(ok);
        assert!(pet_dir.join("spritesheet.webp").exists());
        assert!(pet_dir.join("pet.json").exists());
    }

    #[test]
    fn download_pet_creates_fallback_pet_json() {
        let dir = tmp();
        let app_data = dir.path();
        let pet = Pet {
            slug: "lonely".into(),
            display_name: "Lonely".into(),
            source: None,
            info: None,
            spritesheet_url: None,
            pet_json_url: None,
            path: None,
        };
        let (pet_dir, ok) = download_pet(app_data, &pet).expect("download");
        assert!(ok);
        let pj = fs::read_to_string(pet_dir.join("pet.json")).unwrap();
        assert!(pj.contains("lonely"));
    }

    #[test]
    fn get_downloaded_pets_lists_cached() {
        let dir = tmp();
        let app_data = dir.path();
        let pet_dir = cache_dir(app_data).join("zero-2");
        fs::create_dir_all(&pet_dir).unwrap();
        fs::write(pet_dir.join("spritesheet.webp"), b"x").unwrap();
        fs::write(
            pet_dir.join("pet.json"),
            r#"{"id":"zero-2","displayName":"Zero"}"#,
        )
        .unwrap();

        let list = get_downloaded_pets(app_data);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].slug, "zero-2");
        assert_eq!(list[0].display_name, "Zero");
    }
}
