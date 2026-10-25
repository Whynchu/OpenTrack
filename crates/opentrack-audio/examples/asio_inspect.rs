//! Read-only driver inventory. No stream is created and no settings are changed.
#[cfg(all(windows, feature = "local-asio"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let drivers = maolan_asio::drivers()?;
    for driver in &drivers {
        println!("Installed: {}", driver.name);
    }
    let Some(name) = std::env::args().nth(1) else {
        return Ok(());
    };
    let descriptor = drivers
        .iter()
        .find(|d| d.name == name)
        .ok_or("Driver not found")?;
    let guard =
        maolan_asio::com::initialize().map_err(|e| std::io::Error::from_raw_os_error(e.0))?;
    let driver = maolan_asio::Driver::open(descriptor, &guard)?;
    println!(
        "Driver: {} / version {}",
        driver.driver_name(),
        driver.driver_version()
    );
    println!("Rate: {}", driver.sample_rate()?);
    println!("Buffer: {:?}", driver.buffer_sizes()?);
    println!("Latencies: {:?}", driver.latencies()?);
    let (inputs, outputs) = driver.channels()?;
    for (input, count) in [(true, inputs), (false, outputs)] {
        for n in 0..count {
            println!("Port: {:?}", driver.channel_info(n, input)?);
        }
    }
    println!(
        "Rates: {:?}",
        [22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000]
            .into_iter()
            .filter(|r| driver.can_sample_rate(f64::from(*r)))
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[cfg(not(all(windows, feature = "local-asio")))]
fn main() {
    eprintln!("Use Windows with --features local-asio");
}
