//! Desktop shell 側から Agent へ繋ぐ。
//!
//! GUI は起動のたびにまず接続を試み、handshake が通ればその Agent を使う
//! （`docs/03_ARCHITECTURE.md` §3）。接続できないときだけ Agent を起こすのは
//! 呼び出し側の判断で、この module は「繋いで、型付きの要求を往復させる」ところ
//! までを持つ。

use std::io;
use std::time::Duration;

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::{HandshakeRequest, HandshakeResult, NodeSnapshot};
use runnerdock_protocol::error::ErrorPayload;
use runnerdock_protocol::frame::{DecodeOutcome, decode, encode};
use runnerdock_protocol::ids::RequestId;
use runnerdock_protocol::message::{Message, Request, Response};
use runnerdock_protocol::method::Method;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

use crate::name::pipe_name;

/// 接続や往復で起きた失敗。
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// pipe へ繋げない。Agent が動いていないか、別 SID の pipe である。
    #[error("Agent へ接続できない: {0}")]
    Connect(io::Error),
    /// 送受信中の I/O 失敗。
    #[error("Agent との通信が途切れた: {0}")]
    Io(io::Error),
    /// 相手が契約に合わないフレームを返した。
    #[error("Agent の応答を契約として解釈できない")]
    MalformedResponse,
    /// Agent が型付きのエラーを返した。
    #[error("Agent が要求を拒否した: {}", .0.code.message_key())]
    Refused(Box<ErrorPayload>),
}

/// Agent への接続。1 本の pipe を保持する。
#[derive(Debug)]
pub struct AgentClient {
    pipe: NamedPipeClient,
    buffer: Vec<u8>,
    next_request: u64,
}

impl AgentClient {
    /// SID から決まる pipe へ繋ぎ、handshake まで済ませる。
    ///
    /// # Errors
    ///
    /// 接続できないとき、handshake が通らないときに返す。
    pub async fn connect(sid: &str) -> Result<(Self, HandshakeResult), ClientError> {
        let name = pipe_name(sid).map_err(|error| ClientError::Connect(io::Error::other(error)))?;
        let pipe = ClientOptions::new()
            .open(&name)
            .map_err(ClientError::Connect)?;
        let mut client = Self {
            pipe,
            buffer: Vec::new(),
            next_request: 0,
        };
        let handshake = client.handshake().await?;
        Ok((client, handshake))
    }

    async fn handshake(&mut self) -> Result<HandshakeResult, ClientError> {
        let peer = HandshakeRequest {
            protocol_major: PROTOCOL_MAJOR,
            implementation_version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        let payload = serde_json::to_value(&peer).map_err(|_| ClientError::MalformedResponse)?;
        let result = self.call(Method::SystemHandshake, payload).await?;
        serde_json::from_value(result).map_err(|_| ClientError::MalformedResponse)
    }

    /// 現在の snapshot を取り直す。再接続のたびにここから状態を作り直す。
    ///
    /// # Errors
    ///
    /// 通信に失敗したとき、Agent が拒否したときに返す。
    pub async fn node_snapshot(&mut self) -> Result<NodeSnapshot, ClientError> {
        let result = self.call(Method::NodeSnapshot, json!({})).await?;
        serde_json::from_value(result).map_err(|_| ClientError::MalformedResponse)
    }

    /// 1 往復する。registry にある method しか呼べない。
    ///
    /// # Errors
    ///
    /// 通信に失敗したとき、Agent が型付きエラーを返したときに返す。
    pub async fn call(&mut self, method: Method, payload: Value) -> Result<Value, ClientError> {
        self.next_request += 1;
        let request_id = RequestId(format!("ui-{}", self.next_request));
        let request = Request::new(request_id.clone(), method, payload);
        let bytes =
            encode(&Message::Request(request)).map_err(|_| ClientError::MalformedResponse)?;
        self.pipe.write_all(&bytes).await.map_err(ClientError::Io)?;
        self.pipe.flush().await.map_err(ClientError::Io)?;

        loop {
            match self.read_message().await? {
                Message::Response(response) if response.request_id == request_id => {
                    return unwrap_response(response);
                }
                // event は購読側が読む。ここでは応答を待ち続ける。
                Message::Event(_) => {}
                // 別の requestId の応答や、こちらへ来ないはずの種別。
                _ => return Err(ClientError::MalformedResponse),
            }
        }
    }

    async fn read_message(&mut self) -> Result<Message, ClientError> {
        let mut chunk = [0_u8; 8192];
        loop {
            match decode(&self.buffer) {
                Ok(DecodeOutcome::Decoded { message, consumed }) => {
                    self.buffer.drain(..consumed);
                    return Ok(*message);
                }
                Ok(DecodeOutcome::Incomplete) => {}
                Err(_) => return Err(ClientError::MalformedResponse),
            }
            let read = self.pipe.read(&mut chunk).await.map_err(ClientError::Io)?;
            if read == 0 {
                return Err(ClientError::Io(io::Error::from(
                    io::ErrorKind::UnexpectedEof,
                )));
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }
}

/// 応答を結果へほどく。`ok=false` を成功へ丸めない。
fn unwrap_response(response: Response) -> Result<Value, ClientError> {
    if response.ok {
        response.result.ok_or(ClientError::MalformedResponse)
    } else {
        response
            .error
            .map_or(Err(ClientError::MalformedResponse), |error| {
                Err(ClientError::Refused(Box::new(error)))
            })
    }
}

/// 起動直後の Agent へ、待受が立ち上がるまで何回か繋ぎ直す。
///
/// 無限に待たない。立ち上がらないことを「まだ準備中」として隠すと、UI が
/// いつまでも操作できない理由を出せなくなる。
///
/// # Errors
///
/// 試行回数を使い切っても繋がらないときに、最後の失敗を返す。
pub async fn connect_with_retry(
    sid: &str,
    attempts: u32,
    interval: Duration,
) -> Result<(AgentClient, HandshakeResult), ClientError> {
    let mut last = ClientError::Connect(io::Error::from(io::ErrorKind::NotFound));
    for attempt in 0..attempts.max(1) {
        match AgentClient::connect(sid).await {
            Ok(connected) => return Ok(connected),
            Err(error) => last = error,
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(interval).await;
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runnerdock_protocol::error::ErrorCode;

    #[test]
    fn a_refused_response_is_not_turned_into_a_result() {
        let response = Response::err(
            RequestId::from("ui-1"),
            ErrorPayload::new(ErrorCode::MethodNotServed),
        );

        let error = unwrap_response(response).unwrap_err();

        assert!(matches!(error, ClientError::Refused(_)));
    }

    #[test]
    fn a_success_without_a_result_is_not_accepted() {
        let response = Response {
            protocol_major: PROTOCOL_MAJOR,
            request_id: RequestId::from("ui-1"),
            ok: true,
            result: None,
            error: None,
        };

        assert!(matches!(
            unwrap_response(response).unwrap_err(),
            ClientError::MalformedResponse
        ));
    }

    #[tokio::test]
    async fn connecting_without_a_listening_agent_fails_instead_of_hanging() {
        // 待受けていない SID の pipe。失敗して戻ることを確かめる。
        let sid = "S-1-5-21-999999999-999999999-999999999-4242";

        let error = connect_with_retry(sid, 2, Duration::from_millis(1))
            .await
            .unwrap_err();

        assert!(matches!(error, ClientError::Connect(_)), "{error:?}");
    }
}
