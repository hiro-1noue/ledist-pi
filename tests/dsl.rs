use ledist_pi::{Command, parse_program};

#[test]
fn parses_atomic_frame_and_field_duration() {
    let program =
        parse_program("frame\n set service service_ja\n clear right\nend\nwait ${ja_duration}\n")
            .unwrap();
    assert!(matches!(program.commands[0], Command::Frame(_)));
    assert!(matches!(program.commands[1], Command::WaitField(ref id) if id == "ja_duration"));
}

#[test]
fn reports_line_for_unknown_statement() {
    assert!(
        parse_program("wat 3s")
            .unwrap_err()
            .to_string()
            .contains("1行目")
    );
}
