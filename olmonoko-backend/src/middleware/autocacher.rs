pub const AUTOCACHE_DISALLOWED_PATHS: [&str; 2] = ["/static", "/api"];

use std::{
    future::{ready, Future, Ready},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    body::{BodySize, MessageBody},
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::{Bytes, BytesMut},
    Error,
};

use crate::{
    routes::get_site_url,
    utils::request::{EnhancedRequest, SESSION_COOKIE_NAME},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct AutoCacher;

impl<S: 'static, B> Transform<S, ServiceRequest> for AutoCacher
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BodyAutoCacher<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AutoCacherMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AutoCacherMiddleware { service }))
    }
}

pub struct AutoCacherMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AutoCacherMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Response = ServiceResponse<BodyAutoCacher<B>>;
    type Error = Error;
    type Future = WrapperStream<S, B>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_path = req.path().to_string();
        let enabled = req
            .headers()
            .get(CACHE_RECURSION_PREVENTION_HEADER)
            .is_none();
        let session_id = req.request().get_session_id();
        WrapperStream {
            fut: self.service.call(req),
            _t: PhantomData,
            enabled,
            request_path,
            session_id,
        }
    }
}

#[pin_project::pin_project]
pub struct WrapperStream<S, B>
where
    B: MessageBody,
    S: Service<ServiceRequest>,
{
    #[pin]
    fut: S::Future,
    _t: PhantomData<(B,)>,
    enabled: bool,
    request_path: String,
    session_id: Option<String>,
}

impl<S, B> Future for WrapperStream<S, B>
where
    B: MessageBody,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = Result<ServiceResponse<BodyAutoCacher<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let enabled = self.enabled;
        let request_path = self.request_path.clone();
        let session_id = self.session_id.clone();
        let res = futures_util::ready!(self.project().fut.poll(cx));

        Poll::Ready(res.map(|res| {
            res.map_body(move |_, body| BodyAutoCacher {
                body,
                body_accum: BytesMut::new(),
                request_path,
                enabled,
                session_id,
            })
        }))
    }
}

#[pin_project::pin_project(PinnedDrop)]
pub struct BodyAutoCacher<B> {
    #[pin]
    body: B,
    body_accum: BytesMut,
    enabled: bool,
    request_path: String,
    session_id: Option<String>,
}

pub const CACHE_RECURSION_PREVENTION_HEADER: &str = "X-OLMONOKO-Cache-Ray";

#[pin_project::pinned_drop]
impl<B> PinnedDrop for BodyAutoCacher<B> {
    fn drop(self: Pin<&mut Self>) {
        if let Some(session_id) = self.session_id.clone() {
            let enabled = self.enabled
                && !AUTOCACHE_DISALLOWED_PATHS
                    .iter()
                    .any(|path| self.request_path.starts_with(path));
            if enabled {
                let body = self.body_accum.clone();
                let body_str = String::from_utf8_lossy(&body).to_string();

                // scan body for cacheable content (links beginning with /)
                let mut cacheable_links = vec![];
                // cool regex to find all links in a string (not just hrefs)
                let site_url = get_site_url();
                // FIXME: This doesn't work for all cases
                let pattern = format!("{}.*\"$", regex::escape(&site_url));
                let re = regex::RegexBuilder::new(&pattern)
                    .multi_line(true)
                    .build()
                    .unwrap();

                for cap in re.captures_iter(&body_str) {
                    let link = cap.get(0);
                    if let Some(link) = link {
                        // remove trailing " from link
                        let link = &link.as_str()[..link.as_str().len() - 1];

                        let link_parsed = reqwest::Url::parse(link);
                        if let Ok(link_parsed) = link_parsed {
                            let link_path = link_parsed.path();
                            if AUTOCACHE_DISALLOWED_PATHS
                                .iter()
                                .any(|path| link_path.starts_with(path))
                            {
                                // skip disallowed paths
                                continue;
                            }

                            cacheable_links.push(link);
                        }
                    }
                }
                tracing::info!("Cacheable links: {:?}", cacheable_links);

                for link in cacheable_links {
                    let link = link.to_string();
                    let link_dbg = link.clone();
                    let session_id = session_id.clone();
                    let site_url = site_url.clone();

                    let future = async move {
                        let cookies = reqwest::cookie::Jar::default();
                        cookies.add_cookie_str(
                            &format!("{}={}", SESSION_COOKIE_NAME, session_id),
                            &reqwest::Url::parse(&site_url).unwrap(),
                        );
                        let client = reqwest::Client::builder()
                            .cookie_provider(std::sync::Arc::new(cookies))
                            .build()
                            .unwrap();
                        let response = client
                            .get(&link)
                            .header(CACHE_RECURSION_PREVENTION_HEADER, "true")
                            .send()
                            .await?;
                        if !response.status().is_success() {
                            tracing::warn!(
                                "Failed to cache response for {}: {}",
                                &link,
                                response.status()
                            );
                            return Ok::<(), reqwest::Error>(());
                        }
                        let headers = response.headers().clone();
                        let body = response.text().await?;
                        let key = super::cache_key(&session_id, &link);
                        let data = (headers, body);
                        super::CACHE.insert(key.clone(), data).await;
                        tracing::info!("Cached response for {}", key);
                        Ok::<(), reqwest::Error>(())
                    };

                    std::thread::spawn(move || {
                        let r = tokio::runtime::Runtime::new().unwrap().block_on(future);
                        if let Err(e) = r {
                            tracing::warn!("Failed to cache response for {}: {}", link_dbg, e);
                        }
                    });
                }

                tracing::info!("Started caching internal links for {}", self.request_path);
            }
        }
    }
}

impl<B: MessageBody> MessageBody for BodyAutoCacher<B> {
    type Error = B::Error;

    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Self::Error>>> {
        let this = self.project();

        match this.body.poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                this.body_accum.extend_from_slice(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
