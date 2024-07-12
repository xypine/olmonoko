pub mod from_barcode;

use chrono::Utc;

use crate::utils::time::from_timestamp;
use serde_with::As;
use serde_with::NoneAsEmptyString;

use super::event::local::NewLocalEvent;
use super::event::EventLike;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawBill {
    pub id: i32,

    // either but not both
    pub local_event_id: Option<i32>,
    pub remote_event_id: Option<i32>,

    pub payee_account_number: String,
    pub amount: i32,
    pub reference: String,

    pub payee_name: Option<String>,
    pub payee_email: Option<String>,
    pub payee_address: Option<String>,
    pub payee_phone: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventId {
    Local(i32),
    Remote(i32),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bill {
    pub id: i32,
    pub event_id: EventId,

    pub payee_account_number: String,
    pub amount: i32,
    pub reference: String,
    // due and paid_at are stored in the event
    // as starts_at and duration (paid_at - due)
    pub payee_name: Option<String>,
    pub payee_email: Option<String>,
    pub payee_address: Option<String>,
    pub payee_phone: Option<String>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
impl From<RawBill> for Bill {
    fn from(raw: RawBill) -> Self {
        Self {
            id: raw.id,
            event_id: if let Some(local_event_id) = raw.local_event_id {
                EventId::Local(local_event_id)
            } else {
                EventId::Remote(
                    raw.remote_event_id
                        .expect("either local or remote event id must be set"),
                )
            },
            payee_account_number: raw.payee_account_number,
            amount: raw.amount,
            reference: raw.reference,
            payee_name: raw.payee_name,
            payee_email: raw.payee_email,
            payee_address: raw.payee_address,
            payee_phone: raw.payee_phone,
            created_at: from_timestamp(raw.created_at),
            updated_at: from_timestamp(raw.updated_at),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewBill {
    pub event_id: EventId,

    pub payee_account_number: String,
    pub amount: i32,
    pub reference: String,

    pub payee_name: Option<String>,
    pub payee_email: Option<String>,
    pub payee_address: Option<String>,
    pub payee_phone: Option<String>,
}

pub type NewBillWithEvent = (NewLocalEvent, NewBill);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewBillForm {
    pub event_id: EventId,

    pub payee_account_number: String,
    pub amount: i32,
    pub reference: String,

    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_name: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_email: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_address: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_phone: Option<String>,
}

impl From<NewBillForm> for NewBill {
    fn from(form: NewBillForm) -> Self {
        Self {
            event_id: form.event_id,
            payee_account_number: form.payee_account_number,
            amount: form.amount,
            reference: form.reference,
            payee_name: form.payee_name,
            payee_email: form.payee_email,
            payee_address: form.payee_address,
            payee_phone: form.payee_phone,
        }
    }
}

#[allow(dead_code)]
pub trait BillLike {
    fn payee_account_number(&self) -> &str;
    fn amount(&self) -> i32;
    fn reference(&self) -> &str;
    fn payee_name(&self) -> Option<&str>;
    fn payee_email(&self) -> Option<&str>;
    fn payee_address(&self) -> Option<&str>;
    fn payee_phone(&self) -> Option<&str>;
}

impl BillLike for Bill {
    fn payee_account_number(&self) -> &str {
        &self.payee_account_number
    }
    fn amount(&self) -> i32 {
        self.amount
    }
    fn reference(&self) -> &str {
        &self.reference
    }
    fn payee_name(&self) -> Option<&str> {
        self.payee_name.as_deref()
    }
    fn payee_email(&self) -> Option<&str> {
        self.payee_email.as_deref()
    }
    fn payee_address(&self) -> Option<&str> {
        self.payee_address.as_deref()
    }
    fn payee_phone(&self) -> Option<&str> {
        self.payee_phone.as_deref()
    }
}
impl BillLike for NewBill {
    fn payee_account_number(&self) -> &str {
        &self.payee_account_number
    }
    fn amount(&self) -> i32 {
        self.amount
    }
    fn reference(&self) -> &str {
        &self.reference
    }
    fn payee_name(&self) -> Option<&str> {
        self.payee_name.as_deref()
    }
    fn payee_email(&self) -> Option<&str> {
        self.payee_email.as_deref()
    }
    fn payee_address(&self) -> Option<&str> {
        self.payee_address.as_deref()
    }
    fn payee_phone(&self) -> Option<&str> {
        self.payee_phone.as_deref()
    }
}

pub trait AutoDescription {
    fn generate_description(&self, event: Option<&impl EventLike>) -> String;
}

impl<T: BillLike> AutoDescription for T {
    fn generate_description(&self, event: Option<&impl EventLike>) -> String {
        let paid_info = event
            .and_then(|e| e.duration())
            .map(|_| " (paid)")
            .unwrap_or_default();
        format!(
            r#"=== BILL{paid_info} ===
Payee bank account: {}
Reference: {}
Amount: {}€"#,
            self.payee_account_number(),
            self.reference(),
            (self.amount() as f64) / 100.0,
            // self.payee_name().unwrap_or("Unknown"),
            // self.payee_email().unwrap_or("Unknown"),
            // self.payee_address().unwrap_or("Unknown"),
            // self.payee_phone().unwrap_or("Unknown"),
        )
    }
}
