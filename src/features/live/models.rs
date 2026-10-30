use serde::Deserialize;
use serde_json::Value;

// Wire types of the live-run API, see docs/live-runs-api.md in benchmark-api

// `data` of the 202 of POST /api/v1/live-runs
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveRunAccepted {
    pub run_id: String,
}

// One ordered event of a run, `data` depends on `kind`
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StreamEvent {
    pub run_id: String,
    pub seq: u64,
    pub t_ms: u64,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub data: Value,
}

// First frame of a WebSocket
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Opened {
    pub run_id: String,
    pub dropped_through_seq: u64,
}

// `data` of GET /api/v1/live-runs/{run_id}, the same events as the WebSocket
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReplayData {
    pub dropped_through_seq: u64,
    pub events: Vec<StreamEvent>,
}

// Everything a WebSocket can deliver as text
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Opened(Opened),
    // Samples up to this seq were evicted before they could be read
    SamplesDropped { through_seq: u64 },
    Event(StreamEvent),
}

pub fn parse_frame(text: &str) -> Result<Frame, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;

    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string);

    match kind.as_deref() {
        Some("stream.opened") => serde_json::from_value(value)
            .map(Frame::Opened)
            .map_err(|error| error.to_string()),
        Some("samples.dropped") => value
            .get("through_seq")
            .and_then(Value::as_u64)
            .map(|through_seq| Frame::SamplesDropped { through_seq })
            .ok_or_else(|| "samples.dropped without through_seq".to_string()),
        Some(_) => serde_json::from_value(value)
            .map(Frame::Event)
            .map_err(|error| error.to_string()),
        None => Err("a frame without a type".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_opening_frame() {
        let frame = parse_frame(
            r#"{"type":"stream.opened","run_id":"r1","status":"running","last_seq":4,"truncated":false,"dropped_through_seq":0}"#,
        )
        .unwrap();

        assert_eq!(
            frame,
            Frame::Opened(Opened {
                run_id: "r1".to_string(),
                dropped_through_seq: 0
            })
        );
    }

    #[test]
    fn parses_the_samples_dropped_frame() {
        let frame = parse_frame(r#"{"type":"samples.dropped","through_seq":40}"#).unwrap();

        assert_eq!(frame, Frame::SamplesDropped { through_seq: 40 });
    }

    #[test]
    fn parses_an_event_and_ignores_fields_it_does_not_use() {
        let frame = parse_frame(
            r#"{"v":1,"run_id":"r1","seq":7,"t_ms":120,"at":"2026-10-12T08:00:00.000Z","type":"stage.started","data":{"stage":"build"}}"#,
        )
        .unwrap();

        match frame {
            Frame::Event(event) => {
                assert_eq!(event.seq, 7);
                assert_eq!(event.t_ms, 120);
                assert_eq!(event.kind, "stage.started");
                assert_eq!(event.data["stage"], "build");
            }
            other => panic!("expected an event, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_event_type_is_still_an_event() {
        let frame =
            parse_frame(r#"{"run_id":"r1","seq":9,"t_ms":1,"type":"something.new","data":{}}"#)
                .unwrap();

        assert!(matches!(frame, Frame::Event(_)));
    }

    #[test]
    fn rejects_frames_that_are_not_json_or_have_no_type() {
        assert!(parse_frame("not json").is_err());
        assert!(parse_frame(r#"{"seq":1}"#).is_err());
        assert!(parse_frame(r#"{"type":"samples.dropped"}"#).is_err());
        assert!(parse_frame(r#"{"type":"stream.opened"}"#).is_err());
    }

    #[test]
    fn parses_the_replay_data() {
        let replay: ReplayData = serde_json::from_str(
            r#"{"run_id":"r1","status":"running","last_seq":2,"dropped_through_seq":0,"truncated":false,
                "events":[{"v":1,"run_id":"r1","seq":1,"t_ms":0,"at":"x","type":"run.accepted","data":{}}]}"#,
        )
        .unwrap();

        assert_eq!(replay.dropped_through_seq, 0);
        assert_eq!(replay.events.len(), 1);
    }
}
