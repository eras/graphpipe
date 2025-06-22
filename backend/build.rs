use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    println!("cargo:rerun-if-changed=../frontend/src/");

    // TODO: move all frontend stuff to the frontend dir?
    //let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let frontend_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("frontend");

    // // Change current directory to the frontend directory
    // assert!(env::set_current_dir(&frontend_dir).is_ok());

    eprintln!("Running `npm run build` in {frontend_dir:?}");

    // TODO: it's unclean that the `npm run build` assets are copied into the backend/assets directory
    // Instead, the OUT_DIR variable should be used. So probably `npm run build` should not copy anything
    // and we would deal with copying here. And in addition also the asset embedding should use the same
    // directory. What to do when not using asset embedding is quite a bit less clear, though.

    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .status()
        .expect("Failed to execute `npm run build`");

    if !status.success() {
        panic!("`npm run build` failed!");
    }

    // // Change back to the original directory (optional, but good practice if you have more build steps)
    // assert!(env::set_current_dir(out_dir.parent().unwrap().parent().unwrap()).is_ok());

    // Tell Cargo that if the output of this build script changes, then the project should be recompiled.
    // This is important if `npm run build` generates files that are then embedded.
    // However, if the embedded files are directly in `frontend/dist` or similar,
    // `rerun-if-changed` for the source files should be sufficient.
    // If you're copying files into OUT_DIR, then you might want to add:
    println!("cargo:rerun-if-changed=assets/");
}
