//! method から処理への割り当て。
//!
//! registry は [`Method`] の列挙で固定してあり、列挙にない名前は復号段階で
//! 落ちる（`docs/10_IPC_DATA_MODEL.md` §4）。ここで決めるのは「列挙済みの
//! method をこの版が提供するか」だけで、汎用 `exec` / `readFile` / `writeFile`
//! / `sql` に相当する入口は作らない。

use runnerdock_core::mock::MockBackend;
use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::{
    ForceStopRequest, HandshakeRejection, HandshakeRequest, HandshakeResult, NodeOperationRequest,
};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use runnerdock_protocol::ids::{AgentGeneration, RequestId};
use runnerdock_protocol::message::{Event, Request};
use runnerdock_protocol::method::Method;
use serde_json::{Value, json};

/// 1 接続からの要求を処理する側。
///
/// handshake の順序と frame の検査は [`crate::session`] が持ち、この trait は
/// 「受理した要求をどう処理するか」だけを負う。
pub trait AgentService {
    /// この版が提供する method。handshake の `capabilities` として申告する。
    fn served_methods(&self) -> &'static [Method];

    /// 要求を処理する。
    ///
    /// # Errors
    ///
    /// 処理できない要求に対して、機械判定できる [`ErrorPayload`] を返す。失敗を
    /// 成功へ丸めない。
    fn call(&mut self, request: &Request) -> Result<Value, ErrorPayload>;

    /// handshake に載せる Agent 世代。
    fn agent_generation(&self) -> AgentGeneration;

    /// 直前の処理で溜まった event を取り出す。
    fn drain_events(&mut self) -> Vec<Event>;
}

/// LF-002 の Mock ドメインを IPC から見えるようにした service。
///
/// 実 Backend と GitHub へは接続しない。`node.snapshot` と Node 操作、Operation
/// 参照だけを提供し、残りは [`ErrorCode::MethodNotServed`] で拒否する。
#[derive(Debug)]
pub struct MockAgentService {
    backend: MockBackend,
}

/// この版が提供する method。ここに無い method は拒否する。
const SERVED: &[Method] = &[
    Method::SystemHandshake,
    Method::NodeSnapshot,
    Method::NodeStart,
    Method::NodeStop,
    Method::NodeForceStop,
    Method::OperationGet,
];

impl MockAgentService {
    /// Mock ドメインを包む。
    #[must_use]
    pub const fn new(backend: MockBackend) -> Self {
        Self { backend }
    }

    /// handshake の応答を組む。
    ///
    /// # Errors
    ///
    /// major が一致しないときに [`ErrorCode::ProtocolMismatch`] を返す。
    pub fn handshake(&self, payload: &Value) -> Result<HandshakeResult, ErrorPayload> {
        let peer: HandshakeRequest = parse_payload(payload)?;
        if let Err(rejection) = HandshakeRejection::evaluate(peer.protocol_major) {
            // どちらを更新すべきかを UI が案内できるよう、理由をそのまま渡す。
            let detail = serde_json::to_value(rejection).unwrap_or(Value::Null);
            return Err(
                ErrorPayload::new(ErrorCode::ProtocolMismatch).with_detail("rejection", detail)
            );
        }
        Ok(HandshakeResult {
            protocol_major: PROTOCOL_MAJOR,
            implementation_version: env!("CARGO_PKG_VERSION").to_owned(),
            agent_generation: self.backend.agent_generation().clone(),
            capabilities: SERVED.iter().map(|m| m.as_str().to_owned()).collect(),
        })
    }

    fn node_operation(
        &mut self,
        method: Method,
        request_id: &RequestId,
        payload: &Value,
    ) -> Result<Value, ErrorPayload> {
        let (node_id, expected_revision, confirmation) = if method == Method::NodeForceStop {
            let request: ForceStopRequest = parse_payload(payload)?;
            (
                request.node_id,
                request.expected_revision,
                request.confirmation,
            )
        } else {
            let request: NodeOperationRequest = parse_payload(payload)?;
            (request.node_id, request.expected_revision, None)
        };

        if &node_id != self.backend.node_id() {
            // 知らない Node の要求を、手元の Node へ読み替えない。
            return Err(
                ErrorPayload::new(ErrorCode::PathNotOwned).with_detail("field", json!("nodeId"))
            );
        }

        let accepted = match method {
            Method::NodeStart => self.backend.request_start(request_id, expected_revision),
            Method::NodeStop => self.backend.request_stop(request_id, expected_revision),
            Method::NodeForceStop => self.backend.request_force_stop(
                request_id,
                expected_revision,
                confirmation.as_deref(),
            ),
            _ => return Err(ErrorPayload::new(ErrorCode::MethodNotServed)),
        }?;
        to_value(&accepted)
    }
}

impl AgentService for MockAgentService {
    fn served_methods(&self) -> &'static [Method] {
        SERVED
    }

    fn call(&mut self, request: &Request) -> Result<Value, ErrorPayload> {
        match request.method {
            Method::SystemHandshake => to_value(&self.handshake(&request.payload)?),
            Method::NodeSnapshot => to_value(&self.backend.snapshot()),
            method @ (Method::NodeStart | Method::NodeStop | Method::NodeForceStop) => {
                self.node_operation(method, &request.request_id, &request.payload)
            }
            Method::OperationGet => {
                // 未知の requestId を「完了」や「無し」の成功へ丸めない。
                let phase = self
                    .backend
                    .operation_phase(&request.request_id)
                    .ok_or_else(|| {
                        ErrorPayload::new(ErrorCode::RequestIdConflict)
                            .with_detail("field", json!("requestId"))
                    })?;
                to_value(&phase)
            }
            // registry にはあるが、この版では提供しない。
            _ => Err(ErrorPayload::new(ErrorCode::MethodNotServed)
                .with_detail("method", json!(request.method.as_str()))),
        }
    }

    fn agent_generation(&self) -> AgentGeneration {
        self.backend.agent_generation().clone()
    }

    fn drain_events(&mut self) -> Vec<Event> {
        self.backend.drain_events()
    }
}

/// 受け取った payload を具体型へ変換する。未知 JSON を無検証で扱わない。
fn parse_payload<T: serde::de::DeserializeOwned>(payload: &Value) -> Result<T, ErrorPayload> {
    serde_json::from_value(payload.clone()).map_err(|_| {
        // 相手が送った本文を details へ載せない（`docs/10_IPC_DATA_MODEL.md` §8）。
        ErrorPayload::new(ErrorCode::ProtocolMismatch).with_detail("field", json!("payload"))
    })
}

fn to_value<T: serde::Serialize>(value: &T) -> Result<Value, ErrorPayload> {
    serde_json::to_value(value).map_err(|_| {
        ErrorPayload::new(ErrorCode::ProtocolMismatch).with_detail("field", json!("result"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use runnerdock_core::mock::MockScenario;

    fn service() -> MockAgentService {
        MockAgentService::new(MockBackend::with_scenario(7, MockScenario::Idle))
    }

    fn request(method: Method, payload: Value) -> Request {
        Request::new(RequestId::from("req-1"), method, payload)
    }

    #[test]
    fn handshake_reports_only_the_methods_this_version_serves() {
        let service = service();

        let result = service
            .handshake(&json!({
                "protocolMajor": PROTOCOL_MAJOR,
                "implementationVersion": "0.0.0"
            }))
            .unwrap();

        assert_eq!(result.protocol_major, PROTOCOL_MAJOR);
        assert!(result.capabilities.contains(&"node.snapshot".to_owned()));
        assert!(
            !result
                .capabilities
                .contains(&"credential.import".to_owned())
        );
    }

    #[test]
    fn handshake_rejects_a_different_protocol_major() {
        let error = service()
            .handshake(&json!({
                "protocolMajor": PROTOCOL_MAJOR + 1,
                "implementationVersion": "0.0.0"
            }))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ProtocolMismatch);
    }

    #[test]
    fn a_registered_but_unserved_method_is_refused_instead_of_answered() {
        let error = service()
            .call(&request(Method::CredentialImport, json!({})))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::MethodNotServed);
        assert_eq!(
            error.details.get("method"),
            Some(&json!("credential.import"))
        );
    }

    #[test]
    fn every_method_is_either_served_or_refused() {
        for method in Method::ALL {
            let outcome = service().call(&request(*method, json!({})));

            if SERVED.contains(method) {
                // 提供する method は payload 不足で落ちてもよいが、
                // MethodNotServed にはならない。
                if let Err(error) = outcome {
                    assert_ne!(error.code, ErrorCode::MethodNotServed, "{method:?}");
                }
            } else {
                assert_eq!(
                    outcome.unwrap_err().code,
                    ErrorCode::MethodNotServed,
                    "{method:?}"
                );
            }
        }
    }

    #[test]
    fn node_start_refuses_a_node_id_this_agent_does_not_own() {
        let mut service = service();

        let error = service
            .call(&request(
                Method::NodeStart,
                json!({"nodeId": "node-elsewhere", "expectedRevision": 1}),
            ))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PathNotOwned);
    }

    #[test]
    fn node_start_accepts_a_request_for_the_owned_node() {
        let mut service = service();
        let node_id = service.backend.node_id().clone();
        let revision = service.backend.revision();

        let result = service
            .call(&request(
                Method::NodeStart,
                json!({"nodeId": node_id, "expectedRevision": revision}),
            ))
            .unwrap();

        assert_eq!(result.get("accepted"), Some(&json!(true)));
    }

    #[test]
    fn operation_get_does_not_invent_a_phase_for_an_unknown_request_id() {
        let error = service()
            .call(&request(Method::OperationGet, json!({})))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::RequestIdConflict);
    }
}
