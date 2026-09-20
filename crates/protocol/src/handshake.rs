//! `system.handshake` の版判定。
//!
//! UI・Agent・Guest は同じ互換表で管理する（`docs/03_ARCHITECTURE.md` §8）。
//! 版が一致しないときは操作を拒否し、「たぶん動く」で先へ進めない。

use serde::{Deserialize, Serialize};

/// ワイヤー契約の版。
///
/// 破壊的変更のたびに増やす。実装のリリース版（`Peer::implementation`）とは
/// 別に管理し、UI の表示都合で動かさない。
pub const PROTOCOL_VERSION: u32 = 1;

/// handshake を交わす相手の自己申告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    /// 相手が話すワイヤー契約の版。
    pub protocol_version: u32,
    /// 相手の実装版。診断とログの相関のためだけに使う。
    pub implementation: String,
}

/// `system.handshake` の要求本体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Handshake {
    /// 呼び出し側の自己申告。
    pub peer: Peer,
}

/// handshake を拒否した理由。
///
/// UI へは理由を区別して渡す。どちらを更新すべきかが決まるため、
/// 「接続できません」へ丸めない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "reason")]
pub enum HandshakeRejection {
    /// 相手が古い。相手側の更新が要る。
    PeerTooOld {
        /// 相手が話す版。
        peer: u32,
        /// こちらが話す版。
        expected: u32,
    },
    /// 相手が新しい。こちら側の更新が要る。
    PeerTooNew {
        /// 相手が話す版。
        peer: u32,
        /// こちらが話す版。
        expected: u32,
    },
}

/// 相手の版をこの実装が受け入れられるか判定する。
#[must_use]
pub fn is_compatible(peer_protocol_version: u32) -> bool {
    peer_protocol_version == PROTOCOL_VERSION
}

/// handshake を判定し、拒否する場合は区別できる理由を返す。
///
/// # Errors
///
/// 相手の版がこの実装の版と一致しないとき [`HandshakeRejection`] を返す。
pub fn review(handshake: &Handshake) -> Result<(), HandshakeRejection> {
    let peer = handshake.peer.protocol_version;
    if peer == PROTOCOL_VERSION {
        return Ok(());
    }
    if peer < PROTOCOL_VERSION {
        return Err(HandshakeRejection::PeerTooOld {
            peer,
            expected: PROTOCOL_VERSION,
        });
    }
    Err(HandshakeRejection::PeerTooNew {
        peer,
        expected: PROTOCOL_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handshake_with(protocol_version: u32) -> Handshake {
        Handshake {
            peer: Peer {
                protocol_version,
                implementation: "test".to_owned(),
            },
        }
    }

    #[test]
    fn accepts_a_peer_speaking_the_same_protocol_version() {
        assert!(review(&handshake_with(PROTOCOL_VERSION)).is_ok());
        assert!(is_compatible(PROTOCOL_VERSION));
    }

    #[test]
    fn distinguishes_an_older_peer_from_a_newer_one() {
        assert_eq!(
            review(&handshake_with(PROTOCOL_VERSION - 1)),
            Err(HandshakeRejection::PeerTooOld {
                peer: PROTOCOL_VERSION - 1,
                expected: PROTOCOL_VERSION,
            })
        );
        assert_eq!(
            review(&handshake_with(PROTOCOL_VERSION + 1)),
            Err(HandshakeRejection::PeerTooNew {
                peer: PROTOCOL_VERSION + 1,
                expected: PROTOCOL_VERSION,
            })
        );
    }

    #[test]
    fn serializes_with_the_camel_case_field_names_the_wire_contract_uses() {
        let json = serde_json::to_value(handshake_with(PROTOCOL_VERSION)).unwrap();
        assert_eq!(json["peer"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(json["peer"]["implementation"], "test");
    }

    #[test]
    fn rejects_a_peer_payload_that_omits_the_protocol_version() {
        let error = serde_json::from_str::<Handshake>(r#"{"peer":{"implementation":"x"}}"#)
            .expect_err("a missing protocol version must not default to the current one");
        assert!(error.to_string().contains("protocolVersion"));
    }
}
