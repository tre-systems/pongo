//! Keyboard input handling
//!
//! Both handlers return the new paddle direction (-1 up, 0 stop, 1 down). The
//! WASD/arrow-key mapping is the input contract.

pub fn handle_key_down(key: &str, current_dir: i8) -> i8 {
    match key {
        "ArrowUp" | "w" | "W" => -1,
        "ArrowDown" | "s" | "S" => 1,
        _ => current_dir,
    }
}

pub fn handle_key_up(key: &str, current_dir: i8) -> i8 {
    match key {
        "ArrowUp" | "w" | "W" | "ArrowDown" | "s" | "S" => 0,
        _ => current_dir,
    }
}
