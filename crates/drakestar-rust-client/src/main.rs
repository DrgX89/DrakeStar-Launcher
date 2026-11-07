//! Binario del cliente Rust de DrakeStar. Lo lanza el launcher con
//! `--server <host[:puerto]> --name <usuario>`; tambien se puede usar a mano.

use drakestar_rust_client::client::{self, Config, Error};
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "drakestar-rust-client --server <host[:puerto]> [--name <usuario>] [--protocol <n>]\n  \
         consola: escribe 'quit' (o 'stop') para salir"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = Config::default();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let value = args.get(i + 1).cloned();
        match flag {
            "--server" => {
                let v = match value {
                    Some(v) => v,
                    None => {
                        eprintln!("ERROR {flag} necesita un valor");
                        return ExitCode::from(2);
                    }
                };
                match v.rsplit_once(':') {
                    Some((host, port)) if !host.is_empty() && !port.is_empty() => {
                        cfg.server = host.to_string();
                        cfg.port = port.parse().unwrap_or_else(|_| {
                            eprintln!("ERROR puerto invalido '{port}'");
                            std::process::exit(2)
                        });
                    }
                    _ => cfg.server = v,
                }
                i += 2;
            }
            "--name" => {
                cfg.name = value.unwrap_or_else(|| {
                    eprintln!("ERROR {flag} necesita un valor");
                    std::process::exit(2)
                });
                i += 2;
            }
            "--protocol" => {
                let v = value.unwrap_or_else(|| {
                    eprintln!("ERROR {flag} necesita un valor");
                    std::process::exit(2)
                });
                cfg.protocol = v.parse().unwrap_or_else(|_| {
                    eprintln!("ERROR protocolo invalido '{v}'");
                    std::process::exit(2)
                });
                i += 2;
            }
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("ERROR argumento desconocido: {other}");
                usage();
                return ExitCode::from(2);
            }
        }
    }
    if cfg.server.is_empty() {
        eprintln!("ERROR falta --server");
        usage();
        return ExitCode::from(2);
    }
    match client::run(&cfg, &mut |l| println!("{l}")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !matches!(e, Error::Disconnected(_)) {
                eprintln!("ERROR {e}");
            }
            ExitCode::from(1)
        }
    }
}
