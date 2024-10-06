use reqwest::header::{HeaderMap, HeaderValue};

use olmonoko_common::models::{event::EventOccurrenceHuman, user::UserPublic};

pub async fn get_user_details(
    instance_url: &str,
    api_key: &str,
) -> Result<Option<UserPublic>, reqwest::Error> {
    let path = format!("{instance_url}/api/user/me");
    //println!("calling {path}");

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-OLMONOKO-API-KEY",
        HeaderValue::from_str(api_key).unwrap(),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let request = client.get(&path).build()?;
    let response = client.execute(request).await?;

    if !response.status().is_success() {
        println!(
            "fetching user details was not a success: {}",
            response.status()
        );
        return Ok(None);
    }

    let details: UserPublic = response.json().await?;

    Ok(Some(details))
}

pub async fn get_upcoming_events(
    instance_url: &str,
    api_key: &str,
) -> Result<Option<Vec<EventOccurrenceHuman>>, reqwest::Error> {
    let path = format!("{instance_url}/api/event/occurrences/planning_to_attend");
    //println!("calling {path}");

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-OLMONOKO-API-KEY",
        HeaderValue::from_str(api_key).unwrap(),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let request = client.get(&path).build()?;
    let response = client.execute(request).await?;

    if !response.status().is_success() {
        println!(
            "fetching upcoming events was not a success: {}",
            response.status()
        );
        return Ok(None);
    }

    let details: Vec<EventOccurrenceHuman> = response.json().await?;

    Ok(Some(
        details
            .into_iter()
            .filter(|e| {
                e.starts_at_utc.timestamp() >= olmonoko_common::utils::time::timestamp()
                // TODO: Come up with a nice way to format ongoing events
                //|| e.starts_at_utc.timestamp() + e.duration.unwrap_or_default() as i64
                //    >= olmonoko_common::utils::time::timestamp()
            })
            .map(|e| {
                let mut new = e.clone();
                let ht = chrono_humanize::HumanTime::from(e.starts_at_utc);
                let relative = ht.to_text_en(
                    chrono_humanize::Accuracy::Rough,
                    chrono_humanize::Tense::Future,
                );
                new.starts_at_human = format!("{} ({relative})", new.starts_at_human);
                new
            })
            .collect(),
    ))
}
