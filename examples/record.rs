use rodio;

fn main() {
    let input = rodio::default_input_device().unwrap();
    println!("Default input device: {}", input.name());

    for output in rodio::output_devices() {
        println!("output device: {}", output.name());
    }
}
