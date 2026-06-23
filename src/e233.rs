use crate::{
    AssetRegistry, BdfFont, E233Config, Profile, Region, RgbFrame, ScriptAction, ScriptRunner,
    ScrollSpec,
};
use std::{path::Path, sync::Arc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldSelection {
    None,
    Blank,
    Asset(String),
}
impl FieldSelection {
    pub fn participates(&self) -> bool {
        !matches!(self, Self::None)
    }
}
#[derive(Clone, Debug)]
pub struct DisplaySelection {
    pub service: FieldSelection,
    pub route: FieldSelection,
    pub service_change: FieldSelection,
    pub through_route: FieldSelection,
    pub destination: FieldSelection,
    pub scroll_text: String,
    pub brightness: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Blank,
    Field(&'static str, FieldSelection),
    Scroll(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layout {
    Full(Content),
    ServiceAndRight(Content, Content),
    ServiceAndRightSplit(Content, Content, Content),
    FullWidthSplit(Content, Content),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageDuration {
    Fixed(Duration),
    UntilScrollEnd,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub layout: Layout,
    pub duration: PageDuration,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayPlan {
    pub pages: Vec<Page>,
}

pub fn plan(selection: &DisplaySelection) -> Result<DisplayPlan, String> {
    let s = selection.service.participates();
    let r = selection.route.participates();
    let c = selection.service_change.participates();
    let t = selection.through_route.participates();
    let d = selection.destination.participates();
    let m = !selection.scroll_text.trim().is_empty();
    if t && m {
        return Err("直通先路線名とスクロール文字は同時に使用できません。".into());
    }
    if d && c {
        return Err("行先と種別変更は同時に使用できません。".into());
    }
    if m && !d && !r {
        return Err("スクロール文字を表示する場合は、行先または路線名を選択してください。".into());
    }
    if c && !s {
        return Err(
            "種別変更を表示する場合は、種別または種別欄の「無表示」を選択してください。".into(),
        );
    }
    let fixed = PageDuration::Fixed(Duration::from_secs(3));
    let mut pages = Vec::new();
    for (name, value) in [
        ("destination", &selection.destination),
        ("route", &selection.route),
        ("through_route", &selection.through_route),
        ("service_change", &selection.service_change),
    ] {
        if value.participates() {
            let content = Content::Field(name, value.clone());
            let layout = if s {
                match name {
                    // Route-name artwork is only valid in the upper-right
                    // 80x16 slot; the bottom half stays black.
                    "route" => Layout::ServiceAndRightSplit(
                        Content::Field("service", selection.service.clone()),
                        content,
                        Content::Blank,
                    ),
                    _ => Layout::ServiceAndRight(
                        Content::Field("service", selection.service.clone()),
                        content,
                    ),
                }
            } else {
                Layout::Full(content)
            };
            pages.push(Page {
                layout,
                duration: fixed.clone(),
            });
        }
    }
    if pages.is_empty() && s {
        pages.push(Page {
            layout: Layout::Full(Content::Field("service", selection.service.clone())),
            duration: fixed.clone(),
        });
    }
    if m {
        let top = if d {
            Content::Field("destination", selection.destination.clone())
        } else {
            Content::Field("route", selection.route.clone())
        };
        let scroll = Content::Scroll(selection.scroll_text.trim().to_owned());
        let layout = if s {
            Layout::ServiceAndRightSplit(
                Content::Field("service", selection.service.clone()),
                top,
                scroll,
            )
        } else {
            Layout::FullWidthSplit(top, scroll)
        };
        pages.push(Page {
            layout,
            duration: PageDuration::UntilScrollEnd,
        });
    }
    if pages.is_empty() {
        return Err("表示する項目を選択してください。".into());
    }
    Ok(DisplayPlan { pages })
}

/// Turn an already-validated E233 page plan into the generic page executor.
/// Assets are selected by ID, then resolved only inside the size-specific
/// directories declared by the profile.  This keeps the HTTP API from ever
/// accepting a path and makes fallback behaviour deterministic.
pub fn compile(
    profile: &Profile,
    assets: &AssetRegistry,
    selection: &DisplaySelection,
    data_root: &Path,
) -> Result<ScriptRunner, String> {
    let plan = plan(selection)?;
    let config = profile.e233.as_ref().ok_or("E233設定がありません")?;
    let font = if selection.scroll_text.trim().is_empty() {
        None
    } else {
        Some(load_font(profile, data_root)?)
    };
    let mut actions = Vec::new();
    for page in plan.pages {
        let mut frame =
            RgbFrame::black(profile.profile.canvas_width, profile.profile.canvas_height);
        let mut scroll = None;
        match page.layout {
            Layout::Full(content) => {
                if let Content::Field("service", value) = &content {
                    if !blit(
                        profile, assets, config, "service", value, "full", FULL, &mut frame,
                    )? {
                        if !blit(
                            profile,
                            assets,
                            config,
                            "service",
                            value,
                            "left",
                            SERVICE_LEFT,
                            &mut frame,
                        )? {
                            return Err("種別素材は128x32または48x32で配置してください。".into());
                        }
                    }
                } else {
                    draw_content(profile, assets, config, &content, FULL, "full", &mut frame)?;
                }
            }
            Layout::ServiceAndRight(service, right) => {
                draw_content(
                    profile,
                    assets,
                    config,
                    &service,
                    SERVICE_LEFT,
                    "left",
                    &mut frame,
                )?;
                draw_content(
                    profile, assets, config, &right, RIGHT_FULL, "right", &mut frame,
                )?;
            }
            Layout::ServiceAndRightSplit(service, top, bottom) => {
                draw_content(
                    profile,
                    assets,
                    config,
                    &service,
                    SERVICE_LEFT,
                    "left",
                    &mut frame,
                )?;
                draw_content(
                    profile,
                    assets,
                    config,
                    &top,
                    RIGHT_TOP,
                    "right-top",
                    &mut frame,
                )?;
                match bottom {
                    Content::Scroll(text) => {
                        scroll = Some(scroll_spec(profile, font.as_ref(), text, RIGHT_BOTTOM)?)
                    }
                    other => draw_content(
                        profile,
                        assets,
                        config,
                        &other,
                        RIGHT_BOTTOM,
                        "right-bottom",
                        &mut frame,
                    )?,
                }
            }
            Layout::FullWidthSplit(top, bottom) => {
                draw_content(
                    profile, assets, config, &top, FULL_TOP, "full-top", &mut frame,
                )?;
                match bottom {
                    Content::Scroll(text) => {
                        scroll = Some(scroll_spec(profile, font.as_ref(), text, FULL_BOTTOM)?)
                    }
                    other => draw_content(
                        profile,
                        assets,
                        config,
                        &other,
                        FULL_BOTTOM,
                        "full-bottom",
                        &mut frame,
                    )?,
                }
            }
        }
        actions.push(ScriptAction::Present { frame, scroll });
        match page.duration {
            PageDuration::Fixed(_) => actions.push(ScriptAction::Wait(Duration::from_secs_f64(
                config.page_seconds,
            ))),
            PageDuration::UntilScrollEnd => actions.push(ScriptAction::WaitScrollEnd),
        }
    }
    Ok(ScriptRunner::new(
        profile.profile.canvas_width,
        profile.profile.canvas_height,
        Vec::new(),
        Some(actions),
    ))
}

fn draw_content(
    profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    content: &Content,
    region: Region,
    slot: &str,
    frame: &mut RgbFrame,
) -> Result<(), String> {
    if let Content::Field(group, value) = content {
        if value.participates()
            && !blit(profile, assets, config, group, value, slot, region, frame)?
        {
            return Err(format!("{group} の素材を表示できません"));
        }
    }
    Ok(())
}

fn blit(
    _profile: &Profile,
    assets: &AssetRegistry,
    config: &E233Config,
    group: &str,
    value: &FieldSelection,
    slot: &str,
    region: Region,
    frame: &mut RgbFrame,
) -> Result<bool, String> {
    let FieldSelection::Asset(id) = value else {
        return Ok(false);
    };
    let group_config = config
        .assets
        .get(group)
        .ok_or_else(|| format!("{group} の素材設定がありません"))?;
    let directories = group_config
        .directories
        .get(slot)
        .ok_or_else(|| format!("{group} に {slot} 用素材設定がありません"))?;
    for directory in directories {
        if let Ok((w, h, pixels)) = assets.load_rgb(directory, id) {
            if (w, h) == (region.width, region.height) {
                frame
                    .blit_rgb(region.x as isize, region.y as isize, w, h, &pixels)
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn load_font(profile: &Profile, data_root: &Path) -> Result<Arc<BdfFont>, String> {
    let defaults = profile
        .scroll_defaults
        .as_ref()
        .ok_or("scroll_defaultsがありません")?;
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
    Ok(Arc::new(font))
}

pub(crate) fn scroll_spec(
    profile: &Profile,
    font: Option<&Arc<BdfFont>>,
    text: String,
    region: Region,
) -> Result<ScrollSpec, String> {
    let defaults = profile
        .scroll_defaults
        .as_ref()
        .ok_or("scroll_defaultsがありません")?;
    let color = defaults
        .color
        .strip_prefix('#')
        .ok_or("色は#RRGGBBで指定してください")?;
    if color.len() != 6 {
        return Err("色は#RRGGBBで指定してください".into());
    }
    Ok(ScrollSpec {
        region,
        text,
        font: font.ok_or("スクロールフォントを読み込めません")?.clone(),
        color: [
            u8::from_str_radix(&color[0..2], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&color[2..4], 16).map_err(|_| "色が不正です")?,
            u8::from_str_radix(&color[4..6], 16).map_err(|_| "色が不正です")?,
        ],
        speed_px_per_second: defaults.speed_px_per_second,
        start_padding: defaults.start_padding,
        end_padding: defaults.end_padding,
        repeat: false,
    })
}
pub const FULL: Region = Region {
    x: 0,
    y: 0,
    width: 128,
    height: 32,
};
pub const SERVICE_LEFT: Region = Region {
    x: 0,
    y: 0,
    width: 48,
    height: 32,
};
pub const RIGHT_FULL: Region = Region {
    x: 48,
    y: 0,
    width: 80,
    height: 32,
};
pub const RIGHT_TOP: Region = Region {
    x: 48,
    y: 0,
    width: 80,
    height: 16,
};
pub const RIGHT_BOTTOM: Region = Region {
    x: 48,
    y: 16,
    width: 80,
    height: 16,
};
pub const FULL_TOP: Region = Region {
    x: 0,
    y: 0,
    width: 128,
    height: 16,
};
pub const FULL_BOTTOM: Region = Region {
    x: 0,
    y: 16,
    width: 128,
    height: 16,
};
