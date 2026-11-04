fn main() {
    if let Some(dir) = dirs::data_dir() {
        println!("data_dir: {:?}", dir);
    } else {
        println!("data_dir: None");
    }

    if let Some(dir) = dirs::config_dir() {
        println!("config_dir: {:?}", dir);
    } else {
        println!("config_dir: None");
    }
}
