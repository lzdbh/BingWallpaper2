use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("网络请求失败：{0}")]
    Network(String),
    #[error("Bing 接口返回异常：{0}")]
    Bing(String),
    #[error("文件操作失败：{0}")]
    Io(String),
    #[error("数据库错误：{0}")]
    Database(String),
    #[error("当前平台不支持该壁纸模式")]
    UnsupportedStyle,
    #[error("参数无效：{0}")]
    InvalidInput(String),
    #[error("更新校验失败：{0}")]
    Update(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Network(_) => "Network",
            Self::Bing(_) => "Bing",
            Self::Io(_) => "Io",
            Self::Database(_) => "Database",
            Self::UnsupportedStyle => "Unsupported",
            Self::InvalidInput(_) => "InvalidInput",
            Self::Update(_) => "Update",
            Self::Other(_) => "Unknown",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Self::Network(format!("请求超时（{value}）"))
        } else if value.is_connect() {
            Self::Network(format!("无法连接服务器（{value}）"))
        } else {
            Self::Network(value.to_string())
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Other(format!("JSON 解析失败：{value}"))
    }
}

impl From<image::ImageError> for AppError {
    fn from(value: image::ImageError) -> Self {
        Self::Io(format!("图片处理失败：{value}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
