//! Reproduce the Tokio/GLib busy loop without a display or polkit session.
//!
//! Run with `nested` to keep GLib inside Tokio's `block_on`, or `entered` to
//! keep the runtime available while running GLib outside `block_on`.

use gtk4::glib;
use std::{cell::Cell, rc::Rc, time::Duration};

const ITERATIONS: usize = 200;

fn run_glib_loop() -> usize {
    let context = glib::MainContext::default();
    let main_loop = glib::MainLoop::new(Some(&context), false);
    let completed = Rc::new(Cell::new(0));
    let completed_in_task = Rc::clone(&completed);
    let done = main_loop.clone();

    context.spawn_local(async move {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<u8>(1);
        for i in 0..ITERATIONS {
            sender.send(1).await.expect("receiver is open");
            receiver.recv().await.expect("sender is open");
            completed_in_task.set(i + 1);
        }
        done.quit();
    });

    let watchdog = main_loop.clone();
    glib::timeout_add_local_once(Duration::from_millis(750), move || watchdog.quit());
    main_loop.run();
    completed.get()
}

fn main() {
    let mode = std::env::args().nth(1).expect("expected nested or entered");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Tokio runtime");

    let completed = match mode.as_str() {
        "nested" => runtime.block_on(async { run_glib_loop() }),
        "entered" => {
            let _guard = runtime.enter();
            run_glib_loop()
        }
        _ => panic!("expected nested or entered"),
    };
    println!("{mode}: completed {completed}/{ITERATIONS} channel round trips");
}
