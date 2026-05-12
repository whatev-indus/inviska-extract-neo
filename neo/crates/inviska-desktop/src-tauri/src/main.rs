use inviska_core::{
    ExtractionEvent, ExtractionPlan, ExtractionSelection, InviskaError, MediaFile, MkvToolNix,
    build_extraction_plan, discover_mkvtoolnix, probe_media_file, run_extraction_plan,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, async_runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeRequest {
    tool_directory: Option<PathBuf>,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanRequest {
    tools: MkvToolNix,
    media: MediaFile,
    output_directory: Option<PathBuf>,
    selection: ExtractionSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractRequest {
    plan: ExtractionPlan,
}

#[tauri::command]
fn discover_tools(tool_directory: Option<PathBuf>) -> Result<MkvToolNix, InviskaError> {
    discover_mkvtoolnix(tool_directory.as_deref())
}

#[tauri::command]
fn probe_file(request: ProbeRequest) -> Result<MediaFile, InviskaError> {
    let tools = discover_mkvtoolnix(request.tool_directory.as_deref())?;
    probe_media_file(&tools, request.path)
}

#[tauri::command]
fn plan_extraction(request: PlanRequest) -> ExtractionPlan {
    build_extraction_plan(
        &request.tools,
        &request.media,
        request.output_directory.as_deref(),
        &request.selection,
    )
}

#[tauri::command]
async fn run_extraction(app: AppHandle, request: ExtractRequest) -> Result<(), InviskaError> {
    async_runtime::spawn_blocking(move || {
        run_extraction_plan(&request.plan, |event: ExtractionEvent| {
            let _ = app.emit("extraction-event", event);
        })
    })
    .await
    .map_err(|error| InviskaError::new(error.to_string()))?
}

#[tauri::command]
fn pick_mkv_files() -> Vec<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Matroska files", &["mkv", "mka", "mks", "webm"])
        .pick_files()
        .unwrap_or_default()
}

#[tauri::command]
fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            discover_tools,
            probe_file,
            plan_extraction,
            run_extraction,
            pick_mkv_files,
            pick_folder
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Inviska Extract Neo");
}
