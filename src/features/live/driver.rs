use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message, WebSocketError};
use gloo_timers::future::TimeoutFuture;

use super::{
    api::{fetch_replay, fetch_stored_run},
    models::{parse_frame, Frame},
    state::{Applied, LiveState},
};
use crate::features::connection::ws_endpoint;

// Connections in a row that never opened before the page falls back to polling
const MAX_FAILED_CONNECTIONS: u32 = 3;

// How often the JSON replay is polled when no WebSocket works
const POLL_INTERVAL_MS: u32 = 2_000;

// Wait before asking again when a run already has all the viewers it allows
const TOO_MANY_VIEWERS_WAIT_MS: u32 = 5_000;

// Close codes of the live-run WebSocket, see docs/live-runs-api.md in benchmark-api
const CLOSE_NORMAL: u16 = 1000;
const CLOSE_UNKNOWN_RUN: u16 = 4404;
const CLOSE_TOO_MANY_SOCKETS: u16 = 4429;

// How the page gets its events right now
#[derive(Debug, Clone, PartialEq)]
pub enum FeedState {
    Connecting,
    Live,
    // The stream was lost and a new connection is being tried
    Reconnecting { attempt: u32, reason: String },
    // No WebSocket works, the replay is polled instead
    Polling,
    TooManyViewers,
    // The API cannot be reached
    Offline(String),
    // The run reached its end
    Finished,
    // Neither live nor stored, the run is gone
    Lost,
}

// How one WebSocket connection ended
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEnd {
    // The run reached its end
    Terminal,
    UnknownRun,
    TooManyViewers,
    // It delivered its opening frame and then went away or missed events
    Dropped,
    // It never delivered the opening frame
    NeverOpened(String),
}

// Milliseconds to wait before the next connection, doubling up to eight seconds
pub fn backoff_ms(attempt: u32) -> u32 {
    let steps = attempt.saturating_sub(1).min(4);

    500 * 2u32.pow(steps)
}

// What a close code means for a connection that may or may not have opened
pub fn classify_close(code: u16, opened: bool, terminal: bool) -> StreamEnd {
    match code {
        CLOSE_NORMAL if terminal => StreamEnd::Terminal,
        CLOSE_UNKNOWN_RUN => StreamEnd::UnknownRun,
        CLOSE_TOO_MANY_SOCKETS => StreamEnd::TooManyViewers,
        _ if opened => StreamEnd::Dropped,
        _ => StreamEnd::NeverOpened(format!("closed with code {code} before it opened")),
    }
}

// Follows one run until it ends: WebSocket first, reconnecting after the last event it
// saw, then the polled replay when no WebSocket works, then the stored result when the
// live record has expired. The task is dropped with the page, which closes the socket.
pub async fn follow(
    run_id: String,
    state: Signal<LiveState>,
    mut feed: Signal<FeedState>,
    clock: Signal<u64>,
) {
    let mut failed_connections = 0u32;
    let mut attempt = 0u32;

    loop {
        if state.peek().is_terminal() {
            feed.set(FeedState::Finished);

            return;
        }

        let after = state.peek().last_seq();

        let reason = match stream_once(&run_id, after, state, feed, clock).await {
            StreamEnd::Terminal => {
                feed.set(FeedState::Finished);

                return;
            }
            StreamEnd::UnknownRun => {
                resolve_missing(&run_id, state, feed).await;

                return;
            }
            StreamEnd::TooManyViewers => {
                feed.set(FeedState::TooManyViewers);
                failed_connections = 0;

                TimeoutFuture::new(TOO_MANY_VIEWERS_WAIT_MS).await;

                continue;
            }
            StreamEnd::Dropped => {
                failed_connections = 0;
                attempt = 0;

                "the connection was lost".to_string()
            }
            StreamEnd::NeverOpened(why) => {
                failed_connections += 1;

                why
            }
        };

        if failed_connections >= MAX_FAILED_CONNECTIONS {
            poll_until_done(&run_id, state, feed, clock).await;

            return;
        }

        attempt += 1;
        feed.set(FeedState::Reconnecting { attempt, reason });

        TimeoutFuture::new(backoff_ms(attempt)).await;
    }
}

// One WebSocket connection, applying every frame to the state
async fn stream_once(
    run_id: &str,
    after: u64,
    mut state: Signal<LiveState>,
    mut feed: Signal<FeedState>,
    clock: Signal<u64>,
) -> StreamEnd {
    let url = match ws_endpoint(&format!("/live-runs/{run_id}/ws?after={after}")) {
        Ok(url) => url,
        Err(error) => return StreamEnd::NeverOpened(error.description()),
    };

    let socket = match WebSocket::open(&url) {
        Ok(socket) => socket,
        Err(error) => return StreamEnd::NeverOpened(error.to_string()),
    };

    let mut opened = false;

    // Pinned on the stack, so the stream needs no Unpin. Dropping it closes the socket.
    let mut socket = std::pin::pin!(socket);

    while let Some(message) = socket.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let Ok(frame) = parse_frame(&text) else {
                    continue;
                };

                match frame {
                    Frame::Opened(frame) => {
                        if !state.write().apply_opened(&frame) {
                            return StreamEnd::NeverOpened(
                                "the stream belongs to another run".to_string(),
                            );
                        }

                        opened = true;
                        feed.set(FeedState::Live);
                    }
                    Frame::SamplesDropped { through_seq } => {
                        state.write().apply_dropped(through_seq);
                    }
                    Frame::Event(event) => {
                        let now = *clock.peek();

                        if state.write().apply_event(event, now) == Applied::Gap {
                            // Events are missing, reconnect and ask for them
                            return if opened {
                                StreamEnd::Dropped
                            } else {
                                StreamEnd::NeverOpened("events were missing".to_string())
                            };
                        }

                        if state.peek().is_terminal() {
                            return StreamEnd::Terminal;
                        }
                    }
                }
            }
            Ok(Message::Bytes(_)) => {}
            Err(WebSocketError::ConnectionClose(close)) => {
                let terminal = state.peek().is_terminal();

                return classify_close(close.code, opened, terminal);
            }
            Err(error) => {
                return if opened {
                    StreamEnd::Dropped
                } else {
                    StreamEnd::NeverOpened(error.to_string())
                };
            }
        }
    }

    // The stream ended without a close event
    if state.peek().is_terminal() {
        StreamEnd::Terminal
    } else if opened {
        StreamEnd::Dropped
    } else {
        StreamEnd::NeverOpened("the connection ended before it opened".to_string())
    }
}

// The same events over plain HTTP, for a network where no WebSocket gets through
async fn poll_until_done(
    run_id: &str,
    mut state: Signal<LiveState>,
    mut feed: Signal<FeedState>,
    clock: Signal<u64>,
) {
    loop {
        let after = state.peek().last_seq();

        match fetch_replay(run_id, after).await {
            Ok(replay) => {
                let now = *clock.peek();

                state.write().apply_replay(replay, now);

                if state.peek().is_terminal() {
                    feed.set(FeedState::Finished);

                    return;
                }

                feed.set(FeedState::Polling);
            }
            Err(error) if error.is_not_found() => {
                resolve_missing(run_id, state, feed).await;

                return;
            }
            Err(error) => feed.set(FeedState::Offline(error.description())),
        }

        TimeoutFuture::new(POLL_INTERVAL_MS).await;
    }
}

// The live record is gone: a run that finished was stored, so the stored result answers
async fn resolve_missing(run_id: &str, mut state: Signal<LiveState>, mut feed: Signal<FeedState>) {
    match fetch_stored_run(run_id).await {
        Ok(result) => {
            state.write().finish_from_stored(result);
            feed.set(FeedState::Finished);
        }
        Err(error) if error.is_not_found() => feed.set(FeedState::Lost),
        Err(error) => feed.set(FeedState::Offline(error.description())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_stops_at_eight_seconds() {
        let waits: Vec<u32> = (1..=7).map(backoff_ms).collect();

        assert_eq!(waits, vec![500, 1000, 2000, 4000, 8000, 8000, 8000]);
    }

    #[test]
    fn backoff_for_attempt_zero_is_the_shortest() {
        assert_eq!(backoff_ms(0), 500);
    }

    #[test]
    fn a_normal_close_after_the_terminal_event_ends_the_run() {
        assert_eq!(classify_close(1000, true, true), StreamEnd::Terminal);
    }

    #[test]
    fn a_normal_close_before_the_terminal_event_is_a_dropped_connection() {
        assert_eq!(classify_close(1000, true, false), StreamEnd::Dropped);
    }

    #[test]
    fn the_unknown_run_code_asks_for_the_stored_result() {
        assert_eq!(classify_close(4404, false, false), StreamEnd::UnknownRun);
        assert_eq!(classify_close(4404, true, false), StreamEnd::UnknownRun);
    }

    #[test]
    fn the_full_run_code_asks_to_wait() {
        assert_eq!(
            classify_close(4429, false, false),
            StreamEnd::TooManyViewers
        );
    }

    #[test]
    fn other_codes_depend_on_whether_the_stream_opened() {
        assert_eq!(classify_close(1001, true, false), StreamEnd::Dropped);
        assert_eq!(classify_close(1006, true, false), StreamEnd::Dropped);
        assert!(matches!(
            classify_close(1006, false, false),
            StreamEnd::NeverOpened(_)
        ));
    }
}
