//! Runner Dock の IPC ワイヤー契約。
//!
//! この crate が契約の正典で、JSON Schema と TypeScript 型はここから生成する
//! （`docs/04_TECHNOLOGY_STACK.md` §4）。OS 操作、HTTP、Tauri には依存しない。
//! 秘密本体を通常 DTO として追加しない（`docs/03_ARCHITECTURE.md` §6）。

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod dto;
pub mod error;
pub mod frame;
pub mod ids;
pub mod message;
pub mod method;

/// 現在の protocol major。不一致のメッセージは復号段階で拒否する。
pub const PROTOCOL_MAJOR: u32 = 1;

/// SQLite schema の版。migration の対象。
pub const DB_SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{DecodeOutcome, FrameError, MAX_FRAME_BYTES, decode, encode};
    use crate::ids::{DecimalU64, RequestId};
    use crate::message::{Event, EventKind, Message, Request};
    use crate::method::Method;
    use serde_json::json;

    fn frame_of(payload: &[u8]) -> Vec<u8> {
        let mut framed = u32::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    #[test]
    fn decode_accepts_a_well_formed_request_frame() {
        let request = Message::Request(Request::new(
            RequestId::from("c917d45a"),
            Method::NodeStart,
            json!({"nodeId": "node-home", "expectedRevision": "3"}),
        ));
        let bytes = encode(&request).unwrap();

        let outcome = decode(&bytes).unwrap();

        match outcome {
            DecodeOutcome::Decoded { message, consumed } => {
                assert_eq!(*message, request);
                assert_eq!(consumed, bytes.len());
            }
            DecodeOutcome::Incomplete => panic!("完全なフレームが Incomplete になった"),
        }
    }

    #[test]
    fn decode_reports_incomplete_when_the_prefix_is_short() {
        assert_eq!(decode(&[0x01, 0x02]).unwrap(), DecodeOutcome::Incomplete);
    }

    #[test]
    fn decode_reports_incomplete_when_the_payload_is_truncated() {
        // 切断された断片を完全なメッセージとして扱わない。
        let full = encode(&Message::Event(Event::new(
            DecimalU64::new(1842),
            "9a9a6fec".into(),
            EventKind::ResyncRequired,
            json!({}),
        )))
        .unwrap();
        let truncated = &full[..full.len() - 1];

        assert_eq!(decode(truncated).unwrap(), DecodeOutcome::Incomplete);
    }

    #[test]
    fn decode_rejects_a_zero_length_frame() {
        assert_eq!(decode(&frame_of(b"")).unwrap_err(), FrameError::ZeroLength);
    }

    #[test]
    fn decode_rejects_a_frame_declaring_more_than_the_limit() {
        let declared = MAX_FRAME_BYTES + 1;
        let mut bytes = u32::try_from(declared).unwrap().to_le_bytes().to_vec();
        bytes.extend_from_slice(b"{}");

        assert_eq!(
            decode(&bytes).unwrap_err(),
            FrameError::TooLarge { declared }
        );
    }

    #[test]
    fn decode_rejects_invalid_utf8() {
        assert_eq!(
            decode(&frame_of(&[0xff, 0xfe, 0xfd])).unwrap_err(),
            FrameError::InvalidUtf8
        );
    }

    #[test]
    fn decode_rejects_an_unknown_protocol_major() {
        let payload = json!({
            "protocolMajor": 2,
            "kind": "request",
            "requestId": "r-1",
            "method": "node.start",
            "payload": {}
        })
        .to_string();

        assert_eq!(
            decode(&frame_of(payload.as_bytes())).unwrap_err(),
            FrameError::UnknownProtocolMajor { found: 2 }
        );
    }

    #[test]
    fn decode_rejects_an_unknown_message_kind() {
        let payload = json!({"protocolMajor": 1, "kind": "shutdown"}).to_string();

        assert!(matches!(
            decode(&frame_of(payload.as_bytes())).unwrap_err(),
            FrameError::MalformedPayload { .. }
        ));
    }

    #[test]
    fn decode_rejects_a_method_outside_the_registry() {
        // 汎用 exec は public IPC として存在しない。名前だけ送っても復号で落ちる。
        let payload = json!({
            "protocolMajor": 1,
            "kind": "request",
            "requestId": "r-1",
            "method": "exec",
            "payload": {"command": "whoami"}
        })
        .to_string();

        assert!(matches!(
            decode(&frame_of(payload.as_bytes())).unwrap_err(),
            FrameError::MalformedPayload { .. }
        ));
    }

    #[test]
    fn the_method_registry_excludes_general_purpose_escapes() {
        let names: Vec<String> = Method::ALL
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect();

        for forbidden in ["exec", "readFile", "writeFile", "sql"] {
            assert!(
                !names.iter().any(|n| n.contains(forbidden)),
                "{forbidden} が method registry に入っている"
            );
        }
        assert_eq!(names.len(), Method::ALL.len());
    }

    #[test]
    fn a_dto_tolerates_a_field_added_by_a_newer_minor() {
        // minor 追加は unknown field 許容で互換性を保つ（docs/10 §9）。既知の項目
        // だけを読んで動き続ける。
        let payload = json!({
            "runnerId": "runner-1",
            "backendId": "backend-1",
            "scopeId": "scope-1",
            "desired": "running",
            "local": "running",
            "localFreshness": "fresh",
            "remotePresence": "registered",
            "remoteAvailability": "online_idle",
            "remoteFreshness": "fresh",
            "remoteErrorCode": null,
            "remoteRunnerId": "41",
            "verifiedAt": "2026-09-20T00:00:00.000Z",
            "lastKnownAvailability": null,
            "effective": "ready",
            "somethingAddedLater": {"nested": true}
        });

        let observation: dto::RunnerObservation = serde_json::from_value(payload).unwrap();

        assert_eq!(observation.effective, dto::EffectiveState::Ready);
        assert_eq!(observation.remote_runner_id, Some(ids::DecimalU64::new(41)));
    }

    #[test]
    fn an_unknown_enum_value_is_rejected_rather_than_guessed() {
        // 未知 field は許容するが、既知 field の未知の値は推測しない。
        let payload = json!({"code": "SOMETHING_NEW", "messageKey": "k", "retryable": false,
                             "requiresConfirmation": false, "operationId": null});

        assert!(serde_json::from_value::<error::ErrorPayload>(payload).is_err());
    }

    #[test]
    fn decimal_u64_crosses_the_wire_as_a_string() {
        // JavaScript の number では失われる桁を保つ。
        let large = DecimalU64::new(9_007_199_254_740_993);

        let encoded = serde_json::to_string(&large).unwrap();

        assert_eq!(encoded, "\"9007199254740993\"");
        assert_eq!(serde_json::from_str::<DecimalU64>(&encoded).unwrap(), large);
    }

    #[test]
    fn decimal_u64_rejects_a_json_number() {
        assert!(serde_json::from_str::<DecimalU64>("1842").is_err());
    }

    #[test]
    fn event_sequence_is_serialized_as_a_decimal_string() {
        let event = Event::new(
            DecimalU64::new(1842),
            "9a9a6fec".into(),
            EventKind::RunnerObservationChanged,
            json!({}),
        );

        let value = serde_json::to_value(Message::Event(event)).unwrap();

        assert_eq!(value["sequence"], json!("1842"));
        assert_eq!(value["kind"], json!("event"));
        assert_eq!(value["event"], json!("runner.observation.changed"));
    }

    #[test]
    fn encode_rejects_a_message_larger_than_the_limit() {
        let oversized = Message::Event(Event::new(
            DecimalU64::new(1),
            "gen".into(),
            EventKind::LogAppended,
            json!({ "line": "x".repeat(MAX_FRAME_BYTES + 1) }),
        ));

        assert!(matches!(
            encode(&oversized).unwrap_err(),
            frame::EncodeError::TooLarge { .. }
        ));
    }
}
