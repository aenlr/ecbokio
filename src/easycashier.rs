use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use ureq::Error;
use http::header::{ACCEPT, USER_AGENT};
use serde_json::Value;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::utils;
use crate::utils::{PageReq, APPLICATION_JSON, DEFAULT_USER_AGENT};

pub const EASYCASHIER_URL: &str = "https://backoffice.easycashier.se";

#[derive(Debug)]
pub struct EasyCashier {
    base_url: String,
    pub company: String,
    token: String,
}

#[derive(Deserialize)]
pub struct EasyCashierMetaInformation {
    #[serde(rename = "currentPage")]
    pub current_page: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
    #[serde(rename = "totalResources")]
    pub total_resources: u32,
}

#[derive(Deserialize, Serialize)]
pub struct ZRapportTrans {
    #[serde(rename = "accountNumber")]
    pub account_number: u16,
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub struct ZRapport {
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: u32,
    #[serde(rename = "storeNumber")]
    pub store_number: u32,
    #[serde(rename = "cashRegisterNumber")]
    pub cash_register_number: u32,
    #[serde(rename = "firstReceipt")]
    pub first_receipt: u32,
    #[serde(rename = "lastReceipt")]
    pub last_receipt: u32,
    #[serde(rename = "dateCreated")]
    pub date_created: String,
    #[serde(rename = "companyName")]
    pub company_name: String,
    #[serde(rename = "corporateIdentity")]
    pub corporate_identity: String,


    #[serde(rename = "zReportTransactions")]
    pub z_report_transactions: Vec<ZRapportTrans>,

    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}
impl ZRapport {
    pub fn datum(&self) -> String {
        self.date_created[0..10].to_string()
    }

    pub fn konto(&self, account: u16) -> Decimal {
        self.z_report_transactions.iter()
            .filter(|tr| tr.account_number == account)
            .map(|tr| tr.amount)
            .sum()
    }

    pub fn verifikatnamn(&self) -> String {
        format!("Z, Bu: {} Ka: {} Nr: {} Kv: {} - {}",
            self.store_number,
            self.cash_register_number,
            self.sequence_number,
            self.first_receipt,
            self.last_receipt
        )
    }
}


#[derive(Deserialize)]
pub struct ZRapportListResponse {
    #[serde(rename = "metaInformation")]
    pub meta_information: EasyCashierMetaInformation,
    pub items: Vec<ZRapport>,
}


impl EasyCashier {
    pub fn login(
        base_url: &str,
        username: &str,
        password: &str,
        orgnummer: &Option<String>,
    ) -> Result<EasyCashier, Error> {
        let url = format!("{}/v1/login", base_url);
        let mut body = std::collections::HashMap::new();
        body.insert("username", username);
        body.insert("password", password);
        let res = ureq::post(url)
            .header(ACCEPT, APPLICATION_JSON)
            .send_json(&body)?
            .body_mut()
            .read_json::<serde_json::Map<String, Value>>()?;
        let token = res
            .get("accessToken")
            .and_then(|v| v.as_str())
            .expect("No accessToken in login response");
        let default_company = res
            .get("preferredCorporateIdentity")
            .and_then(|v| v.as_str().map(|v| v.to_string()));
        let company = orgnummer
            .clone()
            .or(default_company)
            .expect("No default company in EasyCashier");
        Ok(EasyCashier {
            company,
            base_url: base_url.to_string(),
            token: token.to_string(),
        })
    }

    pub fn zrapporter(
        &self,
        date: &DateRequest,
        page: &PageReq,
    ) -> Result<ZRapportListResponse, Error> {
        let date_params = format!(
            "dateSelectionType={}&startDate={}&stopDate={}",
            date.date_type,
            utils::format_local_date(&date.start_date),
            utils::format_local_date(&date.end_date)
        );
        let page_params = format!(
            "itemsPerPage={}&pageNumber={}&sortColumn=sequenceNumber&sortDirection=asc",
            page.size, page.page
        );
        let url = format!(
            "{}/v1/company/{}/zReport?{}&{}",
            self.base_url, self.company, page_params, date_params
        );
        ureq::get(url)
            .header(ACCEPT, APPLICATION_JSON)
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header("X-Auth-Token", &self.token)
            .call()?
            .body_mut()
            .read_json::<ZRapportListResponse>()
    }

    pub fn zrapport_pdf(&self, rapport: &ZRapport) -> Result<(Vec<u8>, String), Error> {
        let url = format!(
            "{}/v1/company/{}/zReport/{}/{}/{}/pdf",
            self.base_url,
            self.company,
            rapport.store_number,
            rapport.cash_register_number,
            rapport.sequence_number
        );

        let pdf = ureq::get(url)
            .header(ACCEPT, "application/pdf")
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header("X-Auth-Token", &self.token)
            .call()?
            .body_mut()
            .read_to_vec()?;

        let filename = format!(
            "Z-Rapport_{}_{}-{}-{}.pdf",
            self.company, rapport.store_number, rapport.cash_register_number, rapport.sequence_number
        );

        Ok((pdf, filename))
    }
}

#[derive(Debug)]
pub struct DateRequest {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub date_type: String,
}

impl DateRequest {
    pub fn new(start_date: &Option<NaiveDate>, end_date: &Option<NaiveDate>) -> Self {
        let today = chrono::Local::now().date_naive();
        let (start_date, end_date) = {
            if let Some(start_date) = start_date {
                if let Some(end_date) = end_date {
                    (*start_date, *end_date)
                } else {
                    (*start_date, today)
                }
            } else if let Some(end_date) = end_date {
                if today < *end_date {
                    (today, *end_date)
                } else {
                    (*end_date, *end_date)
                }
            } else {
                (today, today)
            }
        };

        let date_type = {
            if start_date == end_date {
                if start_date == today {
                    "today"
                } else {
                    "date"
                }
            } else {
                "interval"
            }
        };

        Self {
            start_date,
            end_date,
            date_type: date_type.to_string(),
        }
    }
}