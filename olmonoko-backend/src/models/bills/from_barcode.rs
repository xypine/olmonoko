use crate::models::event::local::NewLocalEvent;

use super::EventId;
use super::NewBill;
use super::NewBillWithEvent;
use chrono::NaiveTime;
use serde_with::As;
use serde_with::NoneAsEmptyString;

/// See https://www.finanssiala.fi/wp-content/uploads/2021/03/Bank_bar_code_guide.pdf for the spec

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewBillBarcodeForm {
    pub summary: String,
    pub barcode: String,

    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_name: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_email: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_address: Option<String>,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub payee_phone: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewBillBarcodeFormWithUserId {
    pub user_id: i64,
    pub form: NewBillBarcodeForm,
}

const BILL_DEFAULT_PRIORITY: i64 = 1; // Highest possible priority
impl TryFrom<NewBillBarcodeFormWithUserId> for NewBillWithEvent {
    type Error = &'static str;

    fn try_from(data: NewBillBarcodeFormWithUserId) -> Result<Self, Self::Error> {
        let NewBillBarcodeFormWithUserId { user_id, form } = data;
        let barcode = decode_barcode(&form.barcode).ok_or("invalid barcode")?;
        let due = barcode
            .due
            .ok_or("invoice is missing a due date")?
            .and_time(NaiveTime::MIN)
            .and_utc()
            .timestamp();
        let new_bill = NewBill {
            event_id: EventId::Local(-1), // placeholder
            payee_account_number: barcode.payee_account_number.clone(),
            amount: barcode.amount_cents,
            reference: barcode.reference.clone(),
            payee_name: form.payee_name,
            payee_email: form.payee_email,
            payee_address: form.payee_address,
            payee_phone: form.payee_phone,
        };
        let description = None;
        let uid = format!(
            "bill-{}-{}@olmonoko",
            barcode.payee_account_number, barcode.reference
        );
        let new_event = NewLocalEvent {
            user_id,
            tags: vec!["olmonoko::bill".to_string()],
            priority: Some(BILL_DEFAULT_PRIORITY),
            starts_at: due,
            all_day: true,
            duration: None,
            summary: form.summary,
            description,
            location: None,
            uid,
        };
        Ok((new_event, new_bill))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SupportedBarcodeVersion {
    V4,
    V5,
}
impl TryFrom<char> for SupportedBarcodeVersion {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '4' => Ok(Self::V4),
            '5' => Ok(Self::V5),
            _ => Err("unsupported barcode version"),
        }
    }
}
impl From<SupportedBarcodeVersion> for u8 {
    fn from(version: SupportedBarcodeVersion) -> Self {
        match version {
            SupportedBarcodeVersion::V4 => 4,
            SupportedBarcodeVersion::V5 => 5,
        }
    }
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Barcode {
    pub version: SupportedBarcodeVersion,
    pub payee_account_number: String,
    pub amount_cents: i64,
    pub reference: String,
    pub due: Option<chrono::NaiveDate>,
}
pub fn decode_barcode(barcode: &str) -> Option<Barcode> {
    let mut chars = barcode.chars();
    let input = chars.by_ref();
    let version = SupportedBarcodeVersion::try_from(input.next()?).ok()?;
    // only Finnish IBANs are supported by the spec for now
    let payee_account_number = format!("FI{}", input.take(16).collect::<String>());
    let amount_cents = input.take(6 + 2).collect::<String>().parse().ok()?;
    let reference = match version {
        SupportedBarcodeVersion::V5 => {
            // 2 checksum chars, 21 data chars
            let checksum = input.take(2).collect::<String>();
            let data_raw = input.take(21).collect::<String>();
            // remove leading zeros from data
            let data = data_raw.trim_start_matches('0');
            format!("RF{}{}", checksum, data)
        }
        SupportedBarcodeVersion::V4 => {
            // skip 3 reserved chars
            let raw = input.skip(3).take(20).collect::<String>();
            // remove leading zeros
            raw.trim_start_matches('0').to_string()
        }
    };
    let due_year: u32 = input.take(2).collect::<String>().parse().ok()?;
    let due_month: u32 = input.take(2).collect::<String>().parse().ok()?;
    let due_day: u32 = input.take(2).collect::<String>().parse().ok()?;
    let due = if due_year + due_month + due_day == 0 {
        None
    } else {
        Some(chrono::NaiveDate::from_ymd_opt(
            2000 + due_year as i32,
            due_month,
            due_day,
        )?)
    };
    if input.next().is_some() {
        // the spec doesn't allow extra characters
        return None;
    }

    Some(Barcode {
        version,
        payee_account_number,
        amount_cents,
        reference,
        due,
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_decode_barcode_v4_example_1() {
        let barcode = "479440520200360820048831500000000868516259619897100612";
        let decoded = decode_barcode(barcode).unwrap();
        assert_eq!(decoded.version, SupportedBarcodeVersion::V4);
        assert_eq!(decoded.payee_account_number, "FI7944052020036082");
        assert_eq!(decoded.amount_cents, 488315);
        assert_eq!(decoded.reference, "868516259619897");
        assert_eq!(
            decoded.due,
            Some(chrono::NaiveDate::from_ymd_opt(2010, 6, 12).unwrap())
        );
    }

    #[test]
    fn test_decode_barcode_v4_example_5() {
        let barcode = "416800014000502670009358500000078777679656628687000000";
        let decoded = decode_barcode(barcode).unwrap();
        assert_eq!(decoded.version, SupportedBarcodeVersion::V4);
        assert_eq!(decoded.payee_account_number, "FI1680001400050267");
        assert_eq!(decoded.amount_cents, 93585);
        assert_eq!(decoded.reference, "78777679656628687");
        assert_eq!(decoded.due, None);
    }

    #[test]
    fn test_decode_barcode_v4_example_9() {
        let barcode = "492393900010033910000000200000000000000001357914991224";
        let decoded = decode_barcode(barcode).unwrap();
        assert_eq!(decoded.version, SupportedBarcodeVersion::V4);
        assert_eq!(decoded.payee_account_number, "FI9239390001003391");
        assert_eq!(decoded.amount_cents, 2);
        // disabled until we implement the reference spec
        assert_eq!(decoded.reference, "1357914");
        assert_eq!(
            decoded.due,
            Some(chrono::NaiveDate::from_ymd_opt(2099, 12, 24).unwrap())
        );
    }

    #[test]
    fn test_decode_barcode_v5_example_1() {
        let barcode = "579440520200360820048831509000000868516259619897100612";
        let decoded = decode_barcode(barcode).unwrap();
        assert_eq!(decoded.version, SupportedBarcodeVersion::V5);
        assert_eq!(decoded.payee_account_number, "FI7944052020036082");
        assert_eq!(decoded.amount_cents, 488315);
        assert_eq!(decoded.reference, "RF09868516259619897");
        assert_eq!(
            decoded.due,
            Some(chrono::NaiveDate::from_ymd_opt(2010, 6, 12).unwrap())
        );
    }

    #[test]
    fn test_decode_barcode_v5_example_9() {
        let barcode = "592393900010033910000000295000000000000001357914991224";
        let decoded = decode_barcode(barcode).unwrap();
        assert_eq!(decoded.version, SupportedBarcodeVersion::V5);
        assert_eq!(decoded.payee_account_number, "FI9239390001003391");
        assert_eq!(decoded.amount_cents, 2);
        assert_eq!(decoded.reference, "RF951357914");
        assert_eq!(
            decoded.due,
            Some(chrono::NaiveDate::from_ymd_opt(2099, 12, 24).unwrap())
        );
    }

    #[test]
    fn from_real_bill_1() {
        let form = NewBillBarcodeForm {
            summary: "".to_string(),
            barcode: "463800012701634140000585000000800181102211349629240603".to_string(),
            payee_name: None,
            payee_email: None,
            payee_address: None,
            payee_phone: None,
        };
        let (event, bill) =
            NewBillWithEvent::try_from(NewBillBarcodeFormWithUserId { form, user_id: -1 }).unwrap();
        assert_eq!(bill.payee_account_number, "FI6380001270163414");
        assert_eq!(bill.amount, 5850);
        assert_eq!(bill.reference, "800181102211349629");
        assert_eq!(
            event.starts_at,
            NaiveDate::from_ymd_opt(2024, 6, 3)
                .unwrap()
                .and_time(NaiveTime::MIN)
                .and_utc()
                .timestamp()
        );
        assert_eq!(event.duration, None); // not paid yet
    }
}
