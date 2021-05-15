use anyhow::anyhow;
use chrono::{offset::Utc, Datelike};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    SanType,
};
use std::{convert::TryInto, fs};

use crate::CertConfig;

fn get_default_params(config: &crate::CertConfig) -> CertificateParams {
    // CA cert params
    let mut params: CertificateParams = Default::default();

    params.not_before = Utc::now();
    params.not_after = params
        .not_before
        .with_year(params.not_before.year() + 4)
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

    params
}

pub fn generate_ca_cert(config: &crate::CertConfig) -> anyhow::Result<()> {
    // Get the path
    let ca_pem_path = format!("_certs/{}/ca/ca.pem", config.domain);
    let ca_der_path = format!("_certs/{}/ca/ca.der", config.domain);
    let ca_private_der_path = format!("_certs/{}/ca/ca.key.der", config.domain);

    // Check if CA exits
    if !std::path::Path::new(&ca_pem_path).exists() {
        // CA cert params
        let mut params: CertificateParams = get_default_params(config);

        // Set this to 10 years instead of default 4
        params.not_after = params
            .not_before
            .with_year(params.not_before.year() + 10)
            .unwrap();

        // Make sure folder exists
        std::fs::create_dir_all(format!("_certs/{}/ca", config.domain))?;

        // Create ca
        let ca_cert = Certificate::from_params(params).unwrap();
        println!("{}", ca_cert.serialize_pem().unwrap());
        fs::write(ca_pem_path, &ca_cert.serialize_pem().unwrap().as_bytes())?;
        fs::write(ca_der_path, &ca_cert.serialize_der().unwrap())?;
        fs::write(ca_private_der_path, &ca_cert.serialize_private_key_der())?;

        // TODO: generate pfx file!
    } else {
        println!("CA cert for {} already exists!", config.domain);
    }

    Ok(())
}

fn write_device_json(
    config: &CertConfig,
    name: &String,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> anyhow::Result<()> {
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
    std::fs::create_dir_all(format!("_certs/{}/{}/", config.domain, name))?;

    // Write JSON
    fs::write(
        format!("_certs/{}/{}/{}.json", config.domain, name, name),
        &json_output.as_bytes(),
    )?;

    Ok(())
}

fn write_cert(
    config: &CertConfig,
    name: &String,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> anyhow::Result<()> {
    // Serialize output
    let cert_pem = cert.serialize_pem_with_signer(ca_cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();
    let key_der = cert.serialize_private_key_der();

    // Display certs
    println!("{}", cert_pem);
    println!("{}", key_pem);

    // Create directory if not already
    std::fs::create_dir_all(format!("_certs/{}/{}/", config.domain, name))?;

    // Write to files
    fs::write(
        format!("_certs/{}/{}/{}.pem", config.domain, name, name),
        &cert_pem.as_bytes(),
    )?;
    fs::write(
        format!("_certs/{}/{}/{}.key", config.domain, name, name),
        &key_pem.as_bytes(),
    )?;
    fs::write(
        format!("_certs/{}/{}/{}.der", config.domain, name, name),
        &key_der,
    )?;

    Ok(())
}

pub fn get_ca_cert(config: &crate::CertConfig, name: &String) -> anyhow::Result<Certificate> {
    // Check if it exists
    let ca_cert_pem_path = format!("_certs/{}/{}/{}.pem", config.domain, name, name);
    if std::path::Path::new(&ca_cert_pem_path).exists() {
        panic!("{} already exists!", ca_cert_pem_path);
    }

    // Load CA
    let path = format!("_certs/{}/ca/ca.der", config.domain);
    let ca_cert_der = match fs::read(path.clone()) {
        Ok(d) => d,
        Err(_) => panic!("{} not found. Generate CA first!", path),
    };

    let path = format!("_certs/{}/ca/ca.key.der", config.domain);
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

pub fn generate_server_cert(config: &crate::CertConfig) -> anyhow::Result<()> {
    let name = "server".to_string();

    let server_cert_path = format!("_certs/{}/{}/{}.pem", config.domain, name, name);

    // Check if it exists
    if std::path::Path::new(&server_cert_path).exists() {
        return Err(anyhow!("Server cert for {} already exists!", config.domain));
    }

    // Get CA Cert
    let ca_cert = get_ca_cert(config, &name)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Set the alt name
    params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];

    // Make the cert
    let cert = Certificate::from_params(params)?;

    // Write cert to file(s)
    write_cert(&config, &name, &cert, &ca_cert)?;

    Ok(())
}

pub fn generate_device_cert(config: &crate::CertConfig, name: &String) -> anyhow::Result<()> {
    let device_cert_path = format!("_certs/{}/{}/{}.pem", config.domain, name, name);

    // Check if it exists
    if std::path::Path::new(&device_cert_path).exists() {
        return Err(anyhow!(
            "Device cert for {} on {} already exists!",
            name,
            config.domain
        ));
    }

    // Get CA Cert
    let ca_cert = get_ca_cert(config, name)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Only for client usage
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];

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
