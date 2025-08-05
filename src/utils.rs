use chrono::NaiveDate;

#[derive(Debug)]
pub struct PageReq {
    pub page: u32,
    pub size: u32,
}

pub fn format_local_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:140.0) Gecko/20100101 Firefox/140.0";
pub const APPLICATION_JSON: &'static str = "application/json";