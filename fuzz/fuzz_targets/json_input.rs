//! JSON from outside: the status line input Claude Code sends, and the
//! settings files, read as whatever JSON they hold.
#![no_main]

use libfuzzer_sys::fuzz_target;
use skaldsaga_core::json;
use skaldsaga_core::meter::StatusInput;
use skaldsaga_core::settings::{CheckSettings, MeterSettings, RoutingSettings, SessionSettings};
use skaldsaga_core::statusline;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = StatusInput::parse(text);
    if let Ok(value) = json::parse(text) {
        let _ = MeterSettings::from_settings(Some(&value));
        let _ = SessionSettings::from_settings(Some(&value));
        let _ = RoutingSettings::from_settings(Some(&value));
        let _ = CheckSettings::from_settings(Some(&value));
        let _ = statusline::is_meter_command(&value);
    }
});
