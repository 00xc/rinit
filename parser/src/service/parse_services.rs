use std::path::{
    Path,
    PathBuf,
};

use rinit_service::types::Service;
use snafu::{
    ensure,
    ResultExt,
    Snafu,
};

use crate::{
    parse_service,
    ParseServiceError,
};

#[derive(Snafu, Debug)]
pub enum ServicesParserError {
    #[snafu(display("unable to parse service"))]
    ParsingServiceError { source: ParseServiceError },
    #[snafu(display("could not find service file for {:?}", service))]
    CouldNotFindService { service: String },
    #[snafu(display(
        "the service name is different than the file name for {:?}",
        service_file
    ))]
    NameNotMatchingFile { service_file: PathBuf },
}

unsafe impl Send for ServicesParserError {}
unsafe impl Sync for ServicesParserError {}

pub fn parse_services(
    services: Vec<String>,
    service_dirs: &[PathBuf],
    system: bool,
) -> Result<Vec<Service>, ServicesParserError> {
    let mut services_already_parsed = services.clone();
    let mut results = Vec::new();
    let mut to_parse = services
        .into_iter()
        .map(|service| {
            // If we don't find the services passed as args on the system, return an error
            if let Some(file) = get_service_file(&service, service_dirs, system) {
                Ok((service, file))
            } else {
                Err(ServicesParserError::CouldNotFindService { service })
            }
        })
        .collect::<Result<Vec<(String, PathBuf)>, ServicesParserError>>()?;

    while let Some((name, file)) = to_parse.pop() {
        let service = parse_service(&file).context(ParsingServiceSnafu {})?;
        ensure!(
            service.name() == name,
            NameNotMatchingFileSnafu { service_file: file }
        );
        let mut dependencies: Vec<String> = service.dependencies().into();

        results.push(service);

        // Skip services that we can't found, the dependency graph will
        // handle the error
        to_parse.extend(dependencies.iter().filter_map(|service| {
            if let Some(file) = get_service_file(service, service_dirs, system) {
                Some((service.clone(), file))
            } else {
                None
            }
        }));

        services_already_parsed.append(&mut dependencies);
    }

    Ok(results)
}

fn get_service_file(
    service: &str,
    paths: &[PathBuf],
    system: bool,
) -> Option<PathBuf> {
    paths.iter().find_map(|path| {
        let service_file =
            path.join(Path::new(service).with_extension(if system { "system" } else { "user" }));
        if service_file.exists() {
            Some(service_file)
        } else {
            None
        }
    })
}