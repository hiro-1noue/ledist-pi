use ledist_pi::{AssetRegistry, Profile, compile_pattern};
use std::fs;

#[test]
fn toml_pattern_compiles_a_static_page_without_code_changes() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("train/assets/icon")).unwrap();
    image::RgbImage::new(1, 1)
        .save(root.path().join("train/assets/icon/a.png"))
        .unwrap();
    fs::write(root.path().join("pattern.toml"), "repeat = false\n[[page]]\nseconds = 1\n[[page.layer]]\ndirectory = 'assets/icon'\nasset = 'a'\nx = 0\ny = 0\nwidth = 1\nheight = 1\n").unwrap();
    let profile = Profile::from_toml(
        "[profile]\nid='future'\nname='Future'\ncanvas_width=1\ncanvas_height=1",
    )
    .unwrap();
    let registry = AssetRegistry::scan(&root.path().join("train")).unwrap();
    assert!(
        compile_pattern(
            &profile,
            &registry,
            &root.path().join("pattern.toml"),
            root.path()
        )
        .is_ok()
    );
}
