//! Askama template types for HTML responses.

use askama::Template;

/// Cinema shell page: layout from the legacy `static.html` prototype plus a Datastar demo block.
#[derive(Template)]
#[template(path = "cinema.html")]
pub struct CinemaIndex {
    /// Short label shown in the header (for example a generated user id).
    pub user_label: String,
}
