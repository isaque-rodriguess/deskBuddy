"""Extrai frames do spritesheet do Zero — replica o renderer Hermes/ petdex.

Frame geometry: 192×208px (constantes oficiais do agent.pet.constants)
Sheet: 1536×1872 = 8 cols × 9 rows
Loop: 1100ms / frames_per_state (~183ms/frame)
"""
from PIL import Image
from pathlib import Path

# Caminho do spritesheet
SPRITESHEET = Path.home() / "AppData" / "Local" / "hermes" / "pets" / "zero-2" / "spritesheet.webp"

# Geometria oficial (agent.pet.constants)
FRAME_W = 192
FRAME_H = 208
FRAMES_PER_STATE = 6
LOOP_MS = 1100  # duração total do loop em ms

# Row taxonomy — Codex format (9 rows)
# (agent.pet.constants.CODEX_STATE_ROWS)
STATE_ROWS = [
    "idle",           # row 0
    "running-right",  # row 1
    "running-left",   # row 2
    "waving",         # row 3
    "jumping",        # row 4
    "failed",         # row 5
    "waiting",        # row 6
    "running",        # row 7
    "review",         # row 8
]

# Aliases → PetState (agent.pet.constants.STATE_ALIASES + ROW_TO_STATE)
ALIASES = {
    "idle":        "idle",
    "wave":        "wave",
    "waving":      "wave",
    "jump":        "jump",
    "jumping":     "jump",
    "run":         "run",
    "running":     "run",
    "running-right": "run",
    "running-left":  "run",
    "failed":      "failed",
    "review":      "review",
    "waiting":     "waiting",
}


def state_row_index(state: str, row_count: int = 9) -> int:
    """Retorna o índice da row pra um estado (com aliases)."""
    # Tentar aliases primeiro
    canonical = ALIASES.get(state, state)
    for i, row_name in enumerate(STATE_ROWS[:row_count]):
        if row_name == state or ALIASES.get(row_name) == canonical:
            return i
    return 0  # fallback idle


def load_frames(state: str = "idle", scale: float = 1.0, sheet_path=None) -> list[Image.Image]:
    """Extrai FRAMES_PER_STATE frames de uma row do spritesheet.
    
    Args:
        state: nome do estado
        scale: fator de escala
        sheet_path: path customizado pro spritesheet (default: zero-2)
    """
    if sheet_path is None:
        sheet_path = SPRITESHEET
    else:
        sheet_path = Path(sheet_path)
    sheet = Image.open(sheet_path).convert("RGBA")
    sheet_w, sheet_h = sheet.size

    rows = max(1, sheet_h // FRAME_H)
    row_idx = state_row_index(state, rows)
    top = row_idx * FRAME_H

    # Clamp (replica o Python render.py)
    if top + FRAME_H > sheet_h:
        top = max(0, sheet_h - FRAME_H)

    cols = max(1, sheet_w // FRAME_W)

    frames = []
    for i in range(min(FRAMES_PER_STATE, cols)):
        left = i * FRAME_W
        frame = sheet.crop((left, top, left + FRAME_W, top + FRAME_H))

        # Trim transparent padding (replica _frame_is_blank do render.py)
        alpha = frame.getchannel("A")
        extrema = alpha.getextrema()
        if extrema[1] <= 8:  # max alpha <= 8 → frame vazio
            break

        if scale != 1.0:
            new_w = int(FRAME_W * scale)
            new_h = int(FRAME_H * scale)
            frame = frame.resize((new_w, new_h), Image.LANCZOS)

        frames.append(frame)

    return frames


def get_frame_size(scale: float = 1.0) -> tuple[int, int]:
    """Retorna (largura, altura) de um frame na escala dada."""
    return (int(FRAME_W * scale), int(FRAME_H * scale))


def frame_interval_ms(state: str = "idle") -> float:
    """Retorna o intervalo em ms entre frames pra um estado (loopMs / count)."""
    frames = load_frames(state, scale=1.0)
    count = max(1, len(frames))
    return LOOP_MS / count


if __name__ == "__main__":
    print(f"Sheet: {SPRITESHEET}")
    print(f"Frame: {FRAME_W}×{FRAME_H}, {FRAMES_PER_STATE} frames/state, loop={LOOP_MS}ms")
    print()
    for state_name in STATE_ROWS:
        frames = load_frames(state_name)
        interval = frame_interval_ms(state_name)
        print(f"  {state_name:>15}: {len(frames)} frames, {interval:.0f}ms/frame, size={frames[0].size if frames else 'N/A'}")
