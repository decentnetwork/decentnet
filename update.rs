use std::path::{Path, PathBuf};

fn main() {
    let project_path = std::env::current_dir().unwrap();
    let flutter_path = "skynet";
    let path = Path::new(project_path.to_str().unwrap());
    let flutter_path = path.parent().unwrap().join(flutter_path);
    if flutter_path.exists() {
        let dllfile = flutter_path.join("decentnet.dll");
        if dllfile.exists() {
            std::fs::remove_file(&dllfile).unwrap();
        }
        let dir_var = match std::option_env!("CARGO_TARGET_DIR") {
            Some(dir) => dir,
            None => "target",
        };
        let project_path = PathBuf::from(&dir_var);
        let build_path = project_path.join("release/decentnet.dll");
        if build_path.exists() {
            std::fs::copy(&build_path, &dllfile).unwrap();
        }
    } else {
        println!("Flutter Project not found");
    }
}
