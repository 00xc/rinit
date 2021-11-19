use std::{
    env,
    fs,
    io,
    path::{
        Path,
        PathBuf,
    },
};

use serde::{
    Deserialize,
    Serialize,
};
use snafu::{
    ensure,
    ResultExt,
    Snafu,
};
use xdg::BaseDirectories;

static DEFAULT_PATH: &str = "/sbin:/bin:/usr/sbin:/usr/bin";
static DEFAULT_CONFIG_PLACEHOLDER: &str = "%%default_config%%";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub path: Option<PathBuf>,
    pub configdir: Option<PathBuf>,
    pub rundir: Option<PathBuf>,
    pub datadir: Option<PathBuf>,
    pub service_directories: Vec<PathBuf>,
    pub profile_name: Option<String>,
}

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display("unable to initialize XDG BaseDirectories: {}", source))]
    BaseDirectoriesError { source: xdg::BaseDirectoriesError },
    #[snafu(display("unable to find configuration directory {:?}", configdir))]
    ConfigDirNotFound { configdir: PathBuf },
    #[snafu(display("unable to read configuration file {:?}: {}", config_path, source))]
    ConfigReadError {
        config_path: PathBuf,
        source: io::Error,
    },
    #[snafu(display("unable to parse configuration file {:?}: {}", config_path, source))]
    ConfigFormatError {
        config_path: PathBuf,
        source: toml::de::Error,
    },
    #[snafu(display("unable to convert {:?} to string", path))]
    StringConversionError { path: PathBuf },
}

type Result<T, E = ConfigError> = std::result::Result<T, E>;

impl Config {
    pub fn merge(
        &mut self,
        config: Config,
    ) {
        if config.path.is_some() {
            self.path = config.path;
        }
        if config.rundir.is_some() {
            self.rundir = config.rundir;
        }
        if !config.service_directories.is_empty() {
            self.service_directories = config.service_directories;
        }
        if config.profile_name.is_some() {
            self.profile_name = config.profile_name;
        }
    }

    pub fn new(configdir: Option<String>) -> Result<Self> {
        let uid = unsafe { libc::getuid() };

        let mut config = if !Path::new("kansei.conf").exists() && configdir.is_none() {
            if uid == 0 {
                Self::new_default_config()
            } else {
                let xdg: BaseDirectories =
                    BaseDirectories::with_prefix("kansei").context(BaseDirectoriesError {})?;

                Self::new_user_config(&xdg)?
            }
            // Merge the config read from the default locations
        } else {
            let config_path = if let Some(configdir) = &configdir {
                let configdir = Path::new(configdir);
                ensure!(configdir.exists(), ConfigDirNotFound { configdir });
                // if !configdir.is_dir() {
                //     bail!("path {:?} is not a directory", configdir);
                // }
                configdir.join("kansei.conf")
            } else {
                Path::new("kansei.conf").to_path_buf()
            };
            toml::from_str(&fs::read_to_string(&config_path).with_context(|| {
                ConfigReadError {
                    config_path: config_path.clone(),
                }
            })?)
            .with_context(|| {
                ConfigFormatError {
                    config_path: config_path.clone(),
                }
            })?
        };

        // replace configdir placeholder with actual config dir
        // the user might avoid hard writing the configdir
        let configdir = &config.configdir.as_ref().unwrap();
        let new_arr = config
            .service_directories
            .into_iter()
            .map(|dir| -> Result<PathBuf> {
                Ok(if dir.as_os_str() == DEFAULT_CONFIG_PLACEHOLDER {
                    configdir.join("service")
                } else {
                    dir
                })
            })
            .collect::<Result<_>>()?;

        config.service_directories = new_arr;

        Ok(config)
    }

    fn new_default_config() -> Self {
        Config {
            path: Some(PathBuf::from(
                env::var("PATH").unwrap_or_else(|_| DEFAULT_PATH.to_string()),
            )),
            configdir: Some(PathBuf::from("/etc/kansei")),
            rundir: Some(PathBuf::from("/run/kansei")),
            datadir: Some(PathBuf::from("/var/lib/kansei")),
            service_directories: vec![
                PathBuf::from("/etc/kansei/service"),
                PathBuf::from("/usr/share/kansei/service"),
            ],
            profile_name: None,
        }
    }

    fn new_user_config(xdg: &BaseDirectories) -> Result<Self> {
        Ok(Config {
            path: Some(PathBuf::from(
                env::var("PATH").unwrap_or_else(|_| DEFAULT_PATH.to_string()),
            )),
            configdir: { Some(xdg.get_config_home()) },
            rundir: Some(
                xdg.get_runtime_directory()
                    .context(BaseDirectoriesError {})?
                    .to_path_buf(),
            ),
            datadir: { Some(xdg.get_data_home()) },
            service_directories: vec![
                PathBuf::from(DEFAULT_CONFIG_PLACEHOLDER),
                PathBuf::from("/usr/share/kansei/service"),
            ],
            profile_name: None,
        })
    }
}
