use std::io::{IsTerminal, Write};
use std::str::FromStr;
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

fn read_prompt(prompt: &str) -> std::io::Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush().and_then(|_| {
        let mut val = String::new();
        match std::io::stdin().read_line(&mut val) {
            Ok(_) => Ok(val.trim().to_string()),
            Err(e) => Err(e),
        }
    })
}

pub fn read_prompt_trim(prompt: &str) -> String {
    read_prompt(prompt).unwrap().trim().to_string()
}

fn read_password(prompt: &str) -> std::io::Result<String> {
    // IntelliJ console is broken giving "device not ready" for /dev/tty.
    // Strangely the builtin terminal works fine.
    if std::io::stdin().is_terminal() && !std::env::var("BROKEN_TERMINAL").is_ok() {
        rpassword::prompt_password(prompt)
    } else {
        read_prompt(prompt)
    }
}

pub fn read_password_trim(prompt: &str) -> String {
    read_password(prompt).unwrap().trim().to_string()
}

pub fn to_date(s: String) -> NaiveDate {
    NaiveDate::from_str(&s).unwrap()
}

pub fn get_env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or(default.into())
}

pub fn get_env(key: &str) -> String {
    get_env_or_default(key, "")
}

pub fn format_orgnr(orgnr: &str) -> String {
    let mut orgnr = orgnr.trim().to_string();
    let len = orgnr.len();
    if orgnr.chars().all(|c| c.is_ascii_digit()) && [10, 12].contains(&len) {
        orgnr = format!("{}-{}", &orgnr[0..len - 4], &orgnr[len - 4..len]);
    }
    orgnr
}
