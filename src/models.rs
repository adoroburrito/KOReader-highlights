use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq)]
pub struct BookData {
    pub title: String,
    pub author: String,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    pub chapter: Option<String>,
    pub page: i32,
    pub text: String,
    pub note: Option<String>,
    pub datetime: NaiveDateTime,
}
