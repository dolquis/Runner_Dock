//! 1 接続の読み書き。
//!
//! `docs/10_IPC_DATA_MODEL.md` §2 と §9 の判断をここへ集める。未知のメッセージ
//! 種別、major 不一致、上限超過、不正 UTF-8 は「無視して続ける」ではなく接続を
//! 切る。読み取り途中の切断を完全なメッセージとして扱わない。
//!
//! 具体的な transport（Windows named pipe、試験の `tokio::io::duplex`）に依存
//! させず、`AsyncRead + AsyncWrite` に対して書く。pipe を作れない環境でも、
//! 拒否の判断そのものを試験できるようにするためである。

use runnerdock_core::events::{PushOutcome, SubscriberQueue};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use runnerdock_protocol::frame::{DecodeOutcome, FrameError, LENGTH_PREFIX_BYTES, decode, encode};
use runnerdock_protocol::ids::DecimalU64;
use runnerdock_protocol::message::{Event, EventKind, Message, Response};
use runnerdock_protocol::method::Method;
use serde_json::json;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::service::AgentService;

/// 購読者ごとのキュー上限（`docs/10_IPC_DATA_MODEL.md` §9）。
///
/// この版は `events.subscribe` を提供しない。event が出るのは要求を受理した
/// 直後だけで、背圧と `ResyncRequired` の経路はまだ働かない。遅い購読者への
/// 対処は、購読と多重化を入れる LF-009 で実際に効く形にする。
pub const EVENT_QUEUE_CAPACITY: usize = 256;

/// handshake を待つ上限。
///
/// 繋いだまま何も送らない相手を、待受の占有へ使わせない。GUI の起動直後に
/// handshake を 1 往復するだけなので、これを超えるのは正常な利用ではない。
pub const HANDSHAKE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

// buffer の上限は [`decode`] が担う。宣言長が [`MAX_FRAME_BYTES`] を超えた
// 時点で接続を切るので、溜まるのは長さ prefix と 1 フレーム分までである。
// 同じ判定をここへ二重に置くと、到達しない防御コードになる。

/// 接続が終わった理由。呼び出し側はログと診断にだけ使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEnd {
    /// 相手が正常に閉じた。
    PeerClosed,
    /// フレームが契約に合わないので切った。
    RejectedFrame(FrameError),
    /// handshake より前に別の要求が来たので切った。
    HandshakeRequired,
    /// Agent 側へ Request 以外のメッセージが来たので切った。
    UnexpectedMessageKind,
    /// 書き込みまたは読み取りが I/O エラーで終わった。
    Io(String),
    /// 自分が送ろうとしたメッセージを符号化できなかった。
    ///
    /// 相手の問題ではなく実装側の不具合である。I/O 失敗と混ぜると診断で
    /// 原因を取り違える。
    EncodeFailed,
}

/// 1 接続を終わりまで処理する。
///
/// 戻り値は接続を終えた理由である。`Err` を返さないのは、1 接続の失敗が待受
/// 全体の失敗ではないからで、呼び出し側は理由を記録して次の接続へ進む。
pub async fn serve_connection<S, A>(stream: S, service: &mut A) -> SessionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: AgentService,
{
    let mut stream = stream;
    let mut buffer: Vec<u8> = Vec::with_capacity(LENGTH_PREFIX_BYTES);
    let mut chunk = [0_u8; 8192];
    let mut queue = SubscriberQueue::with_capacity(EVENT_QUEUE_CAPACITY);
    let mut handshake_done = false;

    loop {
        // 溜まっている分を先に処理してから次を読む。1 度の読み取りに複数
        // フレームが入っていても取りこぼさない。
        loop {
            match decode(&buffer) {
                Ok(DecodeOutcome::Incomplete) => break,
                Ok(DecodeOutcome::Decoded { message, consumed }) => {
                    buffer.drain(..consumed);
                    let Message::Request(request) = *message else {
                        // Agent は Response も Event も受け取らない。
                        return SessionEnd::UnexpectedMessageKind;
                    };

                    if !handshake_done && request.method != Method::SystemHandshake {
                        // 互換性を確かめる前に操作させない。理由を返してから切る。
                        let error = ErrorPayload::new(ErrorCode::ProtocolMismatch)
                            .with_detail("reason", json!("handshakeRequired"));
                        let response = Response::err(request.request_id, error);
                        if let Err(end) =
                            write_message(&mut stream, &Message::Response(response)).await
                        {
                            return end;
                        }
                        return SessionEnd::HandshakeRequired;
                    }

                    // 能力の申告と実際の受理を同じ一覧から決める。ここを通さずに
                    // 個々の実装へ委ねると、申告した method と受け付ける method が
                    // 無検査でずれる。
                    let outcome = if service.served_methods().contains(&request.method) {
                        service.call(&request)
                    } else {
                        Err(ErrorPayload::new(ErrorCode::MethodNotServed)
                            .with_detail("method", json!(request.method.as_str())))
                    };
                    if outcome.is_ok() && request.method == Method::SystemHandshake {
                        handshake_done = true;
                    }
                    let response = match outcome {
                        Ok(result) => Response::ok(request.request_id, result),
                        Err(error) => Response::err(request.request_id, error),
                    };
                    if let Err(end) = write_message(&mut stream, &Message::Response(response)).await
                    {
                        return end;
                    }

                    for event in service.drain_events() {
                        if let PushOutcome::Dropped { .. } = queue.push(event) {
                            // 落としたことを黙らない。
                            tracing::warn!(dropped = queue.dropped(), "event を落とした");
                        }
                    }
                    if let Err(end) = flush_events(&mut stream, &mut queue, service).await {
                        return end;
                    }
                }
                Err(error) => {
                    // 上限超過も不正 UTF-8 も、読み捨てて続けず接続を切る。
                    tracing::warn!(?error, "フレームを拒否した");
                    return SessionEnd::RejectedFrame(error);
                }
            }
        }

        // handshake が済むまでは待ち続けない。1 接続ずつ処理する現在の待受では、
        // 繋いだまま何も送らない相手が他の接続を締め出してしまう。
        let read = if handshake_done {
            stream.read(&mut chunk).await
        } else {
            match tokio::time::timeout(HANDSHAKE_DEADLINE, stream.read(&mut chunk)).await {
                Ok(read) => read,
                Err(_) => {
                    tracing::warn!("handshake を待つ時間を超えたので接続を切る");
                    return SessionEnd::HandshakeRequired;
                }
            }
        };
        match read {
            Ok(0) => return SessionEnd::PeerClosed,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(error) => return SessionEnd::Io(error.kind().to_string()),
        }
    }
}

/// キューに溜めた event を送る。状態を落としていたら `ResyncRequired` を先に送る。
async fn flush_events<S, A>(
    stream: &mut S,
    queue: &mut SubscriberQueue,
    service: &A,
) -> Result<(), SessionEnd>
where
    S: AsyncWrite + Unpin,
    A: AgentService,
{
    if queue.needs_resync() {
        let event = Event::new(
            // 取り直しの合図そのものは連番の欠落判定に使わせない。
            DecimalU64::new(0),
            service.agent_generation(),
            EventKind::ResyncRequired,
            json!({"droppedLogs": queue.dropped_logs()}),
        );
        write_message(stream, &Message::Event(event)).await?;
        queue.clear_resync();
    }
    while let Some(event) = queue.pop() {
        write_message(stream, &Message::Event(event)).await?;
    }
    Ok(())
}

async fn write_message<S>(stream: &mut S, message: &Message) -> Result<(), SessionEnd>
where
    S: AsyncWrite + Unpin,
{
    let bytes = match encode(message) {
        Ok(bytes) => bytes,
        Err(error) => {
            // 自分の送信物が上限を超えるのは実装側の不具合。相手には返せない。
            tracing::error!(?error, "送信フレームを符号化できなかった");
            return Err(SessionEnd::EncodeFailed);
        }
    };
    stream
        .write_all(&bytes)
        .await
        .map_err(|error| SessionEnd::Io(error.kind().to_string()))?;
    stream
        .flush()
        .await
        .map_err(|error| SessionEnd::Io(error.kind().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::MockAgentService;
    use runnerdock_core::mock::{MockBackend, MockScenario};
    use runnerdock_protocol::PROTOCOL_MAJOR;
    use runnerdock_protocol::frame::MAX_FRAME_BYTES;
    use runnerdock_protocol::ids::RequestId;
    use runnerdock_protocol::message::Request;
    use tokio::io::{DuplexStream, duplex};

    fn service() -> MockAgentService {
        MockAgentService::new(MockBackend::with_scenario(11, MockScenario::Idle))
    }

    fn handshake_request() -> Message {
        Message::Request(Request::new(
            RequestId::from("req-handshake"),
            Method::SystemHandshake,
            json!({"protocolMajor": PROTOCOL_MAJOR, "implementationVersion": "0.0.0"}),
        ))
    }

    async fn send(client: &mut DuplexStream, message: &Message) {
        client.write_all(&encode(message).unwrap()).await.unwrap();
    }

    /// 1 件読む。1 回の読み取りに複数フレームが入ることがあるので、buffer は
    /// 呼び出しを跨いで保持する。ここで捨てると次の 1 件を取りこぼす。
    async fn read_message(client: &mut DuplexStream, buffer: &mut Vec<u8>) -> Option<Message> {
        let mut chunk = [0_u8; 4096];
        loop {
            match decode(buffer) {
                Ok(DecodeOutcome::Decoded { message, consumed }) => {
                    buffer.drain(..consumed);
                    return Some(*message);
                }
                Ok(DecodeOutcome::Incomplete) => {}
                Err(error) => panic!("{error:?}"),
            }
            let read = client.read(&mut chunk).await.unwrap();
            if read == 0 {
                return None;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
    }

    /// 接続を 1 つ張り、client 側と session の future を返す。
    fn connect() -> (DuplexStream, DuplexStream) {
        duplex(64 * 1024)
    }

    #[tokio::test]
    async fn a_request_before_handshake_is_refused_and_the_connection_closes() {
        let (mut client, server) = connect();
        let mut buffer: Vec<u8> = Vec::new();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });
        send(
            &mut client,
            &Message::Request(Request::new(
                RequestId::from("req-1"),
                Method::NodeSnapshot,
                json!({}),
            )),
        )
        .await;

        let Some(Message::Response(response)) = read_message(&mut client, &mut buffer).await else {
            panic!("応答が来ていない");
        };
        assert!(!response.ok);
        assert_eq!(
            response.error.map(|e| e.code),
            Some(ErrorCode::ProtocolMismatch)
        );
        assert_eq!(session.await.unwrap(), SessionEnd::HandshakeRequired);
    }

    #[tokio::test]
    async fn a_frame_larger_than_the_limit_closes_the_connection_without_a_response() {
        let (mut client, server) = connect();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });

        // 宣言長だけを上限超過にする。本体は送らない。
        let declared = u32::try_from(MAX_FRAME_BYTES + 1).unwrap();
        client.write_all(&declared.to_le_bytes()).await.unwrap();
        client.flush().await.unwrap();

        let end = session.await.unwrap();
        assert_eq!(
            end,
            SessionEnd::RejectedFrame(FrameError::TooLarge {
                declared: MAX_FRAME_BYTES + 1
            })
        );
    }

    #[tokio::test]
    async fn a_zero_length_frame_closes_the_connection() {
        let (mut client, server) = connect();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });

        client.write_all(&0_u32.to_le_bytes()).await.unwrap();
        client.flush().await.unwrap();

        assert_eq!(
            session.await.unwrap(),
            SessionEnd::RejectedFrame(FrameError::ZeroLength)
        );
    }

    #[tokio::test]
    async fn an_unknown_method_name_closes_the_connection_at_decode() {
        let (mut client, server) = connect();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });

        let payload = serde_json::to_vec(&json!({
            "protocolMajor": PROTOCOL_MAJOR,
            "kind": "request",
            "requestId": "req-exec",
            "method": "exec",
            "payload": {"command": "cmd /c dir"}
        }))
        .unwrap();
        let length = u32::try_from(payload.len()).unwrap();
        client.write_all(&length.to_le_bytes()).await.unwrap();
        client.write_all(&payload).await.unwrap();
        client.flush().await.unwrap();

        match session.await.unwrap() {
            SessionEnd::RejectedFrame(FrameError::MalformedPayload { .. }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn a_response_sent_to_the_agent_closes_the_connection() {
        let (mut client, server) = connect();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });

        send(
            &mut client,
            &Message::Response(Response::ok(RequestId::from("req-1"), json!({}))),
        )
        .await;

        assert_eq!(session.await.unwrap(), SessionEnd::UnexpectedMessageKind);
    }

    #[tokio::test]
    async fn after_handshake_a_snapshot_is_served() {
        let (mut client, server) = connect();
        let mut buffer: Vec<u8> = Vec::new();
        let session = tokio::spawn(async move {
            let mut service = service();
            serve_connection(server, &mut service).await
        });

        send(&mut client, &handshake_request()).await;
        let Some(Message::Response(handshake)) = read_message(&mut client, &mut buffer).await
        else {
            panic!("handshake 応答が来ていない");
        };
        assert!(handshake.ok);

        send(
            &mut client,
            &Message::Request(Request::new(
                RequestId::from("req-snapshot"),
                Method::NodeSnapshot,
                json!({}),
            )),
        )
        .await;
        let Some(Message::Response(snapshot)) = read_message(&mut client, &mut buffer).await else {
            panic!("snapshot 応答が来ていない");
        };

        assert!(snapshot.ok);
        assert!(snapshot.result.is_some_and(|r| r.get("nodeId").is_some()));
        drop(client);
        assert_eq!(session.await.unwrap(), SessionEnd::PeerClosed);
    }

    #[tokio::test]
    async fn reconnecting_serves_the_same_snapshot_from_the_same_service() {
        let mut service = service();
        let mut snapshots = Vec::new();

        // 同じ service に対して 2 回繋ぎ直す。GUI を閉じて開き直した場合に
        // あたる。世代が変わらず、snapshot をそのまま取り直せる。
        for _ in 0..2 {
            let (mut client, server) = connect();
            let mut buffer: Vec<u8> = Vec::new();
            let session = async {
                serve_connection(server, &mut service).await;
            };
            let exchange = async {
                send(&mut client, &handshake_request()).await;
                let Some(Message::Response(handshake)) =
                    read_message(&mut client, &mut buffer).await
                else {
                    panic!("handshake 応答が来ていない");
                };
                send(
                    &mut client,
                    &Message::Request(Request::new(
                        RequestId::from("req-snapshot"),
                        Method::NodeSnapshot,
                        json!({}),
                    )),
                )
                .await;
                let Some(Message::Response(snapshot)) =
                    read_message(&mut client, &mut buffer).await
                else {
                    panic!("snapshot 応答が来ていない");
                };
                drop(client);
                (
                    handshake
                        .result
                        .and_then(|r| r.get("agentGeneration").cloned()),
                    snapshot.result,
                )
            };
            let (_, captured) = tokio::join!(session, exchange);
            snapshots.push(captured);
        }

        let (first_generation, first_snapshot) = snapshots[0].clone();
        let (second_generation, second_snapshot) = snapshots[1].clone();
        assert_eq!(first_generation, second_generation);
        assert!(first_snapshot.is_some());
        assert_eq!(first_snapshot, second_snapshot);
    }

    #[tokio::test]
    async fn events_produced_by_an_accepted_request_reach_the_peer() {
        let mut service = service();
        let (mut client, server) = connect();
        let mut buffer: Vec<u8> = Vec::new();
        let node_id = json!(
            MockBackend::with_scenario(11, MockScenario::Idle)
                .node_id()
                .as_str()
        );

        let session = async {
            serve_connection(server, &mut service).await;
        };
        let exchange = async {
            send(&mut client, &handshake_request()).await;
            let _ = read_message(&mut client, &mut buffer).await;
            send(
                &mut client,
                &Message::Request(Request::new(
                    RequestId::from("req-start"),
                    Method::NodeStart,
                    json!({"nodeId": node_id, "expectedRevision": 1}),
                )),
            )
            .await;
            let Some(Message::Response(accepted)) = read_message(&mut client, &mut buffer).await
            else {
                panic!("受理応答が来ていない");
            };
            assert!(accepted.ok, "{accepted:?}");
            let event = read_message(&mut client, &mut buffer).await;
            drop(client);
            event
        };
        let (_, event) = tokio::join!(session, exchange);

        match event {
            Some(Message::Event(event)) => {
                assert_eq!(event.event, EventKind::OperationChanged);
            }
            other => panic!("{other:?}"),
        }
    }
}
