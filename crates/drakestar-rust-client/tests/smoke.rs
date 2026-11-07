//! Prueba de humo del binario: lanza `drakestar-rust-client` contra un servidor
//! TCP de juguete y comprueba que conecta, hace login, recibe un paquete del
//! flujo y sale con codigo 0 cuando el servidor cierra.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

#[test]
fn binary_connects_and_exits_zero() {
    let lst = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = lst.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut s, _) = lst.accept().unwrap();
        let mut r = s.try_clone().unwrap();
        let read_packet = |r: &mut dyn Read| (drakestar_rust_client::proto::read_packet(r).unwrap());
        use drakestar_rust_client::proto as proto;
        let (hs, _) = read_packet(&mut r); // handshake
        assert_eq!(hs, 0x00);
        let (start, body) = read_packet(&mut r); // login start
        assert_eq!(start, 0x00);
        assert_eq!(proto::read_string(&mut std::io::Cursor::new(body)).unwrap(), "humo");
        let mut body = Vec::new();
        body.extend_from_slice(&[0u8; 16]); // uuid nil
        drakestar_rust_client::proto::send_packet(&mut s, 0x02, &body).unwrap();
        let (ack, _) = read_packet(&mut r); // login acknowledged
        assert_eq!(ack, 0x02);
        drakestar_rust_client::proto::send_packet(&mut s, 0x57, b"hola").unwrap();
        s.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
    });

    let bin = env!("CARGO_BIN_EXE_drakestar-rust-client");
    let out = Command::new(bin)
        .args(["--server", &format!("127.0.0.1:{port}"), "--name", "humo"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    server.join().unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "codigo {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("conectando a"), "{stdout}");
    assert!(stdout.contains("login correcto"), "{stdout}");
    assert!(stdout.contains("0x57"), "{stdout}");
}
