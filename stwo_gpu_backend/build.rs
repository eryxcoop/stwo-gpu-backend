const CUDA_LIB_DIR: &str = "/workspaces/cuda-rust-example/cuda";

fn main() {
    // Rerun conditions
    // Source files
    println!(
        "cargo:rerun-if-changed={}/src/batch_inverse.cu",
        CUDA_LIB_DIR
    );
    println!("cargo:rerun-if-changed={}/src/bit_reverse.cu", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/circle.cu", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/utils.cu", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/fri.cu", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/fields.cu", CUDA_LIB_DIR);

    // Header files
    println!(
        "cargo:rerun-if-changed={}/src/batch_inverse.cuh",
        CUDA_LIB_DIR
    );
    println!(
        "cargo:rerun-if-changed={}/src/bit_reverse.cuh",
        CUDA_LIB_DIR
    );
    println!("cargo:rerun-if-changed={}/src/circle.cuh", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/fields.cuh", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/point.cuh", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/utils.cuh", CUDA_LIB_DIR);
    println!("cargo:rerun-if-changed={}/src/fri.cuh", CUDA_LIB_DIR);

    // Build cuda code
    println!("cargo:rustc-link-search={}", CUDA_LIB_DIR);

    let status = std::process::Command::new("make")
        .args([&format!("{}/Makefile", CUDA_LIB_DIR)])
        .status()
        .expect("Failed to execute make");
    if !status.success() {
        panic!("make failed with status: {}", status);
    }
}
