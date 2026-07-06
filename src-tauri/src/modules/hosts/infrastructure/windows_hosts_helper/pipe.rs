/*
 * 核心职责：读写 named pipe 消息。
 * 业务痛点：pipe 通信必须有长度限制和完整读写，避免协议截断。
 * 能力边界：只处理客户端连接和字节消息传输。
 */

use super::*;

pub(super) fn send_request(request: &HelperRequest) -> AppResult<HelperResponse> {
    let pipe = connect_client_pipe(Duration::from_secs(3)).map_err(|error| {
        AppError::new("hosts_helper_unavailable", "无法连接 Windows hosts helper")
            .with_detail(error.to_string())
    })?;
    let request_bytes = serde_json::to_vec(request).map_err(|error| {
        AppError::new("hosts_helper_request_failed", "序列化 helper 请求失败")
            .with_detail(error.to_string())
    })?;
    write_message(pipe.raw(), &request_bytes).map_err(|error| {
        AppError::new("hosts_helper_request_failed", "发送 helper 请求失败")
            .with_detail(error.to_string())
    })?;
    let response_bytes = read_message(pipe.raw()).map_err(|error| {
        AppError::new("hosts_helper_response_failed", "读取 helper 响应失败")
            .with_detail(error.to_string())
    })?;
    serde_json::from_slice::<HelperResponse>(&response_bytes).map_err(|error| {
        AppError::new("hosts_helper_response_failed", "解析 helper 响应失败")
            .with_detail(error.to_string())
    })
}

pub(super) fn connect_client_pipe(timeout: Duration) -> io::Result<WinHandle> {
    let started = Instant::now();
    let pipe_name = wide_null(PIPE_NAME);
    loop {
        let handle = unsafe {
            CreateFileW(
                pipe_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            return Ok(WinHandle(handle));
        }

        let error = io::Error::last_os_error();
        if started.elapsed() >= timeout {
            return Err(error);
        }
        if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) {
            unsafe {
                WaitNamedPipeW(pipe_name.as_ptr(), 500);
            }
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

pub(super) fn create_server_pipe(config: &HelperConfig) -> io::Result<WinHandle> {
    let security = PipeSecurity::new(&config.allowed_user_sid)?;
    let pipe_name = wide_null(PIPE_NAME);
    let handle = unsafe {
        CreateNamedPipeW(
            pipe_name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            PIPE_BUFFER_SIZE,
            PIPE_BUFFER_SIZE,
            0,
            security.attributes_ptr(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(WinHandle(handle))
    }
}

pub(super) fn connect_pipe(handle: HANDLE) -> io::Result<()> {
    let ok = unsafe { ConnectNamedPipe(handle, ptr::null_mut()) };
    if ok != 0 || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn wake_pipe() {
    let _ = connect_client_pipe(Duration::from_millis(500));
}

pub(super) fn read_message(handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    read_exact_handle(handle, &mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "helper message exceeds size limit",
        ));
    }
    let mut bytes = vec![0u8; len];
    read_exact_handle(handle, &mut bytes)?;
    Ok(bytes)
}

pub(super) fn write_message(handle: HANDLE, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "helper message exceeds size limit",
        ));
    }
    write_all_handle(handle, &(bytes.len() as u32).to_le_bytes())?;
    write_all_handle(handle, bytes)?;
    unsafe {
        FlushFileBuffers(handle);
    }
    Ok(())
}

pub(super) fn read_exact_handle(handle: HANDLE, buffer: &mut [u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let mut read = 0u32;
        let ok = unsafe {
            ReadFile(
                handle,
                buffer[offset..].as_mut_ptr(),
                (buffer.len() - offset) as u32,
                &mut read,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            return Err(io::Error::new(ErrorKind::UnexpectedEof, "pipe closed"));
        }
        offset += read as usize;
    }
    Ok(())
}

pub(super) fn write_all_handle(handle: HANDLE, buffer: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                handle,
                buffer[offset..].as_ptr(),
                (buffer.len() - offset) as u32,
                &mut written,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if written == 0 {
            return Err(io::Error::new(
                ErrorKind::WriteZero,
                "pipe write returned zero",
            ));
        }
        offset += written as usize;
    }
    Ok(())
}
