#![cfg(windows)]

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tauri_plugin_updater::UpdaterExt;

// A real loopback HTTP server. Stalled connections remain open until shutdown,
// so EOF cannot masquerade as a successful request timeout. The hard deadline
// also bounds test cleanup if the updater ever stops honouring its timeout.
struct LocalServer {
    origin: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Vec<String>>>,
}

fn read_request_path(stream: &mut TcpStream) -> io::Result<String> {
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut request = Vec::new();
    while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "request headers did not arrive",
            ));
        }
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete request headers",
                ))
            }
            Ok(bytes) => request.extend_from_slice(&buffer[..bytes]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(error) => return Err(error),
        }
        if request.len() > 8192 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request headers too large",
            ));
        }
    }
    String::from_utf8_lossy(&request)
        .split_whitespace()
        .nth(1)
        .map(str::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request path"))
}

impl LocalServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let manifest = serde_json::json!({
            "version": "99.0.0",
            "notes": "Local timeout test; never install",
            "url": format!("{origin}/download-stall"),
            "signature": "deliberately-invalid-test-signature"
        })
        .to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = stop.clone();
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut held_connections = Vec::<TcpStream>::new();
            let mut requests = Vec::new();
            while !stop_worker.load(Ordering::Relaxed) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Windows can inherit the listener's nonblocking mode.
                        // Do not mistake a not-yet-arrived header for a closed request.
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_millis(50)))
                            .unwrap();
                        stream
                            .set_write_timeout(Some(Duration::from_millis(250)))
                            .unwrap();
                        let path = read_request_path(&mut stream)
                            .expect("local server must receive complete request headers");
                        requests.push(path.clone());
                        match path.as_str() {
                            "/manifest-stall" => held_connections.push(stream),
                            "/manifest" => {
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    manifest.len(), manifest
                                );
                                stream.write_all(response.as_bytes()).unwrap();
                            }
                            "/download-stall" => {
                                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\npartial!").unwrap();
                                held_connections.push(stream);
                            }
                            _ => panic!("unexpected local request: {path}"),
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("local server accept failed: {error}"),
                }
            }
            requests
        });
        Self {
            origin,
            stop,
            thread: Some(thread),
        }
    }

    fn shutdown(&mut self) -> Vec<String> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn assert_request_timeout(error: tauri_plugin_updater::Error, elapsed: Duration) {
    assert!(
        matches!(error, tauri_plugin_updater::Error::Reqwest(ref error) if error.is_timeout()),
        "expected a native request timeout, received {error:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(100),
        "request failed before its deadline: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "request exceeded its bounded deadline: {elapsed:?}"
    );
}

#[test]
#[ignore = "creates a Windows Tauri runtime and real loopback server; run explicitly"]
fn native_updater_times_out_stalled_manifest_and_download_without_installing() {
    // Only this test's in-memory context permits localhost HTTP. Production
    // configuration is untouched. No WebView, single-instance plugin or installer.
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey": "unused-in-timeout-test",
            "dangerousInsecureTransportProtocol": true,
            "endpoints": []
        }),
    );
    let app = tauri::Builder::default()
        .any_thread()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .expect("create windowless Windows test runtime");
    let mut server = LocalServer::start();

    // Reduced deadlines make the native regression fast. The JS adapter tests
    // separately prove production IPC uses 15,000 ms / 120,000 ms.
    let timeout = Duration::from_millis(200);
    let stalled = app
        .updater_builder()
        .endpoints(vec![format!("{}/manifest-stall", server.origin)
            .parse()
            .unwrap()])
        .unwrap()
        .no_proxy()
        .timeout(timeout)
        .build()
        .unwrap();
    let started = Instant::now();
    let error = match tauri::async_runtime::block_on(stalled.check()) {
        Err(error) => error,
        Ok(_) => panic!("a nonresponding manifest endpoint must time out"),
    };
    let check_elapsed = started.elapsed();
    assert_request_timeout(error, check_elapsed);

    // A successful subsequent check proves that a timed-out check does not leave
    // the actual native updater unusable.
    let recovered = app
        .updater_builder()
        .endpoints(vec![format!("{}/manifest", server.origin).parse().unwrap()])
        .unwrap()
        .no_proxy()
        .timeout(timeout)
        .build()
        .unwrap();
    let mut update = tauri::async_runtime::block_on(recovered.check())
        .unwrap()
        .unwrap();
    update.timeout = Some(timeout);
    let mut download_times = Vec::new();
    for _ in 0..2 {
        let mut chunks_received = 0;
        let mut completed = false;
        let started = Instant::now();
        // Download only. Never call install() or download_and_install().
        let error = tauri::async_runtime::block_on(
            update.download(|bytes, _| chunks_received += bytes, || completed = true),
        )
        .unwrap_err();
        let elapsed = started.elapsed();
        assert_request_timeout(error, elapsed);
        assert!(
            chunks_received > 0,
            "body stall must occur after a real data chunk"
        );
        assert!(
            !completed,
            "an incomplete body must not reach download completion"
        );
        download_times.push(elapsed);
    }
    assert_eq!(
        server.shutdown(),
        vec![
            "/manifest-stall",
            "/manifest",
            "/download-stall",
            "/download-stall"
        ]
    );
    println!("native manifest timeout: {check_elapsed:?}; body timeouts (including retry): {download_times:?}; installer calls: 0");
}
