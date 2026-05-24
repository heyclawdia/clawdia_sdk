use agent_sdk_core::{
    DestinationKind, DestinationRef, EffectId, EffectIntent, EffectKind, ExecutionEnvironment,
    IsolationCapability, IsolationClass, IsolationRequirement, NetworkIsolationPolicy,
    SecretExposurePolicy, SourceKind, SourceRef, WorkspaceMountMode, testing::FakeIsolationRuntime,
};
use agent_sdk_toolkit::{
    JsonRpcFrame, JsonRpcId, JsonRpcLineCodec, JsonRpcLineEndpoint,
    testing::{
        IsolatedJsonRpcProcess, McpHostProxy, ScriptedAcpAgent, ScriptedAcpClient,
        ScriptedMcpServer,
    },
};
use serde_json::json;

#[test]
fn acp_mock_prompt_cancel_and_denials_cross_json_rpc_lines() {
    let (client_endpoint, agent_endpoint) = JsonRpcLineEndpoint::pair("acp-client", "acp-agent");
    let mut client = ScriptedAcpClient::new(client_endpoint.clone());
    let mut agent = ScriptedAcpAgent::new("session.acp.fake")
        .with_prompt_output("review sdk", "review completed through ACP transport");

    client.initialize().expect("initialize request sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles init")
    );
    let init = client.response().expect("initialize response");
    assert_eq!(
        init.result
            .as_ref()
            .and_then(|value| value.get("protocolVersion"))
            .and_then(|value| value.as_i64()),
        Some(1)
    );

    client
        .new_session("/workspace/project")
        .expect("session/new request sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles session/new")
    );
    let new_session = client.response().expect("session/new response");
    let session_id = new_session
        .result
        .as_ref()
        .and_then(|value| value.get("sessionId"))
        .and_then(|value| value.as_str())
        .expect("session id")
        .to_string();
    assert_eq!(session_id, "session.acp.fake");

    client
        .prompt(&session_id, "review sdk")
        .expect("prompt request sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles prompt")
    );
    let update = client.notification().expect("session update notification");
    assert_eq!(update.method, "session/update");
    let accepted = client.response().expect("prompt completed");
    assert_eq!(
        accepted
            .result
            .as_ref()
            .and_then(|value| value.get("stopReason"))
            .and_then(|value| value.as_str()),
        Some("end_turn")
    );

    client
        .cancel(&session_id)
        .expect("session/cancel notification sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles cancel notification")
    );
    assert!(
        client
            .endpoint()
            .try_receive_frame()
            .expect("no cancel response check")
            .is_none(),
        "ACP session/cancel is a notification and must not receive a response"
    );
    assert!(agent.cancelled_sessions().contains(&session_id));

    agent
        .request_file_read(&agent_endpoint, &session_id, "/workspace/private.txt")
        .expect("agent fs/read_text_file request");
    assert!(
        client
            .handle_next(&client_endpoint)
            .expect("client handles file denial")
    );
    let file_denial = agent
        .response(&agent_endpoint)
        .expect("file denial response");
    assert_eq!(file_denial.error.expect("file denied").code, -32003);

    agent
        .request_terminal_create(&agent_endpoint, &session_id, "cargo")
        .expect("agent terminal/create request");
    assert!(
        client
            .handle_next(&client_endpoint)
            .expect("client handles terminal denial")
    );
    let terminal_denial = agent
        .response(&agent_endpoint)
        .expect("terminal denial response");
    assert_eq!(terminal_denial.error.expect("terminal denied").code, -32004);

    agent
        .request_permission(&agent_endpoint, &session_id, "tool.call.1")
        .expect("agent session/request_permission request");
    assert!(
        client
            .handle_next(&client_endpoint)
            .expect("client handles permission request")
    );
    let tool_response = agent
        .response(&agent_endpoint)
        .expect("tool approval response");
    assert_eq!(
        tool_response
            .result
            .as_ref()
            .and_then(|value| value.get("outcome"))
            .and_then(|value| value.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("cancelled")
    );

    client
        .endpoint()
        .send_raw_line("{not-json")
        .expect("malformed raw frame sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles parse error")
    );
    let parse_error = client.response().expect("parse error response");
    assert_eq!(parse_error.id, JsonRpcId::Null);
    assert_eq!(parse_error.error.expect("parse error").code, -32700);

    assert!(
        client_endpoint
            .sent_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"session/prompt\"")),
        "ACP prompt must cross encoded JSON-RPC lines"
    );
    assert!(
        agent_endpoint
            .received_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"session/cancel\"")),
        "ACP agent must receive encoded cancel notification"
    );
    assert!(
        client_endpoint
            .received_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"terminal/create\"")),
        "ACP client must receive encoded terminal/create request"
    );
    assert!(
        client_endpoint
            .received_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"session/request_permission\"")),
        "ACP client must receive encoded permission request"
    );

    client
        .prompt("session.acp.unknown", "review sdk")
        .expect("unknown-session prompt request sent");
    assert!(
        agent
            .handle_next(&agent_endpoint)
            .expect("agent handles unknown session")
    );
    let unknown_session = client.response().expect("unknown session response");
    assert_eq!(
        unknown_session
            .error
            .expect("unknown session rejected")
            .code,
        -32602
    );
}

#[test]
fn mcp_host_proxy_filters_capabilities_and_calls_allowed_tool_over_json_rpc() {
    let (proxy_endpoint, server_endpoint) =
        JsonRpcLineEndpoint::pair("mcp-host-proxy", "mcp-server");
    let mut proxy = McpHostProxy::new(proxy_endpoint.clone())
        .allow_tool("git.status")
        .allow_resource("docs://sdk")
        .allow_prompt("review");
    let mut server = ScriptedMcpServer::new()
        .tool("git.status", json!({"summary": "clean"}))
        .tool("git.diff", json!({"summary": "hidden sibling"}))
        .resource("docs://sdk", "SDK docs")
        .resource("docs://secret", "hidden resource")
        .prompt("review", json!({"description": "allowed prompt"}))
        .prompt("secret-review", json!({"description": "hidden prompt"}));

    proxy.initialize().expect("initialize request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles initialize")
    );
    assert!(
        proxy
            .response()
            .expect("initialize response")
            .error
            .is_none()
    );
    proxy.initialized().expect("initialized notification");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles initialized notification")
    );
    assert!(server.initialized());

    proxy.list_tools().expect("tools/list request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles list")
    );
    let list = proxy.response().expect("tools/list response");
    assert_eq!(
        list.result
            .as_ref()
            .and_then(|value| value.get("tools"))
            .and_then(|value| value.as_array())
            .expect("filtered tools")
            .len(),
        1,
        "host proxy response must not expose unselected sibling tools"
    );
    assert_eq!(
        proxy
            .allowed_tool_names_from_response(&list)
            .expect("filter allowed tools"),
        vec!["git.status".to_string()]
    );

    proxy.list_resources().expect("resources/list request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles resources/list")
    );
    let resources = proxy.response().expect("resources/list response");
    assert_eq!(
        resources
            .result
            .as_ref()
            .and_then(|value| value.get("resources"))
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("uri"))
            .and_then(|value| value.as_str()),
        Some("docs://sdk")
    );

    proxy.list_prompts().expect("prompts/list request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles prompts/list")
    );
    let prompts = proxy.response().expect("prompts/list response");
    assert_eq!(
        prompts
            .result
            .as_ref()
            .and_then(|value| value.get("prompts"))
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("name"))
            .and_then(|value| value.as_str()),
        Some("review")
    );

    proxy
        .call_tool("git.status", json!({"path": "."}))
        .expect("allowed tool request sent");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles allowed tool")
    );
    let call = proxy.response().expect("tool call response");
    assert!(call.error.is_none());
    assert!(
        call.result
            .as_ref()
            .and_then(|value| value.get("content"))
            .is_some()
    );

    let denied = proxy
        .call_tool("git.diff", json!({}))
        .expect_err("unselected sibling tool is denied before server call");
    assert_eq!(denied.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);
    assert!(
        !proxy_endpoint
            .sent_lines()
            .iter()
            .any(|line| line.contains("\"name\":\"git.diff\"")),
        "unselected MCP sibling tool must not be sent to the server"
    );

    proxy
        .read_resource("docs://sdk")
        .expect("allowed resource read request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles resource")
    );
    let resource = proxy.response().expect("resource read response");
    assert!(resource.error.is_none());

    proxy_endpoint
        .send_raw_line("{bad-mcp-json")
        .expect("malformed raw MCP frame");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles malformed frame")
    );
    let malformed = proxy.response().expect("malformed response");
    assert_eq!(malformed.error.expect("parse error").code, -32700);

    assert!(
        server_endpoint
            .received_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"tools/call\"")),
        "MCP allowed tool call must cross encoded JSON-RPC lines"
    );

    server
        .request_sampling(&server_endpoint)
        .expect("server sampling request");
    assert!(
        proxy
            .handle_next(&proxy_endpoint)
            .expect("proxy handles sampling denial")
    );
    let sampling_denial = server
        .response(&server_endpoint)
        .expect("sampling denial response");
    assert_eq!(sampling_denial.error.expect("sampling denied").code, -32010);

    server
        .request_elicitation(&server_endpoint)
        .expect("server elicitation request");
    assert!(
        proxy
            .handle_next(&proxy_endpoint)
            .expect("proxy handles elicitation denial")
    );
    let elicitation_denial = server
        .response(&server_endpoint)
        .expect("elicitation denial response");
    assert_eq!(
        elicitation_denial.error.expect("elicitation denied").code,
        -32010
    );

    server_endpoint
        .send_result(JsonRpcId::Number(999), json!({}))
        .expect("unexpected duplicate/unknown response sent");
    assert!(
        proxy.response().is_err(),
        "host proxy must reject responses that do not match an outstanding request"
    );
}

#[test]
fn mcp_server_rejects_operation_before_initialized_notification() {
    let (proxy_endpoint, server_endpoint) =
        JsonRpcLineEndpoint::pair("mcp-host-proxy", "mcp-server");
    let mut proxy = McpHostProxy::new(proxy_endpoint.clone()).allow_tool("git.status");
    let mut server = ScriptedMcpServer::new().tool("git.status", json!({"summary": "clean"}));

    proxy.initialize().expect("initialize request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server handles initialize")
    );
    assert!(
        proxy
            .response()
            .expect("initialize response")
            .error
            .is_none()
    );

    proxy.list_tools().expect("premature tools/list request");
    assert!(
        server
            .handle_next(&server_endpoint)
            .expect("server sends lifecycle error")
    );
    let error = proxy.response().expect("premature list response");
    assert_eq!(error.error.expect("lifecycle error").code, -32002);
    assert!(!server.initialized());
}

#[test]
fn mcp_inside_isolation_starts_runtime_before_json_rpc_exchange() {
    let runtime = FakeIsolationRuntime::with_report(
        agent_sdk_core::IsolationCapabilityReport::sandbox("runtime.fake.mcp"),
    );
    let environment = isolated_environment("env.protocol.mcp", "runtime.fake.mcp");
    let process = environment
        .process(["mcp-server", "--stdio"])
        .build()
        .expect("isolated MCP process spec");
    let isolated = IsolatedJsonRpcProcess::start(
        &runtime,
        environment.clone(),
        process,
        effect_intent(&environment, "effect.protocol.mcp.start"),
    )
    .expect("isolated MCP process started");
    let mut proxy = McpHostProxy::new(isolated.host_endpoint.clone()).allow_tool("project.index");
    let mut server = ScriptedMcpServer::new().tool("project.index", json!({"indexed": true}));

    proxy.initialize().expect("initialize request");
    assert!(
        server
            .handle_next(&isolated.process_endpoint)
            .expect("isolated server handles initialize")
    );
    assert!(
        proxy
            .response()
            .expect("initialize response")
            .error
            .is_none()
    );
    proxy.initialized().expect("initialized notification");
    assert!(
        server
            .handle_next(&isolated.process_endpoint)
            .expect("isolated server handles initialized notification")
    );
    proxy
        .call_tool("project.index", json!({"workspace": "workspace.primary"}))
        .expect("allowed isolated MCP tool call");
    assert!(
        server
            .handle_next(&isolated.process_endpoint)
            .expect("isolated server handles call")
    );
    assert!(proxy.response().expect("tool response").error.is_none());

    assert_eq!(
        runtime.calls(),
        vec!["capability_report".to_string(), "start_process".to_string()],
        "isolation runtime launch must happen before MCP protocol exchange"
    );
    assert!(
        isolated
            .host_endpoint
            .sent_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"tools/call\""))
    );
    assert_eq!(
        isolated
            .start_result
            .adapter_session_ref
            .expect("adapter session ref")
            .as_str(),
        "adapter.session.fake.isolation"
    );
}

#[test]
fn acp_external_agent_inside_isolation_uses_json_rpc_after_runtime_start() {
    let runtime = FakeIsolationRuntime::with_report(
        agent_sdk_core::IsolationCapabilityReport::sandbox("runtime.fake.acp"),
    );
    let environment = isolated_environment("env.protocol.acp", "runtime.fake.acp");
    let process = environment
        .process(["acp-agent", "--stdio"])
        .build()
        .expect("isolated ACP process spec");
    let isolated = IsolatedJsonRpcProcess::start(
        &runtime,
        environment.clone(),
        process,
        effect_intent(&environment, "effect.protocol.acp.start"),
    )
    .expect("isolated ACP process started");
    let mut client = ScriptedAcpClient::new(isolated.host_endpoint.clone());
    let mut agent = ScriptedAcpAgent::new("session.acp.isolated")
        .with_prompt_output("summarize", "summary via isolated ACP");

    client.initialize().expect("initialize request");
    assert!(
        agent
            .handle_next(&isolated.process_endpoint)
            .expect("isolated ACP initialize")
    );
    assert!(
        client
            .response()
            .expect("initialize response")
            .error
            .is_none()
    );
    client
        .new_session("/workspace/project")
        .expect("session/new request");
    assert!(
        agent
            .handle_next(&isolated.process_endpoint)
            .expect("isolated ACP session/new")
    );
    let session_id = client
        .response()
        .expect("session/new response")
        .result
        .as_ref()
        .and_then(|value| value.get("sessionId"))
        .and_then(|value| value.as_str())
        .expect("session id")
        .to_string();
    client
        .prompt(&session_id, "summarize")
        .expect("prompt request");
    assert!(
        agent
            .handle_next(&isolated.process_endpoint)
            .expect("isolated ACP prompt")
    );
    assert_eq!(
        client.notification().expect("prompt output").method,
        "session/update"
    );
    assert_eq!(
        client
            .response()
            .expect("prompt response")
            .result
            .as_ref()
            .and_then(|value| value.get("stopReason"))
            .and_then(|value| value.as_str()),
        Some("end_turn")
    );

    assert_eq!(
        runtime.calls(),
        vec!["capability_report".to_string(), "start_process".to_string()],
        "isolation runtime launch must happen before ACP protocol exchange"
    );
    assert!(
        isolated
            .process_endpoint
            .received_lines()
            .iter()
            .any(|line| line.contains("\"method\":\"session/prompt\""))
    );
}

#[test]
fn raw_json_rpc_frame_parser_rejects_non_json_direct_calls() {
    let error = JsonRpcFrame::from_line("not-json").expect_err("invalid JSON denied");
    assert_eq!(
        error.kind(),
        agent_sdk_core::AgentErrorKind::InvalidStateTransition
    );

    let (left, _right) = JsonRpcLineEndpoint::pair("left", "right");
    let newline = left
        .send_raw_line("{\"jsonrpc\":\"2.0\"}\n{\"jsonrpc\":\"2.0\"}")
        .expect_err("embedded newline denied");
    assert_eq!(
        newline.kind(),
        agent_sdk_core::AgentErrorKind::InvalidStateTransition
    );
}

#[test]
fn json_rpc_response_contract_rejects_invalid_response_shapes() {
    let parse_error = JsonRpcFrame::from_line(
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}"#,
    )
    .expect("parse-error response with null id is valid");
    let JsonRpcFrame::Response(response) = parse_error else {
        panic!("expected response frame");
    };
    assert_eq!(response.id, JsonRpcId::Null);

    assert!(
        JsonRpcFrame::from_line(
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"}}"#
        )
        .is_err(),
        "JSON-RPC responses must include id, using null when no request id is available"
    );
    assert!(
        JsonRpcFrame::from_line(
            r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-32603,"message":"bad"}}"#
        )
        .is_err(),
        "JSON-RPC responses cannot include both result and error"
    );
    assert!(
        JsonRpcFrame::from_line(r#"{"jsonrpc":"2.0","id":null,"method":"tools/list"}"#).is_err(),
        "JSON-RPC request ids must not be null"
    );
}

#[test]
fn json_rpc_line_codec_round_trips_stdio_delimited_bytes() {
    let frame = JsonRpcFrame::Request(agent_sdk_toolkit::JsonRpcRequest::new(
        1,
        "tools/list",
        json!({}),
    ));
    let mut bytes = Vec::new();

    JsonRpcLineCodec::write_frame(&mut bytes, &frame).expect("frame written");

    assert!(bytes.ends_with(b"\n"));
    assert_eq!(
        bytes.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "stdio transport must use one newline delimiter and no embedded newlines"
    );

    let mut reader = std::io::Cursor::new(bytes);
    assert_eq!(
        JsonRpcLineCodec::read_frame(&mut reader).expect("frame read"),
        Some(frame)
    );
    assert!(
        JsonRpcLineCodec::read_frame(&mut reader)
            .expect("eof read")
            .is_none()
    );
}

fn isolated_environment(environment_id: &str, runtime_ref: &str) -> ExecutionEnvironment {
    ExecutionEnvironment::require(
        IsolationRequirement::at_least(IsolationClass::Sandbox)
            .prefer(runtime_ref)
            .require_capabilities([
                IsolationCapability::NoNetworkGuarantee,
                IsolationCapability::ReadOnlyRoot,
                IsolationCapability::Cleanup,
                IsolationCapability::ProcessTimeout,
                IsolationCapability::IoRedaction,
            ]),
    )
    .environment_id(environment_id)
    .workspace("workspace.primary", WorkspaceMountMode::Snapshot)
    .network(NetworkIsolationPolicy::Disabled)
    .secrets(SecretExposurePolicy::no_ambient())
    .ephemeral()
    .source(SourceRef::with_kind(SourceKind::Sdk, "source.sdk.protocol"))
    .destination(DestinationRef::with_kind(
        DestinationKind::ExternalRuntime,
        "destination.protocol.runtime",
    ))
    .build()
    .expect("environment builds")
}

fn effect_intent(environment: &ExecutionEnvironment, effect_id: &str) -> EffectIntent {
    let mut intent = EffectIntent::new(
        EffectId::new(effect_id),
        EffectKind::IsolatedProcessStart,
        environment.subject_ref(),
        environment.source.clone(),
        "start protocol mock inside isolated process",
    );
    intent.destination = Some(environment.destination.clone());
    intent
}
