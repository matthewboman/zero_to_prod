use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};
use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;

    create_unconfirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

// #[tokio::test]
// async fn newsletters_returns_400_for_invalid_data() {
//     let app        = spawn_app().await;
//     let test_cases = vec![
//         (
//             serde_json::json!({
//                 "content": {
//                     "text": "Newsletter body as plain text",
//                     "html": "<p>Newsletter body as HTML</p>",
//                 }
//             }),
//             "missing title",
//         ),
//         (
//             serde_json::json!({
//                 "title": "Newsletter!"
//             }),
//             "missing content",
//         ),
//     ];

//     for (invalid_body, error_message) in test_cases {
//         let response = app.post_newsletters(invalid_body).await;

//         assert_eq!(
//             400, 
//             response.status().as_u16(),
//             "The API did not fail with 400 bad request when the payload was {}",
//             error_message
//         );
//     }
// }

// #[tokio::test]
// async fn request_missing_authorization_are_rejected() {
//     let app = spawn_app().await;

//     let newsletter_request_body = serde_json::json!({
//         "title": "Newsletter title",
//         "content": {
//             "text": "newsletter body",
//             "html": "<p>newsletter body</p>"
//         }
//     });

//     let response = reqwest::Client::new()
//         .post(&format!("{}/admin/newsletters", &app.address))
//         .json(&newsletter_request_body)
//         .send()
//         .await
//         .expect("Failed to execute request");

//     assert_eq!(401, response.status().as_u16());
//     assert_eq!(r#"Basic realm="publish""#, response.headers()["WWW-Authenticate"]);
// }

// #[tokio::test]
// async fn non_existing_user_is_rejected() {
//     let app = spawn_app().await;
    
//     let username = Uuid::new_v4().to_string();
//     let password = Uuid::new_v4().to_string();
//     let newsletter_request_body = serde_json::json!({
//         "title": "Newsletter title",
//         "content": {
//             "text": "newsletter body",
//             "html": "<p>newsletter body</p>"
//         }
//     });

//     let response = reqwest::Client::new()
//         .post(&format!("{}/admin/newsletters", &app.address))
//         .basic_auth(username, Some(password))
//         .json(&newsletter_request_body)
//         .send()
//         .await
//         .expect("Failed to execute request");

//     assert_eq!(401, response.status().as_u16());
//     assert_eq!(r#"Basic realm="publish""#, response.headers()["WWW-Authenticate"]);
// }

// #[tokio::test]
// async fn invalid_password_is_rejected() {
//     let app = spawn_app().await;
    
//     let username = &app.test_user.username;
//     let password = Uuid::new_v4().to_string();

//     assert_ne!(app.test_user.password, password);

//     let newsletter_request_body = serde_json::json!({
//         "title": "Newsletter title",
//         "content": {
//             "text": "newsletter body",
//             "html": "<p>newsletter body</p>"
//         }
//     });

//     let response = reqwest::Client::new()
//         .post(&format!("{}/admin/newsletters", &app.address))
//         .basic_auth(username, Some(password))
//         .json(&newsletter_request_body)
//         .send()
//         .await
//         .expect("Failed to execute request");

//     assert_eq!(401, response.status().as_u16());
//     assert_eq!(r#"Basic realm="publish""#, response.headers()["WWW-Authenticate"]);
// }

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await.html;
    reqwest::get(confirmation_link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}