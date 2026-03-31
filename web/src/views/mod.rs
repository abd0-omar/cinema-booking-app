//! Server-rendered HTML and Datastar (SSE) handlers.

use crate::error::Error;
use crate::templates::CinemaIndex;
use askama::Template;
use async_stream::stream;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use datastar::{axum::ReadSignals, prelude::PatchElements};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

const HELLO_MESSAGE: &str = "Hello, world!";

/// `GET /` — Askama-rendered cinema shell.
pub async fn cinema_index() -> Result<Html<String>, Error> {
    let id = uuid::Uuid::new_v4();
    let compact = id.simple().to_string();
    let short = compact.chars().take(12).collect::<String>();
    let page = CinemaIndex {
        user_label: format!("user: {short}"),
    };
    Ok(Html(page.render()?))
}

#[derive(Debug, Deserialize)]
pub struct HelloSignals {
    pub delay: u64,
}

/// Datastar action target: streams [`PatchElements`] that progressively update `#message`.
pub async fn ds_hello_world(ReadSignals(signals): ReadSignals<HelloSignals>) -> impl IntoResponse {
    let delay_ms = signals.delay.max(1);
    let stream = stream! {
        for i in 0..HELLO_MESSAGE.len() {
            let slice = &HELLO_MESSAGE[..=i];
            let elements = format!("<div id=\"message\">{slice}</div>");
            let patch = PatchElements::new(elements);
            yield Ok::<Event, Infallible>(patch.into());
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
