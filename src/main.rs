use learn_wgpu::run;

fn main() {
    // Required for wgpu error messages to be printed
    env_logger::init();

    run().unwrap();
}
