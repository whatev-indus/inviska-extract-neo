use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

pub type Result<T> = std::result::Result<T, InviskaError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviskaError {
    pub message: String,
}

impl InviskaError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for InviskaError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MkvToolNix {
    pub directory: PathBuf,
    pub mkvmerge: PathBuf,
    pub mkvextract: PathBuf,
    pub interface: MkvExtractInterface,
    pub version: Option<ToolVersion>,
}

impl MkvToolNix {
    pub fn from_directory(path: impl AsRef<Path>, interface: MkvExtractInterface) -> Self {
        let dir = normalise_tool_dir(path.as_ref());
        Self {
            mkvmerge: dir.join(executable_name("mkvmerge")),
            mkvextract: dir.join(executable_name("mkvextract")),
            directory: dir,
            interface,
            version: None,
        }
    }

    pub fn with_version(mut self, version: Option<ToolVersion>) -> Self {
        if let Some(version) = version {
            self.interface = if version.major >= 17 {
                MkvExtractInterface::Modern
            } else {
                MkvExtractInterface::Legacy
            };
            self.version = Some(version);
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MkvExtractInterface {
    Legacy,
    Modern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ToolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
}

impl ToolVersion {
    pub fn parse(text: &str) -> Option<Self> {
        let version = text
            .split_whitespace()
            .find(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
        let mut parts = version.split(|c| c == '.' || c == '-');
        Some(Self {
            major: parts.next()?.parse().ok()?,
            minor: parts.next().unwrap_or("0").parse().ok()?,
            patch: parts.next().and_then(|value| value.parse().ok()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaFile {
    pub path: PathBuf,
    pub tracks: Vec<TrackInfo>,
    pub attachments: Vec<AttachmentInfo>,
    pub has_chapters: bool,
    pub has_cuesheet: bool,
    pub has_tags: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: u32,
    pub kind: TrackKind,
    pub codec: String,
    pub codec_id: String,
    pub language: String,
    pub name: Option<String>,
    pub supported: bool,
    pub extension: String,
}

impl TrackInfo {
    pub fn from_legacy_mkvmerge_line(line: &str) -> Option<Self> {
        let rest = line.strip_prefix("Track ID ")?;
        let (id, rest) = rest.split_once(':')?;
        let id = id.parse().ok()?;
        let rest = rest.trim();
        let (kind, rest) = rest.split_once(' ')?;
        let kind = TrackKind::from_mkvmerge_type(kind)?;
        let codec = between(rest, '(', ')')?.to_owned();
        let codec_id = read_property(rest, "codec_id:").unwrap_or_default();
        let supported = codec_supported(&codec_id);
        let extension = codec_extension(&codec_id).to_owned();

        Some(Self {
            id,
            kind,
            codec,
            codec_id,
            language: read_property(rest, "language:").unwrap_or_default(),
            name: read_property(rest, "track_name:").map(|name| name.replace("\\s", " ")),
            supported,
            extension,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
    Subtitles,
}

impl TrackKind {
    pub fn from_mkvmerge_type(value: &str) -> Option<Self> {
        match value {
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            "subtitles" => Some(Self::Subtitles),
            _ => None,
        }
    }

    pub fn output_label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Subtitles => "Subtitles",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub id: u32,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
}

impl AttachmentInfo {
    pub fn from_legacy_mkvmerge_line(line: &str) -> Option<Self> {
        let rest = line.strip_prefix("Attachment ID ")?;
        let (id, rest) = rest.split_once(':')?;
        let id = id.parse().ok()?;
        let file_name = read_property(rest, "file_name:")
            .or_else(|| between(rest, '\'', '\'').map(ToOwned::to_owned))?;
        Some(Self {
            id,
            file_name: file_name.replace("\\s", " "),
            mime_type: read_property(rest, "mime_type:"),
            size: read_property(rest, "size:").and_then(|value| value.parse().ok()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionSelection {
    pub track_ids: Vec<u32>,
    pub timestamps: bool,
    pub cues: bool,
    pub chapters: bool,
    pub cuesheet: bool,
    pub tags: bool,
    pub attachment_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionPhase {
    Tracks(Vec<TrackOutput>),
    Timestamps(Vec<TrackOutput>),
    Cues(Vec<TrackOutput>),
    Chapters(PathBuf),
    Cuesheet(PathBuf),
    Tags(PathBuf),
    Attachments(Vec<AttachmentOutput>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackOutput {
    pub track_id: u32,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentOutput {
    pub attachment_id: u32,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionPlan {
    pub source_path: PathBuf,
    pub output_directory: PathBuf,
    pub phases: Vec<ExtractionPhase>,
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionEvent {
    pub phase_index: usize,
    pub progress: Option<u8>,
    pub stream: ProcessStream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStream {
    Stdout,
    Stderr,
}

pub fn discover_mkvtoolnix(configured_dir: Option<&Path>) -> Result<MkvToolNix> {
    let mut candidates = Vec::new();
    if let Some(dir) = configured_dir {
        candidates.push(normalise_tool_dir(dir));
    }
    candidates.extend(default_tool_dirs());

    for candidate in candidates {
        let tools = MkvToolNix::from_directory(&candidate, MkvExtractInterface::Modern);
        if tools.mkvmerge.is_file() && tools.mkvextract.is_file() {
            let version = read_tool_version(&tools.mkvmerge).ok().flatten();
            return Ok(tools.with_version(version));
        }
    }

    Err(InviskaError::new("MKVToolNix could not be found."))
}

pub fn read_tool_version(mkvmerge: &Path) -> Result<Option<ToolVersion>> {
    let output = Command::new(mkvmerge).arg("--version").output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(ToolVersion::parse(&text))
}

pub fn probe_media_file(tools: &MkvToolNix, path: impl AsRef<Path>) -> Result<MediaFile> {
    let path = path.as_ref();
    let json_output = Command::new(&tools.mkvmerge)
        .arg("--identification-format")
        .arg("json")
        .arg("-i")
        .arg(path)
        .output()?;
    if json_output.status.success() {
        if let Ok(file) =
            parse_mkvmerge_identification_json(path, &String::from_utf8_lossy(&json_output.stdout))
        {
            return Ok(file);
        }
    }

    let output = Command::new(&tools.mkvmerge).arg("-i").arg(path).output()?;
    if !output.status.success() {
        return Err(InviskaError::new(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(parse_mkvmerge_identification(
        path,
        &String::from_utf8_lossy(&output.stdout),
    ))
}

pub fn parse_mkvmerge_identification_json(path: &Path, text: &str) -> Result<MediaFile> {
    let root: Value =
        serde_json::from_str(text).map_err(|error| InviskaError::new(error.to_string()))?;
    let mut file = MediaFile {
        path: path.to_owned(),
        tracks: Vec::new(),
        attachments: Vec::new(),
        has_chapters: false,
        has_cuesheet: false,
        has_tags: false,
    };

    if let Some(tracks) = root.get("tracks").and_then(Value::as_array) {
        for track in tracks {
            let Some(id) = value_u32(track.get("id")) else {
                continue;
            };
            let Some(kind) = track
                .get("type")
                .and_then(Value::as_str)
                .and_then(TrackKind::from_mkvmerge_type)
            else {
                continue;
            };
            let properties = track.get("properties").unwrap_or(&Value::Null);
            let codec_id = properties
                .get("codec_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let supported = codec_supported(&codec_id);
            let extension = codec_extension(&codec_id).to_owned();
            file.tracks.push(TrackInfo {
                id,
                kind,
                codec: track
                    .get("codec")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                codec_id,
                language: properties
                    .get("language")
                    .or_else(|| properties.get("language_ietf"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name: properties
                    .get("track_name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                supported,
                extension,
            });
        }
    }

    if let Some(attachments) = root.get("attachments").and_then(Value::as_array) {
        for attachment in attachments {
            let Some(id) = value_u32(attachment.get("id")) else {
                continue;
            };
            let Some(file_name) = attachment.get("file_name").and_then(Value::as_str) else {
                continue;
            };
            file.attachments.push(AttachmentInfo {
                id,
                file_name: file_name.to_owned(),
                mime_type: attachment
                    .get("content_type")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                size: attachment.get("size").and_then(Value::as_u64),
            });
        }
    }

    file.has_chapters = root
        .get("chapters")
        .and_then(Value::as_array)
        .is_some_and(|chapters| !chapters.is_empty());
    file.has_tags = root
        .get("global_tags")
        .or_else(|| root.get("track_tags"))
        .and_then(Value::as_array)
        .is_some_and(|tags| !tags.is_empty());
    file.has_cuesheet = root
        .pointer("/container/properties/cuesheet")
        .or_else(|| root.pointer("/container/properties/cue_sheet"))
        .is_some_and(|value| value.as_str().is_some_and(|text| !text.is_empty()));

    Ok(file)
}

pub fn parse_mkvmerge_identification(path: &Path, text: &str) -> MediaFile {
    let mut file = MediaFile {
        path: path.to_owned(),
        tracks: Vec::new(),
        attachments: Vec::new(),
        has_chapters: false,
        has_cuesheet: false,
        has_tags: false,
    };

    for line in text.lines().map(str::trim) {
        if let Some(track) = TrackInfo::from_legacy_mkvmerge_line(line) {
            file.tracks.push(track);
        } else if let Some(attachment) = AttachmentInfo::from_legacy_mkvmerge_line(line) {
            file.attachments.push(attachment);
        } else if line.starts_with("Chapters:") {
            file.has_chapters = true;
        } else if line.contains("cuesheet") || line.contains("cue sheet") {
            file.has_cuesheet = true;
        } else if line.starts_with("Tags:") || line.contains("global tags") {
            file.has_tags = true;
        }
    }

    file
}

fn value_u32(value: Option<&Value>) -> Option<u32> {
    value?.as_u64().and_then(|value| value.try_into().ok())
}

pub fn build_extraction_plan(
    tools: &MkvToolNix,
    media: &MediaFile,
    output_directory: Option<&Path>,
    selection: &ExtractionSelection,
) -> ExtractionPlan {
    let output_directory = output_directory
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_owned)
        .or_else(|| media.path.parent().map(Path::to_owned))
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = media
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("extracted");

    let selected_tracks: Vec<_> = media
        .tracks
        .iter()
        .filter(|track| selection.track_ids.contains(&track.id) && track.supported)
        .collect();
    let mut phases = Vec::new();

    let track_outputs = outputs_for_tracks(&output_directory, stem, &selected_tracks, "");
    if !track_outputs.is_empty() {
        phases.push(ExtractionPhase::Tracks(track_outputs.clone()));
    }
    if selection.timestamps && !track_outputs.is_empty() {
        phases.push(ExtractionPhase::Timestamps(outputs_for_tracks(
            &output_directory,
            stem,
            &selected_tracks,
            "-Timestamps.txt",
        )));
    }
    if selection.cues && !track_outputs.is_empty() {
        phases.push(ExtractionPhase::Cues(outputs_for_tracks(
            &output_directory,
            stem,
            &selected_tracks,
            "-Cues.txt",
        )));
    }
    if selection.chapters && media.has_chapters {
        phases.push(ExtractionPhase::Chapters(
            output_directory.join(format!("{stem}_Chapters.xml")),
        ));
    }
    if selection.cuesheet && media.has_cuesheet {
        phases.push(ExtractionPhase::Cuesheet(
            output_directory.join(format!("{stem}_Cuesheet.cue")),
        ));
    }
    if selection.tags && media.has_tags {
        phases.push(ExtractionPhase::Tags(
            output_directory.join(format!("{stem}_Tags.xml")),
        ));
    }

    let attachment_outputs: Vec<_> = media
        .attachments
        .iter()
        .filter(|attachment| selection.attachment_ids.contains(&attachment.id))
        .map(|attachment| AttachmentOutput {
            attachment_id: attachment.id,
            output_path: output_directory
                .join(format!("{stem}_Attachments"))
                .join(&attachment.file_name),
        })
        .collect();
    if !attachment_outputs.is_empty() {
        phases.push(ExtractionPhase::Attachments(attachment_outputs));
    }

    let commands = phases
        .iter()
        .cloned()
        .map(|phase| mkvextract_command(tools, &media.path, phase))
        .collect();

    ExtractionPlan {
        source_path: media.path.clone(),
        output_directory,
        phases,
        commands,
    }
}

fn outputs_for_tracks(
    output_directory: &Path,
    stem: &str,
    tracks: &[&TrackInfo],
    suffix: &str,
) -> Vec<TrackOutput> {
    let mut instances = HashMap::new();
    tracks
        .iter()
        .map(|track| {
            let instance = instances.entry(track.kind).or_insert(0);
            *instance += 1;
            let output_name = if suffix.is_empty() {
                track_output_name(stem, track, *instance)
            } else {
                format!(
                    "{}_{}{:02}{suffix}",
                    stem,
                    track.kind.output_label(),
                    *instance
                )
            };
            TrackOutput {
                track_id: track.id,
                output_path: output_directory.join(output_name),
            }
        })
        .collect()
}

pub fn run_extraction_plan<F>(plan: &ExtractionPlan, mut on_event: F) -> Result<()>
where
    F: FnMut(ExtractionEvent) + Send,
{
    for (phase_index, command) in plan.commands.iter().enumerate() {
        create_parent_directories(command)?;
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let (tx, rx) = mpsc::channel::<Result<ExtractionEvent>>();
        let stdout_thread = child.stdout.take().map(|stdout| {
            let tx = tx.clone();
            thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let event = line.map(|text| ExtractionEvent {
                        phase_index,
                        progress: parse_progress(&text),
                        stream: ProcessStream::Stdout,
                        text,
                    });
                    if tx.send(event.map_err(InviskaError::from)).is_err() {
                        break;
                    }
                }
            })
        });
        let stderr_thread = child.stderr.take().map(|stderr| {
            let tx = tx.clone();
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    let event = line.map(|text| ExtractionEvent {
                        phase_index,
                        progress: parse_progress(&text),
                        stream: ProcessStream::Stderr,
                        text,
                    });
                    if tx.send(event.map_err(InviskaError::from)).is_err() {
                        break;
                    }
                }
            })
        });
        drop(tx);

        for event in rx {
            on_event(event?);
        }

        if let Some(stdout_thread) = stdout_thread {
            stdout_thread
                .join()
                .map_err(|_| InviskaError::new("stdout reader thread panicked"))?;
        }
        if let Some(stderr_thread) = stderr_thread {
            stderr_thread
                .join()
                .map_err(|_| InviskaError::new("stderr reader thread panicked"))?;
        }
        let status = child.wait()?;

        if !status.success() && status.code() != Some(1) {
            let cues_without_cues =
                matches!(plan.phases.get(phase_index), Some(ExtractionPhase::Cues(_)))
                    && status.code() == Some(2);
            if !cues_without_cues {
                return Err(InviskaError::new(format!(
                    "mkvextract exited with status {status}"
                )));
            }
        }

        on_event(ExtractionEvent {
            phase_index,
            progress: Some(100),
            stream: ProcessStream::Stdout,
            text: format!("Phase {} complete", phase_index + 1),
        });
    }
    Ok(())
}

pub fn mkvextract_command(
    tools: &MkvToolNix,
    source_path: impl AsRef<Path>,
    phase: ExtractionPhase,
) -> CommandSpec {
    let source_path = source_path.as_ref();
    let mut args = Vec::new();

    match tools.interface {
        MkvExtractInterface::Modern => args.push(path_arg(source_path)),
        MkvExtractInterface::Legacy => {}
    }

    match phase {
        ExtractionPhase::Tracks(outputs) => {
            push_phase_args(&mut args, tools.interface, "tracks", "tracks", source_path);
            push_track_outputs(&mut args, outputs);
        }
        ExtractionPhase::Timestamps(outputs) => {
            push_phase_args(
                &mut args,
                tools.interface,
                "timestamps_v2",
                "timecodes_v2",
                source_path,
            );
            push_track_outputs(&mut args, outputs);
        }
        ExtractionPhase::Cues(outputs) => {
            push_phase_args(&mut args, tools.interface, "cues", "cues", source_path);
            push_track_outputs(&mut args, outputs);
        }
        ExtractionPhase::Chapters(output) => {
            push_phase_args(
                &mut args,
                tools.interface,
                "chapters",
                "chapters",
                source_path,
            );
            if tools.interface == MkvExtractInterface::Legacy {
                args.push("--redirect-output".to_owned());
            }
            args.push(path_arg(output));
        }
        ExtractionPhase::Cuesheet(output) => {
            push_phase_args(
                &mut args,
                tools.interface,
                "cuesheet",
                "cuesheet",
                source_path,
            );
            if tools.interface == MkvExtractInterface::Legacy {
                args.push("--redirect-output".to_owned());
            }
            args.push(path_arg(output));
        }
        ExtractionPhase::Tags(output) => {
            push_phase_args(&mut args, tools.interface, "tags", "tags", source_path);
            if tools.interface == MkvExtractInterface::Legacy {
                args.push("--redirect-output".to_owned());
            }
            args.push(path_arg(output));
        }
        ExtractionPhase::Attachments(outputs) => {
            push_phase_args(
                &mut args,
                tools.interface,
                "attachments",
                "attachments",
                source_path,
            );
            for output in outputs {
                args.push(format!(
                    "{}:{}",
                    output.attachment_id,
                    path_arg(output.output_path)
                ));
            }
        }
    }

    args.insert(
        match tools.interface {
            MkvExtractInterface::Modern => 2,
            MkvExtractInterface::Legacy => 2,
        },
        "--gui-mode".to_owned(),
    );

    CommandSpec {
        program: tools.mkvextract.clone(),
        args,
    }
}

pub fn codec_extension(codec_id: &str) -> &'static str {
    match codec_id {
        "V_MPEG4/ISO/AVC" => "h264",
        "V_MPEGH/ISO/HEVC" => "h265",
        "V_MS/VFW/FOURCC" => "avi",
        "V_VP8" | "V_VP9" => "ivf",
        "V_MPEG1" | "V_MPEG2" => "mpg",
        "V_THEORA" | "A_VORBIS" | "S_KATE" => "ogg",
        "A_FLAC" => "flac",
        "A_MPEG/L3" => "mp3",
        "A_MPEG/L2" => "mp2",
        "A_MPEG/L1" => "mp1",
        "A_PCM/INT/LIT" | "A_PCM/INT/BIG" => "wav",
        "A_WAVPACK4" => "wv",
        "A_OPUS" => "opus",
        "A_ALAC" => "caf",
        "A_TTA1" => "tta",
        "S_TEXT/ASS" => "ass",
        "S_TEXT/SSA" => "ssa",
        "S_TEXT/UTF8" => "srt",
        "S_VOBSUB" => "sub",
        "S_TEXT/USF" => "usf",
        "S_HDMV/PGS" => "sup",
        "S_TEXT/WEBVTT" => "wvtt",
        value if value.starts_with("V_REAL/") || value.starts_with("A_REAL") => "rm",
        value if value.starts_with("V_") => "unknown_video",
        value if value.starts_with("A_") => "unknown_audio",
        value if value.starts_with("S_") => "unknown_sub",
        _ => "unknown",
    }
}

pub fn codec_supported(codec_id: &str) -> bool {
    !matches!(
        codec_id,
        "V_UNCOMPRESSED"
            | "V_MPEG4/ISO/SP"
            | "V_MPEG4/ISO/ASP"
            | "V_MPEG4/MS/V3"
            | "V_QUICKTIME"
            | "V_PRORES"
            | "A_PCM/FLOAT/IEEE"
            | "A_MPC"
            | "A_MS/ACM"
            | "A_QUICKTIME"
            | "S_IMAGE/BMP"
    )
}

fn default_tool_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Applications/MKVToolNix.app/Contents/MacOS"));
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            dirs.push(PathBuf::from(program_files).join("MKVToolNix"));
        }
    }

    dirs
}

fn normalise_tool_dir(path: &Path) -> PathBuf {
    let path = trim_trailing_separator(path);
    if cfg!(target_os = "macos") && path.extension().is_some_and(|ext| ext == "app") {
        path.join("Contents").join("MacOS")
    } else {
        path
    }
}

fn trim_trailing_separator(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    while value.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR) {
        let mut text = value.to_string_lossy().into_owned();
        text.pop();
        value = OsString::from(text);
    }
    PathBuf::from(value)
}

fn create_parent_directories(command: &CommandSpec) -> Result<()> {
    for arg in &command.args {
        let Some((_, path)) = arg.split_once(':') else {
            continue;
        };
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn track_output_name(stem: &str, track: &TrackInfo, instance: usize) -> String {
    format!(
        "{}_{}{:02}.{}",
        stem,
        track.kind.output_label(),
        instance,
        track.extension
    )
}

fn push_phase_args(
    args: &mut Vec<String>,
    interface: MkvExtractInterface,
    modern_phase: &str,
    legacy_phase: &str,
    source_path: &Path,
) {
    match interface {
        MkvExtractInterface::Modern => args.push(modern_phase.to_owned()),
        MkvExtractInterface::Legacy => {
            args.push(legacy_phase.to_owned());
            args.push(path_arg(source_path));
        }
    }
}

fn push_track_outputs(args: &mut Vec<String>, outputs: Vec<TrackOutput>) {
    for output in outputs {
        args.push(format!(
            "{}:{}",
            output.track_id,
            path_arg(output.output_path)
        ));
    }
}

fn parse_progress(text: &str) -> Option<u8> {
    let percent_index = text.rfind('%')?;
    let digits: String = text[..percent_index]
        .chars()
        .rev()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse().ok()
}

fn between(value: &str, start: char, end: char) -> Option<&str> {
    let start_index = value.find(start)? + start.len_utf8();
    let end_index = value[start_index..].find(end)? + start_index;
    Some(&value[start_index..end_index])
}

fn read_property(value: &str, key: &str) -> Option<String> {
    let start = value.find(key)? + key.len();
    let end = value[start..]
        .find(' ')
        .map(|index| start + index)
        .unwrap_or(value.len());
    Some(value[start..end].to_owned())
}

fn path_arg(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

fn executable_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_mkvmerge_track_line() {
        let line = "Track ID 2: subtitles (SubRip/SRT) codec_id:S_TEXT/UTF8 language:eng track_name:Styled\\sSubs";
        let track = TrackInfo::from_legacy_mkvmerge_line(line).unwrap();

        assert_eq!(track.id, 2);
        assert_eq!(track.kind, TrackKind::Subtitles);
        assert_eq!(track.codec, "SubRip/SRT");
        assert_eq!(track.codec_id, "S_TEXT/UTF8");
        assert_eq!(track.language, "eng");
        assert_eq!(track.name.as_deref(), Some("Styled Subs"));
        assert_eq!(track.extension, "srt");
        assert!(track.supported);
    }

    #[test]
    fn parses_mkvmerge_identification() {
        let text = "\
File 'movie.mkv': container: Matroska
Track ID 0: video (AVC/H.264/MPEG-4p10) codec_id:V_MPEG4/ISO/AVC language:und
Track ID 1: audio (Opus) codec_id:A_OPUS language:eng track_name:Main\\sAudio
Attachment ID 1: type 'image/png', size 1234 bytes, file_name:cover.png
Chapters: 4 entries
";
        let file = parse_mkvmerge_identification(Path::new("movie.mkv"), text);

        assert_eq!(file.tracks.len(), 2);
        assert_eq!(file.attachments.len(), 1);
        assert!(file.has_chapters);
    }

    #[test]
    fn parses_mkvmerge_json_identification() {
        let text = r#"
        {
          "tracks": [
            {
              "id": 0,
              "type": "video",
              "codec": "AVC/H.264/MPEG-4p10",
              "properties": {
                "codec_id": "V_MPEG4/ISO/AVC",
                "language": "und"
              }
            },
            {
              "id": 1,
              "type": "audio",
              "codec": "Opus",
              "properties": {
                "codec_id": "A_OPUS",
                "language_ietf": "en",
                "track_name": "Main Audio"
              }
            }
          ],
          "attachments": [
            {
              "id": 1,
              "file_name": "cover.png",
              "content_type": "image/png",
              "size": 1234
            }
          ],
          "chapters": [{"num_entries": 4}],
          "global_tags": [{"num_entries": 1}]
        }
        "#;
        let file = parse_mkvmerge_identification_json(Path::new("movie.mkv"), text).unwrap();

        assert_eq!(file.tracks.len(), 2);
        assert_eq!(file.tracks[1].language, "en");
        assert_eq!(file.tracks[1].name.as_deref(), Some("Main Audio"));
        assert_eq!(file.attachments[0].mime_type.as_deref(), Some("image/png"));
        assert!(file.has_chapters);
        assert!(file.has_tags);
    }

    #[test]
    fn builds_modern_track_extraction_command() {
        let tools = MkvToolNix::from_directory("/tools", MkvExtractInterface::Modern);
        let command = mkvextract_command(
            &tools,
            "/video/source.mkv",
            ExtractionPhase::Tracks(vec![TrackOutput {
                track_id: 0,
                output_path: "/out/video.h264".into(),
            }]),
        );

        assert_eq!(
            command.program,
            Path::new("/tools").join(executable_name("mkvextract"))
        );
        assert_eq!(
            command.args,
            vec![
                "/video/source.mkv",
                "tracks",
                "--gui-mode",
                "0:/out/video.h264"
            ]
        );
    }

    #[test]
    fn builds_output_names_with_per_kind_numbering() {
        let tools = MkvToolNix::from_directory("/tools", MkvExtractInterface::Modern);
        let media = MediaFile {
            path: "/video/source.mkv".into(),
            tracks: vec![
                test_track(0, TrackKind::Video, "h264"),
                test_track(1, TrackKind::Audio, "opus"),
                test_track(2, TrackKind::Audio, "aac"),
                test_track(3, TrackKind::Subtitles, "srt"),
            ],
            attachments: vec![],
            has_chapters: false,
            has_cuesheet: false,
            has_tags: false,
        };
        let plan = build_extraction_plan(
            &tools,
            &media,
            Some(Path::new("/out")),
            &ExtractionSelection {
                track_ids: vec![0, 1, 2, 3],
                timestamps: true,
                cues: false,
                chapters: false,
                cuesheet: false,
                tags: false,
                attachment_ids: vec![],
            },
        );

        let ExtractionPhase::Tracks(track_outputs) = &plan.phases[0] else {
            panic!("expected track phase");
        };
        assert_eq!(
            track_outputs[0].output_path,
            PathBuf::from("/out/source_Video01.h264")
        );
        assert_eq!(
            track_outputs[1].output_path,
            PathBuf::from("/out/source_Audio01.opus")
        );
        assert_eq!(
            track_outputs[2].output_path,
            PathBuf::from("/out/source_Audio02.aac")
        );
        assert_eq!(
            track_outputs[3].output_path,
            PathBuf::from("/out/source_Subtitles01.srt")
        );

        let ExtractionPhase::Timestamps(timestamp_outputs) = &plan.phases[1] else {
            panic!("expected timestamps phase");
        };
        assert_eq!(
            timestamp_outputs[1].output_path,
            PathBuf::from("/out/source_Audio01-Timestamps.txt")
        );
    }

    #[test]
    fn builds_legacy_timestamp_command() {
        let tools = MkvToolNix::from_directory("/tools", MkvExtractInterface::Legacy);
        let command = mkvextract_command(
            &tools,
            "/video/source.mkv",
            ExtractionPhase::Timestamps(vec![TrackOutput {
                track_id: 1,
                output_path: "/out/audio_timestamps.txt".into(),
            }]),
        );

        assert_eq!(
            command.args,
            vec![
                "timecodes_v2",
                "/video/source.mkv",
                "--gui-mode",
                "1:/out/audio_timestamps.txt"
            ]
        );
    }

    #[test]
    fn parses_progress_from_gui_output() {
        assert_eq!(parse_progress("#GUI#progress 42%"), Some(42));
        assert_eq!(parse_progress("Progress: 100%"), Some(100));
    }

    fn test_track(id: u32, kind: TrackKind, extension: &str) -> TrackInfo {
        TrackInfo {
            id,
            kind,
            codec: extension.to_owned(),
            codec_id: String::new(),
            language: "und".to_owned(),
            name: None,
            supported: true,
            extension: extension.to_owned(),
        }
    }
}
