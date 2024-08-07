#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EventFilter {
    pub summary_like: Option<String>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub min_priority: Option<Priority>,
    pub max_priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub attendance_planned: Option<bool>,
    pub attendance_actual: Option<bool>,
    pub show_filter: bool,
}
impl EventFilter {
    pub fn is_defined(&self) -> bool {
        self.summary_like.is_some()
            || self.after.is_some()
            || self.before.is_some()
            || self.min_priority.is_some()
            || self.max_priority.is_some()
            || self.tags.is_some()
            || self.exclude_tags.is_some()
    }
}

use serde_with::As;
use serde_with::NoneAsEmptyString;

use crate::models::event::Priority;
use crate::models::ics_source::deserialize_checkbox;
use crate::models::ics_source::serialize_checkbox;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct RawEventFilter {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub summary_like: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub min_priority: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub max_priority: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub tags: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub exclude_tags: Option<String>,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attendance_planned: bool,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attendance_planned_filter_enabled: bool,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attendance_actual: bool,
    #[serde(
        deserialize_with = "deserialize_checkbox",
        serialize_with = "serialize_checkbox",
        default
    )]
    pub attendance_actual_filter_enabled: bool,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub show_filter: Option<String>,
}
impl From<RawEventFilter> for EventFilter {
    fn from(raw: RawEventFilter) -> Self {
        Self {
            summary_like: raw.summary_like,
            after: None,
            before: None,
            min_priority: raw.min_priority.and_then(|s| s.parse().ok()),
            max_priority: raw.max_priority.and_then(|s| s.parse().ok()),
            tags: raw.tags.map(|s| {
                s.split(',')
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
            exclude_tags: raw.exclude_tags.map(|s| {
                s.split(',')
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
            attendance_actual: if raw.attendance_actual_filter_enabled {
                Some(raw.attendance_actual)
            } else {
                None
            },
            attendance_planned: if raw.attendance_planned_filter_enabled {
                Some(raw.attendance_planned)
            } else {
                None
            },
            show_filter: raw.show_filter.map(|s| s == "true").unwrap_or(false),
        }
    }
}
impl RawEventFilter {
    pub fn is_defined(&self) -> bool {
        self.summary_like.is_some()
            || self.min_priority.is_some()
            || self.max_priority.is_some()
            || self.tags.is_some()
            || self.exclude_tags.is_some()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawEventDateFilter {
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub after: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub before: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawEventFilterWithDate {
    #[serde(flatten)]
    pub base: RawEventFilter,
    #[serde(flatten)]
    pub date: RawEventDateFilter,
}
impl From<RawEventFilterWithDate> for EventFilter {
    fn from(raw: RawEventFilterWithDate) -> Self {
        let mut base = EventFilter::from(raw.base);
        let after = raw.date.after.and_then(|s| s.parse().ok());
        let before = raw.date.before.and_then(|s| s.parse().ok());
        base.after = after;
        base.before = before;
        base
    }
}
impl RawEventFilterWithDate {
    pub fn is_defined(&self) -> bool {
        self.base.is_defined() || self.date.after.is_some() || self.date.before.is_some()
    }
}
