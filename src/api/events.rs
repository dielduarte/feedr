use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::{Stream, StreamExt};

use super::AppState;
use crate::poller::PollerEvent;

/// Live poller activity for the UI. A client that falls too far behind gets a `resync` event
/// instead of the missed ones, telling it to reload what it shows.
pub async fn stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.poller.subscribe()).map(|received| {
        let event = match received {
            Ok(event) => Event::default().json_data(event),
            Err(BroadcastStreamRecvError::Lagged(_)) => {
                Event::default().json_data(PollerEvent::Resync)
            }
        };
        Ok(event.expect("poller events always serialize"))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
