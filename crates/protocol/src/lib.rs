//! Runner Dock の IPC ワイヤー契約。
//!
//! この crate が契約の正典で、JSON Schema と TypeScript 型はここから生成する
//! （`docs/04_TECHNOLOGY_STACK.md` §4）。OS 操作、HTTP、Tauri には依存しない。
//! 秘密本体を通常 DTO として追加しない（`docs/03_ARCHITECTURE.md` §6）。
//!
//! # `core` との責務分担
//!
//! ワイヤーへ出る型はドメイン概念であってもこの crate が持ち、`crates/core` は
//! それを `use` する。`BackendKind` や `DesiredState` のように、ドメインの語彙で
//! あると同時に IPC と SQLite の CHECK 制約にも現れる型を両側で定義すると、
//! 同じ概念が 2 つになり変換と乖離が生まれる。LF-002 はその乖離を防ぐために
//! 契約を 1 か所へ寄せるタスクなので、重複より片方向の依存を採る。
//!
//! `core` 側に置くのは、ワイヤーへ出ない判断の型（観測の解釈結果、鮮度の計算、
//! Operation の内部記録など）に限る。

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod dto;
pub mod error;
pub mod frame;
pub mod ids;
pub mod message;
pub mod method;

/// 現在の protocol major。不一致のメッセージは復号段階で拒否する。
///
/// SQLite schema の版は、migration を持つ `crates/storage` 側で定義する。
/// ワイヤー契約と保存形式は別の互換性単位なので、ここに並べない。
pub const PROTOCOL_MAJOR: u32 = 1;

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
    fn decode_consumes_only_one_frame_when_more_bytes_follow() {
        // buffer に次のフレームの先頭が続いていても、1 件分だけ取り除く。
        let first = encode(&Message::Event(Event::new(
            DecimalU64::new(1),
            "gen".into(),
            EventKind::ResyncRequired,
            json!({}),
        )))
        .unwrap();
        let second = encode(&Message::Event(Event::new(
            DecimalU64::new(2),
            "gen".into(),
            EventKind::OperationChanged,
            json!({}),
        )))
        .unwrap();
        let mut buffer = first.clone();
        buffer.extend_from_slice(&second);

        let DecodeOutcome::Decoded { consumed, .. } = decode(&buffer).unwrap() else {
            panic!("1 件目を復号できなかった");
        };

        assert_eq!(consumed, first.len());
        // 残りから 2 件目がそのまま復号できる。
        let DecodeOutcome::Decoded { message, .. } = decode(&buffer[consumed..]).unwrap() else {
            panic!("2 件目を復号できなかった");
        };
        let Message::Event(event) = *message else {
            panic!("event ではない");
        };
        assert_eq!(event.sequence, DecimalU64::new(2));
    }

    #[test]
    fn a_frame_of_exactly_the_limit_is_accepted() {
        // 上限ちょうどは拒否しない。`>` と `>=` の取り違えを止める。
        let filler = MAX_FRAME_BYTES - 2;
        let payload = format!("{{{}}}", " ".repeat(filler));
        assert_eq!(payload.len(), MAX_FRAME_BYTES);

        // JSON としては空 object なので、メッセージ種別が無い方の失敗になる。
        // 長さ判定を通過したこと自体が確認したい点。
        let outcome = decode(&frame_of(payload.as_bytes())).unwrap_err();

        assert!(
            !matches!(outcome, FrameError::TooLarge { .. }),
            "上限ちょうどを TooLarge にしている"
        );
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

    // ---- docs/10_IPC_DATA_MODEL.md §3 の例をそのまま契約試験にする ----------

    #[test]
    fn the_documented_request_example_decodes() {
        let documented = json!({
            "protocolMajor": 1,
            "kind": "request",
            "requestId": "c917d45a-bf92-4ac0-8c3b-c0d731277658",
            "method": "node.start",
            "payload": {
                "nodeId": "node-home",
                "expectedRevision": 3
            }
        });

        let message: Message = serde_json::from_value(documented).unwrap();

        let Message::Request(request) = message else {
            panic!("request として復号されなかった");
        };
        assert_eq!(request.method, Method::NodeStart);
        let payload: dto::NodeOperationRequest = serde_json::from_value(request.payload).unwrap();
        // revision は 10 進文字列ではなく数値。§5 が文字列を課すのは GitHub ID と
        // event sequence だけで、§3 の例も数値で書かれている。
        assert_eq!(payload.expected_revision, 3);
    }

    #[test]
    fn the_documented_response_example_decodes() {
        let documented = json!({
            "protocolMajor": 1,
            "kind": "response",
            "requestId": "c917d45a-bf92-4ac0-8c3b-c0d731277658",
            "ok": true,
            "result": {
                "operationId": "op-7c28",
                "accepted": true
            }
        });

        let message: Message = serde_json::from_value(documented).unwrap();

        let Message::Response(response) = message else {
            panic!("response として復号されなかった");
        };
        assert!(response.ok);
        let result: dto::OperationAccepted =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert!(result.accepted);
        // 例に無い項目は既定値。受付済みであって完了ではない。
        assert!(!result.deduplicated);
    }

    #[test]
    fn the_documented_event_example_decodes() {
        let documented = json!({
            "protocolMajor": 1,
            "kind": "event",
            "sequence": "1842",
            "agentGeneration": "9a9a6fec",
            "event": "runner.observation.changed",
            "payload": {
                "runnerId": "runner-linux",
                "local": "running",
                "remotePresence": "registered",
                "remoteAvailability": "unknown",
                "remoteFreshness": "stale",
                "remoteErrorCode": "AUTH_EXPIRED",
                "verifiedAt": "2026-09-19T03:12:45Z",
                "desired": "running"
            }
        });

        let message: Message = serde_json::from_value(documented).unwrap();

        let Message::Event(event) = message else {
            panic!("event として復号されなかった");
        };
        assert_eq!(event.sequence, ids::DecimalU64::new(1842));
        assert_eq!(event.event, EventKind::RunnerObservationChanged);
        // §3 の event payload は観測の一部だけを運ぶ。封筒は payload を検査せずに
        // 通し、method ごとの具体型へは受理後に変換する。
        assert_eq!(event.payload["remoteErrorCode"], json!("AUTH_EXPIRED"));
    }

    #[test]
    fn a_timestamp_must_be_utc_rfc3339() {
        // 秒精度とミリ秒精度の両方を受ける。
        assert!(ids::Timestamp::is_well_formed("2026-09-19T03:12:45Z"));
        assert!(ids::Timestamp::is_well_formed("2026-09-20T00:00:00.000Z"));
        // うるう年の 2 月 29 日と、RFC3339 が認めるうるう秒。
        assert!(ids::Timestamp::is_well_formed("2024-02-29T00:00:00Z"));
        assert!(ids::Timestamp::is_well_formed("2016-12-31T23:59:60Z"));
        assert!(ids::Timestamp::is_well_formed("2000-02-29T00:00:00Z"));

        for bad in [
            "2026-09-19T03:12:45+09:00", // 時差付き
            "2026-09-19 03:12:45Z",      // T がない
            "2026-09-19T03:12:45",       // Z がない
            "2026-09-19",                // 日付だけ
            "",
            // 桁と区切りは合っているが暦日・時刻として無効なもの。
            "2026-99-99T99:99:99Z",
            "2026-13-01T00:00:00Z", // 月が範囲外
            "2026-00-01T00:00:00Z", // 月が 0
            "2026-09-31T00:00:00Z", // 9 月は 30 日まで
            "2026-09-00T00:00:00Z", // 日が 0
            "2026-02-29T00:00:00Z", // 2026 年はうるう年ではない
            "2026-09-19T24:00:00Z", // 時が範囲外
            "2026-09-19T00:60:00Z", // 分が範囲外
            "2026-09-19T00:00:61Z", // 秒が範囲外
        ] {
            assert!(
                serde_json::from_value::<ids::Timestamp>(json!(bad)).is_err(),
                "{bad} を受理している"
            );
        }
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
