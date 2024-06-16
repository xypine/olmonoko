#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EventFilter {
    pub summary_like: Option<String>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub min_priority: Option<i64>,
    pub max_priority: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct RawEventFilter {
    pub summary_like: Option<String>,
    pub min_priority: Option<String>,
    pub max_priority: Option<String>,
    pub tags: Option<String>,
    pub exclude_tags: Option<String>,
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
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawEventDateFilter {
    pub after: Option<String>,
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
