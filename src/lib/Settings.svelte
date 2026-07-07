<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  type PetInfo = {
    slug: string;
    name: string;
    source: string;
    downloaded: boolean;
  };

  type PetConfig = {
    slug: string;
    scale: number;
    enabled: boolean;
    run_enabled: boolean;
  };

  const C_BG = "#0d0d0d";
  const C_CARD = "#242424";
  const C_BORDER = "#333";
  const C_TEXT = "#e8e8e8";
  const C_MUTED = "#777";
  const C_ACCENT = "#6c63ff";
  const C_GREEN = "#22c55e";
  const C_RED = "#ef4444";
  const C_YELLOW = "#eab308";

  const VISIBLE_COUNT = 50;

  let localPets: PetInfo[] = [];
  let allPets: PetInfo[] = [];
  let displayPets: PetInfo[] = [];
  let searchQuery = "";
  let gridStatus = "Loading...";
  let gridStatusColor = C_MUTED;

  let selectedSlug = "";
  let activeSlug = "";

  let enabled = true;
  let runEnabled = true;
  let scalePct = 70;

  let avatars: Record<string, string> = {};
  let pendingAvatars = new Set<string>();
  let showDeleteModal = false;
  let deleteTarget: PetInfo | null = null;
  let isApplying = false;

  let searchTouched = false;

  async function loadConfig() {
    const cfg = await invoke<PetConfig>("get_config");
    selectedSlug = cfg.slug;
    activeSlug = cfg.slug;
    enabled = cfg.enabled;
    runEnabled = cfg.run_enabled;
    scalePct = Math.round(cfg.scale * 100);
  }

  async function loadLocal() {
    localPets = await invoke<PetInfo[]>("list_local_pets");
    renderGrid();
  }

  async function loadAll() {
    gridStatus = "Fetching manifest...";
    gridStatusColor = C_YELLOW;
    try {
      allPets = await invoke<PetInfo[]>("list_all_pets");
      gridStatus = `Loaded ${allPets.length} pets from petdex`;
      gridStatusColor = C_MUTED;
    } catch (e) {
      gridStatus = "Failed to fetch manifest";
      gridStatusColor = C_RED;
    }
    renderGrid();
  }

  async function loadAvatar(slug: string) {
    if (avatars[slug] || pendingAvatars.has(slug)) return;
    pendingAvatars.add(slug);
    try {
      const url = await invoke<string | null>("get_avatar", { slug });
      if (url) {
        avatars[slug] = url;
        avatars = { ...avatars };
      }
    } finally {
      pendingAvatars.delete(slug);
    }
  }

  function renderGrid() {
    const local = localPets.map((p) => ({ ...p, downloaded: true }));
    const downloadedSlugs = new Set(local.map((p) => p.slug));
    const remote = allPets
      .filter((p) => !downloadedSlugs.has(p.slug))
      .map((p) => ({ ...p, downloaded: false }));
    let combined = [...local, ...remote];

    if (searchQuery && searchQuery !== "Search pets...") {
      const q = searchQuery.toLowerCase().trim();
      combined = combined.filter(
        (p) =>
          p.name.toLowerCase().includes(q) || p.slug.toLowerCase().includes(q),
      );
    }

    displayPets = combined.slice(0, VISIBLE_COUNT);
    const total = localPets.length + allPets.length;
    gridStatus = `Showing ${Math.min(combined.length, VISIBLE_COUNT)} of ${total} — type to narrow it down.`;
    gridStatusColor = C_MUTED;

    for (const p of displayPets) {
      loadAvatar(p.slug);
    }
  }

  function onSearchInput() {
    searchQuery = searchEntryEl?.value ?? "";
    renderGrid();
  }

  let searchEntryEl: HTMLInputElement | undefined;

  function onSearchFocus() {
    if (searchEntryEl && searchEntryEl.value === "Search pets...") {
      searchEntryEl.value = "";
      searchTouched = true;
    }
  }

  function onSearchBlur() {
    if (searchEntryEl && searchEntryEl.value === "" && searchTouched) {
      searchEntryEl.value = "Search pets...";
    }
  }

  async function onCardClick(pet: PetInfo) {
    if (pet.downloaded) {
      selectedSlug = pet.slug;
      renderGrid();
    } else {
      gridStatus = `Downloading ${pet.name}...`;
      gridStatusColor = C_YELLOW;
      try {
        const ok = await invoke<boolean>("download_pet_cmd", { slug: pet.slug });
        if (ok) {
          await loadLocal();
          await loadAll();
          selectedSlug = pet.slug;
          gridStatus = `✓ Downloaded ${pet.name}!`;
          gridStatusColor = C_GREEN;
        } else {
          gridStatus = "✗ Download failed";
          gridStatusColor = C_RED;
        }
      } catch (e) {
        gridStatus = `✗ ${e}`;
        gridStatusColor = C_RED;
      }
      renderGrid();
    }
  }

  function onTrashClick(pet: PetInfo, e: Event) {
    e.stopPropagation();
    deleteTarget = pet;
    showDeleteModal = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    try {
      await invoke("delete_pet_cmd", { slug: deleteTarget.slug });
      if (deleteTarget.slug === activeSlug) {
        selectedSlug = "zero-2";
      }
      showDeleteModal = false;
      deleteTarget = null;
      await loadLocal();
      await loadAll();
      renderGrid();
    } catch (e) {
      gridStatus = `✗ Delete failed: ${e}`;
      gridStatusColor = C_RED;
    }
  }

  function cancelDelete() {
    showDeleteModal = false;
    deleteTarget = null;
  }

  async function onApply() {
    isApplying = true;
    try {
      await invoke("apply_config", {
        args: {
          slug: selectedSlug,
          scale: scalePct / 100,
          enabled,
          run_enabled: runEnabled,
        },
      });
      activeSlug = selectedSlug;
      gridStatus = "✓ Applied!";
      gridStatusColor = C_GREEN;
    } catch (e) {
      gridStatus = `✗ Apply failed: ${e}`;
      gridStatusColor = C_RED;
    } finally {
      isApplying = false;
    }
  }

  onMount(async () => {
    await loadConfig();
    await loadLocal();
    await loadAll();
    if (searchEntryEl) searchEntryEl.value = "Search pets...";
  });
</script>

<main style="background:{C_BG}; color:{C_TEXT}; font-family: 'Segoe UI', Inter, system-ui, sans-serif; height: 100vh; overflow-y: auto;">
  <div style="padding: 0 20px;">
    <!-- Header -->
    <h1 style="font-size: 16px; font-weight: bold; margin: 16px 0 2px; text-align: left;">🐾 Pet</h1>
    <p style="color: {C_MUTED}; font-size: 9px; margin: 0 0 12px; line-height: 1.4;">
      Choose an animated petdex mascot that floats over your desktop<br>and reacts with animations.
    </p>

    <!-- Choose a pet + On/Off toggle -->
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
      <span style="font-size: 12px; font-weight: bold;">Choose a pet</span>
      <div style="display: flex; gap: 2px;">
        <button
          style="font-size: 8px; padding: 2px 6px; cursor: pointer; border: none; color: {!enabled ? 'white' : C_MUTED}; background: {!enabled ? C_RED : C_CARD};"
          onclick={() => enabled = false}
        > Off </button>
        <button
          style="font-size: 8px; padding: 2px 6px; font-weight: bold; cursor: pointer; border: none; color: white; background: {enabled ? C_ACCENT : C_CARD};"
          onclick={() => enabled = true}
        > On </button>
      </div>
    </div>
    <p style="color: {C_MUTED}; font-size: 8px; margin: 0 0 8px;">Picking one installs it (if needed) and makes it active.</p>

    <!-- Search -->
    <input
      bind:this={searchEntryEl}
      onfocus={onSearchFocus}
      onblur={onSearchBlur}
      oninput={onSearchInput}
      style="width: 100%; box-sizing: border-box; background: {C_CARD}; color: {C_TEXT}; border: 1px solid {C_ACCENT}; border-radius: 3px; padding: 6px 8px; font-size: 10px; margin-bottom: 8px; outline: none;"
    />

    <!-- Grid 2 cols -->
    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 4px; margin-bottom: 4px;">
      {#each displayPets as pet (pet.slug)}
        {@const selected = pet.slug === selectedSlug}
        <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
        <div
          role="button"
          tabindex="0"
          onclick={() => onCardClick(pet)}
          style="display: flex; align-items: center; gap: 10px; padding: 6px 8px; background: {selected ? '#1a1a3a' : C_CARD}; border: 1px solid {selected ? C_ACCENT : C_BORDER}; border-radius: 4px; cursor: pointer; min-height: 52px;"
        >
          <div style="width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; flex-shrink: 0;">
            {#if avatars[pet.slug]}
              <img src={avatars[pet.slug]} alt={pet.name} style="width: 40px; height: 40px; object-fit: contain;" />
            {:else}
              <div style="width: 40px; height: 40px; background: {C_BORDER}; border-radius: 4px;"></div>
            {/if}
          </div>
          <div style="flex: 1; min-width: 0; text-align: left;">
            <div style="font-size: 10px; font-weight: {selected ? 'bold' : 'normal'}; color: {selected ? 'white' : C_TEXT}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{pet.name}</div>
            <div style="font-size: 8px; color: {C_MUTED};">{pet.slug}{pet.downloaded ? ' · installed' : ''}</div>
          </div>
          {#if selected && pet.downloaded}
            <button
              style="color: {C_RED}; background: transparent; border: none; cursor: pointer; font-size: 14px; padding: 4px;"
              onclick={(e) => onTrashClick(pet, e)}
            >🗑</button>
          {/if}
        </div>
      {/each}
    </div>
    <p style="color: {gridStatusColor}; font-size: 8px; margin: 6px 0;">{gridStatus}</p>

    <!-- Size -->
    <hr style="border: none; border-top: 1px solid {C_BORDER}; margin: 16px 0 0;" />
    <div style="display: flex; justify-content: space-between; align-items: center; margin: 12px 0 4px;">
      <span style="font-size: 12px; font-weight: bold;">Size</span>
      <span style="color: {C_MUTED}; font-size: 10px;">{scalePct}%</span>
    </div>
    <p style="color: {C_MUTED}; font-size: 8px; margin: 0 0 4px;">Resize the floating mascot.</p>
    <input
      type="range"
      min="30"
      max="150"
      step="5"
      bind:value={scalePct}
      style="width: 100%; margin-bottom: 8px; accent-color: {C_ACCENT};"
    />

    <!-- Roam -->
    <hr style="border: none; border-top: 1px solid {C_BORDER}; margin: 4px 0 0;" />
    <div style="display: flex; justify-content: space-between; align-items: center; margin: 12px 0 4px;">
      <span style="font-size: 12px; font-weight: bold;">Roam</span>
      <div style="display: flex; gap: 2px;">
        <button
          style="font-size: 8px; padding: 2px 6px; cursor: pointer; border: none; color: {!runEnabled ? 'white' : C_MUTED}; background: {!runEnabled ? C_RED : C_CARD};"
          onclick={() => runEnabled = false}
        > Off </button>
        <button
          style="font-size: 8px; padding: 2px 6px; font-weight: bold; cursor: pointer; border: none; color: white; background: {runEnabled ? C_ACCENT : C_CARD};"
          onclick={() => runEnabled = true}
        > On </button>
      </div>
    </div>
    <p style="color: {C_MUTED}; font-size: 8px; margin: 0 0 12px;">Let the pet wander the window on its own while idle.</p>

    <!-- Apply -->
    <button
      onclick={onApply}
      disabled={isApplying}
      style="width: 100%; box-sizing: border-box; padding: 8px 16px; background: {C_ACCENT}; color: white; border: none; border-radius: 4px; font-size: 10px; font-weight: bold; cursor: {isApplying ? 'wait' : 'pointer'}; margin-bottom: 20px;"
    >{isApplying ? "Applying..." : "✓ Apply Changes"}</button>
  </div>
</main>

{#if showDeleteModal}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div
    style="position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 100;"
    onclick={cancelDelete}
  >
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div
      style="background: {C_BG}; border: 1px solid {C_BORDER}; border-radius: 8px; padding: 24px; text-align: center; min-width: 240px;"
      onclick={(e) => e.stopPropagation()}
    >
      <p style="font-size: 11px; font-weight: bold; margin: 0 0 16px;">Delete {deleteTarget?.name}?</p>
      <div style="display: flex; gap: 6px; justify-content: center;">
        <button
          onclick={cancelDelete}
          style="color: {C_TEXT}; background: {C_CARD}; border: none; padding: 4px 12px; font-size: 9px; cursor: pointer; border-radius: 3px;"
        >Cancel</button>
        <button
          onclick={confirmDelete}
          style="color: white; background: {C_RED}; border: none; padding: 4px 12px; font-size: 9px; font-weight: bold; cursor: pointer; border-radius: 3px;"
        >Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: #0d0d0d;
    overflow: hidden;
  }
  main {
    user-select: none;
  }
</style>
