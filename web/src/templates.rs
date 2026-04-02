//! Askama template types for HTML responses.

use askama::Template;

/// Cinema shell page: seat map shell; Datastar is loaded for future reactive UI.
#[derive(Template)]
#[template(path = "cinema.html")]
pub struct CinemaIndex {
    /// Short label shown in the header (for example a generated user id).
    pub user_label: String,
}

/// Trailbase password login form (`GET /login`).
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginPage {
    pub error_message: Option<&'static str>,
    pub success_message: Option<&'static str>,
}

/// Trailbase email/password registration form (`GET /signup`).
#[derive(Template)]
#[template(path = "signup.html")]
pub struct SignupPage {
    pub error_message: Option<&'static str>,
}
