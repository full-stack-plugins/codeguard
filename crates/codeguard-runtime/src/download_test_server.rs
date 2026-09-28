//! 仅用于本地 HTTPS 契约的固定证书服务；不进入生产构建。
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
/// 单次请求测试服务；停止或超时后回收线程，不保留后台任务。
pub(crate) struct DownloadTestServer {
    pub(crate) url: String,
    pub(crate) seen: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl DownloadTestServer {
    pub(crate) fn new(response: Vec<u8>, pause: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("https://{}/package", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let seen = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            let seen = seen.clone();
            std::thread::spawn(move || {
                let end = Instant::now() + Duration::from_secs(5);
                while Instant::now() < end && !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((socket, _)) => {
                            // macOS 接受的连接可能继承监听器的非阻塞模式。
                            // 固定测试服务使用阻塞 TLS，并由读写期限约束。
                            socket.set_nonblocking(false).unwrap();
                            socket
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            socket
                                .set_write_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let cert = rustls::pki_types::CertificateDer::from(
                                include_bytes!("../tests/fixtures/download_tls/cert.der").to_vec(),
                            );
                            let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
                                rustls::pki_types::PrivatePkcs8KeyDer::from(
                                    include_bytes!("../tests/fixtures/download_tls/key.der")
                                        .to_vec(),
                                ),
                            );
                            let config = rustls::ServerConfig::builder()
                                .with_no_client_auth()
                                .with_single_cert(vec![cert], key)
                                .unwrap();
                            let connection =
                                rustls::ServerConnection::new(Arc::new(config)).unwrap();
                            let mut stream = rustls::StreamOwned::new(connection, socket);
                            let mut request = Vec::new();
                            let mut chunk = [0; 1024];
                            while request.len() < 8192
                                && !request.windows(4).any(|w| w == b"\r\n\r\n")
                            {
                                match stream.read(&mut chunk) {
                                    Ok(0) => return,
                                    Err(_) => return,
                                    Ok(n) => request.extend_from_slice(&chunk[..n]),
                                }
                            }
                            seen.store(true, Ordering::Relaxed);
                            let resume = Instant::now() + pause;
                            while Instant::now() < resume {
                                if stop.load(Ordering::Relaxed) {
                                    return;
                                }
                                std::thread::sleep(Duration::from_millis(2));
                            }
                            let _ = stream.write_all(&response);
                            stream.conn.send_close_notify();
                            let _ = stream.flush();
                            return;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(1))
                        }
                        Err(_) => return,
                    }
                }
            })
        };
        Self {
            url,
            seen,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for DownloadTestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
