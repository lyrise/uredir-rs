use clap::Parser;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::sleep;

#[derive(Parser)]
struct Opts {
    #[clap(short, long, required = true)]
    local_addr: String,

    #[clap(short, long, required = true)]
    remote_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();

    let recv_socket = UdpSocket::bind(&opts.local_addr).await?;
    let send_socket = UdpSocket::bind("0.0.0.0:0").await?;

    let (tx, mut rx) = mpsc::channel(100);

    let sent_bytes = Arc::new(AtomicUsize::new(0));
    let received_bytes = Arc::new(AtomicUsize::new(0));

    let sent_bytes_clone = Arc::clone(&sent_bytes);
    let received_bytes_clone = Arc::clone(&received_bytes);

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let sent = sent_bytes_clone.swap(0, Ordering::Relaxed);
            let received = received_bytes_clone.swap(0, Ordering::Relaxed);
            println!(
                "Sent: {} bytes/sec, Received: {} bytes/sec",
                sent / 10,
                received / 10
            );
        }
    });

    tokio::spawn(async move {
        loop {
            let mut buf = vec![0; 65535];
            match recv_socket.recv_from(&mut buf).await {
                Ok((amt, _)) => {
                    received_bytes.fetch_add(amt, Ordering::Relaxed);
                    if let Err(e) = tx.send(buf[..amt].to_vec()).await {
                        eprintln!("Failed to send packet: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to receive packet: {}", e);
                }
            }
        }
    });

    while let Some(buf) = rx.recv().await {
        let mut total_sent = 0;
        while total_sent < buf.len() {
            match send_socket
                .send_to(&buf[total_sent..], &opts.remote_addr)
                .await
            {
                Ok(sent) => {
                    total_sent += sent;
                    sent_bytes.fetch_add(sent, Ordering::Relaxed);
                }
                Err(e) => {
                    eprintln!("Failed to send packet: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
