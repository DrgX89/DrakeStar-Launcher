use drakestar_core::{
    compat, launch,
    profile::Profile,
    release::{sha256_wellformed, Manifest},
    Channel,
};
use std::process::ExitCode;

fn usage() {
    eprintln!("drakestar <comando>\n  profile-check <archivo.json>   valida un perfil\n  manifest-check <archivo.json>  valida un manifiesto de releases\n  plan <perfil.json> [java] [jar] [raiz]  muestra el plan de lanzamiento (no ejecuta)\n  compat                         matriz de compatibilidad");
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("profile-check") if a.len() == 2 => match std::fs::read_to_string(&a[1])
            .map_err(|e| e.to_string())
            .and_then(|s| Profile::from_json(&s).map_err(|e| e.to_string()))
        {
            Ok(p) => {
                println!(
                    "OK  {} [{}] canal={} modo={:?}",
                    p.id,
                    p.name,
                    p.channel.label(),
                    p.mode
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("ERROR {e}");
                ExitCode::from(1)
            }
        },
        Some("manifest-check") if a.len() == 2 => {
            let s = match std::fs::read_to_string(&a[1]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("ERROR {e}");
                    return ExitCode::from(1);
                }
            };
            match Manifest::from_json(&s) {
                Ok(m) => {
                    let bad = m
                        .releases
                        .iter()
                        .flat_map(|r| &r.artifacts)
                        .filter(|x| !sha256_wellformed(&x.sha256))
                        .count();
                    println!(
                        "{} releases, {} artefactos con SHA-256 invalido",
                        m.releases.len(),
                        bad
                    );
                    if bad == 0 {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("ERROR {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("plan") if a.len() >= 2 => {
            let s = std::fs::read_to_string(&a[1]).unwrap_or_default();
            match Profile::from_json(&s) {
                Ok(p) => {
                    let java = a.get(2).map(String::as_str).unwrap_or("java");
                    let jar = a.get(3).map(String::as_str).unwrap_or("drakespurpur.jar");
                    let root = a.get(4).map(String::as_str).unwrap_or(".drakestar");
                    println!(
                        "Java requerido: {}",
                        p.java
                            .major
                            .unwrap_or_else(|| launch::required_java(&p.minecraft))
                    );
                    match launch::local_server_plan(&p, java, jar, root) {
                        Some(pl) => {
                            println!("cwd: {}\n{} {}", pl.cwd, pl.program, pl.args.join(" "))
                        }
                        None => println!("(este modo no genera plan de servidor local)"),
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("ERROR {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("compat") => {
            use drakestar_core::profile::{Loader, Mode};
            for m in [
                Mode::DrakesRust,
                Mode::DrakesJava,
                Mode::LocalServer,
                Mode::Online,
            ] {
                println!("{:?}: mods Fabric={:?} · plugins de servidor visibles={:?} · GUI Slimefun={:?}", m, compat::java_mods(m, Loader::Fabric), compat::server_plugins_visible(m), compat::slimefun_gui(m));
            }
            let _ = Channel::Stable;
            ExitCode::SUCCESS
        }
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
