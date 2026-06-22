use ledist_pi::{BdfFont, Region, ScriptAction, ScriptEvent, ScriptRunner, ScrollSpec};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn scroll_advances_by_elapsed_time_and_then_allows_wait_scroll_end() {
    let font = Arc::new(BdfFont::parse_bdf("STARTFONT 2.1\nFONTBOUNDINGBOX 1 1 0 0\nSTARTCHAR A\nENCODING 65\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\nENDFONT\n").unwrap());
    let scroll = ScrollSpec {
        region: Region {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        text: "A".into(),
        font,
        color: [255, 0, 0],
        speed_px_per_second: 1.0,
        start_padding: 0,
        end_padding: 0,
        repeat: false,
    };
    let mut runner = ScriptRunner::new(
        2,
        1,
        vec![
            ScriptAction::Present {
                frame: ledist_pi::RgbFrame::black(2, 1),
                scroll: Some(scroll),
            },
            ScriptAction::WaitScrollEnd,
            ScriptAction::Blank,
        ],
        None,
    );
    let start = Instant::now();
    assert!(matches!(
        runner.tick(start).unwrap().first(),
        Some(ScriptEvent::Present(_))
    ));
    assert!(matches!(
        runner.tick(start + Duration::from_secs(4)).unwrap().last(),
        Some(ScriptEvent::Blank)
    ));
}

#[test]
fn infinite_cycle_restarts_after_its_wait() {
    let mut runner = ScriptRunner::new(
        1,
        1,
        Vec::new(),
        Some(vec![
            ScriptAction::Brightness(17),
            ScriptAction::Wait(Duration::from_secs(1)),
        ]),
    );
    let start = Instant::now();
    assert!(matches!(
        runner.tick(start).unwrap().first(),
        Some(ScriptEvent::Brightness(17))
    ));
    assert!(matches!(
        runner.tick(start + Duration::from_secs(2)).unwrap().first(),
        Some(ScriptEvent::Brightness(17))
    ));
}
