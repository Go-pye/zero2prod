use crate::domain::SubscriberEmail;
use reqwest::Client;
use serde::Serialize;
use secrecy::{Secret, ExposeSecret};

#[derive(Clone)]
pub struct EmailClient {
    http_client: reqwest::Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
    // api_key: String,
}


#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
  from: &'a str,
  to: &'a str,
  subject: &'a str,
  text_body: &'a str,
  html_body: &'a str,
}

impl EmailClient {
  pub fn new(base_url: String, sender: SubscriberEmail, authorization_token: Secret<String>) -> Self {
    let http_client = Client::builder()
      .timeout(std::time::Duration::from_secs(10))
      .build()
      .unwrap();
    Self {
      http_client,
      base_url,
      sender,
      authorization_token
    }
  }

  pub async fn send_email(
    &self,
    recipient: SubscriberEmail,
    subject: &str,
    text_content: &str,
    html_content: &str,
  ) -> Result<(), reqwest::Error> {
    let request_body = SendEmailRequest {
      from: self.sender.as_ref(),
      to: recipient.as_ref(),
      subject,
      text_body: text_content,
      html_body: html_content,
    };

    let url = format!("{}/emails", self.base_url);
    let builder = self
      .http_client
      .post(&url)
      .header(
        "X-Postmark-Server-Token",
        self.authorization_token.expose_secret()
      )
      .json(&request_body)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::domain::SubscriberEmail;
  use crate::email_client::EmailClient;
  use fake::faker::internet::en::SafeEmail;
  use fake::faker::lorem::en::{Paragraph, Sentence};
  use fake::{Fake, Faker};
  use wiremock::{Mock, MockServer, ResponseTemplate};
  use wiremock::matchers::{header, header_exists, path, method};
  use wiremock::matchers::any;
  use wiremock::Request;
  use claims::{assert_ok, assert_err};
  use super::*;

  struct SendEmailBodyMatcher;

  fn subject() -> String {
    Sentence(1..2).fake()
  }

  fn content() -> String {
    Paragraph(1..2).fake()
  }
  fn email() -> SubscriberEmail {
    SubscriberEmail::parse(SafeEmail().fake()).unwrap()
  }

  fn email_client(base_url: String) -> EmailClient {
    EmailClient::new(base_url, email(), Secret::new(Faker.fake()))
  }

  impl wiremock::Match for SendEmailBodyMatcher {
    fn matches(&self, request: &Request) -> bool {
      let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
      if let Ok(body) = result {
        dbg!(&body);
        body.get("From").is_some()
        && body.get("To").is_some()
        && body.get("Subject").is_some()
        && body.get("TextBody").is_some()
        && body.get("HtmlBody").is_some()
      } else {
        false
      }
    }
  }

  #[tokio::test]
  async fn send_email_sends_the_expected_request() {
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());

    Mock::given(header_exists("X-Postmark-Server-Token"))
    .and(header("Content-Type", "application/json"))
    .and(path("/emails"))
    .and(method("POST"))
    .and(SendEmailBodyMatcher)
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&mock_server)
    .await;

    let _ = email_client.send_email(email(), &subject(), &content(), &content()).await;
  }

  #[tokio::test]
  async fn send_email_succeeds_if_the_server_returns_200() {
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());

    Mock::given(any())
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&mock_server)
      .await;

    let outcome = email_client.send_email(email(), &subject(), &content(), &content()).await;

    assert_ok!(outcome);
  }

  #[tokio::test]
  async fn send_email_fails_if_the_server_returns_500() {
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());


    Mock::given(any()).respond_with(ResponseTemplate::new(500))
      .expect(1)
      .mount(&mock_server)
      .await;

    let outcome = email_client.send_email(email(), &subject(), &content(), &content()).await;

    assert_err!(outcome);
  }

}