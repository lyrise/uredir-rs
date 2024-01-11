use clap::Parser;
use std::error::Error;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

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

    tokio::spawn(async move {
        loop {
            let mut buf = vec![0; 65535];
            match recv_socket.recv_from(&mut buf).await {
                Ok((amt, _)) => {
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
