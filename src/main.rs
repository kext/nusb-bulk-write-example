use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

const PACKET_SIZE: usize = 64;

fn main() {
    let interface = nusb::list_devices()
        .unwrap()
        .find(|dev| dev.vendor_id() == 0x07b4 && dev.product_id() == 0x0866)
        .expect("No device matching found")
        .open()
        .expect("Could not open device")
        .claim_interface(0)
        .expect("Could not claim interface");
    let should_stop = Arc::new(AtomicBool::new(false));
    {
        let should_stop = Arc::clone(&should_stop);
        ctrlc::set_handler(move || {
            should_stop.store(true, Ordering::Relaxed);
        })
        .unwrap();
    }
    let mut queues = [interface.bulk_out_queue(1)];
    println!("Filling queue");
    for queue in &mut queues {
        for _ in 0..128 {
            queue.submit(vec![0; PACKET_SIZE]);
        }
    }
    let mut t0 = Instant::now();
    let mut n = 0;
    while !should_stop.load(Ordering::Relaxed) {
        for queue in &mut queues {
            match futures_lite::future::block_on(queue.next_complete()).into_result() {
                Ok(v) => {
                    n += PACKET_SIZE;
                    if n >= 1_000_000 {
                        let t = Instant::now();
                        let d = t.duration_since(t0).as_secs_f64();
                        println!(
                            "Transferred {n} Bytes in {:.2}s ({:.3}MBit/s)",
                            d,
                            (n * 8) as f64 / d / 1e6
                        );
                        t0 = t;
                        n = 0;
                    }
                    let mut v = v.reuse();
                    v.resize(PACKET_SIZE, 0);
                    queue.submit(v);
                }
                Err(e) => {
                    println!("{e}");
                    return;
                }
            }
        }
    }
}
