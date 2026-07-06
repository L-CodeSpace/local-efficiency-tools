/*
 * 核心职责：实现 macOS helper 的 Unix socket 协议读写。
 * 业务痛点：本地 IPC 必须有消息长度限制，避免异常输入拖垮 root daemon。
 * 能力边界：只处理 socket 连接和 JSON 消息传输。
 */

use super::*;

pub(super) fn send_request(
    request: &MacosHostsHelperRequest,
) -> AppResult<MacosHostsHelperResponse> {
    let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|error| {
        AppError::new("hosts_helper_unavailable", "无法连接 macOS hosts helper")
            .with_detail(error.to_string())
    })?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let request_bytes = serde_json::to_vec(request).map_err(|error| {
        AppError::new("hosts_helper_request_failed", "序列化 helper 请求失败")
            .with_detail(error.to_string())
    })?;
    write_message(&mut stream, &request_bytes).map_err(|error| {
        AppError::new("hosts_helper_request_failed", "发送 helper 请求失败")
            .with_detail(error.to_string())
    })?;
    let response_bytes = read_message(&mut stream).map_err(|error| {
        AppError::new("hosts_helper_response_failed", "读取 helper 响应失败")
            .with_detail(error.to_string())
    })?;
    serde_json::from_slice::<MacosHostsHelperResponse>(&response_bytes).map_err(|error| {
        AppError::new("hosts_helper_response_failed", "解析 helper 响应失败")
            .with_detail(error.to_string())
    })
}

pub(super) fn read_message(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "helper message exceeds size limit",
        ));
    }
    let mut bytes = vec![0u8; len];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn write_message(stream: &mut UnixStream, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "helper message exceeds size limit",
        ));
    }
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}
