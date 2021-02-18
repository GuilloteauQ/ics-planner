mod config;
use crate::config::Config;
fn main() {
    let filename = "config.yml";
    let config_file =
        std::fs::read_to_string(filename).expect("Something went wrong reading the config file");
    let config: Config = serde_yaml::from_str(&config_file).expect("Could not read config file");
    // config.to_ics("test.ics".to_string());
    config.to_ics();
}
