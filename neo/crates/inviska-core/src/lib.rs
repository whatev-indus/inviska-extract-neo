use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MkvToolNix {
    pub mkvmerge: PathBuf,
    pub mkvextract: PathBuf,
    pub interface: MkvExtractInterface,
}

impl MkvToolNix {
    pub fn from_directory(path: impl AsRef<Path>, interface: MkvExtractInterface) -> Self {
        let dir = path.as_ref();
        Self {
            mkvmerge: dir.join(executable_name("mkvmerge")),
            mkvextract: dir.join(executable_name("mkvextract")),
            interface,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MkvExtractInterface {
    Legacy,
    Modern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    pub id: u32,
    pub kind: TrackKind,
    pub codec: String,
    pub codec_id: String,
    pub language: String,
    pub name: Option<String>,
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

        Some(Self {
            id,
            kind,
            codec,
            codec_id: read_property(rest, "codec_id:").unwrap_or_default(),
            language: read_property(rest, "language:").unwrap_or_default(),
            name: read_property(rest, "track_name:").map(|name| name.replace("\\s", " ")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionPhase {
    Tracks(Vec<TrackOutput>),
    Timestamps(Vec<TrackOutput>),
    Cues(Vec<TrackOutput>),
    Chapters(PathBuf),
    Cuesheet(PathBuf),
    Tags(PathBuf),
    Attachments(Vec<AttachmentOutput>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackOutput {
    pub track_id: u32,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentOutput {
    pub attachment_id: u32,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
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
            args.push(path_arg(output));
        }
        ExtractionPhase::Tags(output) => {
            push_phase_args(&mut args, tools.interface, "tags", "tags", source_path);
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

        assert_eq!(command.program, PathBuf::from("/tools/mkvextract"));
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
}
