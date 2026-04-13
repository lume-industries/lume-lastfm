use lume_text_slide::{self as text_slide, Font, FontAssets, Lazy};
use text_slide::{Palette, TextAlign, TextBlock, Vertex, RuntimeOverlay, SlideSpec};

static FONT: Lazy<FontAssets> = Lazy::new(|| text_slide::make_font_assets(Font::Apricot_Mono));

pub use lastfm_sidecar::{TrackRow, LastfmPayload};

const MAX_TRACKS: usize = 4;

static SPEC_BYTES: Lazy<Vec<u8>> = Lazy::new(|| text_slide::serialize_spec(&lastfm_slide_spec()));

pub fn lastfm_slide_spec() -> SlideSpec<Vertex> {
    text_slide::default_panel_spec("lastfm_scene", build_overlay(None), palette(), FONT.atlas.clone())
}

pub fn serialized_spec() -> &'static [u8] {
    &SPEC_BYTES
}

fn build_overlay(payload: Option<&LastfmPayload>) -> RuntimeOverlay<Vertex> {
    if let Some(payload) = payload {
        let headers: Vec<String> = payload
            .tracks
            .iter()
            .take(MAX_TRACKS)
            .map(|track| {
                if track.status == "now playing" {
                    format!("LIVE  {}", track.song)
                } else {
                    track.song.clone()
                }
            })
            .collect();
        let details: Vec<String> = payload
            .tracks
            .iter()
            .take(MAX_TRACKS)
            .map(|track| {
                let mut detail = format!("{}  {}", track.artist, track.album);
                if track.status != "now playing" && !track.played_at.is_empty() {
                    detail.push_str(&format!("  {}", track.played_at));
                }
                detail
            })
            .collect();

        let mut blocks = vec![
            title_block("LAST.FM"),
            TextBlock {
                text: &payload.username,
                x: 160.0,
                y: 46.0,
                scale: 0.82,
                color: [0.78, 0.82, 0.92, 1.0],
                align: TextAlign::Center,
                wrap_cols: None,
            },
        ];

        if payload.tracks.is_empty() {
            blocks.push(TextBlock {
                text: "No recent scrobbles returned by Last.fm.",
                x: 160.0,
                y: 112.0,
                scale: 0.92,
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Center,
                wrap_cols: Some(24),
            });
        } else {
            for (idx, track) in payload.tracks.iter().take(MAX_TRACKS).enumerate() {
                let y = 70.0 + idx as f32 * 30.0;
                blocks.push(TextBlock {
                    text: &headers[idx],
                    x: 34.0,
                    y,
                    scale: 0.90,
                    color: if track.status == "now playing" {
                        [0.98, 0.82, 0.42, 1.0]
                    } else {
                        [1.0, 1.0, 1.0, 1.0]
                    },
                    align: TextAlign::Left,
                    wrap_cols: Some(26),
                });
                blocks.push(TextBlock {
                    text: &details[idx],
                    x: 34.0,
                    y: y + 12.0,
                    scale: 0.66,
                    color: [0.74, 0.82, 0.92, 1.0],
                    align: TextAlign::Left,
                    wrap_cols: Some(34),
                });
            }
        }

        blocks.push(footer_block(&payload.updated));
        return text_slide::compose_overlay(&blocks, &FONT);
    }

    text_slide::compose_overlay(&[
        title_block("LAST.FM"),
        TextBlock {
            text: "Loading recent scrobbles...",
            x: 160.0,
            y: 112.0,
            scale: 0.96,
            color: [1.0, 1.0, 1.0, 1.0],
            align: TextAlign::Center,
            wrap_cols: Some(24),
        },
    ], &FONT)
}

fn title_block(text: &'static str) -> TextBlock<'static> {
    TextBlock {
        text,
        x: 160.0,
        y: 26.0,
        scale: 1.10,
        color: [0.98, 0.32, 0.20, 1.0],
        align: TextAlign::Center,
        wrap_cols: None,
    }
}

fn footer_block<'a>(text: &'a str) -> TextBlock<'a> {
    TextBlock {
        text,
        x: 160.0,
        y: 198.0,
        scale: 0.78,
        color: [0.72, 0.82, 0.92, 1.0],
        align: TextAlign::Center,
        wrap_cols: None,
    }
}

fn palette() -> Palette {
    Palette {
        background: [0.08, 0.02, 0.04, 1.0],
        panel: [0.18, 0.04, 0.08, 0.96],
        accent: [0.48, 0.10, 0.12, 0.96],
        accent_soft: [0.28, 0.08, 0.10, 0.96],
    }
}

#[cfg(target_arch = "wasm32")]
lume_text_slide::VRX_64_slide::export_traced_entrypoints! {
    init = slide_init,
    update = slide_update,
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_spec_ptr() -> *const u8 {
    SPEC_BYTES.as_ptr()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_spec_len() -> u32 {
    SPEC_BYTES.len() as u32
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_abi_version() -> u32 {
    lume_text_slide::VRX_64_slide::ABI_VERSION
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
fn slide_init() -> i32 {
    runtime_state::state().refresh();
    0
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
fn slide_update(_dt: f32) -> i32 {
    let mut state = runtime_state::state();
    if let Some(payload) = text_slide::channel_runtime::poll_json::<LastfmPayload>(&mut state.response_buf) {
        state.payload = Some(payload);
    }
    state.refresh();
    1
}

#[cfg(target_arch = "wasm32")]
mod runtime_state {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use super::{LastfmPayload, build_overlay};
    use text_slide::channel_runtime;
    use crate::text_slide;

    pub struct RuntimeState {
        pub payload: Option<LastfmPayload>,
        pub overlay_bytes: Vec<u8>,
        pub response_buf: Vec<u8>,
    }

    impl RuntimeState {
        fn new() -> Self {
            let mut state = Self {
                payload: None,
                overlay_bytes: Vec::new(),
                response_buf: vec![0u8; text_slide::channel_runtime::CHANNEL_BUF_BYTES],
            };
            state.refresh();
            state
        }

        pub fn refresh(&mut self) {
            self.overlay_bytes =
                text_slide::serialize_overlay(&build_overlay(self.payload.as_ref()));
        }
    }

    static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

    pub fn state() -> MutexGuard<'static, RuntimeState> {
        STATE
            .get_or_init(|| Mutex::new(RuntimeState::new()))
            .lock()
            .unwrap()
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_overlay_ptr() -> *const u8 {
    runtime_state::state().overlay_bytes.as_ptr()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_overlay_len() -> u32 {
    runtime_state::state().overlay_bytes.len() as u32
}

#[cfg(test)]
mod tests {
    use lastfm_sidecar::parse_recent_tracks;
    use super::*;

    #[test]
    fn spec_valid() {
        lastfm_slide_spec().validate().unwrap();
    }

    #[test]
    fn parse_recent_tracks_marks_now_playing() {
        let body = r##"{
            "recenttracks": {
                "track": [
                    {
                        "name": "Live Song",
                        "artist": {"#text": "Band"},
                        "album": {"#text": "Record"},
                        "@attr": {"nowplaying": "true"}
                    },
                    {
                        "name": "Older Song",
                        "artist": {"#text": "Band"},
                        "album": {"#text": "Record"},
                        "date": {"uts": "1710849600"}
                    }
                ]
            }
        }"##;
        let payload = parse_recent_tracks("rodger", body, 0).expect("parse lastfm payload");
        assert_eq!(payload.tracks[0].status, "now playing");
        assert!(payload.tracks[1].played_at.ends_with("UTC"));
    }

    #[test]
    fn parse_recent_tracks_handles_single_track_object() {
        let body = r##"{
            "recenttracks": {
                "track": {
                    "name": "Only Song",
                    "artist": {"#text": "Lone Artist"},
                    "album": {"#text": "Solo"}
                }
            }
        }"##;
        let payload = parse_recent_tracks("rodger", body, 0).expect("parse lastfm payload");
        assert_eq!(payload.tracks.len(), 1);
        assert_eq!(payload.tracks[0].song, "Only Song");
    }
}
