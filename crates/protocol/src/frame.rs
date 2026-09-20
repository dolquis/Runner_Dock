//! フレーム復号。`docs/10_IPC_DATA_MODEL.md` §2。
//!
//! `uint32 little-endian length + UTF-8 JSON payload`。長さ 0、1 MiB 超過、
//! 不正 UTF-8、未知 major を拒否する。読み取り途中の切断を完全なメッセージとして
//! 扱わない（途中なら [`DecodeOutcome::Incomplete`]）。

use crate::PROTOCOL_MAJOR;
use crate::message::Message;

/// フレームの最大長（payload のバイト数）。
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// 長さ prefix のバイト数。
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// 復号に失敗した理由。いずれも接続を拒否する側に倒す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// 長さ 0 のフレーム。
    ZeroLength,
    /// 上限超過。読み捨てずに接続を切る。
    TooLarge { declared: usize },
    /// payload が UTF-8 でない。
    InvalidUtf8,
    /// JSON として、または既知のメッセージ種別として解釈できない。
    MalformedPayload { detail: String },
    /// 未知の protocol major。無視して誤操作するより拒否する。
    UnknownProtocolMajor { found: u32 },
}

/// 1 回の復号試行の結果。
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeOutcome {
    /// フレームが揃っていない。次の読み取りを待つ。
    Incomplete,
    /// 1 件復号できた。`consumed` は buffer から取り除くバイト数。
    Decoded {
        message: Box<Message>,
        consumed: usize,
    },
}

/// buffer の先頭から 1 フレームを復号する。
///
/// buffer は呼び出し側が保持し、`consumed` バイトだけ前方から取り除く。
///
/// # Errors
///
/// 長さ 0、上限超過、不正 UTF-8、JSON 不正、未知 major のときに [`FrameError`] を返す。
pub fn decode(buffer: &[u8]) -> Result<DecodeOutcome, FrameError> {
    let Some(prefix) = buffer.get(..LENGTH_PREFIX_BYTES) else {
        return Ok(DecodeOutcome::Incomplete);
    };
    let mut length_bytes = [0_u8; LENGTH_PREFIX_BYTES];
    length_bytes.copy_from_slice(prefix);
    let declared = u32::from_le_bytes(length_bytes) as usize;

    if declared == 0 {
        return Err(FrameError::ZeroLength);
    }
    if declared > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { declared });
    }

    let end = LENGTH_PREFIX_BYTES + declared;
    let Some(payload) = buffer.get(LENGTH_PREFIX_BYTES..end) else {
        // 宣言長に満たない。切断されただけの断片を完全なメッセージにしない。
        return Ok(DecodeOutcome::Incomplete);
    };

    let text = std::str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)?;

    // major だけ先に見る。未知 major の本文を既知の型へ当てはめようとしない。
    let raw: serde_json::Value =
        serde_json::from_str(text).map_err(|error| FrameError::MalformedPayload {
            detail: error.to_string(),
        })?;
    match raw.get("protocolMajor").and_then(serde_json::Value::as_u64) {
        Some(major) if major == u64::from(PROTOCOL_MAJOR) => {}
        Some(major) => {
            return Err(FrameError::UnknownProtocolMajor {
                found: u32::try_from(major).unwrap_or(u32::MAX),
            });
        }
        None => {
            return Err(FrameError::MalformedPayload {
                detail: "protocolMajor がない".to_owned(),
            });
        }
    }

    let message: Message =
        serde_json::from_value(raw).map_err(|error| FrameError::MalformedPayload {
            detail: error.to_string(),
        })?;

    Ok(DecodeOutcome::Decoded {
        message: Box::new(message),
        consumed: end,
    })
}

/// 符号化に失敗した理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// 直列化した結果が上限を超えた。
    TooLarge { size: usize },
    /// 直列化そのものに失敗した。
    Serialization { detail: String },
}

/// メッセージを 1 フレームへ符号化する。
///
/// # Errors
///
/// 直列化に失敗したとき、または結果が [`MAX_FRAME_BYTES`] を超えたときに返す。
pub fn encode(message: &Message) -> Result<Vec<u8>, EncodeError> {
    let payload = serde_json::to_vec(message).map_err(|error| EncodeError::Serialization {
        detail: error.to_string(),
    })?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(EncodeError::TooLarge {
            size: payload.len(),
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| EncodeError::TooLarge {
        size: payload.len(),
    })?;

    let mut framed = Vec::with_capacity(LENGTH_PREFIX_BYTES + payload.len());
    framed.extend_from_slice(&length.to_le_bytes());
    framed.extend_from_slice(&payload);
    Ok(framed)
}
