use lettre::{
    message::{header, SinglePart},
    transport::smtp::{authentication::Credentials, response::Code},
    Message, SmtpTransport, Transport,
};
use std::fs;

use crate::packages::settings::SETTINGS;

#[derive(Default)]
pub struct SendEmail<'a> {
    pub to: Vec<&'a str>,
    pub cc: Vec<&'a str>,
    pub bcc: Vec<&'a str>,
    pub subject: &'a str,
    pub template_path: &'a str,
    pub placeholders: &'a [(String, String)],
}

pub async fn send_email(args: SendEmail<'_>) -> Result<Code, Box<dyn std::error::Error>> {
    let smtp_username = SETTINGS.read().smtp.username.to_owned();
    let smtp_password = SETTINGS.read().smtp.password.to_owned();
    let smtp_server = &SETTINGS.read().smtp.server;
    let smtp_port = SETTINGS.read().smtp.port;

    // Read and process template
    let mut html_template = fs::read_to_string(args.template_path)?;
    for (key, value) in args.placeholders {
        html_template = html_template.replace(key, value);
    }

    // Build the email
    let mut email_builder = Message::builder().from(smtp_username.parse()?).subject(args.subject);

    // Add TO recipients
    for to_addr in args.to {
        email_builder = email_builder.to(to_addr.parse()?);
    }

    // Add CC recipients if the vector is not empty
    if !args.cc.is_empty() {
        for cc_addr in args.cc {
            email_builder = email_builder.cc(cc_addr.parse()?);
        }
    }

    // Add BCC recipients if the vector is not empty
    if !args.bcc.is_empty() {
        for bcc_addr in args.bcc {
            email_builder = email_builder.bcc(bcc_addr.parse()?);
        }
    }

    // Build the email with HTML content
    let email = email_builder.singlepart(SinglePart::builder().header(header::ContentType::TEXT_HTML).body(html_template))?;

    // Setup SMTP transport
    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = SmtpTransport::starttls_relay(smtp_server)?.credentials(creds).port(smtp_port).build();

    // Send the email
    let result = mailer.send(&email);
    match result {
        Ok(response) => Ok(response.code()),
        Err(err) => Err(Box::new(err)),
    }
}
