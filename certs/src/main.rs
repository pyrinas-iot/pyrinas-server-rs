extern crate clap;
extern crate rcgen;

mod config;
mod device;

use clap::{App, Arg, SubCommand};

use chrono::{offset::Utc, Datelike};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    SanType,
};
use std::{convert::TryInto, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: ca cert creation or reading it from disk
    // TODO: command line arguments

    let matches = App::new("")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jared Wolff <hello@jaredwolff.com>")
        .about("Generates certs according to an attached file.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("ca").about("generate CA cert"))
        .subcommand(SubCommand::with_name("server").about("generate server certs"))
        .subcommand(
            SubCommand::with_name("device")
                .about("generate device certs")
                .arg(Arg::with_name("name").help("name of the device. (usually uid)")),
        )
        .get_matches();

    // Config
    let config = matches.value_of("config").unwrap_or("config.toml");

    // Open file
    let config = match fs::read(config) {
        Ok(d) => d,
        Err(e) => panic!("Error reading config. Error: {}", e),
    };

    // Deseralize config
    let config: config::Config = match toml::from_slice(&config) {
        Ok(c) => c,
        Err(e) => panic!("Unable to decode config file. Error: {}", e),
    };

    // Determine which command
    if let Some(_) = matches.subcommand_matches("ca") {
        // Get the path
        let ca_pem_path = format!("_certs/{}/ca.pem", config.domain);
        let ca_der_path = format!("_certs/{}/ca.der", config.domain);
        let ca_private_der_path = format!("_certs/{}/ca.key.der", config.domain);

        // Check if CA exits
        if !std::path::Path::new(&ca_pem_path).exists() {
            // CA cert params
            let mut params: CertificateParams = Default::default();

            params.not_before = Utc::now();
            params.not_after = params
                .not_before
                .with_year(params.not_before.year() + 10)
                .unwrap();

            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

            params
                .distinguished_name
                .push(DnType::CountryName, config.country.clone());
            params
                .distinguished_name
                .push(DnType::OrganizationName, config.organization.clone());
            params
                .distinguished_name
                .push(DnType::CommonName, config.domain.clone());

            params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];

            params.extended_key_usages = vec![
                ExtendedKeyUsagePurpose::ServerAuth,
                ExtendedKeyUsagePurpose::ClientAuth,
            ];

            // Make sure folder exists
            std::fs::create_dir_all(format!("_certs/{}", config.domain))?;

            // Create ca
            let ca_cert = Certificate::from_params(params).unwrap();
            println!("{}", ca_cert.serialize_pem().unwrap());
            fs::write(ca_pem_path, &ca_cert.serialize_pem().unwrap().as_bytes())?;
            fs::write(ca_der_path, &ca_cert.serialize_der().unwrap())?;
            fs::write(ca_private_der_path, &ca_cert.serialize_private_key_der())?;
        } else {
            println!("Cert already exists!");
        }
    }

    // Create separate device cert using CA
    if let Some(args) = matches.subcommand_matches("device") {
        // Make sure we have name
        let name = match args.value_of("name") {
            Some(n) => n,
            None => panic!("Name must be provided for device"),
        };

        // Check if it exists
        let ca_cert_pem_path = format!("_certs/{}/{}.pem", config.domain, name);
        if std::path::Path::new(&ca_cert_pem_path).exists() {
            panic!("{} already exists!", ca_cert_pem_path);
        }

        // Load CA
        let path = format!("_certs/{}/ca.der", config.domain);
        let ca_cert_der = match fs::read(path.clone()) {
            Ok(d) => d,
            Err(_) => panic!("{} not found. Generate CA first!", path),
        };

        let path = format!("_certs/{}/ca.key.der", config.domain);
        let ca_cert_key_der = match fs::read(path.clone()) {
            Ok(d) => d,
            Err(_) => panic!("{} not found. Generate CA first!", path),
        };

        // Import the CA
        let ca_cert_params = CertificateParams::from_ca_cert_der(
            ca_cert_der.as_slice(),
            ca_cert_key_der.as_slice().try_into()?,
        )
        .expect("certificate params");
        let ca_cert = Certificate::from_params(ca_cert_params)?;

        // CA cert params
        let mut params: CertificateParams = Default::default();

        params.not_before = Utc::now();
        params.not_after = params
            .not_before
            .with_year(params.not_before.year() + 4)
            .unwrap();

        params
            .distinguished_name
            .push(DnType::CountryName, config.country.clone());
        params
            .distinguished_name
            .push(DnType::OrganizationName, config.organization.clone());
        params
            .distinguished_name
            .push(DnType::CommonName, config.domain.clone());

        // if matches.subcommand_matches("server") {
        //     params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];
        // } else {
        //     params.subject_alt_names = vec![SanType::Rfc822Name(format!(
        //         "{}@{}",
        //         name,
        //         config.domain.clone()
        //     ))];
        // }

        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // Make the cert
        let cert = Certificate::from_params(params)?;

        // Serialize output
        let cert_pem = cert.serialize_pem_with_signer(&ca_cert).unwrap();
        let key_pem = cert.serialize_private_key_pem();
        let key_der = cert.serialize_private_key_der();

        // Display certs
        println!("{}", cert_pem);
        println!("{}", key_pem);

        // Write to files
        std::fs::create_dir_all(format!("_certs/{}", config.domain))?;
        fs::write(
            format!("_certs/{}/{}.pem", config.domain, name),
            &cert_pem.as_bytes(),
        )?;
        fs::write(
            format!("_certs/{}/{}.key", config.domain, name),
            &key_pem.as_bytes(),
        )?;
        fs::write(format!("_certs/{}/{}.der", config.domain, name), &key_der)?;

        // TODO: export as JSON format as well.
        let json_device_cert = device::DeviceCert {
            private_key: key_pem.clone(),
            client_cert: cert_pem.clone(),
            ca_cert: ca_cert.serialize_pem().unwrap(),
            client_id: name.to_string(),
        };

        let json_output = serde_json::to_string(&json_device_cert)?;

        fs::write(
            format!("_certs/{}/{}.json", config.domain, name),
            &json_output.as_bytes(),
        )?;
    }

    Ok(())
}
