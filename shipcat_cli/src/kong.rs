use std::io::{self, Write};
use std::collections::BTreeMap;

use super::{Manifest, Result, Config};
use super::structs::Kong;
use super::structs::kongfig::{kongfig_apis, kongfig_consumers};
use super::structs::kongfig::{Api, Consumer, Plugin, Upstream, Certificate};
use super::config::KongConfig;

/// KongOutput matches the format expected by the Kong Configurator script
#[derive(Serialize)]
struct KongOutput {
    pub apis: BTreeMap<String, Kong>,
    pub kong: KongConfig,
}

/// KongOutput for Kongfig
#[derive(Serialize, Deserialize)]
struct KongfigOutput {
    pub host: String,
    pub headers: Vec<String>,
    pub apis: Vec<Api>,
    pub consumers: Vec<Consumer>,
    pub plugins: Vec<Plugin>,
    pub upstreams: Vec<Upstream>,
    pub certificates: Vec<Certificate>
}

impl KongfigOutput {
    fn new(data: KongOutput) -> Self {
        KongfigOutput {
            host: data.kong.clone().config_url.into(),
            headers: vec![],
            apis: kongfig_apis(data.apis, data.kong.clone()),
            consumers: kongfig_consumers(data.kong.clone()),
            plugins: vec![],
            upstreams: vec![],
            certificates: vec![],
        }
    }
}

/// KongOutput in CRD form
#[derive(Serialize)]
struct KongCrdOutput {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: KongOutput,
}
#[derive(Serialize)]
struct Metadata {
    name: String
}
impl KongCrdOutput {
    fn new(region: &str, data: KongOutput) -> Self {
        KongCrdOutput {
            apiVersion: "shipcat.babylontech.co.uk/v1".into(),
            kind: "KongConfig".into(),
            metadata: Metadata {
                name: format!("shipcat-kong-{}", region),
            },
            spec: data,
        }
    }
}

fn generate_kong_output(conf: &Config, region: &str) -> Result<KongOutput> {
    let mut apis = BTreeMap::new();

    // Generate list of APIs to feed to Kong
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::stubbed(&svc, conf, region)?; // does not need secrets
        if !mf.disabled && mf.regions.contains(&region.to_string()) {
            debug!("Found service {} in region {}", mf.name, region);
            if let Some(k) = mf.kong {
                apis.insert(svc, k);
            }
        }
    }

    // Add general Kong region config
    let reg = conf.regions[region].clone();
    for (name, api) in reg.kong.extra_apis.clone() {
        apis.insert(name, api);
    }
    Ok(KongOutput { apis, kong: reg.kong })
}

#[derive(Serialize, Deserialize, Debug)]
pub enum KongOutputMode {
    /// Kongfig CRD - TODO:
    Crd,
    /// Kongfig raw yaml
    Kongfig,
}

/// Generate Kong config from a filled in global config
pub fn output(conf: &Config, region: &str, mode: KongOutputMode) -> Result<()> {
    let data = generate_kong_output(conf, &region)?;
    let output = match mode {
        KongOutputMode::Crd => {
            let res = KongCrdOutput::new(region, data);
            serde_yaml::to_string(&res)?
        },
        KongOutputMode::Kongfig => {
            let res = KongfigOutput::new(data);
            serde_yaml::to_string(&res)?
        }
    };
    let _ = io::stdout().write(format!("{}\n", output).as_bytes());

    Ok(())
}

/// Return the config_url for the given region
pub fn config_url(conf: &Config, region: &str) -> Result<()> {
    let reg = conf.regions[&region.to_string()].clone();
    println!("{}", reg.kong.config_url);
    Ok(())
}
