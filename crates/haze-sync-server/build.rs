use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source_dir = manifest_dir.join("src/control_plane/service");
    let mut generated = String::new();
    for index in 0..=6 {
        let path = source_dir.join(format!("methods_{index:02}.rs"));
        println!("cargo:rerun-if-changed={}", path.display());
        let methods = fs::read_to_string(&path).expect("read control-plane service fragment");
        generated.push_str("impl ControlPlaneServices {\n");
        generated.push_str(&methods);
        generated.push_str("\n}\n");
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("out dir"))
        .join("control_plane_service_methods.rs");
    fs::write(output, generated).expect("write generated control-plane service methods");
}
