const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const SETTINGS_KEY = "inviska-extract-neo-settings";

const state = {
  tools: null,
  media: null,
  files: [],
  selections: new Map(),
  activeIndex: -1,
};

const els = {
  toolDir: document.querySelector("#tool-dir"),
  outputDir: document.querySelector("#output-dir"),
  filePath: document.querySelector("#file-path"),
  discover: document.querySelector("#discover"),
  probe: document.querySelector("#probe"),
  extract: document.querySelector("#extract"),
  extractQueue: document.querySelector("#extract-queue"),
  previewPlan: document.querySelector("#preview-plan"),
  selectSupported: document.querySelector("#select-supported"),
  clearSelection: document.querySelector("#clear-selection"),
  pickTools: document.querySelector("#pick-tools"),
  pickOutput: document.querySelector("#pick-output"),
  pickFile: document.querySelector("#pick-file"),
  tracks: document.querySelector("#tracks"),
  files: document.querySelector("#files"),
  attachments: document.querySelector("#attachments"),
  status: document.querySelector("#tool-status"),
  progress: document.querySelector("#progress"),
  progressLabel: document.querySelector("#progress-label"),
  planPreview: document.querySelector("#plan-preview"),
  planCount: document.querySelector("#plan-count"),
  log: document.querySelector("#log"),
  toggles: {
    timestamps: document.querySelector("#timestamps"),
    cues: document.querySelector("#cues"),
    chapters: document.querySelector("#chapters"),
    cuesheet: document.querySelector("#cuesheet"),
    tags: document.querySelector("#tags"),
  },
};

loadSettings();

listen("extraction-event", (event) => {
  const payload = event.payload;
  if (payload.progress !== null && payload.progress !== undefined) {
    els.progress.value = payload.progress;
    els.progressLabel.textContent = `${payload.progress}%`;
  }
  if (payload.text) {
    appendLog(payload.text);
  }
});

els.toolDir.addEventListener("change", saveSettings);
els.outputDir.addEventListener("change", saveSettings);

els.discover.addEventListener("click", async () => {
  await discoverTools();
});

els.probe.addEventListener("click", async () => {
  await probeCurrentFile();
});

els.extract.addEventListener("click", async () => {
  await extractSelected();
});

els.extractQueue.addEventListener("click", async () => {
  await extractQueue();
});

els.previewPlan.addEventListener("click", async () => {
  await previewQueue();
});

for (const toggle of Object.values(els.toggles)) {
  toggle.addEventListener("change", () => {
    saveActiveSelection();
    renderFiles();
  });
}

els.selectSupported.addEventListener("click", () => {
  if (!state.media) return;
  const selection = selectionFor(state.media);
  selection.track_ids = state.media.tracks
    .filter((track) => track.supported)
    .map((track) => track.id);
  selection.attachment_ids = state.media.attachments.map((attachment) => attachment.id);
  state.selections.set(selectionKey(state.media), selection);
  renderMedia(state.media);
  renderFiles();
});

els.clearSelection.addEventListener("click", () => {
  if (!state.media) return;
  state.selections.set(selectionKey(state.media), emptySelection());
  renderMedia(state.media);
  renderFiles();
});

els.pickTools.addEventListener("click", async () => {
  const folder = await invoke("pick_folder");
  if (folder) {
    els.toolDir.value = folder;
    saveSettings();
    state.tools = null;
    await discoverTools();
  }
});

els.pickOutput.addEventListener("click", async () => {
  const folder = await invoke("pick_folder");
  if (folder) {
    els.outputDir.value = folder;
    saveSettings();
  }
});

els.pickFile.addEventListener("click", async () => {
  const files = await invoke("pick_mkv_files");
  if (files.length) {
    els.filePath.value = files[0];
    await probeFiles(files);
  }
});

window.addEventListener("dragover", (event) => {
  event.preventDefault();
});

window.addEventListener("drop", async (event) => {
  event.preventDefault();
  const paths = [...event.dataTransfer.files]
    .map((file) => file.path)
    .filter(Boolean);
  if (paths.length) {
    els.filePath.value = paths[0];
    await probeFiles(paths);
  }
});

async function discoverTools() {
  setBusy("Finding MKVToolNix...");
  try {
    state.tools = await invoke("discover_tools", {
      toolDirectory: emptyToNull(els.toolDir.value),
    });
    els.status.textContent = state.tools.version
      ? `MKVToolNix ${state.tools.version.major}.${state.tools.version.minor}`
      : "MKVToolNix found";
    appendLog(`Using tools in ${state.tools.directory}`);
  } catch (error) {
    showError(error);
  } finally {
    setIdle();
  }
}

async function probeCurrentFile() {
  if (!els.filePath.value.trim()) return;
  await probeFiles([els.filePath.value.trim()]);
}

async function probeFiles(paths) {
  if (!state.tools) {
    await discoverTools();
  }
  setBusy(paths.length === 1 ? "Reading file..." : `Reading ${paths.length} files...`);
  try {
    const loaded = [];
    for (const path of paths) {
      const media = await invoke("probe_file", {
        request: {
          toolDirectory: emptyToNull(els.toolDir.value),
          path,
        },
      });
      loaded.push(media);
      appendLog(`Loaded ${media.path}`);
    }
    state.files = [...state.files, ...loaded];
    for (const media of loaded) {
      const key = selectionKey(media);
      if (!state.selections.has(key)) {
        state.selections.set(key, defaultSelection(media));
      }
    }
    setActiveFile(state.files.length - loaded.length);
    renderFiles();
  } catch (error) {
    showError(error);
  } finally {
    setIdle();
  }
}

async function extractSelected() {
  if (!state.tools || !state.media) return;
  saveActiveSelection();
  const selection = selectionFor(state.media);
  setBusy("Planning extraction...");
  setExtractionButtonsDisabled(true);
  try {
    const plan = await planFor(state.media, selection);
    if (plan.commands.length === 0) {
      appendLog("Nothing selected for extraction.");
      renderPlanPreview([]);
      return;
    }
    renderPlanPreview([plan]);
    els.progress.value = 0;
    els.progressLabel.textContent = "Starting";
    await invoke("run_extraction", { request: { plan } });
    els.progress.value = 100;
    els.progressLabel.textContent = "Complete";
  } catch (error) {
    showError(error);
  } finally {
    setExtractionButtonsDisabled(false);
    setIdle();
  }
}

async function extractQueue() {
  if (!state.tools || state.files.length === 0) return;
  saveActiveSelection();
  setExtractionButtonsDisabled(true);
  els.progress.value = 0;
  try {
    let planned = 0;
    for (const media of state.files) {
      const selection = selectionFor(media);
      const plan = await planFor(media, selection);
      if (plan.commands.length === 0) {
        continue;
      }
      planned += 1;
      appendLog(`Extracting ${media.path}`);
      els.progressLabel.textContent = `Extracting ${planned}`;
      await invoke("run_extraction", { request: { plan } });
    }
    els.progress.value = 100;
    els.progressLabel.textContent = planned ? "Queue complete" : "Nothing selected";
  } catch (error) {
    showError(error);
  } finally {
    setExtractionButtonsDisabled(false);
    setIdle();
  }
}

async function previewQueue() {
  if (!state.tools || state.files.length === 0) return;
  saveActiveSelection();
  try {
    const plans = [];
    for (const media of state.files) {
      const plan = await planFor(media, selectionFor(media));
      if (plan.commands.length > 0) {
        plans.push(plan);
      }
    }
    renderPlanPreview(plans);
  } catch (error) {
    showError(error);
  }
}

async function planFor(media, selection) {
  return await invoke("plan_extraction", {
    request: {
      tools: state.tools,
      media,
      outputDirectory: emptyToNull(els.outputDir.value),
      selection,
    },
  });
}

function renderMedia(media) {
  if (!media) {
    els.tracks.classList.add("empty");
    els.tracks.textContent = "No file loaded";
    els.attachments.classList.add("empty");
    els.attachments.textContent = "No attachments loaded";
    for (const toggle of Object.values(els.toggles)) {
      toggle.checked = false;
      toggle.disabled = true;
    }
    els.extract.disabled = true;
    els.extractQueue.disabled = state.files.length === 0;
    els.previewPlan.disabled = state.files.length === 0;
    els.selectSupported.disabled = true;
    els.clearSelection.disabled = true;
    return;
  }

  const selection = selectionFor(media);
  els.tracks.classList.toggle("empty", media.tracks.length === 0);
  els.tracks.innerHTML = media.tracks.length
    ? media.tracks.map((track) => renderTrack(track, selection)).join("")
    : "No tracks found";

  els.attachments.classList.toggle("empty", media.attachments.length === 0);
  els.attachments.innerHTML = media.attachments.length
    ? media.attachments.map((attachment) => renderAttachment(attachment, selection)).join("")
    : "No attachments found";

  els.toggles.timestamps.checked = selection.timestamps;
  els.toggles.cues.checked = selection.cues;
  els.toggles.chapters.checked = selection.chapters && media.has_chapters;
  els.toggles.cuesheet.checked = selection.cuesheet && media.has_cuesheet;
  els.toggles.tags.checked = selection.tags && media.has_tags;
  els.toggles.chapters.disabled = !media.has_chapters;
  els.toggles.cuesheet.disabled = !media.has_cuesheet;
  els.toggles.tags.disabled = !media.has_tags;
  els.extract.disabled = false;
  els.extractQueue.disabled = false;
  els.previewPlan.disabled = false;
  els.selectSupported.disabled = false;
  els.clearSelection.disabled = false;

  els.tracks.querySelectorAll("input").forEach((input) => {
    input.addEventListener("change", saveActiveSelection);
  });
  els.attachments.querySelectorAll("input").forEach((input) => {
    input.addEventListener("change", saveActiveSelection);
  });
}

function renderFiles() {
  els.files.classList.toggle("empty", state.files.length === 0);
  els.files.innerHTML = state.files.length
    ? state.files
        .map((media, index) => {
          const active = index === state.activeIndex ? "active" : "";
          const name = String(media.path).split(/[\\/]/).pop();
          const selected = selectionSummary(media);
          return `
            <div class="file-row ${active}">
              <button class="file-select" type="button" data-file-index="${index}">
                <strong>${escapeHtml(name)}</strong>
                <small>${selected}</small>
              </button>
              <button class="file-remove" type="button" title="Remove file" data-remove-index="${index}">X</button>
            </div>
          `;
        })
        .join("")
    : "No files loaded";

  for (const button of document.querySelectorAll("[data-file-index]")) {
    button.addEventListener("click", () => {
      saveActiveSelection();
      setActiveFile(Number(button.dataset.fileIndex));
      renderFiles();
    });
  }
  for (const button of document.querySelectorAll("[data-remove-index]")) {
    button.addEventListener("click", () => {
      removeFile(Number(button.dataset.removeIndex));
    });
  }
}

function setActiveFile(index) {
  state.activeIndex = index;
  state.media = state.files[index] || null;
  if (state.media) {
    els.filePath.value = state.media.path;
    renderMedia(state.media);
  }
}

function removeFile(index) {
  const [removed] = state.files.splice(index, 1);
  if (removed) {
    state.selections.delete(selectionKey(removed));
  }
  if (state.files.length === 0) {
    state.activeIndex = -1;
    state.media = null;
    els.filePath.value = "";
    renderMedia(null);
  } else {
    setActiveFile(Math.min(index, state.files.length - 1));
  }
  renderFiles();
}

function renderTrack(track, selection) {
  const disabled = track.supported ? "" : "disabled";
  const checked = selection.track_ids.includes(track.id) ? "checked" : "";
  const name = track.name ? ` · ${escapeHtml(track.name)}` : "";
  return `
    <label class="item ${disabled}">
      <input type="checkbox" data-track-id="${track.id}" ${checked} ${disabled} />
      <span>${track.kind} ${track.id}</span>
      <strong>${escapeHtml(track.codec)}</strong>
      <small>${escapeHtml(track.language || "und")}${name} · .${track.extension}</small>
    </label>
  `;
}

function renderAttachment(attachment, selection) {
  const checked = selection.attachment_ids.includes(attachment.id) ? "checked" : "";
  return `
    <label class="item">
      <input type="checkbox" data-attachment-id="${attachment.id}" ${checked} />
      <span>Attachment ${attachment.id}</span>
      <strong>${escapeHtml(attachment.file_name)}</strong>
      <small>${escapeHtml(attachment.mime_type || "")}</small>
    </label>
  `;
}

function readSelection() {
  return {
    track_ids: checkedValues("[data-track-id]"),
    timestamps: els.toggles.timestamps.checked,
    cues: els.toggles.cues.checked,
    chapters: els.toggles.chapters.checked,
    cuesheet: els.toggles.cuesheet.checked,
    tags: els.toggles.tags.checked,
    attachment_ids: checkedValues("[data-attachment-id]"),
  };
}

function saveActiveSelection() {
  if (!state.media) return;
  state.selections.set(selectionKey(state.media), readSelection());
}

function selectionFor(media) {
  const key = selectionKey(media);
  if (!state.selections.has(key)) {
    state.selections.set(key, defaultSelection(media));
  }
  return state.selections.get(key);
}

function selectionKey(media) {
  return String(media.path);
}

function defaultSelection(media) {
  return {
    ...emptySelection(),
    track_ids: media.tracks.filter((track) => track.supported).map((track) => track.id),
  };
}

function emptySelection() {
  return {
    track_ids: [],
    timestamps: false,
    cues: false,
    chapters: false,
    cuesheet: false,
    tags: false,
    attachment_ids: [],
  };
}

function selectionSummary(media) {
  const selection = selectionFor(media);
  const count = selection.track_ids.length + selection.attachment_ids.length;
  const extras = [
    selection.timestamps && "timestamps",
    selection.cues && "cues",
    selection.chapters && "chapters",
    selection.cuesheet && "cuesheet",
    selection.tags && "tags",
  ].filter(Boolean);
  const suffix = extras.length ? ` + ${extras.join(", ")}` : "";
  return `${count} selected of ${media.tracks.length + media.attachments.length}${suffix}`;
}

function checkedValues(selector) {
  return [...document.querySelectorAll(selector)]
    .filter((input) => input.checked)
    .map((input) => Number(input.dataset.trackId || input.dataset.attachmentId));
}

function setExtractionButtonsDisabled(disabled) {
  els.extract.disabled = disabled || !state.media;
  els.extractQueue.disabled = disabled || state.files.length === 0;
  els.previewPlan.disabled = disabled || state.files.length === 0;
}

function renderPlanPreview(plans) {
  const commands = plans.flatMap((plan) =>
    plan.commands.map((command) => ({
      source: plan.source_path,
      command,
    })),
  );
  els.planCount.textContent = commands.length === 1 ? "1 command" : `${commands.length} commands`;
  els.planPreview.textContent = commands
    .map(({ source, command }, index) => {
      const args = command.args.map(shellQuote).join(" ");
      return `${index + 1}. ${source}\n${shellQuote(command.program)} ${args}`;
    })
    .join("\n\n");
}

function shellQuote(value) {
  const text = String(value);
  return /[\s"'\\]/.test(text) ? `"${text.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"` : text;
}

function setBusy(message) {
  els.progressLabel.textContent = message;
}

function setIdle() {
  if (els.progressLabel.textContent.endsWith("...")) {
    els.progressLabel.textContent = "Idle";
  }
}

function appendLog(text) {
  els.log.textContent += `${text}\n`;
  els.log.scrollTop = els.log.scrollHeight;
}

function showError(error) {
  appendLog(error?.message || String(error));
  els.progressLabel.textContent = "Error";
}

function emptyToNull(value) {
  const trimmed = value.trim();
  return trimmed.length ? trimmed : null;
}

function loadSettings() {
  try {
    const settings = JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}");
    els.toolDir.value = settings.toolDir || "";
    els.outputDir.value = settings.outputDir || "";
  } catch {
    localStorage.removeItem(SETTINGS_KEY);
  }
}

function saveSettings() {
  localStorage.setItem(
    SETTINGS_KEY,
    JSON.stringify({
      toolDir: els.toolDir.value.trim(),
      outputDir: els.outputDir.value.trim(),
    }),
  );
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
