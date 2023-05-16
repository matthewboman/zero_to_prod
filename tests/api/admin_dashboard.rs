use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard_html().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;
    let login_body = serde_json::json!({
        "current_password":   &wrong_password,
        "new_password":       &new_password,
        "new_password_check": &new_password
    });
    let response = app.post(&login_body).await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_change_password_html().await;
    assert!(html.contains(&format!("Welcome {}", app.test_user.username)));

    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html.contains(&format!("You have successfully logged out")));

    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to!(&response, "/login");

}