use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::Rng;
use std::{env, fs};

#[derive(Debug, Clone)]
pub struct Email {
    pub recipient: String,
    pub subject: String,
    pub message: String,
}

fn send_email_blocking(email: Email) {
    let from = env::var("SMTP_USER").unwrap_or_default();

    let message_encrypted_payload = email.message;

    let mut rng = rand::rng();
    let mut str = String::new();
    for _ in 0..10 {
        let num = rng.random_range(0..10);
        str.push_str(&num.to_string());
    }

    let payload_to_encrypt_path = format!("payload_to_encrypt_{}.txt", str);
    let payload_encrypted_path = format!("encrypted_{}.asc", str);
    fs::write(&payload_to_encrypt_path, &message_encrypted_payload).expect("Unable to write file");

    if env::var("UNENCRYPTED_EMAILS")
        .map(|x| x == "true")
        .unwrap_or(false)
    {
        log::warn!("Skipping mail encryption");
        fs::copy(&payload_to_encrypt_path, &payload_encrypted_path).expect("Unable to copy file");
    } else {
        //encrypt payload
        std::process::Command::new("gpg")
            .args([
                "--armor",
                "--output",
                &payload_encrypted_path,
                "--encrypt",
                "--recipient",
                &email.recipient,
                "--trust-model",
                "always",
                &payload_to_encrypt_path,
            ])
            .output()
            .expect("failed to execute process");
    }

    let encrypted_data = fs::read_to_string(&payload_encrypted_path).expect("Unable to read file");

    fs::remove_file(&payload_to_encrypt_path).expect("Unable to remove file");
    fs::remove_file(&payload_encrypted_path).expect("Unable to remove file");

    log::info!("Sending email from: {}", from);
    let email = Message::builder()
        .from(from.parse().unwrap()) // Replace with your email
        .to(email.recipient.parse().unwrap()) // Replace with recipient email
        .subject(email.subject)
        .body(encrypted_data)
        .unwrap();

    let mailer = SmtpTransport::starttls_relay(&env::var("SMTP_HOST").unwrap_or_default())
        .unwrap()
        .credentials(Credentials::new(
            env::var("SMTP_USER").unwrap_or_default(),
            env::var("SMTP_PASSWORD").unwrap_or_default(),
        ))
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => eprintln!("Could not send email: {:?}", e),
    }
}
pub async fn send_email(email: Email) {
    // Create thread
    let join_thread = std::thread::spawn(|| {
        // Send email
        send_email_blocking(email);
    });

    // Wait for thread to finish
    loop {
        if join_thread.is_finished() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    match join_thread.join() {
        Ok(_) => log::info!("Email sent successfully!"),
        Err(e) => log::error!("Could not send email: {:?}", e),
    }
}
