// tpt-gpu-driver-info — print device information for a TPT GPU
//
// Usage: tpt-gpu-driver-info [/dev/dri/card0]

use tpt_gpu_driver::{Device, QueryType};

fn main() {
    let path = std::env::args().nth(1)
        .unwrap_or_else(|| "/dev/dri/card0".to_string());

    let dev = Device::open(&path).unwrap_or_else(|e| {
        eprintln!("Failed to open {path}: {e}");
        std::process::exit(1);
    });

    let vram_mb = dev.query(QueryType::VramSize).unwrap_or(0) / (1024 * 1024);
    let warps   = dev.query(QueryType::NumWarps).unwrap_or(0);
    let ctas    = dev.query(QueryType::NumCtas).unwrap_or(0);
    let lanes   = dev.query(QueryType::WarpLanes).unwrap_or(0);
    let ver     = dev.query(QueryType::DriverVer).unwrap_or(0);

    println!("TPT GPU — {path}");
    println!("  Driver version : {}.{}", ver >> 16, ver & 0xFFFF);
    println!("  VRAM           : {vram_mb} MiB");
    println!("  Warps          : {warps}");
    println!("  Max CTAs       : {ctas}");
    println!("  Lanes / warp   : {lanes}");
}
