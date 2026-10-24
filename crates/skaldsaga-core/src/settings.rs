//! `.harness/settings.json`: the writer's tunable settings.
//!
//! Only the parts the pilot needs are modelled here (the session meter). The
//! file may hold other keys; they are preserved untouched when we write.

use crate::fsio::{self, FsError};
use crate::json::{self, Value, ValueExt as _};
use crate::project::Project;

/// Default: suggest wrapping up at 50% of the context window (HANDOFF, Default settings).
pub const DEFAULT_SUGGEST_WRAP_UP_PERCENT: f64 = 50.0;
/// Default: wrap up regardless at 70%.
pub const DEFAULT_WRAP_UP_PERCENT: f64 = 70.0;

/// The session meter's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterSettings {
    /// At this share of the context window, recommend wrapping up at the next natural pause.
    pub suggest_wrap_up_percent: f64,
    /// At this share, wrap up regardless.
    pub wrap_up_percent: f64,
    /// The writer's own status line, which the meter runs and shows in place of its own line.
    /// Stored as the original `statusLine` object (`type`, `command`, `padding`, ...).
    pub wrapped_statusline: Option<Value>,
}

impl Default for MeterSettings {
    fn default() -> Self {
        MeterSettings {
            suggest_wrap_up_percent: DEFAULT_SUGGEST_WRAP_UP_PERCENT,
            wrap_up_percent: DEFAULT_WRAP_UP_PERCENT,
            wrapped_statusline: None,
        }
    }
}

impl MeterSettings {
    /// Reads the meter section from a whole settings document, falling back to
    /// defaults for anything missing or out of range, so a hand-edited typo
    /// never stops the meter.
    pub fn from_settings(doc: Option<&Value>) -> Self {
        let mut s = MeterSettings::default();
        let Some(meter) = doc.and_then(|d| d.get("meter")) else {
            return s;
        };
        let pct = |key: &str| {
            meter
                .get(key)
                .and_then(Value::as_f64)
                .filter(|n| (1.0..=100.0).contains(n))
        };
        if let Some(n) = pct("suggest_wrap_up_percent") {
            s.suggest_wrap_up_percent = n;
        }
        if let Some(n) = pct("wrap_up_percent") {
            s.wrap_up_percent = n;
        }
        // A suggestion that comes after the hard stop would never fire.
        if s.suggest_wrap_up_percent > s.wrap_up_percent {
            s.suggest_wrap_up_percent = s.wrap_up_percent;
        }
        s.wrapped_statusline = meter
            .get("wrapped_statusline")
            .filter(|v| {
                v.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| !c.trim().is_empty())
            })
            .cloned();
        s
    }

    /// Loads the meter settings for a project.
    pub fn load(project: &Project) -> Result<Self, FsError> {
        let doc = fsio::read_json_object(&project.settings_path())?;
        Ok(Self::from_settings(doc.as_ref()))
    }
}

/// The meter section written by first-run setup, with the decided defaults.
pub fn default_meter_section() -> Value {
    let mut meter = json::object();
    meter.set(
        "suggest_wrap_up_percent",
        Value::from(DEFAULT_SUGGEST_WRAP_UP_PERCENT as u64),
    );
    meter.set(
        "wrap_up_percent",
        Value::from(DEFAULT_WRAP_UP_PERCENT as u64),
    );
    meter
}

/// Default: a proposal read by this many other nodes goes through the strict
/// lane (decided in build step 7).
pub const DEFAULT_STRICT_FAN_OUT: usize = 10;

/// How the dependency checker routes proposals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingSettings {
    /// Fan-out at which a proposal escalates to the strict lane.
    pub strict_fan_out: usize,
}

impl Default for RoutingSettings {
    fn default() -> Self {
        RoutingSettings {
            strict_fan_out: DEFAULT_STRICT_FAN_OUT,
        }
    }
}

impl RoutingSettings {
    /// Reads `routing` from a whole settings document, falling back to the
    /// default for anything missing or unusable.
    pub fn from_settings(doc: Option<&Value>) -> Self {
        let strict_fan_out = doc
            .and_then(|d| d.get_path(&["routing", "strict_fan_out"]))
            .and_then(Value::as_u64)
            .filter(|n| *n >= 1)
            .map_or(DEFAULT_STRICT_FAN_OUT, |n| n as usize);
        RoutingSettings { strict_fan_out }
    }

    pub fn load(project: &Project) -> Result<Self, FsError> {
        let doc = fsio::read_json_object(&project.settings_path())?;
        Ok(Self::from_settings(doc.as_ref()))
    }
}

/// Starting values for the script soft checks in `complete` (build step 9).
/// The pacing pair is decided (Default settings); the rest were chosen in the
/// step 9 build and are listed in its pull request for the author to adjust.
pub const DEFAULT_UNDER_SHARE: f64 = 0.5;
pub const DEFAULT_OVER_SHARE: f64 = 2.0;
/// An arc's pivot belief ending more than this share of the book's scenes
/// before the climax counts as an early pivot.
pub const DEFAULT_EARLY_PIVOT_SHARE: f64 = 0.25;
/// A thread whose scenes for a stage fall more than this share outside that
/// movement counts as out of alignment.
pub const DEFAULT_MISALIGNED_SHARE: f64 = 0.5;
/// Two attribute keys this close (after dropping underscores) look alike.
pub const DEFAULT_SIMILAR_KEY_MAX_DISTANCE: usize = 2;

/// Thresholds for the script soft checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckSettings {
    /// `pacing.under_share`: a movement under this share of its expected
    /// scene count is short.
    pub under_share: f64,
    /// `pacing.over_share`: a movement over this multiple of its expected
    /// scene count is long.
    pub over_share: f64,
    /// `checks.early_pivot_share`
    pub early_pivot_share: f64,
    /// `checks.misaligned_share`
    pub misaligned_share: f64,
    /// `checks.similar_key_max_distance`
    pub similar_key_max_distance: usize,
}

impl Default for CheckSettings {
    fn default() -> Self {
        CheckSettings {
            under_share: DEFAULT_UNDER_SHARE,
            over_share: DEFAULT_OVER_SHARE,
            early_pivot_share: DEFAULT_EARLY_PIVOT_SHARE,
            misaligned_share: DEFAULT_MISALIGNED_SHARE,
            similar_key_max_distance: DEFAULT_SIMILAR_KEY_MAX_DISTANCE,
        }
    }
}

impl CheckSettings {
    /// Reads the thresholds from a whole settings document, falling back to
    /// the default for anything missing or out of range.
    pub fn from_settings(doc: Option<&Value>) -> Self {
        let mut s = CheckSettings::default();
        let num = |path: &[&str], ok: fn(f64) -> bool| {
            doc.and_then(|d| d.get_path(path))
                .and_then(Value::as_f64)
                .filter(|n| ok(*n))
        };
        if let Some(n) = num(&["pacing", "under_share"], |n| n > 0.0 && n <= 1.0) {
            s.under_share = n;
        }
        if let Some(n) = num(&["pacing", "over_share"], |n| n >= 1.0) {
            s.over_share = n;
        }
        if let Some(n) = num(&["checks", "early_pivot_share"], |n| n > 0.0 && n <= 1.0) {
            s.early_pivot_share = n;
        }
        if let Some(n) = num(&["checks", "misaligned_share"], |n| (0.0..1.0).contains(&n)) {
            s.misaligned_share = n;
        }
        if let Some(n) = doc
            .and_then(|d| d.get_path(&["checks", "similar_key_max_distance"]))
            .and_then(Value::as_u64)
            .filter(|n| *n <= 5)
        {
            s.similar_key_max_distance = n as usize;
        }
        s
    }

    pub fn load(project: &Project) -> Result<Self, FsError> {
        let doc = fsio::read_json_object(&project.settings_path())?;
        Ok(Self::from_settings(doc.as_ref()))
    }
}

/// The whole settings file as first-run setup writes it (HANDOFF, Default
/// settings, and the models per sub-agent in Architecture). Every value is a
/// starting point the writer can tune. `format` lets later versions migrate
/// the file.
const DEFAULT_SETTINGS: &str = r#"{
  "format": 1,
  "meter": {
    "suggest_wrap_up_percent": 50,
    "wrap_up_percent": 70
  },
  "housekeeper": {
    "rolling_pass_every_answers": 5,
    "scoped_pass_reader_cap": 25,
    "full_pass_every_sessions": 10
  },
  "flags_per_pause": 2,
  "routing": {
    "strict_fan_out": 10
  },
  "pacing": {
    "under_share": 0.5,
    "over_share": 2.0
  },
  "checks": {
    "early_pivot_share": 0.25,
    "misaligned_share": 0.5,
    "similar_key_max_distance": 2
  },
  "models": {
    "housekeeper": "strong",
    "completeness_rubric": "strong",
    "weaver": "strong",
    "outliner": "strong",
    "harvester": "mid",
    "researcher": "mid"
  }
}"#;

/// The default settings document.
pub fn default_settings() -> Value {
    json::parse(DEFAULT_SETTINGS).expect("default settings are valid JSON")
}

/// Adds any default settings missing from `doc` (top-level keys, and keys
/// inside top-level sections), never changing a value that is already there.
/// Returns true if anything was added.
pub fn fill_missing(doc: &mut Value) -> bool {
    if !doc.is_object() {
        *doc = json::object();
    }
    let mut changed = false;
    let Value::Object(defaults) = default_settings() else {
        return false;
    };
    for (key, default) in defaults {
        match doc.get_mut(&key) {
            None => {
                doc.set(&key, default);
                changed = true;
            }
            Some(existing) => {
                if let (Value::Object(have), Value::Object(want)) = (existing, default) {
                    for (k, v) in want {
                        if !have.contains_key(&k) {
                            have.insert(k, v);
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    changed
}

/// Records (or clears) the writer's wrapped status line, keeping every other
/// key in the settings file as it was.
pub fn store_wrapped_statusline(project: &Project, wrapped: Option<Value>) -> Result<(), FsError> {
    let path = project.settings_path();
    let mut doc = fsio::read_json_object(&path)?.unwrap_or_else(json::object);
    let mut meter = doc
        .get("meter")
        .filter(|m| m.is_object())
        .cloned()
        .unwrap_or_else(default_meter_section);
    match wrapped {
        Some(v) => meter.set("wrapped_statusline", v),
        None => {
            meter.remove_key("wrapped_statusline");
        }
    }
    doc.set("meter", meter);
    fsio::write_json(&path, &doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;
    use crate::testutil::TempDir;

    #[test]
    fn defaults_when_missing() {
        assert_eq!(MeterSettings::from_settings(None), MeterSettings::default());
        let doc = parse(r#"{"other":true}"#).unwrap();
        assert_eq!(
            MeterSettings::from_settings(Some(&doc)),
            MeterSettings::default()
        );
    }

    #[test]
    fn reads_values_and_ignores_bad_ones() {
        let doc =
            parse(r#"{"meter":{"suggest_wrap_up_percent":40,"wrap_up_percent":"lots"}}"#).unwrap();
        let s = MeterSettings::from_settings(Some(&doc));
        assert_eq!(s.suggest_wrap_up_percent, 40.0);
        assert_eq!(s.wrap_up_percent, DEFAULT_WRAP_UP_PERCENT);

        let doc =
            parse(r#"{"meter":{"suggest_wrap_up_percent":0,"wrap_up_percent":150}}"#).unwrap();
        assert_eq!(
            MeterSettings::from_settings(Some(&doc)),
            MeterSettings::default()
        );
    }

    #[test]
    fn suggestion_never_comes_after_hard_stop() {
        let doc =
            parse(r#"{"meter":{"suggest_wrap_up_percent":80,"wrap_up_percent":60}}"#).unwrap();
        let s = MeterSettings::from_settings(Some(&doc));
        assert_eq!((s.suggest_wrap_up_percent, s.wrap_up_percent), (60.0, 60.0));
    }

    #[test]
    fn wrapped_statusline_needs_a_command() {
        let doc =
            parse(r#"{"meter":{"wrapped_statusline":{"type":"command","command":"  "}}}"#).unwrap();
        assert!(
            MeterSettings::from_settings(Some(&doc))
                .wrapped_statusline
                .is_none()
        );
        let doc =
            parse(r#"{"meter":{"wrapped_statusline":{"type":"command","command":"my-line"}}}"#)
                .unwrap();
        assert!(
            MeterSettings::from_settings(Some(&doc))
                .wrapped_statusline
                .is_some()
        );
    }

    #[test]
    fn defaults_match_the_meter_constants() {
        let s = MeterSettings::from_settings(Some(&default_settings()));
        assert_eq!(s, MeterSettings::default());
    }

    #[test]
    fn filling_missing_settings_keeps_existing_values() {
        let mut doc = parse(
            r#"{"meter":{"wrap_up_percent":65,"wrapped_statusline":{"command":"x"}},"mine":true}"#,
        )
        .unwrap();
        assert!(fill_missing(&mut doc));
        assert_eq!(doc["meter"]["wrap_up_percent"], 65);
        assert_eq!(doc["meter"]["suggest_wrap_up_percent"], 50);
        assert_eq!(doc["meter"]["wrapped_statusline"]["command"], "x");
        assert_eq!(doc["models"]["harvester"], "mid");
        assert_eq!(doc["mine"], true);
        assert!(!fill_missing(&mut doc), "a second pass adds nothing");
    }

    #[test]
    fn storing_wrapped_line_preserves_other_keys() {
        let dir = TempDir::new();
        let project = Project::new(dir.path());
        std::fs::create_dir_all(project.harness_dir()).unwrap();
        std::fs::write(
            project.settings_path(),
            r#"{"models":{"housekeeper":"strong"},"meter":{"wrap_up_percent":65}}"#,
        )
        .unwrap();
        let line = parse(r#"{"type":"command","command":"mine"}"#).unwrap();
        store_wrapped_statusline(&project, Some(line)).unwrap();
        let s = MeterSettings::load(&project).unwrap();
        assert_eq!(s.wrap_up_percent, 65.0);
        assert!(s.wrapped_statusline.is_some());
        let text = std::fs::read_to_string(project.settings_path()).unwrap();
        assert!(text.contains("housekeeper"));

        store_wrapped_statusline(&project, None).unwrap();
        assert!(
            MeterSettings::load(&project)
                .unwrap()
                .wrapped_statusline
                .is_none()
        );
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;

    #[test]
    fn routing_reads_the_threshold_and_falls_back() {
        let doc = |s: &str| json::parse(s).unwrap();
        assert_eq!(RoutingSettings::from_settings(None).strict_fan_out, 10);
        let d = doc(r#"{"routing": {"strict_fan_out": 4}}"#);
        assert_eq!(RoutingSettings::from_settings(Some(&d)).strict_fan_out, 4);
        for bad in [
            r#"{"routing": {"strict_fan_out": 0}}"#,
            r#"{"routing": {"strict_fan_out": "x"}}"#,
        ] {
            assert_eq!(
                RoutingSettings::from_settings(Some(&doc(bad))).strict_fan_out,
                10
            );
        }
        // First-run setup writes the same default.
        let defaults = doc(DEFAULT_SETTINGS);
        assert_eq!(
            RoutingSettings::from_settings(Some(&defaults)).strict_fan_out,
            10
        );
    }
}

#[cfg(test)]
mod check_tests {
    use super::*;

    #[test]
    fn check_thresholds_read_and_fall_back() {
        let doc = |s: &str| json::parse(s).unwrap();
        assert_eq!(CheckSettings::from_settings(None), CheckSettings::default());
        let d = doc(
            r#"{"pacing":{"under_share":0.4,"over_share":3},"checks":{"early_pivot_share":0.5,"misaligned_share":0.25,"similar_key_max_distance":1}}"#,
        );
        let s = CheckSettings::from_settings(Some(&d));
        assert_eq!(
            (s.under_share, s.over_share, s.early_pivot_share),
            (0.4, 3.0, 0.5)
        );
        assert_eq!((s.misaligned_share, s.similar_key_max_distance), (0.25, 1));
        for bad in [
            r#"{"pacing":{"under_share":0,"over_share":0.5}}"#,
            r#"{"checks":{"early_pivot_share":"x","misaligned_share":1,"similar_key_max_distance":99}}"#,
        ] {
            assert_eq!(
                CheckSettings::from_settings(Some(&doc(bad))),
                CheckSettings::default()
            );
        }
        // First-run setup writes the same defaults.
        assert_eq!(
            CheckSettings::from_settings(Some(&default_settings())),
            CheckSettings::default()
        );
    }
}
