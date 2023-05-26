use argparse::{ArgumentParser, StoreTrue, Store};
use yaml_rust::{YamlLoader, Yaml};
use std::fs;
use std::process;
use rand::random;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use crate::checker::{CheckResult};

struct CmdOptions {
    verbose: bool,
    name: String,
}

pub struct ProbeConfig {
    pub host: String,
    pub check_type: String,
    pub interval: i64,
    pub mtu: i64,
    pub source_ip: String,
    pub labels: HashMap<String, String>
}

pub struct ProcessConfig {
    pub id: u16,
    pub process_name: String,
    pub labels_to_add: HashMap<String, String>,
    pub match_labels: HashMap<String, Vec<String>>,
    pub values: Vec<String>,
    pub sender: Option<Sender<CheckResult>>,
    pub match_value: String
}

pub struct OutputConfig {
    pub id: u16,
    pub output_name: String,
    pub match_labels: HashMap<String, Vec<String>>,
    pub sender: Option<Sender<CheckResult>>,
    pub config: HashMap<String, String>
}

pub fn load_config() -> (Vec<ProbeConfig>, Vec<ProcessConfig>, Vec<OutputConfig>) {
    let mut cmd_opts = CmdOptions{verbose: false, name: String::from("none")};
    {
        let mut parser = ArgumentParser::new();
        parser.set_description("yaml config parser");
        parser.refer(&mut cmd_opts.verbose)
            .add_option(&["-v"], StoreTrue, "Be verbose");
        parser.refer(&mut cmd_opts.name)
            .add_option(&["-c", "--config"], Store, "Config file")
            .required();
        parser.parse_args_or_exit();
    }
    let text_config: String = match fs::read_to_string(cmd_opts.name) {
        Ok(config_file) => config_file,
        Err(err) => {
            println!("Failed to load config: {}", err);
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
    match cfg[0]["hosts"] {
        yaml_rust::Yaml::Hash(ref h) => {
            for (key, value) in h {
                let mut host = ProbeConfig{
                    host: value["addr"].clone().into_string().unwrap(),
                    check_type: value["check"].clone().into_string().unwrap(),
                    interval: value["interval"].clone().into_i64().unwrap(),
                    mtu: value["mtu"].clone().into_i64().unwrap_or_default(),
                    source_ip: value["source_ip"].clone().into_string().unwrap_or_default(),
                    labels: HashMap::new()
                };
                host.labels.insert(String::from("name"), key.clone().into_string().unwrap());
                match value["labels"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (l_key, l_value) in l {
                            host.labels.insert(l_key.clone().into_string().unwrap(), l_value.clone().into_string().unwrap());
                        }
                    },
                    _ => {}
                }
                probes.push(host);
                
            };
        },
        _ => {}
    }
    let mut processes = Vec::<ProcessConfig>::new();
    match cfg[0]["processes"] {
        yaml_rust::Yaml::Array(ref h) => {
            for value in h {
                let mut process = ProcessConfig{
                    id: random::<u16>(),
                    process_name: value["process_name"].clone().into_string().unwrap(),
                    match_value: value["match_value"].clone().into_string().unwrap(),
                    values: Vec::new(),
                    labels_to_add: HashMap::new(),
                    sender: None,
                    match_labels: HashMap::new()
                };
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
                                _ => {}
                            }
                            process.match_labels.insert(m_name.clone().into_string().unwrap(), match_values);
                        }
                    },
                    _ => {}
                }
                match value["labels_to_add"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            process.labels_to_add.insert(m_name.clone().into_string().unwrap(), m_value.clone().into_string().unwrap());
                        }
                    },
                    _ => {}
                }
                match value["values"] {
                    yaml_rust::Yaml::Array(ref l) => {
                        for v in l {
                            process.values.push(v.clone().into_string().unwrap());
                        }
                    },
                    _ => {}
                }
                processes.push(process);
            }
        },
        _ => {}
    }
    let mut outputs = Vec::<OutputConfig>::new();
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
                                _ => {}
                            }
                            output.match_labels.insert(m_name.clone().into_string().unwrap(), match_values);
                        }
                    },
                    _ => {}
                }
                match value["config"] {
                    yaml_rust::Yaml::Hash(ref l) => {
                        for (m_name, m_value) in l {
                            output.config.insert(m_name.clone().into_string().unwrap(), m_value.clone().into_string().unwrap());
                        }
                    },
                    _ => {}
                }
                outputs.push(output);
            }
        },
        _ => {}
    }
    return (probes, processes, outputs);
}
