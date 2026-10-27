//! Standalone check for the dedicated-thread native editor host.
//! Build with `cargo build -p opentrack --example vst3_host_probe`.

#[path = "../src/vst3_host.rs"]
#[allow(dead_code)]
mod vst3_host;

#[cfg(windows)]
fn main() {
    use opentrack_plugins::HostedProcessor;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};

    let effect = Arc::new(
        opentrack_vst3::Vst3Effect::load(
            std::path::Path::new(
                r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3",
            ),
            opentrack_plugins::PluginClassId::from_hex("375453564336634D6C612D3736206D6F")
                .expect("class id"),
            48_000,
            512,
        )
        .expect("load Waves"),
    );
    let stop = Arc::new(AtomicBool::new(false));
    let worker_effect = Arc::clone(&effect);
    let worker_stop = Arc::clone(&stop);
    let worker = std::thread::spawn(move || {
        let mut frame = 0_u64;
        while !worker_stop.load(Ordering::Acquire) {
            let mut left = [0.0_f32; 64];
            let mut right = [0.0_f32; 64];
            for (index, sample) in left.iter_mut().enumerate() {
                *sample = (std::f32::consts::TAU * 220.0 * (frame + index as u64) as f32
                    / 48_000.0)
                    .sin()
                    * 0.25;
            }
            right.copy_from_slice(&left);
            let _ = worker_effect.process(&mut left, &mut right);
            frame += 64;
            std::thread::sleep(Duration::from_millis(1));
        }
    });

    let host =
        vst3_host::EditorHost::open("OpenTrack dedicated editor probe", 0, Arc::clone(&effect))
            .expect("open dedicated editor");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(90) && host.is_alive() {
        if effect.take_dirty() {
            println!("parameter edit received");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    println!("editor alive={}", host.is_alive());
    host.close();
    stop.store(true, Ordering::Release);
    let _ = worker.join();
}

#[cfg(not(windows))]
fn main() {}
