use log::{error, debug};
use clap;
use yaml_rust::{YamlLoader, Yaml, yaml};
use std::fs;
use std::process;
use rand::random;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use crate::checker::{CheckResult};

pub struct ProbeConfig {
    pub name: String,
    pub host: String,
    pub check_type: String,
    pub interval: i64,
    pub config: HashMap<String, yaml::Yaml>,
    pub labels: HashMap<String, String>
}

pub struct ProcessConfig {
    pub id: u16,
    pub process_name: String,
    pub keep_name: bool,
    pub labels_to_add: HashMap<String, String>,
    pub match_labels: HashMap<String, Vec<String>>,
    pub values: Vec<String>,
    pub sender: Option<Sender<CheckResult>>,
    pub match_value: String,
    pub config: HashMap<String, yaml::Yaml>,
}

pub struct OutputConfig {
    pub id: u16,
    pub output_name: String,
    pub match_labels: HashMap<String, Vec<String>>,
    pub sender: Option<Sender<CheckResult>>,
    pub config: HashMap<String, yaml::Yaml>
}

pub fn load_config() -> (Vec<ProbeConfig>, Vec<ProcessConfig>, Vec<OutputConfig>) {
    let app = clap::Command::new("rust-checker")
        .version("0.0.1")
        .author("Me")
        .about("test clap")
        .arg(clap::arg!(config: -c --config <config> "config file")
             .required(true))
        .arg(clap::arg!(remote: -r --remote)
             .action(clap::ArgAction::SetTrue))
        .get_matches();
    let text_config: String = match fs::read_to_string(app.get_one::<String>("config").unwrap()) {
        Ok(config_file) => config_file,
        Err(err) => {
            error!("Failed to load config: {}", err);
            process::exit(1);
        },
    };
    let cfg: Vec::<Yaml> = match YamlLoader::load_from_str(&text_config) {
        Ok(docs) => docs,
        Err(e) => {
            println!("Failed to parse config: {}", e);
            process::exit(1);
        },
    };
    let mut probes = Vec::<ProbeConfig>::new();
    debug!("Loading probes");
    match cfg[0]["hosts"] {
        yaml_rust::Yaml::Hash(ref h) => {
            for (key, value) in h {
                let mut host = ProbeConfig{
                    name: key.clone().into_string().unwrap(),
                    host: value["addr"].clone().into_string().unwrap(),
                    check_type: value["check"].clone().into_string().unwrap(),
                    interval: value["interval"].clone().into_i64().unwrap(),
                    config: HashMap::new(),
                    labels: HashMap::new()
                };
                debug!("Probe name: {}, host: {}", host.name, host.host);
                match value["labels"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (l_key, l_value) in l {
                            host.labels.insert(l_key.clone().into_string().unwrap(), l_value.clone().into_string().unwrap());
                        }
                    },
                    _ => {
                        error!("Labels should be an array");
                        process::exit(1);
                        }
                }
                match value["config"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            host.config.insert(m_name.clone().into_string().unwrap(), m_value.clone());
                        }
                    },
                    _ => {
                        error!("Config should be a HashMap");
                        process::exit(1);
                    }
                }
                probes.push(host);
            };
            if *app.get_one::<bool>("remote").unwrap() {
                probes.push(
                    ProbeConfig{
                        name: format!("remote_listener"),
                        host: format!("remote_listener"),
                        check_type: format!("remote_listener"),
                        interval: 0,
                        config: HashMap::new(),
                        labels: HashMap::new()
                    });
            }
        },
        _ => {
            error!("Probe should be an array");
            process::exit(1);
        }
    }
    let mut processes = Vec::<ProcessConfig>::new();
    debug!("Loading processes");
    match cfg[0]["processes"] {
        yaml_rust::Yaml::Array(ref h) => {
            for value in h {
                let mut process = ProcessConfig{
                    id: random::<u16>(),
                    keep_name: value["keep_name"].clone().as_bool().unwrap_or_default(),
                    process_name: value["process_name"].clone().into_string().unwrap(),
                    match_value: value["match_value"].clone().into_string().unwrap(),
                    values: Vec::new(),
                    labels_to_add: HashMap::new(),
                    sender: None,
                    config: HashMap::new(),
                    match_labels: HashMap::new()
                };
                debug!("Process name: {}", process.process_name);
                match value["match_labels"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            let mut match_values = Vec::new();
                            match m_value {
                                yaml_rust::Yaml::String(ref s) => {
                                    match_values.push(s.clone());
                                },
                                yaml_rust::Yaml::Array(ref s) => {
                                    for s_val in s {
                                        match_values.push(s_val.clone().into_string().unwrap());
                                    }
                                },
                                _ => {
                                    error!("match_label values should be an array");
                                    process::exit(1);
                                }
                            }
                            process.match_labels.insert(m_name.clone().into_string().unwrap(), match_values);
                        }
                    },
                    _ => {
                        error!("match_label should be a hashmap");
                        process::exit(1);
                    }
                }
                match value["labels_to_add"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            process.labels_to_add.insert(m_name.clone().into_string().unwrap(), m_value.clone().into_string().unwrap());
                        }
                    },
                    _ => {
                        error!("labels_to_add should be a hashmap");
                        process::exit(1);
                    }
                }
                match value["config"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            println!("{:?} {:?}", m_name, m_value);
                            process.config.insert(m_name.clone().into_string().unwrap(), m_value.clone());
                        }
                    },
                    _ => {
                        error!("config should be an hashmap");
                        process::exit(1);
                    }
                }
                processes.push(process);
            }
        },
        _ => {
            error!("probe should be an array");
            process::exit(1);
        }
    }
    let mut outputs = Vec::<OutputConfig>::new();
    debug!("Loading outputs");
    match cfg[0]["outputs"] {
        yaml_rust::Yaml::Array(ref h) => {
            for value in h {
                let mut output = OutputConfig {
                    output_name: value["output_name"].clone().into_string().unwrap(),
                    id: random::<u16>(),
                    sender: None,
                    config: HashMap::new(),
                    match_labels: HashMap::new()
                };
                debug!("Output name: {}", output.output_name);
                match value["match_labels"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            let mut match_values = Vec::new();
                            match m_value {
                                yaml_rust::Yaml::String(ref s) => {
                                    match_values.push(s.clone());
                                },
                                yaml_rust::Yaml::Array(ref s) => {
                                    for s_val in s {
                                        match_values.push(s_val.clone().into_string().unwrap());
                                    }
                                },
                                _ => {
                                    error!("match_label should be an array or string");
                                    process::exit(1);
                                }
                            }
                            output.match_labels.insert(m_name.clone().into_string().unwrap(), match_values);
                        }
                    },
                    _ => {}
                }
                match value["config"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            output.config.insert(m_name.clone().into_string().unwrap(), m_value.clone());
                        }
                    },
                    _ => {
                        error!("config should be an hashmap");
                        process::exit(1);
                    }
                }
                outputs.push(output);
            }
        },
        _ => {
            error!("output should be an array");
            process::exit(1);
        }
    }
    return (probes, processes, outputs);
}
