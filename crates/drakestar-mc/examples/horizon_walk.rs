//! Camino del motor Horizon sin Tauri: lo mismo que hace `horizon_play`, para
//! probarlo sin depender de clicks en la UI.
//!
//! Uso: cargo run --release -p drakestar-mc --example horizon_walk -- \
//!        <dir-de-datos> [instancia] [host:puerto]

use drakestar_mc::horizon;
use drakestar_mc::Dirs;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("falta el dir de datos");
    let id = args.next().unwrap_or_else(|| "vanilla-latest".to_string());
    let server = args.next();

    let dirs = Dirs::new(PathBuf::from(&root));
    let hz = horizon::paths(&dirs, &id);

    if !hz.installed() {
        println!("== instalando binarios de Horizon");
        if let Err(e) = horizon::install(&hz) {
            eprintln!("ERROR install: {e}");
            std::process::exit(1);
        }
    }
    let inputs = match horizon::ensure_inputs(&hz, &dirs, horizon::MC_NATIVE, &drakestar_mc::NullSink) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("ERROR ensure_inputs: {e}");
            std::process::exit(1);
        }
    };
    println!("== insumos: {} ", inputs.version_json.display());
    let jdk = match horizon::prepare_java(&dirs, &drakestar_mc::NullSink) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("ERROR jdk: {e}");
            std::process::exit(1);
        }
    };
    println!("== jdk: {}", jdk.display());
    let t0 = std::time::Instant::now();
    if let Err(e) = horizon::prepare(&hz, &inputs, &jdk) {
        eprintln!("ERROR prepare: {e}");
        std::process::exit(1);
    }
    println!("== recursos preparados en {:.1}s: {}", t0.elapsed().as_secs_f32(), hz.resources().display());
    let plan = horizon::launch_plan(&hz, server.as_deref(), None, horizon::DEFAULT_RENDER_DISTANCE);
    println!("== plan de lanzamiento:");
    println!("   programa: {}", plan.program.display());
    println!("   args: {}", plan.args.join(" "));
    for (k, v) in &plan.envs {
        println!("   env {k}={v}");
    }
    let lock = horizon::read_lock(&hz).unwrap();
    println!("== lock: java {}, manifests aceptados {}", lock.java_major, lock.accepted_manifest.len());
}