use std::convert::Infallible;

use axum::Router;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::{Stream, StreamExt};

use crate::db::Db;
use crate::poller::PollerHandle;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub poller: PollerHandle,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/events", get(events))
        .with_state(state)
}

/// Live poller activity for the UI. A client that falls too far behind gets a `resync` event
/// instead of the missed ones, telling it to reload what it shows.
async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.poller.subscribe()).map(|received| {
        let event = match received {
            Ok(event) => Event::default().json_data(event),
            Err(BroadcastStreamRecvError::Lagged(_)) => {
                Event::default().json_data(serde_json::json!({ "type": "resync" }))
            }
        };
        Ok(event.expect("poller events always serialize"))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
