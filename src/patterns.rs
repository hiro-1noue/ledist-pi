//! Generic, TOML-defined page patterns for non-E233 vehicle profiles.
use crate::{
    AssetRegistry, BdfFont, Profile, Region, RgbFrame, ScriptAction, ScriptRunner, ScrollSpec,
};
use serde::Deserialize;
use std::{path::Path, sync::Arc, time::Duration};

#[derive(Debug, Deserialize)]
pub struct Pattern {
    #[serde(default = "yes")]
    pub repeat: bool,
    #[serde(default)]
    pub page: Vec<PatternPage>,
}
fn yes() -> bool {
    true
}
#[derive(Debug, Deserialize)]
pub struct PatternPage {
    #[serde(default = "three")]
    pub seconds: f64,
    #[serde(default)]
    pub until_scroll_end: bool,
    #[serde(default)]
    pub layer: Vec<Layer>,
    #[serde(default)]
    pub scroll: Option<Scroll>,
}
fn three() -> f64 {
    3.0
}
#[derive(Debug, Deserialize)]
pub struct Layer {
    pub directory: String,
    pub asset: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}
#[derive(Debug, Deserialize)]
pub struct Scroll {
    pub text: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub fn load_and_compile(
    profile: &Profile,
    assets: &AssetRegistry,
    path: &Path,
    data_root: &Path,
) -> Result<ScriptRunner, String> {
    let pattern: Pattern =
        toml::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if pattern.page.is_empty() {
        return Err("patterns TOMLにpageがありません".into());
    }
    let font = load_font(profile, data_root)?;
    let mut actions = Vec::new();
    for page in pattern.page {
        let mut frame =
            RgbFrame::black(profile.profile.canvas_width, profile.profile.canvas_height);
        for layer in page.layer {
            if layer.x.saturating_add(layer.width) > profile.profile.canvas_width
                || layer.y.saturating_add(layer.height) > profile.profile.canvas_height
            {
                return Err("patternsの画像領域がキャンバス外です".into());
            }
            let (w, h, pixels) = assets
                .load_rgb(&layer.directory, &layer.asset)
                .map_err(|e| e.to_string())?;
            if (w, h) != (layer.width, layer.height) {
                return Err(format!(
                    "{}:{} は{}x{}である必要があります",
                    layer.directory, layer.asset, layer.width, layer.height
                ));
            }
            frame
                .blit_rgb(layer.x as isize, layer.y as isize, w, h, &pixels)
                .map_err(|e| e.to_string())?;
        }
        let scroll = match page.scroll {
            None => None,
            Some(scroll) => {
                if scroll.x.saturating_add(scroll.width) > profile.profile.canvas_width
                    || scroll.y.saturating_add(scroll.height) > profile.profile.canvas_height
                {
                    return Err("patternsのスクロール領域がキャンバス外です".into());
                }
                Some(make_scroll(profile, font.as_ref(), scroll)?)
            }
        };
        if page.until_scroll_end && scroll.is_none() {
            return Err("until_scroll_endにはscrollが必要です".into());
        }
        actions.push(ScriptAction::Present { frame, scroll });
        actions.push(if page.until_scroll_end {
            ScriptAction::WaitScrollEnd
        } else {
            ScriptAction::Wait(Duration::from_secs_f64(page.seconds))
        });
    }
    let (first, cycle) = if pattern.repeat {
        (Vec::new(), Some(actions))
    } else {
        (actions, None)
    };
    Ok(ScriptRunner::new(
        profile.profile.canvas_width,
        profile.profile.canvas_height,
        first,
        cycle,
    ))
}
fn load_font(profile: &Profile, data_root: &Path) -> Result<Option<Arc<BdfFont>>, String> {
    let Some(defaults) = &profile.scroll_defaults else {
        return Ok(None);
    };
    let path = data_root.join("fonts").join(&defaults.font);
    let mut font = BdfFont::parse_bdf(
        &std::fs::read_to_string(path.join("shnmk16.bdf")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    font.merge_fallback(
        BdfFont::parse_bdf(
            &std::fs::read_to_string(path.join("shnm8x16a.bdf")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
    );
    Ok(Some(Arc::new(font)))
}
fn make_scroll(
    profile: &Profile,
    font: Option<&Arc<BdfFont>>,
    scroll: Scroll,
) -> Result<ScrollSpec, String> {
    let defaults = profile
        .scroll_defaults
        .as_ref()
        .ok_or("scroll_defaultsがありません")?;
    let value = defaults
        .color
        .strip_prefix('#')
        .ok_or("色は#RRGGBBで指定してください")?;
    if value.len() != 6 {
        return Err("色は#RRGGBBで指定してください".into());
    }
    Ok(ScrollSpec {
        region: Region {
            x: scroll.x,
            y: scroll.y,
            width: scroll.width,
            height: scroll.height,
        },
        text: scroll.text,
        font: font.ok_or("スクロールフォントを読み込めません")?.clone(),
        color: [
            u8::from_str_radix(&value[0..2], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&value[2..4], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&value[4..6], 16).map_err(|_| "色が不正です")?,
        ],
        speed_px_per_second: defaults.speed_px_per_second,
        start_padding: defaults.start_padding,
        end_padding: defaults.end_padding,
        repeat: false,
    })
}
