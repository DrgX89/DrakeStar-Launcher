//! Login con cuenta de Microsoft (device code) y sesion en JSON por stdout.
//!
//! Uso: `cargo run --release -p drakestar-rust-client --example login_session`
//! Los mensajes van a stderr; stdout sale limpio con el JSON, listo para
//! tuberear a `horizon-client --session-stdin`.

use drakestar_rust_client::auth;

fn main() {
    // El flujo imprime el codigo y la URL por stderr; el JSON sale por stdout.
    let session = match auth::authenticate(&mut |l| eprintln!("{l}")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR login: {e}");
            std::process::exit(1);
        }
    };
    // Horizon valida: name, uuid, access_token y user_type = cuenta Microsoft.
    println!(
        "{{\"name\":\"{}\",\"uuid\":\"{}\",\"access_token\":\"{}\",\"user_type\":\"msa\"}}",
        session.name, session.uuid, session.access_token
    );
    eprintln!(
        "sesion lista: {} ({}) — dale este nombre/UUID a la org para la whitelist",
        session.name, session.uuid
    );
}