use toolhub_ipc::*;

#[test]
fn os_identity_is_stable_and_ignores_claimed_admin() {
    let first = current_principal().unwrap();
    std::env::set_var("TOOLHUB_PRINCIPAL", "local.admin");
    std::env::set_var("TOOLHUB_ADMIN", "1");
    assert_eq!(first, current_principal().unwrap());
    assert_ne!(first, "local.admin");
}

#[test]
fn controller_is_pinned_to_path_and_content() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("desktop.exe");
    std::fs::write(&image, b"controller").unwrap();
    let pin = ControllerImage::pin(&image).unwrap();
    let peer = PeerIdentity {
        principal: current_principal().unwrap(),
        image: image.clone(),
    };
    assert!(pin.verify(&peer).unwrap());
    let ordinary = PeerIdentity {
        principal: peer.principal.clone(),
        image: std::env::current_exe().unwrap(),
    };
    assert!(!pin.verify(&ordinary).unwrap());
    std::fs::write(&image, b"replacement").unwrap();
    assert!(!pin.verify(&peer).unwrap());
}

#[test]
fn malformed_frame_keeps_next_frame_boundary() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.push(b'{');
    write_frame(
        &mut bytes,
        &toolhub_protocol::JsonRpcRequest::new(9, "ping", serde_json::json!({})),
    )
    .unwrap();
    let mut reader = std::io::Cursor::new(bytes);
    assert!(toolhub_protocol::parse_request(&read_frame(&mut reader).unwrap()).is_err());
    assert_eq!(
        read_request(&mut reader).unwrap().id,
        Some(serde_json::json!(9))
    );
}

#[test]
fn oversized_response_becomes_structured_bounded_error() {
    let response=toolhub_protocol::JsonRpcResponse{jsonrpc:"2.0".into(),id:Some(serde_json::json!(3)),result:Some(serde_json::json!({"data":"x".repeat(toolhub_protocol::limits::MAX_REQUEST_BYTES)})),error:None};
    let bytes=serialize_response(&response).unwrap();
    let bounded:toolhub_protocol::JsonRpcResponse=serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bounded.id,Some(serde_json::json!(3)));
    assert_eq!(bounded.error.unwrap().data.unwrap()["error_code"],"payload_too_large");
}
