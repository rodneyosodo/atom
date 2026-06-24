use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

use crate::{
    config::{Config, SmtpTls},
    error::AppError,
};

pub(crate) async fn send_invitation_email(
    cfg: &Config,
    email: &str,
    redirect_url: &str,
    token: &str,
) -> Result<(), AppError> {
    let invitation_url = url_with_params(redirect_url, &[("token", token)]);
    let Some(smtp) = cfg.smtp.as_ref() else {
        if cfg.dev_allow_unverified_email_login {
            tracing::warn!(
                email,
                invitation_url,
                "SMTP is not configured; skipping invitation email in development bypass mode"
            );
            return Ok(());
        }
        return Err(AppError::Internal(anyhow::anyhow!(
            "SMTP is not configured"
        )));
    };

    let message = Message::builder()
        .from(
            smtp.from
                .parse()
                .map_err(|e| AppError::bad_request(format!("invalid SMTP from address: {e}")))?,
        )
        .to(email
            .parse()
            .map_err(|e| AppError::bad_request(format!("invalid email address: {e}")))?)
        .subject("You have been invited")
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Accept your invitation by opening this link:\n\n{invitation_url}\n"
        ))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("build email: {e}")))?;

    let mut builder = match smtp.tls {
        SmtpTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host),
        SmtpTls::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("smtp starttls: {e}")))?,
        SmtpTls::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("smtp tls: {e}")))?,
    }
    .port(smtp.port);

    if let (Some(username), Some(password)) = (&smtp.username, &smtp.password) {
        builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
    }

    builder
        .build()
        .send(message)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("send invitation email: {e}")))?;
    Ok(())
}

fn url_with_params(base: &str, params: &[(&str, &str)]) -> String {
    match url::Url::parse(base) {
        Ok(mut parsed) => {
            {
                let mut pairs = parsed.query_pairs_mut();
                for (key, value) in params {
                    if !value.is_empty() {
                        pairs.append_pair(key, value);
                    }
                }
            }
            parsed.to_string()
        }
        Err(_) => {
            let mut url = base.to_string();
            let mut first = !url.contains('?');
            for (key, value) in params {
                if !value.is_empty() {
                    url.push(if first { '?' } else { '&' });
                    first = false;
                    url.push_str(key);
                    url.push('=');
                    url.push_str(value);
                }
            }
            url
        }
    }
}
