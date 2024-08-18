use actix_web::{
    cookie::{Cookie, SameSite},
    HttpResponseBuilder,
};

/// A temporary message displayed to the user on the next page load and then removed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FlashLevel {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
}
impl std::fmt::Display for FlashLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlashLevel::Info => write!(f, "info"),
            FlashLevel::Warning => write!(f, "warning"),
            FlashLevel::Error => write!(f, "error"),
        }
    }
}
impl From<&str> for FlashLevel {
    fn from(s: &str) -> Self {
        match s {
            "info" => FlashLevel::Info,
            "warning" => FlashLevel::Warning,
            "error" => FlashLevel::Error,
            _ => FlashLevel::Error,
        }
    }
}

pub const FLASH_COOKIE_NAME: &str = "X-OLMONOKO-Flash";
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlashMessage {
    level: FlashLevel,
    message: String,
}

#[allow(dead_code)]
impl FlashMessage {
    pub fn new(level: FlashLevel, message: &str) -> Self {
        Self {
            level,
            message: message.to_string(),
        }
    }
    pub fn from_cookie(cookie: &Cookie) -> Self {
        Self::from(cookie.value())
    }
    pub fn to_cookie(&self) -> Cookie {
        Cookie::build(
            FLASH_COOKIE_NAME,
            format!("{}:{}", self.level, self.message),
        )
        .path("/")
        .same_site(SameSite::Strict)
        .finish()
    }

    pub fn info(message: &str) -> Self {
        Self::new(FlashLevel::Info, message)
    }
    pub fn warning(message: &str) -> Self {
        Self::new(FlashLevel::Warning, message)
    }
    pub fn error(message: &str) -> Self {
        Self::new(FlashLevel::Error, message)
    }
}
impl std::fmt::Display for FlashMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.level, self.message)
    }
}
impl From<&str> for FlashMessage {
    fn from(s: &str) -> Self {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        let level = FlashLevel::from(parts[0]);
        let message = parts[1].to_string();
        Self { level, message }
    }
}
fn with_flash_message(
    mut builder: HttpResponseBuilder,
    flash: FlashMessage,
) -> HttpResponseBuilder {
    builder.cookie(flash.to_cookie());
    builder
}
pub trait WithFlashMessage {
    fn with_flash_message(self, flash: FlashMessage) -> Self;
}
impl WithFlashMessage for HttpResponseBuilder {
    fn with_flash_message(self, flash: FlashMessage) -> Self {
        with_flash_message(self, flash)
    }
}
