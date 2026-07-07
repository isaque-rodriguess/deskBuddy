"""Desktop Pet Zero — Menu estilo Hermes com scroll virtual."""
import tkinter as tk
import threading
import json
import os
import sys
import random
import math
import time
import shutil
from pathlib import Path
from PIL import Image, ImageTk, ImageDraw
import pystray

from sprite_parser import load_frames, LOOP_MS, FRAME_W, FRAME_H, STATE_ROWS
from petdex_api import fetch_manifest, download_pet, get_downloaded_pets, CACHE_DIR

HERMES_HOME = Path.home() / "AppData" / "Local" / "hermes"
PETS_DIR = HERMES_HOME / "pets"
CONFIG_PATH = Path(__file__).parent / "pet_config.json"

STRIDE_PER_LOOP = 0.8; REST_CHANCE = 0.62
PAUSE_MIN, PAUSE_MAX, PAUSE_MEAN = 1.5, 13.0, 4.2
STROLL_MIN_FRAC, STROLL_MIN_PX = 0.45, 110; STROLL_BIAS = 0.85; ARRIVE_EPS = 2.0
FPS = 60; GRAVITY_PX_S2 = 1800; FEET_DROP_PX = 4; TASKBAR_H = 48

C_BG = "#0d0d0d"; C_CARD = "#242424"; C_BORDER = "#333"
C_TEXT = "#e8e8e8"; C_MUTED = "#777"
C_ACCENT = "#6c63ff"; C_GREEN = "#22c55e"; C_RED = "#ef4444"; C_YELLOW = "#eab308"

STATE_MAP = {"idle": "idle", "run": "running", "run_right": "running-right",
    "run_left": "running-left", "wave": "waving", "jump": "jumping",
    "failed": "failed", "review": "review", "waiting": "waiting"}
IDLE_STATES = ["idle", "idle", "idle", "idle", "waving", "waiting", "review"]

ANIM_SPEED = {"idle": 1.6, "waving": 1.3, "waiting": 1.4, "review": 1.3,
    "run_right": 1.0, "run_left": 1.0, "jump": 0.8}

def exp_dwell():
    u = 1.0 - random.random(); return min(PAUSE_MAX, max(PAUSE_MIN, -math.log(u) * PAUSE_MEAN))

def get_installed_pets():
    pets, seen = [], set()
    for d in sorted(PETS_DIR.iterdir()) if PETS_DIR.exists() else []:
        if d.is_dir() and not d.name.startswith(".") and (d / "spritesheet.webp").exists():
            try:
                data = json.loads((d / "pet.json").read_text())
                pets.append({"slug": d.name, "name": data.get("displayName", d.name), "source": "hermes"})
            except: pets.append({"slug": d.name, "name": d.name, "source": "hermes"})
            seen.add(d.name)
    for p in get_downloaded_pets():
        if p["slug"] not in seen: p["source"] = "petdex"; pets.append(p); seen.add(p["slug"])
    return pets

def load_config():
    defaults = {"slug": "zero-2", "scale": 0.7, "enabled": True, "run_enabled": True}
    if CONFIG_PATH.exists():
        try: defaults.update(json.loads(CONFIG_PATH.read_text()))
        except: pass
    return defaults

def save_config(cfg): CONFIG_PATH.write_text(json.dumps(cfg, indent=2))

def make_tray_icon(slug="zero-2", size=64):
    try:
        frames = load_frames("idle", scale=size / FRAME_W)
        if frames: return frames[0].resize((size, size), Image.LANCZOS)
    except: pass
    img = Image.new("RGBA", (size, size), (30, 30, 30, 255))
    ImageDraw.Draw(img).ellipse([8, 8, size-8, size-8], fill=(108, 99, 255, 255))
    return img


class App:
    VISIBLE_COUNT = 50  # renderizar só os 50 primeiros

    def __init__(self):
        self.config = load_config()
        self.pet_slug = self.config.get("slug", "zero-2")
        self.scale = self.config.get("scale", 0.7)
        self.run_enabled = self.config.get("run_enabled", True)
        self.selected_slug = self.pet_slug
        self.local_pets, self.all_pets, self.downloaded_slugs = [], [], set()
        self.display_pets = []
        self._img_cache = {}

        self.root = tk.Tk()
        self.root.title("🐾 Desktop Pet")
        self.root.geometry("680x480")
        self.root.configure(bg=C_BG)
        self.root.minsize(500, 350)
        self.root.protocol("WM_DELETE_WINDOW", self._on_close)

        sw, sh = self.root.winfo_screenwidth(), self.root.winfo_screenheight()
        self.root.geometry(f"+{sw//2 - 340}+{sh//2 - 240}")

        # ── Pet Overlay ─────────────────────────────────────
        self.fw = int(FRAME_W * self.scale)
        self.fh = int(FRAME_H * self.scale)
        self.pet_win = tk.Toplevel(self.root)
        self.pet_win.overrideredirect(True)
        self.pet_win.attributes("-topmost", True)
        self.pet_bg = "#010101"
        self.pet_win.attributes("-transparentcolor", self.pet_bg)
        self.pet_win.config(bg=self.pet_bg)
        self.sw, self.sh = self.root.winfo_screenwidth(), self.root.winfo_screenheight()
        self.pos_x = 80.0
        self.ground_y = float(self.sh - TASKBAR_H - self.fh + FEET_DROP_PX)
        self.pos_y = self.ground_y
        self.pet_win.geometry(f"{self.fw}x{self.fh}+{int(self.pos_x)}+{int(self.pos_y)}")
        self.canvas = tk.Canvas(self.pet_win, width=self.fw, height=self.fh, bg=self.pet_bg, highlightthickness=0)
        self.canvas.pack()
        self.canvas.bind("<Button-1>", self._on_pet_press)
        self.canvas.bind("<B1-Motion>", self._on_pet_drag)
        self.canvas.bind("<ButtonRelease-1>", self._on_pet_release)

        self.sprite_cache, self.sprite_mirror = {}, {}
        self._load_sprites()
        self.current_state, self.anim_idx, self.anim_last = "idle", 0, time.time()
        self.image_on_canvas, self.mirror = None, False
        self.vel_y = 0.0
        self._pet_dragging = False; self._pet_drag_ox = 0; self._pet_drag_oy = 0
        self.phase, self.pause_until = "pause", time.time() + 2.0
        self.walk_target, self.walk_speed, self.roam_dir = self.pos_x, 0.0, 0

        # ── Settings (estilo Hermes) ───────────────────────
        c = tk.Frame(self.root, bg=C_BG); c.pack(fill="both", expand=True)
        cs = tk.Canvas(c, bg=C_BG, highlightthickness=0)
        scrollbar = tk.Scrollbar(c, orient="vertical", command=cs.yview)
        self.scroll_frame = tk.Frame(cs, bg=C_BG)
        self.scroll_frame.bind("<Configure>", lambda e: cs.configure(scrollregion=cs.bbox("all")))
        cs.create_window((0, 0), window=self.scroll_frame, anchor="nw", width=640)
        cs.configure(yscrollcommand=scrollbar.set)
        cs.pack(side="left", fill="both", expand=True)
        scrollbar.pack(side="right", fill="y")
        cs.bind("<Enter>", lambda e: cs.bind_all("<MouseWheel>", lambda ev: cs.yview_scroll(-1*(ev.delta//120), "units")))
        cs.bind("<Leave>", lambda e: cs.unbind_all("<MouseWheel>"))
        p = self.scroll_frame

        # Header
        tk.Label(p, text="🐾  Pet", fg=C_TEXT, bg=C_BG, font=("Segoe UI", 16, "bold"), anchor="w").pack(fill="x", padx=20, pady=(16, 2))
        tk.Label(p, text="Choose an animated petdex mascot that floats over your desktop\nand reacts with animations.", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 9), anchor="w", justify="left").pack(fill="x", padx=20, pady=(0, 12))

        # Choose a pet
        hdr = tk.Frame(p, bg=C_BG); hdr.pack(fill="x", padx=20, pady=(0, 6))
        tk.Label(hdr, text="Choose a pet", fg=C_TEXT, bg=C_BG, font=("Segoe UI", 12, "bold"), anchor="w").pack(side="left")
        # On/Off
        self.enabled_var = tk.BooleanVar(value=True)
        tg = tk.Frame(hdr, bg=C_BG); tg.pack(side="right")
        self.tog_off = tk.Label(tg, text=" Off ", fg=C_MUTED, bg=C_CARD, font=("Segoe UI", 8), cursor="hand2", padx=6, pady=2)
        self.tog_off.pack(side="left")
        self.tog_on = tk.Label(tg, text=" On ", fg="white", bg=C_ACCENT, font=("Segoe UI", 8, "bold"), cursor="hand2", padx=6, pady=2)
        self.tog_on.pack(side="left")
        def tog_en():
            v = not self.enabled_var.get(); self.enabled_var.set(v)
            if v: self.tog_off.config(fg=C_MUTED, bg=C_CARD); self.tog_on.config(fg="white", bg=C_ACCENT)
            else: self.tog_off.config(fg="white", bg=C_RED); self.tog_on.config(fg=C_MUTED, bg=C_CARD)
        self.tog_off.bind("<Button-1>", lambda e: tog_en())
        self.tog_on.bind("<Button-1>", lambda e: tog_en())
        tk.Label(p, text="Picking one installs it (if needed) and makes it active.", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 8), anchor="w").pack(fill="x", padx=20, pady=(0, 8))

        # Search
        self.search_entry = tk.Entry(p, bg=C_CARD, fg=C_TEXT, insertbackground=C_TEXT, font=("Segoe UI", 10), highlightthickness=1, highlightcolor=C_ACCENT, relief="flat")
        self.search_entry.pack(fill="x", padx=20, ipady=6, pady=(0, 8))
        self.search_entry.insert(0, "Search pets...")
        self.search_entry.bind("<FocusIn>", lambda e: (self.search_entry.delete(0, "end"),) if self.search_entry.get() == "Search pets..." else None)
        self.search_entry.bind("<FocusOut>", lambda e: (self.search_entry.insert(0, "Search pets..."),) if not self.search_entry.get() else None)
        self.search_entry.bind("<KeyRelease>", lambda e: self._render_grid())

        # Grid 2 colunas
        self.grid_frame = tk.Frame(p, bg=C_BG)
        self.grid_frame.pack(fill="x", padx=20)
        self.grid_frame.columnconfigure(0, weight=1)
        self.grid_frame.columnconfigure(1, weight=1)
        self.grid_status = tk.Label(p, text="Loading...", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 8), anchor="w")
        self.grid_status.pack(fill="x", padx=20, pady=(6, 0))

        # Size
        tk.Frame(p, height=1, bg=C_BORDER).pack(fill="x", padx=20, pady=(16, 0))
        sz_h = tk.Frame(p, bg=C_BG); sz_h.pack(fill="x", padx=20, pady=(12, 4))
        tk.Label(sz_h, text="Size", fg=C_TEXT, bg=C_BG, font=("Segoe UI", 12, "bold"), anchor="w").pack(side="left")
        self.scale_var = tk.DoubleVar(value=int(self.scale * 100))
        self.scale_pct = tk.Label(sz_h, text=f"{int(self.scale*100)}%", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 10))
        self.scale_pct.pack(side="right")
        tk.Label(p, text="Resize the floating mascot.", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 8), anchor="w").pack(fill="x", padx=20, pady=(0, 4))
        tk.Scale(p, from_=30, to=150, resolution=5, orient="horizontal", variable=self.scale_var, command=lambda v: self.scale_pct.config(text=f"{int(float(v))}%"), bg=C_BG, fg=C_TEXT, troughcolor=C_CARD, highlightthickness=0, sliderlength=16, length=520).pack(fill="x", padx=20, pady=(0, 8))

        # Roam
        tk.Frame(p, height=1, bg=C_BORDER).pack(fill="x", padx=20, pady=(4, 0))
        rm_h = tk.Frame(p, bg=C_BG); rm_h.pack(fill="x", padx=20, pady=(12, 4))
        tk.Label(rm_h, text="Roam", fg=C_TEXT, bg=C_BG, font=("Segoe UI", 12, "bold"), anchor="w").pack(side="left")
        self.run_var = tk.BooleanVar(value=True)
        rm_t = tk.Frame(rm_h, bg=C_BG); rm_t.pack(side="right")
        rm_off = tk.Label(rm_t, text=" Off ", fg=C_MUTED, bg=C_CARD, font=("Segoe UI", 8), cursor="hand2", padx=6, pady=2)
        rm_off.pack(side="left")
        rm_on = tk.Label(rm_t, text=" On ", fg="white", bg=C_ACCENT, font=("Segoe UI", 8, "bold"), cursor="hand2", padx=6, pady=2)
        rm_on.pack(side="left")
        def tog_rm():
            v = not self.run_var.get(); self.run_var.set(v)
            if v: rm_off.config(fg=C_MUTED, bg=C_CARD); rm_on.config(fg="white", bg=C_ACCENT)
            else: rm_off.config(fg="white", bg=C_RED); rm_on.config(fg=C_MUTED, bg=C_CARD)
        rm_off.bind("<Button-1>", lambda e: tog_rm())
        rm_on.bind("<Button-1>", lambda e: tog_rm())
        tk.Label(p, text="Let the pet wander the window on its own while idle.", fg=C_MUTED, bg=C_BG, font=("Segoe UI", 8), anchor="w").pack(fill="x", padx=20, pady=(0, 12))

        # Apply
        ab = tk.Label(p, text="✓  Apply Changes", fg="white", bg=C_ACCENT, font=("Segoe UI", 10, "bold"), cursor="hand2", padx=16, pady=8)
        ab.pack(fill="x", padx=20, pady=(0, 20))
        ab.bind("<Button-1>", lambda e: self._apply())
        ab.bind("<Enter>", lambda e: ab.config(bg="#5a52dd"))
        ab.bind("<Leave>", lambda e: ab.config(bg=C_ACCENT))

        self._refresh_local()
        threading.Thread(target=self._load_manifest, daemon=True).start()
        self._tick()

    def _load_sprites(self):
        self.sprite_cache, self.sprite_mirror = {}, {}
        ss = PETS_DIR / self.pet_slug / "spritesheet.webp"
        if not ss.exists(): ss = CACHE_DIR / self.pet_slug / "spritesheet.webp"
        if not ss.exists(): return
        for s in STATE_ROWS:
            try:
                pil = load_frames(s, scale=self.scale, sheet_path=ss)
                if pil:
                    self.sprite_cache[s] = [ImageTk.PhotoImage(f) for f in pil]
                    self.sprite_mirror[s] = [ImageTk.PhotoImage(f.transpose(Image.FLIP_LEFT_RIGHT)) for f in pil]
            except: pass

    def _get_frames(self, state, mirror=False):
        row = STATE_MAP.get(state, "idle")
        return (self.sprite_mirror if mirror else self.sprite_cache).get(row, self.sprite_cache.get("idle", []))

    def _change_pet(self, slug, scale=None):
        self.pet_slug = slug
        if scale is not None: self.scale = scale
        self.fw, self.fh = int(FRAME_W * self.scale), int(FRAME_H * self.scale)
        self.ground_y = float(self.sh - TASKBAR_H - self.fh + FEET_DROP_PX)
        self.pos_y = self.ground_y
        self.pet_win.geometry(f"{self.fw}x{self.fh}")
        self.canvas.config(width=self.fw, height=self.fh)
        self._load_sprites()
        self.image_on_canvas, self.anim_idx = None, 0

    def _apply(self):
        s = self.scale_var.get() / 100.0; e = self.enabled_var.get(); r = self.run_var.get()
        save_config({"slug": self.selected_slug, "scale": s, "enabled": e, "run_enabled": r})
        self._change_pet(self.selected_slug, s)
        self.run_enabled = r

    def _refresh_local(self):
        self.local_pets = get_installed_pets()
        self.downloaded_slugs = {p["slug"] for p in self.local_pets if p.get("source") == "petdex"}
        manifest_map = {p.get("slug"): p for p in self.all_pets}
        for p in self.local_pets:
            if p["slug"] in manifest_map: p["info"] = manifest_map[p["slug"]]
        self._render_grid()

    def _render_grid(self):
        for w in self.grid_frame.winfo_children(): w.destroy()
        d = [{**p, "downloaded": True} for p in self.local_pets]
        for p in self.all_pets:
            if p.get("slug") not in self.downloaded_slugs:
                d.append({"slug": p.get("slug", ""), "name": p.get("displayName", "?"), "downloaded": False, "info": p})
        try: q = self.search_entry.get().lower().strip()
        except: q = ""
        if q and q != "search pets...": d = [p for p in d if q in p.get("name", "").lower() or q in p.get("slug", "").lower()]
        self.display_pets = d
        total = len(self.local_pets) + len(self.all_pets)
        showing = min(len(d), self.VISIBLE_COUNT)
        self.grid_status.config(text=f"Showing {showing} of {total} — type to narrow it down.")
        # Só renderizar os primeiros VISIBLE_COUNT em 2 colunas
        for i, p in enumerate(d[:self.VISIBLE_COUNT]):
            self._card(self.grid_frame, p, i // 2, i % 2)

    def _card(self, parent, pet, row, col):
        s, dl = pet["slug"], pet.get("downloaded", False)
        sel = s == self.selected_slug
        bg = "#1a1a3a" if sel else C_CARD
        border = C_ACCENT if sel else C_BORDER

        f = tk.Frame(parent, bg=bg, cursor="hand2", highlightthickness=1, highlightcolor=border)
        f.grid(row=row, column=col, padx=2, pady=2, sticky="nsew")

        av = tk.Canvas(f, bg=bg, highlightthickness=0, width=40, height=40)
        av.pack(side="left", padx=(8, 10), pady=6)
        if s in self._img_cache:
            self._set_cvs(av, self._img_cache[s], 20, 20)
        else:
            def lp(canvas_ref=av, slug=s, info=pet.get("info", {})):
                try:
                    ss = PETS_DIR / slug / "spritesheet.webp"
                    if not ss.exists(): ss = CACHE_DIR / slug / "spritesheet.webp"
                    if not ss.exists():
                        url = info.get("spritesheetUrl", "")
                        if url:
                            CACHE_DIR.mkdir(exist_ok=True); ss = CACHE_DIR / f"{slug}_p.webp"
                            if not ss.exists():
                                import urllib.request
                                with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "P/1"}), timeout=15) as r:
                                    ss.write_bytes(r.read())
                    if ss.exists():
                        fr = load_frames("idle", scale=0.25, sheet_path=ss)
                        if fr:
                            img = ImageTk.PhotoImage(fr[0])
                            self._img_cache[slug] = img
                            self.root.after(0, lambda c=canvas_ref, i=img: self._set_cvs(c, i, 20, 20))
                except: pass
            threading.Thread(target=lp, daemon=True).start()

        txt = tk.Frame(f, bg=bg); txt.pack(side="left", fill="x", expand=True, pady=6)
        name = pet["name"][:25]
        tk.Label(txt, text=name, fg="white" if sel else C_TEXT, bg=bg, font=("Segoe UI", 10, "bold" if sel else "normal"), anchor="w").pack(fill="x")
        sub = s + (" · installed" if dl else "")
        tk.Label(txt, text=sub, fg=C_MUTED, bg=bg, font=("Segoe UI", 8), anchor="w").pack(fill="x")

        def on_click(e, x=s): self._clk(x)
        for w in [f, txt] + txt.winfo_children():
            try: w.bind("<Button-1>", on_click)
            except: pass

        if sel and dl:
            tr = tk.Label(f, text="🗑", fg=C_RED, bg=bg, font=("Segoe UI", 10), cursor="hand2")
            tr.pack(side="right", padx=8)
            tr.bind("<Button-1>", lambda e: self._delete())

    def _set_cvs(self, c, img, x=35, y=25): c._img = img; c.delete("all"); c.create_image(x, y, image=img)

    def _clk(self, s):
        if s in self.downloaded_slugs: self.selected_slug = s; self._apply(); self._render_grid()
        else:
            self.grid_status.config(text="Downloading...", fg=C_YELLOW)
            def dl():
                info = next((p for p in self.all_pets if p.get("slug") == s), None)
                if info:
                    _, ok = download_pet(info)
                    def after():
                        if ok: self.downloaded_slugs.add(s); self.selected_slug = s; self._refresh_local(); self.grid_status.config(text="✓ Done!", fg=C_GREEN)
                        else: self.grid_status.config(text="✗ Failed", fg=C_RED)
                    self.root.after(0, after)
            threading.Thread(target=dl, daemon=True).start()

    def _delete(self):
        s = self.selected_slug
        n = next((p["name"] for p in self.local_pets if p["slug"] == s), s)
        confirm = tk.Toplevel(self.root)
        confirm.title("Delete"); confirm.geometry("280x120"); confirm.configure(bg=C_BG)
        confirm.attributes("-topmost", True); confirm.transient(self.root); confirm.grab_set()
        sw = self.root.winfo_screenwidth()
        confirm.geometry(f"+{sw//2 - 140}+{self.root.winfo_y() + 100}")
        tk.Label(confirm, text=f"Delete {n}?", fg=C_TEXT, bg=C_BG, font=("Segoe UI", 11, "bold")).pack(pady=(15, 8))
        bf = tk.Frame(confirm, bg=C_BG); bf.pack()
        def do():
            d1, d2 = CACHE_DIR / s, PETS_DIR / s
            if d1.exists(): shutil.rmtree(d1)
            if d2.exists(): shutil.rmtree(d2)
            confirm.destroy()
            if s == self.pet_slug: self.selected_slug = "zero-2"; self._apply()
            self._refresh_local()
        cancel = tk.Label(bf, text="Cancel", fg=C_TEXT, bg=C_CARD, font=("Segoe UI", 9), cursor="hand2", padx=12, pady=4)
        cancel.pack(side="left", padx=(0, 6)); cancel.bind("<Button-1>", lambda e: confirm.destroy())
        delete = tk.Label(bf, text="Delete", fg="white", bg=C_RED, font=("Segoe UI", 9, "bold"), cursor="hand2", padx=12, pady=4)
        delete.pack(side="left"); delete.bind("<Button-1>", lambda e: do())

    def _load_manifest(self):
        self.all_pets = fetch_manifest()
        self.root.after(0, self._refresh_local)

    # ── Animation ───────────────────────────────────────────
    def _tick(self):
        try:
            now = time.time()
            dt = min(0.05, now - getattr(self, '_last_tick', now))
            self._last_tick = now
            if self.run_enabled and not self._pet_dragging: self._roam(now, dt)
            self.ground_y = float(self.sh - TASKBAR_H - self.fh + FEET_DROP_PX)
            frames = self._get_frames(self.current_state, mirror=self.mirror)
            if frames:
                speed = ANIM_SPEED.get(self.current_state, 1.0)
                frame_ms = (LOOP_MS / max(1, len(frames))) * speed
                if now - self.anim_last >= frame_ms / 1000.0:
                    self.anim_idx = (self.anim_idx + 1) % len(frames); self.anim_last = now
                img = frames[self.anim_idx]
                if self.image_on_canvas:
                    self.canvas.coords(self.image_on_canvas, self.fw//2, self.fh//2)
                    self.canvas.itemconfig(self.image_on_canvas, image=img)
                else:
                    self.image_on_canvas = self.canvas.create_image(self.fw//2, self.fh//2, image=img)
            self.pet_win.geometry(f"+{int(self.pos_x)}+{int(self.pos_y)}")
            self.root.after(1, self._tick)
        except tk.TclError: pass

    def _roam(self, now, dt):
        if self.pos_y < self.ground_y - 1:
            self.vel_y += GRAVITY_PX_S2 * dt; self.pos_y += self.vel_y * dt
            if self.pos_y >= self.ground_y: self.pos_y = self.ground_y; self.vel_y = 0; self._land()
            self.current_state = "jump"; return
        if self.phase == "pause" and now >= self.pause_until: self._plan()
        elif self.phase == "walk":
            rem = self.walk_target - self.pos_x; step = self.walk_speed * dt
            if abs(rem) <= max(ARRIVE_EPS, step): self.pos_x = self.walk_target; self._begin_pause()
            else: self.pos_x += (1 if rem > 0 else -1) * step; self.roam_dir = 1 if rem > 0 else -1
        self.pos_x = max(10, min(self.sw - self.fw - 10, self.pos_x))
        if self.phase == "walk": self.current_state = "run_right"; self.mirror = (self.roam_dir < 0)
        else: self.current_state = "idle"; self.mirror = (self.pos_x + self.fw/2) < (self.sw/2)

    def _land(self):
        self.phase = "pause"; self.roam_dir = 0
        self.current_state = random.choice(IDLE_STATES)
        self.anim_idx, self.anim_last = 0, time.time()
        self.pause_until = time.time() + 0.8

    def _pick_target(self):
        m = 10; lb, rb = m, max(m, self.sw - self.fw - m); sp = rb - lb
        if sp <= 4: return self.pos_x
        rl, rr = self.pos_x - lb, rb - self.pos_x
        go_r = (random.random() < STROLL_BIAS) == (rr >= rl)
        room = max(0, rr if go_r else rl); md = min(room, max(sp * STROLL_MIN_FRAC, STROLL_MIN_PX))
        return self.pos_x + (md + random.random() * max(0, room - md)) * (1 if go_r else -1)

    def _begin_pause(self):
        self.phase, self.roam_dir = "pause", 0
        self.current_state = random.choice(IDLE_STATES)
        self.anim_idx, self.anim_last = 0, time.time()
        self.pause_until = time.time() + exp_dwell()

    def _begin_walk(self):
        t = self._pick_target(); diff = t - self.pos_x
        if abs(diff) < ARRIVE_EPS: self._begin_pause(); return
        self.walk_target, self.phase = t, "walk"
        self.walk_speed = self.fw * STRIDE_PER_LOOP / (LOOP_MS / 1000.0)
        self.roam_dir = 1 if diff > 0 else -1

    def _plan(self): (self._begin_pause if random.random() < REST_CHANCE else self._begin_walk)()

    def _on_pet_press(self, e):
        self._pet_dragging = True; self._pet_drag_ox = e.x_root - self.pos_x; self._pet_drag_oy = e.y_root - self.pos_y; self.vel_y = 0
    def _on_pet_drag(self, e):
        if not self._pet_dragging: return
        self.pos_x = max(0, min(self.sw - self.fw, e.x_root - self._pet_drag_ox))
        self.pos_y = max(0, min(self.sh - self.fh, e.y_root - self._pet_drag_oy))
    def _on_pet_release(self, e):
        self._pet_dragging = False
        if self.pos_y < self.ground_y - 1: self.vel_y = 0
        else: self.pos_y = self.ground_y; self._begin_pause(); self.pause_until = time.time() + 0.8

    def _on_close(self):
        save_config({"slug": self.pet_slug, "scale": self.scale, "enabled": True, "run_enabled": self.run_enabled})
        self.root.destroy(); sys.exit(0)

    def run(self): self.root.mainloop()


def run_tray(app):
    def on_toggle(i, item):
        if app.root.winfo_viewable(): app.root.withdraw()
        else: app.root.deiconify()
    def on_quit(i, item):
        icon.stop(); app.root.after(100, app._on_close)
    icon = pystray.Icon("Pet", make_tray_icon(app.pet_slug, 64), "Desktop Pet",
                        pystray.Menu(pystray.MenuItem("Show/Hide", on_toggle), pystray.Menu.SEPARATOR, pystray.MenuItem("Quit", on_quit)))
    icon.run()


if __name__ == "__main__":
    config = load_config()
    if not config.get("enabled", True): sys.exit(0)
    app = App()
    threading.Thread(target=run_tray, args=(app,), daemon=True).start()
    app.run()
