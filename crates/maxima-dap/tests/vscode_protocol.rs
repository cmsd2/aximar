//! Tests verifying that the exact JSON messages VS Code sends are correctly
//! deserialized through the `emmy_dap_types` types used by `maxima-dap`.
//!
//! These tests catch regressions in our type assumptions and ensure the
//! serde attributes on our custom types (e.g. `MaximaLaunchArguments`)
//! match VS Code's camelCase conventions.

use emmy_dap_types::base_message::{BaseMessage, Sendable};
use emmy_dap_types::events::{Event, OutputEventBody, StoppedEventBody};
use emmy_dap_types::requests::{Command, Request};
use emmy_dap_types::responses::{
    ContinueResponse, EvaluateResponse, Response, ResponseBody, ResponseMessage,
    ScopesResponse, SetBreakpointsResponse, StackTraceResponse, ThreadsResponse,
    VariablesResponse,
};
use emmy_dap_types::types::{
    Breakpoint, Capabilities, OutputEventCategory, Scope, ScopePresentationhint, Source,
    StackFrame, StoppedEventReason, Thread, Variable,
};

use maxima_dap::types::MaximaLaunchArguments;

// ---------------------------------------------------------------------------
// Helper: parse raw JSON as a DAP Request (same code path as server.rs)
// ---------------------------------------------------------------------------

fn parse_request(json: &str) -> Request {
    let raw: serde_json::Value = serde_json::from_str(json).expect("invalid JSON");
    serde_json::from_value(raw).expect("failed to parse as DAP Request")
}

// ===================================================================
// 1. Request deserialization — exact JSON VS Code sends
// ===================================================================

#[test]
fn deserialize_initialize_request() {
    let json = r#"{
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {
            "clientID": "vscode",
            "clientName": "Visual Studio Code",
            "adapterID": "maxima",
            "pathFormat": "path",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsVariablePaging": true,
            "supportsRunInTerminalRequest": true,
            "locale": "en-us",
            "supportsProgressReporting": true,
            "supportsInvalidatedEvent": true,
            "supportsMemoryReferences": true,
            "supportsArgsCanBeInterpretedByShell": true,
            "supportsMemoryEvent": true,
            "supportsStartDebuggingRequest": true
        }
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 1);

    match &request.command {
        Command::Initialize(args) => {
            assert_eq!(args.client_id.as_deref(), Some("vscode"));
            assert_eq!(
                args.client_name.as_deref(),
                Some("Visual Studio Code")
            );
            assert_eq!(args.adapter_id, "maxima");
            assert_eq!(args.lines_start_at1, Some(true));
            assert_eq!(args.columns_start_at1, Some(true));
        }
        other => panic!("expected Initialize, got {:?}", other),
    }
}

#[test]
fn deserialize_initialize_request_with_lines_start_at_0() {
    // Some clients may send linesStartAt1: false
    let json = r#"{
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {
            "adapterID": "maxima",
            "linesStartAt1": false,
            "columnsStartAt1": false
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Initialize(args) => {
            assert_eq!(args.lines_start_at1, Some(false));
            assert_eq!(args.columns_start_at1, Some(false));
        }
        other => panic!("expected Initialize, got {:?}", other),
    }
}

#[test]
fn deserialize_launch_request_full() {
    // VS Code sends launch with our custom fields flattened into additional_data
    let json = r#"{
        "seq": 2,
        "type": "request",
        "command": "launch",
        "arguments": {
            "__restart": false,
            "program": "/home/user/project/main.mac",
            "evaluate": "main()",
            "stopOnEntry": false,
            "maximaPath": "/usr/local/bin/maxima",
            "noDebug": false
        }
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 2);

    match &request.command {
        Command::Launch(args) => {
            assert_eq!(args.no_debug, Some(false));

            // Extract our custom arguments from additional_data
            let data = args.additional_data.as_ref().expect("missing additional_data");
            let launch_args: MaximaLaunchArguments =
                serde_json::from_value(data.clone()).expect("failed to parse MaximaLaunchArguments");

            assert_eq!(launch_args.program, "/home/user/project/main.mac");
            assert_eq!(launch_args.evaluate.as_deref(), Some("main()"));
            assert_eq!(launch_args.stop_on_entry, false);
            assert_eq!(
                launch_args.maxima_path.as_deref(),
                Some("/usr/local/bin/maxima")
            );
            // Default backend
            assert_eq!(launch_args.backend, "local");
        }
        other => panic!("expected Launch, got {:?}", other),
    }
}

#[test]
fn deserialize_launch_request_minimal() {
    // Minimal launch: only the required "program" field
    let json = r#"{
        "seq": 2,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": "/tmp/test.mac"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Launch(args) => {
            let data = args.additional_data.as_ref().expect("missing additional_data");
            let launch_args: MaximaLaunchArguments =
                serde_json::from_value(data.clone()).expect("failed to parse MaximaLaunchArguments");

            assert_eq!(launch_args.program, "/tmp/test.mac");
            assert_eq!(launch_args.evaluate, None);
            assert_eq!(launch_args.stop_on_entry, false);
            assert_eq!(launch_args.maxima_path, None);
            assert_eq!(launch_args.backend, "local");
            assert_eq!(launch_args.cwd, None);
        }
        other => panic!("expected Launch, got {:?}", other),
    }
}

#[test]
fn deserialize_launch_request_with_cwd() {
    let json = r#"{
        "seq": 2,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": "main.mac",
            "cwd": "/home/user/project",
            "stopOnEntry": true
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Launch(args) => {
            let data = args.additional_data.as_ref().expect("missing additional_data");
            let launch_args: MaximaLaunchArguments =
                serde_json::from_value(data.clone()).expect("failed to parse MaximaLaunchArguments");

            assert_eq!(launch_args.program, "main.mac");
            assert_eq!(launch_args.cwd.as_deref(), Some("/home/user/project"));
            assert_eq!(launch_args.stop_on_entry, true);
        }
        other => panic!("expected Launch, got {:?}", other),
    }
}

#[test]
fn deserialize_set_breakpoints_request() {
    // VS Code sends setBreakpoints with source and breakpoint locations
    let json = r#"{
        "seq": 3,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {
                "name": "main.mac",
                "path": "/home/user/project/main.mac"
            },
            "breakpoints": [
                { "line": 5 },
                { "line": 12, "condition": "x > 0" },
                { "line": 20, "hitCondition": "3" }
            ],
            "lines": [5, 12, 20],
            "sourceModified": false
        }
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 3);

    match &request.command {
        Command::SetBreakpoints(args) => {
            assert_eq!(args.source.name.as_deref(), Some("main.mac"));
            assert_eq!(
                args.source.path.as_deref(),
                Some("/home/user/project/main.mac")
            );
            let bps = args.breakpoints.as_ref().expect("missing breakpoints");
            assert_eq!(bps.len(), 3);
            assert_eq!(bps[0].line, 5);
            assert_eq!(bps[1].line, 12);
            assert_eq!(bps[1].condition.as_deref(), Some("x > 0"));
            assert_eq!(bps[2].line, 20);
        }
        other => panic!("expected SetBreakpoints, got {:?}", other),
    }
}

#[test]
fn deserialize_set_breakpoints_clear_all() {
    // VS Code sends empty breakpoints array to clear all breakpoints for a file
    let json = r#"{
        "seq": 4,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {
                "name": "main.mac",
                "path": "/home/user/project/main.mac"
            },
            "breakpoints": [],
            "lines": [],
            "sourceModified": false
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::SetBreakpoints(args) => {
            let bps = args.breakpoints.as_ref().expect("missing breakpoints");
            assert!(bps.is_empty());
        }
        other => panic!("expected SetBreakpoints, got {:?}", other),
    }
}

#[test]
fn deserialize_configuration_done_request() {
    let json = r#"{
        "seq": 5,
        "type": "request",
        "command": "configurationDone"
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 5);
    assert!(matches!(request.command, Command::ConfigurationDone));
}

#[test]
fn deserialize_threads_request() {
    let json = r#"{
        "seq": 6,
        "type": "request",
        "command": "threads"
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 6);
    assert!(matches!(request.command, Command::Threads));
}

#[test]
fn deserialize_continue_request() {
    let json = r#"{
        "seq": 7,
        "type": "request",
        "command": "continue",
        "arguments": {
            "threadId": 1,
            "singleThread": false
        }
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 7);

    match &request.command {
        Command::Continue(args) => {
            assert_eq!(args.thread_id, 1);
            assert_eq!(args.single_thread, Some(false));
        }
        other => panic!("expected Continue, got {:?}", other),
    }
}

#[test]
fn deserialize_next_request() {
    let json = r#"{
        "seq": 8,
        "type": "request",
        "command": "next",
        "arguments": {
            "threadId": 1,
            "singleThread": true,
            "granularity": "statement"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Next(args) => {
            assert_eq!(args.thread_id, 1);
            assert_eq!(args.single_thread, Some(true));
        }
        other => panic!("expected Next, got {:?}", other),
    }
}

#[test]
fn deserialize_step_in_request() {
    let json = r#"{
        "seq": 9,
        "type": "request",
        "command": "stepIn",
        "arguments": {
            "threadId": 1,
            "singleThread": true,
            "granularity": "statement"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::StepIn(args) => {
            assert_eq!(args.thread_id, 1);
        }
        other => panic!("expected StepIn, got {:?}", other),
    }
}

#[test]
fn deserialize_stack_trace_request() {
    let json = r#"{
        "seq": 10,
        "type": "request",
        "command": "stackTrace",
        "arguments": {
            "threadId": 1,
            "startFrame": 0,
            "levels": 20
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::StackTrace(args) => {
            assert_eq!(args.thread_id, 1);
            assert_eq!(args.start_frame, Some(0));
            assert_eq!(args.levels, Some(20));
        }
        other => panic!("expected StackTrace, got {:?}", other),
    }
}

#[test]
fn deserialize_scopes_request() {
    let json = r#"{
        "seq": 11,
        "type": "request",
        "command": "scopes",
        "arguments": {
            "frameId": 0
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Scopes(args) => {
            assert_eq!(args.frame_id, 0);
        }
        other => panic!("expected Scopes, got {:?}", other),
    }
}

#[test]
fn deserialize_variables_request() {
    let json = r#"{
        "seq": 12,
        "type": "request",
        "command": "variables",
        "arguments": {
            "variablesReference": 1
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Variables(args) => {
            assert_eq!(args.variables_reference, 1);
        }
        other => panic!("expected Variables, got {:?}", other),
    }
}

#[test]
fn deserialize_variables_request_with_filter() {
    let json = r#"{
        "seq": 12,
        "type": "request",
        "command": "variables",
        "arguments": {
            "variablesReference": 5,
            "filter": "indexed",
            "start": 0,
            "count": 10
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Variables(args) => {
            assert_eq!(args.variables_reference, 5);
            assert_eq!(args.start, Some(0));
            assert_eq!(args.count, Some(10));
        }
        other => panic!("expected Variables, got {:?}", other),
    }
}

#[test]
fn deserialize_evaluate_request_watch() {
    let json = r#"{
        "seq": 13,
        "type": "request",
        "command": "evaluate",
        "arguments": {
            "expression": "x + y",
            "frameId": 0,
            "context": "watch"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Evaluate(args) => {
            assert_eq!(args.expression, "x + y");
            assert_eq!(args.frame_id, Some(0));
        }
        other => panic!("expected Evaluate, got {:?}", other),
    }
}

#[test]
fn deserialize_evaluate_request_hover() {
    let json = r#"{
        "seq": 14,
        "type": "request",
        "command": "evaluate",
        "arguments": {
            "expression": "result",
            "frameId": 1,
            "context": "hover"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Evaluate(args) => {
            assert_eq!(args.expression, "result");
            assert_eq!(args.frame_id, Some(1));
        }
        other => panic!("expected Evaluate, got {:?}", other),
    }
}

#[test]
fn deserialize_evaluate_request_repl() {
    // Debug console input
    let json = r#"{
        "seq": 15,
        "type": "request",
        "command": "evaluate",
        "arguments": {
            "expression": "integrate(x^2, x, 0, 1)",
            "context": "repl"
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::Evaluate(args) => {
            assert_eq!(args.expression, "integrate(x^2, x, 0, 1)");
            // No frame ID in REPL context
            assert_eq!(args.frame_id, None);
        }
        other => panic!("expected Evaluate, got {:?}", other),
    }
}

#[test]
fn deserialize_disconnect_request() {
    let json = r#"{
        "seq": 20,
        "type": "request",
        "command": "disconnect",
        "arguments": {
            "restart": false,
            "terminateDebuggee": true
        }
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 20);
    assert!(matches!(request.command, Command::Disconnect(_)));
}

#[test]
fn deserialize_disconnect_request_minimal() {
    // VS Code sometimes sends disconnect with empty or no arguments
    let json = r#"{
        "seq": 21,
        "type": "request",
        "command": "disconnect",
        "arguments": {}
    }"#;

    let request = parse_request(json);
    assert!(matches!(request.command, Command::Disconnect(_)));
}

// ===================================================================
// 2. MaximaLaunchArguments — camelCase field mapping
// ===================================================================

#[test]
fn launch_args_camel_case_mapping() {
    // Verify the serde(rename_all = "camelCase") works correctly for all fields
    let json = r#"{
        "program": "/test/main.mac",
        "maximaPath": "/opt/maxima/bin/maxima",
        "stopOnEntry": true,
        "evaluate": "test()",
        "cwd": "/test",
        "backend": "local"
    }"#;

    let args: MaximaLaunchArguments =
        serde_json::from_str(json).expect("failed to parse MaximaLaunchArguments");

    assert_eq!(args.program, "/test/main.mac");
    assert_eq!(args.maxima_path.as_deref(), Some("/opt/maxima/bin/maxima"));
    assert_eq!(args.stop_on_entry, true);
    assert_eq!(args.evaluate.as_deref(), Some("test()"));
    assert_eq!(args.cwd.as_deref(), Some("/test"));
    assert_eq!(args.backend, "local");
}

#[test]
fn launch_args_unknown_fields_ignored() {
    // VS Code may include fields we don't know about (e.g. __restart, noDebug)
    // These end up in additional_data, and our type should ignore them gracefully
    let json = r#"{
        "program": "/test/main.mac",
        "__restart": false,
        "noDebug": false,
        "__sessionId": "abc-123",
        "unknownFutureField": 42
    }"#;

    let args: MaximaLaunchArguments =
        serde_json::from_str(json).expect("unknown fields should be ignored");
    assert_eq!(args.program, "/test/main.mac");
}

#[test]
fn launch_args_windows_path() {
    let json = r#"{
        "program": "C:\\Users\\dev\\project\\main.mac",
        "maximaPath": "C:\\maxima-5.47.0\\bin\\maxima.bat"
    }"#;

    let args: MaximaLaunchArguments =
        serde_json::from_str(json).expect("Windows paths should work");
    assert_eq!(args.program, r"C:\Users\dev\project\main.mac");
    assert_eq!(
        args.maxima_path.as_deref(),
        Some(r"C:\maxima-5.47.0\bin\maxima.bat")
    );
}

// ===================================================================
// 3. Response serialization — our responses must be valid DAP JSON
// ===================================================================

#[test]
fn serialize_initialize_response() {
    let msg = BaseMessage {
        seq: 1,
        message: Sendable::Response(Response {
            request_seq: 1,
            success: true,
            message: None,
            body: Some(ResponseBody::Initialize(Capabilities {
                supports_configuration_done_request: Some(true),
                supports_evaluate_for_hovers: Some(true),
                supports_function_breakpoints: Some(false),
                supports_conditional_breakpoints: Some(false),
                supports_step_back: Some(false),
                ..Default::default()
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");

    assert_eq!(json["type"], "response");
    assert_eq!(json["seq"], 1);
    assert_eq!(json["request_seq"], 1);
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "initialize");

    // Verify capabilities are present in body
    let body = &json["body"];
    assert_eq!(body["supportsConfigurationDoneRequest"], true);
    assert_eq!(body["supportsEvaluateForHovers"], true);
    assert_eq!(body["supportsFunctionBreakpoints"], false);
}

#[test]
fn serialize_launch_response() {
    let msg = BaseMessage {
        seq: 2,
        message: Sendable::Response(Response {
            request_seq: 2,
            success: true,
            message: None,
            body: Some(ResponseBody::Launch),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "response");
    assert_eq!(json["command"], "launch");
    assert_eq!(json["success"], true);
}

#[test]
fn serialize_set_breakpoints_response() {
    let msg = BaseMessage {
        seq: 3,
        message: Sendable::Response(Response {
            request_seq: 3,
            success: true,
            message: None,
            body: Some(ResponseBody::SetBreakpoints(SetBreakpointsResponse {
                breakpoints: vec![
                    Breakpoint {
                        id: Some(1),
                        verified: true,
                        source: Some(Source {
                            path: Some("/test/main.mac".to_string()),
                            ..Default::default()
                        }),
                        line: Some(5),
                        message: None,
                        ..Default::default()
                    },
                    Breakpoint {
                        id: Some(2),
                        verified: false,
                        line: Some(3),
                        message: Some(
                            "Line 3 is not inside a function definition.".to_string(),
                        ),
                        ..Default::default()
                    },
                ],
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "setBreakpoints");
    let bps = json["body"]["breakpoints"].as_array().expect("missing breakpoints");
    assert_eq!(bps.len(), 2);
    assert_eq!(bps[0]["id"], 1);
    assert_eq!(bps[0]["verified"], true);
    assert_eq!(bps[0]["line"], 5);
    assert_eq!(bps[1]["id"], 2);
    assert_eq!(bps[1]["verified"], false);
    assert!(bps[1]["message"].as_str().unwrap().contains("not inside"));
}

#[test]
fn serialize_threads_response() {
    let msg = BaseMessage {
        seq: 4,
        message: Sendable::Response(Response {
            request_seq: 6,
            success: true,
            message: None,
            body: Some(ResponseBody::Threads(ThreadsResponse {
                threads: vec![Thread {
                    id: 1,
                    name: "Maxima".to_string(),
                }],
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "threads");
    let threads = json["body"]["threads"].as_array().expect("missing threads");
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["id"], 1);
    assert_eq!(threads[0]["name"], "Maxima");
}

#[test]
fn serialize_stack_trace_response() {
    let msg = BaseMessage {
        seq: 5,
        message: Sendable::Response(Response {
            request_seq: 10,
            success: true,
            message: None,
            body: Some(ResponseBody::StackTrace(StackTraceResponse {
                stack_frames: vec![
                    StackFrame {
                        id: 0,
                        name: "foo".to_string(),
                        source: Some(Source {
                            name: Some("main.mac".to_string()),
                            path: Some("/test/main.mac".to_string()),
                            ..Default::default()
                        }),
                        line: 7,
                        column: 1,
                        end_line: None,
                        end_column: None,
                        can_restart: None,
                        instruction_pointer_reference: None,
                        module_id: None,
                        presentation_hint: None,
                    },
                ],
                total_frames: Some(1),
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "stackTrace");
    let frames = json["body"]["stackFrames"].as_array().expect("missing stackFrames");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0]["id"], 0);
    assert_eq!(frames[0]["name"], "foo");
    assert_eq!(frames[0]["line"], 7);
    assert_eq!(frames[0]["column"], 1);
    assert_eq!(json["body"]["totalFrames"], 1);
}

#[test]
fn serialize_scopes_response() {
    let msg = BaseMessage {
        seq: 6,
        message: Sendable::Response(Response {
            request_seq: 11,
            success: true,
            message: None,
            body: Some(ResponseBody::Scopes(ScopesResponse {
                scopes: vec![Scope {
                    name: "Locals".to_string(),
                    presentation_hint: Some(ScopePresentationhint::Locals),
                    variables_reference: 1,
                    named_variables: None,
                    indexed_variables: None,
                    expensive: false,
                    source: None,
                    line: None,
                    column: None,
                    end_line: None,
                    end_column: None,
                }],
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "scopes");
    let scopes = json["body"]["scopes"].as_array().expect("missing scopes");
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0]["name"], "Locals");
    assert_eq!(scopes[0]["presentationHint"], "locals");
    assert_eq!(scopes[0]["variablesReference"], 1);
    assert_eq!(scopes[0]["expensive"], false);
}

#[test]
fn serialize_variables_response() {
    let msg = BaseMessage {
        seq: 7,
        message: Sendable::Response(Response {
            request_seq: 12,
            success: true,
            message: None,
            body: Some(ResponseBody::Variables(VariablesResponse {
                variables: vec![
                    Variable {
                        name: "x".to_string(),
                        value: "5".to_string(),
                        type_field: None,
                        presentation_hint: None,
                        evaluate_name: None,
                        variables_reference: 0,
                        named_variables: None,
                        indexed_variables: None,
                        memory_reference: None,
                    },
                    Variable {
                        name: "result".to_string(),
                        value: "[1, 2, 3]".to_string(),
                        type_field: None,
                        presentation_hint: None,
                        evaluate_name: Some("result".to_string()),
                        variables_reference: 2,
                        named_variables: None,
                        indexed_variables: Some(3),
                        memory_reference: None,
                    },
                ],
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "variables");
    let vars = json["body"]["variables"].as_array().expect("missing variables");
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0]["name"], "x");
    assert_eq!(vars[0]["value"], "5");
    assert_eq!(vars[0]["variablesReference"], 0);
    assert_eq!(vars[1]["name"], "result");
    assert_eq!(vars[1]["variablesReference"], 2);
    assert_eq!(vars[1]["indexedVariables"], 3);
}

#[test]
fn serialize_continue_response() {
    let msg = BaseMessage {
        seq: 8,
        message: Sendable::Response(Response {
            request_seq: 7,
            success: true,
            message: None,
            body: Some(ResponseBody::Continue(ContinueResponse {
                all_threads_continued: Some(true),
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "continue");
    assert_eq!(json["body"]["allThreadsContinued"], true);
}

#[test]
fn serialize_evaluate_response() {
    let msg = BaseMessage {
        seq: 9,
        message: Sendable::Response(Response {
            request_seq: 13,
            success: true,
            message: None,
            body: Some(ResponseBody::Evaluate(EvaluateResponse {
                result: "1/3".to_string(),
                type_field: None,
                presentation_hint: None,
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            })),
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["command"], "evaluate");
    assert_eq!(json["body"]["result"], "1/3");
    assert_eq!(json["body"]["variablesReference"], 0);
}

#[test]
fn serialize_error_response() {
    let msg = BaseMessage {
        seq: 10,
        message: Sendable::Response(Response {
            request_seq: 2,
            success: false,
            message: Some(ResponseMessage::Error("program not found: /bad/path.mac".to_string())),
            body: None,
            error: None,
        }),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "response");
    assert_eq!(json["success"], false);
    assert_eq!(json["message"], "program not found: /bad/path.mac");
}

// ===================================================================
// 4. Event serialization — stopped, output, terminated, initialized
// ===================================================================

#[test]
fn serialize_stopped_event() {
    let msg = BaseMessage {
        seq: 1,
        message: Sendable::Event(Event::Stopped(StoppedEventBody {
            reason: StoppedEventReason::Breakpoint,
            description: None,
            thread_id: Some(1),
            preserve_focus_hint: None,
            text: None,
            all_threads_stopped: Some(true),
            hit_breakpoint_ids: None,
        })),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "event");
    assert_eq!(json["event"], "stopped");
    assert_eq!(json["body"]["reason"], "breakpoint");
    assert_eq!(json["body"]["threadId"], 1);
    assert_eq!(json["body"]["allThreadsStopped"], true);
}

#[test]
fn serialize_stopped_event_step() {
    let msg = BaseMessage {
        seq: 2,
        message: Sendable::Event(Event::Stopped(StoppedEventBody {
            reason: StoppedEventReason::Step,
            description: None,
            thread_id: Some(1),
            preserve_focus_hint: None,
            text: None,
            all_threads_stopped: Some(true),
            hit_breakpoint_ids: None,
        })),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["body"]["reason"], "step");
}

#[test]
fn serialize_output_event() {
    let msg = BaseMessage {
        seq: 3,
        message: Sendable::Event(Event::Output(OutputEventBody {
            category: Some(OutputEventCategory::Stdout),
            output: "x = 42\n".to_string(),
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        })),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "event");
    assert_eq!(json["event"], "output");
    assert_eq!(json["body"]["category"], "stdout");
    assert_eq!(json["body"]["output"], "x = 42\n");
}

#[test]
fn serialize_output_event_stderr() {
    let msg = BaseMessage {
        seq: 4,
        message: Sendable::Event(Event::Output(OutputEventBody {
            category: Some(OutputEventCategory::Stderr),
            output: "Warning: SBCL required\n".to_string(),
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        })),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["body"]["category"], "stderr");
}

#[test]
fn serialize_initialized_event() {
    let msg = BaseMessage {
        seq: 5,
        message: Sendable::Event(Event::Initialized),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "event");
    assert_eq!(json["event"], "initialized");
}

#[test]
fn serialize_terminated_event() {
    let msg = BaseMessage {
        seq: 6,
        message: Sendable::Event(Event::Terminated(None)),
    };

    let json = serde_json::to_value(&msg).expect("failed to serialize");
    assert_eq!(json["type"], "event");
    assert_eq!(json["event"], "terminated");
}

// ===================================================================
// 5. Transport framing format
// ===================================================================

#[test]
fn content_length_framing_format() {
    // Verify that our serialized messages produce valid Content-Length framing
    let msg = BaseMessage {
        seq: 1,
        message: Sendable::Response(Response {
            request_seq: 1,
            success: true,
            message: None,
            body: Some(ResponseBody::Launch),
            error: None,
        }),
    };

    let body = serde_json::to_string(&msg).expect("failed to serialize");
    let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    // Verify the frame is well-formed
    assert!(framed.starts_with("Content-Length: "));
    assert!(framed.contains("\r\n\r\n"));

    // Extract content length from header
    let header_end = framed.find("\r\n\r\n").unwrap();
    let header = &framed[..header_end];
    let length: usize = header
        .strip_prefix("Content-Length: ")
        .unwrap()
        .parse()
        .unwrap();
    let body_str = &framed[header_end + 4..];
    assert_eq!(body_str.len(), length);

    // Verify the body is valid JSON that round-trips
    let parsed: serde_json::Value = serde_json::from_str(body_str).expect("invalid JSON body");
    assert_eq!(parsed["type"], "response");
}

#[test]
fn content_length_handles_unicode() {
    // Content-Length is in bytes, not characters — important for non-ASCII output
    let msg = BaseMessage {
        seq: 1,
        message: Sendable::Event(Event::Output(OutputEventBody {
            category: Some(OutputEventCategory::Stdout),
            output: "π = 3.14159…\n".to_string(),
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        })),
    };

    let body = serde_json::to_string(&msg).expect("failed to serialize");
    // The body string contains Unicode escapes or raw UTF-8
    let byte_len = body.len();
    let _char_len = body.chars().count();

    // For this particular string the byte length should differ from char count
    // due to multi-byte UTF-8 characters (π, …)
    // The Content-Length header must use byte length
    let framed = format!("Content-Length: {}\r\n\r\n{}", byte_len, body);
    let header_end = framed.find("\r\n\r\n").unwrap();
    let extracted_body = &framed[header_end + 4..];
    assert_eq!(extracted_body.len(), byte_len);

    // Verify the JSON is still valid
    let _: serde_json::Value = serde_json::from_str(extracted_body).expect("invalid JSON body");
}

// ===================================================================
// 6. Round-trip: request → deserialize → build response → serialize
// ===================================================================

#[test]
fn round_trip_initialize() {
    // Simulate the full flow: VS Code sends initialize, we build and serialize a response
    let request_json = r#"{
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {
            "adapterID": "maxima",
            "linesStartAt1": true,
            "columnsStartAt1": true
        }
    }"#;

    let request = parse_request(request_json);

    // Build response (same logic as handle_initialize)
    let response = BaseMessage {
        seq: 1,
        message: Sendable::Response(Response {
            request_seq: request.seq,
            success: true,
            message: None,
            body: Some(ResponseBody::Initialize(Capabilities {
                supports_configuration_done_request: Some(true),
                supports_evaluate_for_hovers: Some(true),
                ..Default::default()
            })),
            error: None,
        }),
    };

    // Serialize and verify VS Code can understand it
    let response_json = serde_json::to_value(&response).expect("failed to serialize response");
    assert_eq!(response_json["type"], "response");
    assert_eq!(response_json["request_seq"], 1);
    assert_eq!(response_json["success"], true);
    assert_eq!(response_json["command"], "initialize");
    assert!(response_json["body"]["supportsConfigurationDoneRequest"]
        .as_bool()
        .unwrap_or(false));
}

#[test]
fn round_trip_launch_with_evaluate() {
    let request_json = r#"{
        "seq": 3,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": "/home/user/test.mac",
            "evaluate": "run_tests()",
            "stopOnEntry": true
        }
    }"#;

    let request = parse_request(request_json);

    match &request.command {
        Command::Launch(args) => {
            let data = args.additional_data.as_ref().unwrap();
            let launch_args: MaximaLaunchArguments =
                serde_json::from_value(data.clone()).unwrap();

            assert_eq!(launch_args.program, "/home/user/test.mac");
            assert_eq!(launch_args.evaluate.as_deref(), Some("run_tests()"));
            assert_eq!(launch_args.stop_on_entry, true);

            // Build response
            let response = BaseMessage {
                seq: 2,
                message: Sendable::Response(Response {
                    request_seq: request.seq,
                    success: true,
                    message: None,
                    body: Some(ResponseBody::Launch),
                    error: None,
                }),
            };

            let json = serde_json::to_value(&response).unwrap();
            assert_eq!(json["command"], "launch");
            assert_eq!(json["request_seq"], 3);
        }
        other => panic!("expected Launch, got {:?}", other),
    }
}

// ===================================================================
// 7. Edge cases and error handling
// ===================================================================

#[test]
fn unknown_command_does_not_panic() {
    // VS Code may send commands we don't handle (e.g. "source", "loadedSources")
    // The raw JSON should still parse, and we match on _ in the server
    let json = r#"{
        "seq": 99,
        "type": "request",
        "command": "source",
        "arguments": {
            "sourceReference": 1
        }
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).expect("invalid JSON");
    // This should parse as a Request even though source isn't in our match arms
    let request: Result<Request, _> = serde_json::from_value(raw);
    // emmy_dap_types may parse it or fail — either is fine as long as it doesn't panic
    // The server.rs code pre-checks the command string before parsing
    let _ = request;
}

#[test]
fn large_seq_numbers() {
    // VS Code seq numbers can get large in long debug sessions
    let json = r#"{
        "seq": 999999,
        "type": "request",
        "command": "threads"
    }"#;

    let request = parse_request(json);
    assert_eq!(request.seq, 999999);
}

#[test]
fn set_breakpoints_source_without_name() {
    // VS Code may omit the source name and only send path
    let json = r#"{
        "seq": 10,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {
                "path": "/home/user/project/main.mac"
            },
            "breakpoints": [
                { "line": 10 }
            ]
        }
    }"#;

    let request = parse_request(json);
    match &request.command {
        Command::SetBreakpoints(args) => {
            assert_eq!(args.source.name, None);
            assert_eq!(
                args.source.path.as_deref(),
                Some("/home/user/project/main.mac")
            );
        }
        other => panic!("expected SetBreakpoints, got {:?}", other),
    }
}
