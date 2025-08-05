use crate::utils::{APPLICATION_JSON, PageReq};
use chrono::NaiveDate;
use http::header::{ACCEPT, AUTHORIZATION};
use mime::Mime;
use multipart::client::lazy::Multipart;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use ureq::Error;

pub const BOKIO_API_URL: &str = "https://api.bokio.se";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct JournalEntryAccount {
    pub id: i64,
    pub account: i32,
    #[serde(with = "rust_decimal::serde::float")]
    pub debit: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub credit: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub title: String,
    #[serde(rename = "journalEntryNumber")]
    pub journal_entry_number: String,
    pub date: String,
    pub items: Vec<JournalEntryAccount>,
    #[serde(rename = "reversingJournalEntryId")]
    pub reversing_journal_entry_id: Option<String>,
    #[serde(rename = "reversedByJournalEntryId")]
    pub reversed_by_journal_entry_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JournalEntryListing {
    #[serde(rename = "totalItems")]
    pub total_items: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
    #[serde(rename = "currentPage")]
    pub current_page: u32,
    pub items: Vec<JournalEntry>,
}

#[derive(Serialize)]
pub struct CreateJournalAccount {
    pub account: i32,
    #[serde(with = "rust_decimal::serde::float")]
    pub debit: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub credit: Decimal,
}

#[derive(Serialize)]
pub struct CreateJournal {
    pub title: String,
    pub date: String,
    pub items: Vec<CreateJournalAccount>,
}

#[derive(Debug, Deserialize)]
pub struct UploadResponse {
    pub id: String,
    pub description: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(rename = "journalEntryId")]
    pub journal_entry_id: String,
}

pub struct Bokio {
    base_url: String,
    company_id: String,
    auth_header: String,
}

impl Bokio {
    pub fn new(base_url: &str, company_id: &str, token: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            company_id: company_id.to_string(),
            auth_header: format!("Bearer {}", token),
        }
    }

    pub fn create_journal_entry(&self, entry: &CreateJournal) -> Result<JournalEntry, Error> {
        let url = format!(
            "{}/companies/{}/journal-entries",
            self.base_url, self.company_id
        );

        ureq::post(&url)
            .header(ACCEPT, APPLICATION_JSON)
            .header(AUTHORIZATION, &self.auth_header)
            .send_json(&entry)?
            .body_mut()
            .read_json::<JournalEntry>()
    }

    pub fn upload(
        &self,
        filename: &str,
        content_type: &str,
        journal_entry_id: &str,
    ) -> Result<UploadResponse, Error> {
        let url = format!("{}/companies/{}/uploads", self.base_url, self.company_id);

        let mut m = Multipart::new();
        let file = std::fs::File::open(&filename).expect(&format!("Kunde inte öppna {}", filename));
        let basename = std::path::Path::new(&filename)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        m.add_stream(
            "file",
            file,
            Some(basename),
            Mime::from_str(content_type).ok(),
        );
        m.add_text("journalEntryId", journal_entry_id);

        let mut prepared = m.prepare().unwrap();
        let mut vec: Vec<u8> = Vec::new();
        std::io::copy(&mut prepared, &mut vec).unwrap();
        let boundary = prepared.boundary();
        ureq::post(url)
            .content_type(format!("multipart/form-data; boundary={}", boundary))
            .header(ACCEPT, APPLICATION_JSON)
            .header(AUTHORIZATION, &self.auth_header)
            .send(&vec)?
            .body_mut()
            .read_json::<UploadResponse>()
    }

    fn _list_journal_entries(&self, page: &PageReq) -> Result<JournalEntryListing, Error> {
        let url = format!(
            "{}/companies/{}/journal-entries?page={}&pageSize={}",
            self.base_url, self.company_id, page.page, page.size
        );

        ureq::get(url)
            .header(ACCEPT, APPLICATION_JSON)
            .header(AUTHORIZATION, &self.auth_header)
            .call()?
            .body_mut()
            .read_json::<JournalEntryListing>()
    }

    pub fn list_journal(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<JournalEntry>, Error> {
        let mut page = PageReq { page: 1, size: 100 };
        let mut result: Vec<JournalEntry> = Vec::new();
        let mut reached_end = false;
        // Results are returned in descending order
        while !reached_end {
            let lst = self._list_journal_entries(&page)?;
            if lst.items.is_empty() {
                break;
            }

            if let Some(start_date) = start_date {
                if let Some(end_date) = end_date {
                    for entry in lst.items {
                        let date = entry.date.parse::<NaiveDate>().unwrap();
                        if date < start_date {
                            reached_end = true;
                            break;
                        } else if date <= end_date && entry.title.starts_with("Z") {
                            result.push(entry);
                        }
                    }
                } else {
                    for entry in lst.items {
                        let date = entry.date.parse::<NaiveDate>().unwrap();
                        if date < start_date {
                            reached_end = true;
                        } else if entry.title.starts_with("Z") {
                            result.push(entry);
                        }
                    }
                }
            } else if let Some(end_date) = end_date {
                for entry in lst.items {
                    let date = entry.date.parse::<NaiveDate>().unwrap();
                    if date <= end_date && entry.title.starts_with("Z") {
                        result.push(entry);
                    }
                }
            } else {
                for entry in lst.items {
                    if entry.title.starts_with("Z") {
                        result.push(entry);
                    }
                }
            }

            page.page += 1;
        }

        Ok(result)
    }
}
