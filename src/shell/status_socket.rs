use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};

use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_timers::future::TimeoutFuture;
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;

use crate::features::{
    connection::{mark, probe, ws_endpoint, ConnectionState, HealthSample, HEALTH},
    live::{refresh_active, ActiveRun, ACTIVE_RUN},
};

// The page sends a ping this often, the answer is the API latency in the pill
const PING_EVERY_MS: u32 = 10_000;
// How often the ping task looks at whether the socket is still open
const PING_TICK_MS: u32 = 500;

// Frames of GET /api/v1/status/ws
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Frame {
    // The run that executes now, null when nothing runs
    Active { data: Option<ActiveRun> },
    Health { data: HealthData },
    Pong { id: u64 },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct HealthData {
    database: bool,
    version: String,
}

fn parse_frame(text: &str) -> Option<Frame> {
    serde_json::from_str(text).ok()
}

// What the API pill needs, put together from two kinds of frame
#[derive(Debug, Default)]
struct Gauge {
    latency_ms: Option<u64>,
    health: Option<HealthData>,
}

impl Gauge {
    // The sample is complete once the latency and the health are both known
    fn sample(&self) -> Option<HealthSample> {
        let latency_ms = self.latency_ms?;
        let health = self.health.as_ref()?;

        Some(HealthSample {
            latency_ms,
            // Like the HTTP probe, no version while the database is unreachable
            version: health.database.then(|| health.version.clone()),
            database_ok: health.database,
        })
    }
}

fn round_trip_ms(sent_at: f64, now: f64) -> u64 {
    (now - sent_at).max(0.0).round() as u64
}

// Wait before the next attempt to open the socket: 1, 2, 4, 8, then 16 seconds
fn reconnect_ms(attempt: u32) -> u32 {
    1_000 * 2u32.pow(attempt.min(4))
}

// Keeps the active run chip and the API pill current on every page with one WebSocket.
// While the socket is down, one plain request each asks the API directly, so the pill
// still tells an unreachable API from a blocked WebSocket, and the socket is retried.
pub async fn keep_status_current() {
    let mut attempt = 0u32;

    loop {
        let Ok(url) = ws_endpoint("/status/ws") else {
            // No valid API address, the pages show that error themselves
            return;
        };

        if follow(&url).await {
            attempt = 0;
        } else {
            attempt = attempt.saturating_add(1);
        }

        if HEALTH.peek().is_some() {
            *HEALTH.write() = None;
        }

        probe().await;
        refresh_active().await;

        TimeoutFuture::new(reconnect_ms(attempt.saturating_sub(1))).await;
    }
}

// One connection until it ends. True when the API answered at least once.
async fn follow(url: &str) -> bool {
    let Ok(socket) = WebSocket::open(url) else {
        return false;
    };

    let (mut sink, mut stream) = socket.split();
    let open = Rc::new(Cell::new(true));
    let pending = Rc::new(Cell::new(None::<(u64, f64)>));

    // Pings from the start, then every PING_EVERY_MS, until the socket is gone
    {
        let open = Rc::clone(&open);
        let pending = Rc::clone(&pending);

        spawn_local(async move {
            let mut id = 0u64;
            let mut waited = PING_EVERY_MS;

            while open.get() {
                if waited >= PING_EVERY_MS {
                    id += 1;
                    pending.set(Some((id, js_sys::Date::now())));

                    let frame = format!(r#"{{"type":"ping","id":{id}}}"#);

                    if sink.send(Message::Text(frame)).await.is_err() {
                        break;
                    }

                    waited = 0;
                }

                TimeoutFuture::new(PING_TICK_MS).await;
                waited += PING_TICK_MS;
            }
        });
    }

    let mut gauge = Gauge::default();
    let mut answered = false;

    while let Some(message) = stream.next().await {
        let text = match message {
            Ok(Message::Text(text)) => text,
            Ok(Message::Bytes(_)) => continue,
            Err(_) => break,
        };

        let Some(frame) = parse_frame(&text) else {
            continue;
        };

        answered = true;
        mark(ConnectionState::Online);

        match frame {
            Frame::Active { data } => {
                if *ACTIVE_RUN.peek() != data {
                    *ACTIVE_RUN.write() = data;
                }
            }
            Frame::Health { data } => {
                gauge.health = Some(data);
                publish(&gauge);
            }
            Frame::Pong { id } => {
                if let Some((sent_id, sent_at)) = pending.get() {
                    if sent_id == id {
                        gauge.latency_ms = Some(round_trip_ms(sent_at, js_sys::Date::now()));
                        pending.set(None);
                        publish(&gauge);
                    }
                }
            }
        }
    }

    open.set(false);

    answered
}

fn publish(gauge: &Gauge) {
    if let Some(sample) = gauge.sample() {
        if HEALTH.peek().as_ref() != Some(&sample) {
            *HEALTH.write() = Some(sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_three_kinds_of_frame() {
        let active = parse_frame(r#"{"type":"active","data":null}"#).expect("parses");
        assert!(matches!(active, Frame::Active { data: None }));

        let running = parse_frame(
            r#"{"type":"active","data":{"run_id":"a","stage":"workload","elapsed_ms":5,
                "workload_progress":null}}"#,
        )
        .expect("parses");
        assert!(matches!(running, Frame::Active { data: Some(_) }));

        let health = parse_frame(
            r#"{"type":"health","data":{"ok":true,"database":true,"version":"0.1.0"}}"#,
        )
        .expect("parses");
        assert!(matches!(health, Frame::Health { .. }));

        let pong = parse_frame(r#"{"type":"pong","id":3}"#).expect("parses");
        assert!(matches!(pong, Frame::Pong { id: 3 }));
    }

    #[test]
    fn skips_frames_it_does_not_know() {
        assert!(parse_frame(r#"{"type":"future","data":1}"#).is_none());
        assert!(parse_frame("not json").is_none());
    }

    #[test]
    fn the_sample_waits_for_both_the_latency_and_the_health() {
        let mut gauge = Gauge::default();
        assert!(gauge.sample().is_none());

        gauge.latency_ms = Some(14);
        assert!(gauge.sample().is_none());

        gauge.health = Some(HealthData {
            database: true,
            version: "0.1.0".to_string(),
        });

        assert_eq!(
            gauge.sample(),
            Some(HealthSample {
                latency_ms: 14,
                version: Some("0.1.0".to_string()),
                database_ok: true,
            })
        );
    }

    #[test]
    fn an_unreachable_database_has_no_version() {
        let gauge = Gauge {
            latency_ms: Some(9),
            health: Some(HealthData {
                database: false,
                version: "0.1.0".to_string(),
            }),
        };

        let sample = gauge.sample().expect("complete");

        assert!(!sample.database_ok);
        assert_eq!(sample.version, None);
    }

    #[test]
    fn the_round_trip_is_never_negative() {
        assert_eq!(round_trip_ms(1_000.0, 1_012.4), 12);
        assert_eq!(round_trip_ms(1_000.0, 990.0), 0);
    }

    #[test]
    fn reconnecting_waits_longer_up_to_sixteen_seconds() {
        assert_eq!(reconnect_ms(0), 1_000);
        assert_eq!(reconnect_ms(1), 2_000);
        assert_eq!(reconnect_ms(4), 16_000);
        assert_eq!(reconnect_ms(30), 16_000);
    }
}
