"""petdex_api — Fetch + cache the petdex manifest and download pets."""
import json
import urllib.request
import urllib.error
from pathlib import Path
from functools import lru_cache

MANIFEST_URL = "https://petdex.dev/api/manifest"
CACHE_DIR = Path(__file__).parent / "petdex_cache"
MANIFEST_CACHE = CACHE_DIR / "manifest.json"


def fetch_manifest(force=False):
    """Busca o manifest do petdex (com cache local)."""
    CACHE_DIR.mkdir(exist_ok=True)

    if not force and MANIFEST_CACHE.exists():
        try:
            data = json.loads(MANIFEST_CACHE.read_text(encoding="utf-8"))
            if "pets" in data and len(data["pets"]) > 100:
                return data["pets"]
        except Exception:
            pass

    try:
        req = urllib.request.Request(MANIFEST_URL, headers={"User-Agent": "DesktopPet/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        pets = data.get("pets", [])
        MANIFEST_CACHE.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
        return pets
    except Exception as e:
        print(f"⚠ Failed to fetch manifest: {e}")
        return []


def download_pet(pet_info):
    """Baixa spritesheet + pet.json de um pet do petdex.
    
    Returns: (pet_dir_path, success_bool)
    """
    slug = pet_info["slug"]
    pet_dir = CACHE_DIR / slug
    pet_dir.mkdir(exist_ok=True)

    ss_path = pet_dir / "spritesheet.webp"
    pj_path = pet_dir / "pet.json"

    try:
        # Download spritesheet
        ss_url = pet_info.get("spritesheetUrl", "")
        if ss_url and not ss_path.exists():
            req = urllib.request.Request(ss_url, headers={"User-Agent": "DesktopPet/1.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                ss_path.write_bytes(resp.read())

        # Download pet.json
        pj_url = pet_info.get("petJsonUrl", "")
        if pj_url and not pj_path.exists():
            req = urllib.request.Request(pj_url, headers={"User-Agent": "DesktopPet/1.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                pj_path.write_bytes(resp.read())

        # Create pet.json if missing
        if not pj_path.exists():
            pj_path.write_text(json.dumps({
                "id": slug,
                "displayName": pet_info.get("displayName", slug),
                "description": f"Downloaded from petdex.dev",
                "spritesheetPath": "spritesheet.webp"
            }, indent=2))

        return pet_dir, True
    except Exception as e:
        print(f"⚠ Failed to download {slug}: {e}")
        return pet_dir, False


def get_downloaded_pets():
    """Lista pets já baixados no cache local."""
    pets = []
    if not CACHE_DIR.exists():
        return pets
    for d in CACHE_DIR.iterdir():
        if d.is_dir() and (d / "spritesheet.webp").exists():
            pj = d / "pet.json"
            try:
                data = json.loads(pj.read_text()) if pj.exists() else {}
                pets.append({
                    "slug": d.name,
                    "name": data.get("displayName", d.name),
                    "path": str(d),
                })
            except Exception:
                pets.append({"slug": d.name, "name": d.name, "path": str(d)})
    return pets
