use chrono::{Datelike, Utc};
use p12::PFX;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, SanType,
};
use std::{convert::TryFrom, fs, io};
use thiserror::Error;

use crate::CertConfig;

#[derive(Debug, Error)]
pub enum CertsError {
    #[error("filesystem error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("rcgen error: {source}")]
    CertGen {
        #[from]
        source: rcgen::RcgenError,
    },

    #[error("pfx gen error")]
    PfxGen,

    #[error("cert for {name} already exists!")]
    AlreadyExists { name: String },

    /// Serde json error
    #[error("serde json error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    /// Error from CLI portion of code
    #[error("cli error: {source}")]
    CliError {
        #[from]
        source: crate::CliError,
    },
}

fn get_default_params(config: &crate::CertConfig) -> CertificateParams {
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

    params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];

    params.use_authority_key_identifier_extension = true;

    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];

    params
}

pub fn generate_ca_cert(config: &crate::CertConfig) -> Result<(), CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    // Get the path
    let ca_pem_path = format!("{}/certs/{}/ca/ca.pem", config_path, config.domain);
    let ca_der_path = format!("{}/certs/{}/ca/ca.der", config_path, config.domain);
    let ca_private_der_path = format!("{}/certs/{}/ca/ca.key.der", config_path, config.domain);

    // Check if CA exits
    if std::path::Path::new(&ca_pem_path).exists() {
        return Err(CertsError::AlreadyExists {
            name: "ca".to_string(),
        });
    }
    // CA cert params
    let mut params: CertificateParams = get_default_params(config);

    // This can sign things!
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    // Server mode only?
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::Any];

    // Set this to 10 years instead of default 4
    params.not_after = params
        .not_before
        .with_year(params.not_before.year() + 10)
        .unwrap();

    // Make sure folder exists
    std::fs::create_dir_all(format!("{}/certs/{}/ca", config_path, config.domain))?;

    // Create ca
    let ca_cert = Certificate::from_params(params).unwrap();
    println!("{}", ca_cert.serialize_pem().unwrap());
    fs::write(ca_pem_path, &ca_cert.serialize_pem().unwrap().as_bytes())?;
    fs::write(ca_der_path, &ca_cert.serialize_der().unwrap())?;
    fs::write(ca_private_der_path, &ca_cert.serialize_private_key_der())?;

    Ok(())
}

fn write_device_json(
    config: &CertConfig,
    name: &String,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> Result<(), CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    // Serialize output
    let cert_pem = cert.serialize_pem_with_signer(ca_cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();
    let ca_cert = ca_cert.serialize_pem().unwrap();

    // TODO: export as JSON format as well.
    let json_device_cert = crate::device::DeviceCert {
        private_key: key_pem,
        client_cert: cert_pem,
        ca_cert: ca_cert,
        client_id: name.to_string(),
    };

    let json_output = serde_json::to_string(&json_device_cert)?;

    // Make sure there's a directory
    std::fs::create_dir_all(format!("{}/certs/{}/{}/", config_path, config.domain, name))?;

    // Write JSON
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.json",
            config_path, config.domain, name, name
        ),
        &json_output.as_bytes(),
    )?;

    Ok(())
}

pub fn write_cert(
    config: &CertConfig,
    name: &String,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> Result<(), CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    // Serialize output
    let cert_pem = cert.serialize_pem_with_signer(ca_cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();

    // Display certs
    println!("{}", cert_pem);
    println!("{}", key_pem);

    // Create directory if not already
    std::fs::create_dir_all(format!("{}/certs/{}/{}/", config_path, config.domain, name))?;

    // Write to files
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.pem",
            config_path, config.domain, name, name
        ),
        &cert_pem.as_bytes(),
    )?;
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.key",
            config_path, config.domain, name, name
        ),
        &key_pem.as_bytes(),
    )?;

    Ok(())
}

fn write_pfx(
    config: &CertConfig,
    name: &String,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> Result<(), CertsError> {
    // Config path
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    // Path to pfx
    let ca_pfx_path = format!(
        "{}/certs/{}/{}/{}.pfx",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&ca_pfx_path).exists() {
        return Err(CertsError::AlreadyExists {
            name: name.to_string(),
        });
    }

    let cert_der = cert.serialize_der_with_signer(&ca_cert)?;
    let key_der = cert.serialize_private_key_der();

    // Generate pfx file!
    let ca_pfx = PFX::new(
        &cert_der,
        &key_der,
        Some(&ca_cert.serialize_der()?),
        &config.pfx_pass,
        &name,
    )
    .ok_or(CertsError::PfxGen)?
    .to_der()
    .to_vec();

    // Write it
    fs::write(ca_pfx_path, ca_pfx)?;

    Ok(())
}

pub fn get_ca_cert(config: &crate::CertConfig) -> Result<Certificate, CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    // Load CA
    let path = format!("{}/certs/{}/ca/ca.der", config_path, config.domain);
    let ca_cert_der = match fs::read(path.clone()) {
        Ok(d) => d,
        Err(_) => panic!("{} not found. Generate CA first!", path),
    };

    let path = format!("{}/certs/{}/ca/ca.key.der", config_path, config.domain);
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

    // Return the cert or error
    Ok(Certificate::from_params(ca_cert_params)?)
}

pub fn generate_server_cert(config: &crate::CertConfig) -> Result<(), CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();
    let name = "server".to_string();

    let server_cert_path = format!(
        "{}/certs/{}/{}/{}.pem",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&server_cert_path).exists() {
        return Err(CertsError::AlreadyExists {
            name: name.to_string(),
        });
    }

    // Get CA Cert
    let ca_cert = get_ca_cert(config)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Server auth only
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    // Set the alt name
    params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];

    // Make the cert
    let cert = Certificate::from_params(params)?;

    // Write cert to file(s)
    write_cert(&config, &name, &cert, &ca_cert)?;

    // Write pfx
    write_pfx(&config, &name, &cert, &ca_cert)?;

    Ok(())
}

pub fn generate_device_cert(config: &crate::CertConfig, name: &String) -> Result<(), CertsError> {
    let config_path = crate::get_config_path()?.to_string_lossy().to_string();

    let device_cert_path = format!(
        "{}/certs/{}/{}/{}.pem",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&device_cert_path).exists() {
        return Err(CertsError::AlreadyExists {
            name: name.to_string(),
        });
    }

    // Get CA Cert
    let ca_cert = get_ca_cert(config)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Set the alt name
    params.subject_alt_names = vec![SanType::Rfc822Name(format!(
        "{}@{}",
        name,
        config.domain.clone()
    ))];

    // Make the cert
    let cert = Certificate::from_params(params)?;

    // Write all cert info to file(s)
    write_cert(&config, &name, &cert, &ca_cert)?;

    // Write nRF Connect Desktop compatable JSON for cert install
    write_device_json(&config, &name, &cert, &ca_cert)?;

    Ok(())
}
