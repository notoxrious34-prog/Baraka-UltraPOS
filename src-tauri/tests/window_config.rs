//! BRK-059: pins the main window geometry from `tauri.conf.json` per ADR-066.
//! Default inner size 1280x640, hard minimum 1024x600, created in setup()
//! (config `create: false`, see BRK-058). Outer-window fit on the owner's
//! screen is NOT verified here; a later AUD task measures it.

use serde_json::Value;

fn windows_array(config_text: &str) -> Vec<Value> {
    let config: Value =
        serde_json::from_str(config_text).expect("tauri.conf.json must parse as JSON");
    let windows = config
        .pointer("/app/windows")
        .expect("tauri.conf.json must contain app.windows");
    windows
        .as_array()
        .expect("app.windows must be an array")
        .clone()
}

fn window_number(window: &Value, key: &str) -> u64 {
    window
        .get(key)
        .unwrap_or_else(|| panic!("main window must define {key}"))
        .as_u64()
        .unwrap_or_else(|| panic!("main window {key} must be a JSON integer"))
}

fn window_bool_absent_or_true(window: &Value, key: &str) {
    match window.get(key) {
        None | Some(Value::Bool(true)) => {}
        other => panic!("main window {key} must be absent or true, found {other:?}"),
    }
}

#[test]
fn main_window_geometry_matches_adr066() {
    let windows = windows_array(include_str!("../tauri.conf.json"));
    assert_eq!(
        windows.len(),
        1,
        "expected exactly one window entry, found {}",
        windows.len()
    );
    let main = &windows[0];

    assert_eq!(window_number(main, "width"), 1280, "width must be 1280");
    assert_eq!(window_number(main, "height"), 640, "height must be 640");
    assert_eq!(
        window_number(main, "minWidth"),
        1024,
        "minWidth must be 1024"
    );
    assert_eq!(
        window_number(main, "minHeight"),
        600,
        "minHeight must be 600"
    );
    assert!(
        window_number(main, "minWidth") <= window_number(main, "width"),
        "minWidth must not exceed width"
    );
    assert!(
        window_number(main, "minHeight") <= window_number(main, "height"),
        "minHeight must not exceed height"
    );
    assert_eq!(
        main.get("create"),
        Some(&Value::Bool(false)),
        "main window must keep create=false (built in setup)"
    );
}

#[test]
fn main_window_maximized_policy_brk060() {
    let windows = windows_array(include_str!("../tauri.conf.json"));
    let main = &windows[0];

    assert_eq!(
        main.get("maximized"),
        Some(&Value::Bool(true)),
        "main window must open maximized (BRK-060)"
    );
    assert_eq!(
        main.get("center"),
        Some(&Value::Bool(true)),
        "main window must keep center=true"
    );
    // Resize/maximize/restore depend on these; tao defaults are true.
    window_bool_absent_or_true(main, "resizable");
    window_bool_absent_or_true(main, "maximizable");
    window_bool_absent_or_true(main, "decorations");
}
