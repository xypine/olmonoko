use olmonoko_common::AppState;
use resend_rs::types::CreateEmailBaseOptions;
pub use resend_rs::Error;
use resend_rs::{Resend, Result};

pub async fn send_verification_email(
    data: &AppState,
    email: &str,
    secret: &str,
) -> Result<(), Error> {
    let site_url = data.site_url.as_str();

    let from = "OLMONOKO <onboarding@olmonoko.ruta.fi>";
    let to = [email];
    let subject = "Welcome to OLMONOKO!";
    let link = format!("{}/api/user/verify/{}", site_url, secret);
    let text =
        format!("Welcome to OLMONOKO! Please verify your email by clicking this link: {link}");
    let html = format!(
        "<p>Welcome to OLMONOKO! Please verify your email by clicking this link: <a href='{link}'>Verify</a></p>"
    );

    if std::env::var("RESEND_API_KEY").is_err() {
        tracing::warn!("RESEND_API_KEY not set, email not sent");
        tracing::info!("Email to {} with content: {}", email, text);
        return Ok(());
    }

    let resend = Resend::default();
    let options = CreateEmailBaseOptions::new(from, to, subject)
        .with_text(&text)
        .with_html(&html);
    let _email = resend.emails.send(options).await?;
    Ok(())
}
