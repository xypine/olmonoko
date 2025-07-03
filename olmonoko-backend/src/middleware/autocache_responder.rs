use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpResponse,
};

use olmonoko_common::get_site_url;

use crate::db::request::EnhancedRequest;

use super::autocacher::AUTOCACHE_DISALLOWED_PATHS;

pub async fn autocache_responder(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<EitherBody<impl MessageBody>>, Error> {
    let site_url = get_site_url();
    let recursion_prevention_header = req
        .headers()
        .get(super::autocacher::CACHE_RECURSION_PREVENTION_HEADER);
    let uri = req.uri().to_string();
    if recursion_prevention_header.is_none() && !AUTOCACHE_DISALLOWED_PATHS.contains(&uri.as_str())
    {
        let session_id = req.request().get_session_id();
        if let Some(session_id) = session_id {
            let link = format!("{}{}", site_url, uri);
            let cache_key = super::cache_key(&session_id, &link);
            let cache_hit = super::CACHE.get(&cache_key).await;
            if let Some((headers, body)) = cache_hit {
                tracing::debug!("Cache hit for key: {}", cache_key);
                let mut response = HttpResponse::Ok().body(body);
                let new_headers = response.headers_mut();
                for (name, value) in headers.iter() {
                    let name_str = name.to_string();
                    let value_str = value.to_str().unwrap();
                    let name_actix =
                        actix_web::http::header::HeaderName::from_bytes(name_str.as_bytes())
                            .unwrap();
                    let value_actix =
                        actix_web::http::header::HeaderValue::from_str(value_str).unwrap();
                    new_headers.append(name_actix, value_actix);
                }

                return Ok(ServiceResponse::new(req.into_parts().0, response).map_into_right_body());
            } else {
                tracing::debug!("Cache miss for key: {}", cache_key);
            }
        }
    }
    // pre-processing
    let res = next.call(req).await?;
    Ok(res.map_into_left_body())
    // post-processing
}
