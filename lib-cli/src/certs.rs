use p12::PFX;
use pem::Pem;
use promptly::prompt_default;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyUsagePurpose, SanType,
};
use serialport::{self, SerialPort};
use std::{
    convert::TryInto,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    thread,
};
use thiserror::Error;
use time::{ext::NumericalDuration, OffsetDateTime};

use crate::{config, ota, CertCmd, CertConfig, CertEntry, CertSubcommand};

/// Default serial port for MAC
pub const DEFAULT_MAC_PORT: &str = "/dev/tty.SLAB_USBtoUART";

/// At prefix
pub const AT_PREFIX: &str = "at ";

/// Default security tag for Pyrinas
pub const DEFAULT_PYRINAS_SECURITY_TAG: u32 = 1234;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Timeout error waiting for response from device.")]
    TimeoutError,

    #[error("{source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("{source}")]
    CertGen {
        #[from]
        source: rcgen::RcgenError,
    },

    #[error("pfx gen error")]
    PfxGen,

    #[error("cert for {name} already exists!")]
    AlreadyExists { name: String },

    /// Serde json error
    #[error("{source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    #[error("{source}")]
    ConfigError {
        #[from]
        source: config::Error,
    },

    #[error("{source}")]
    OtaError {
        #[from]
        source: ota::Error,
    },

    #[error("{source}")]
    SerialError {
        #[from]
        source: serialport::Error,
    },

    #[error("{source}")]
    PromptError {
        #[from]
        source: promptly::ReadlineError,
    },

    #[error("{source}")]
    PemError {
        #[from]
        source: pem::PemError,
    },

    #[error("err: {0}")]
    CustomError(String),
}

fn get_default_params(config: &crate::CertConfig) -> CertificateParams {
    // CA cert params
    let mut params: CertificateParams = Default::default();

    params.not_before = OffsetDateTime::now_utc();
    params.not_after = params.not_before.checked_add((365 * 4).days()).unwrap();

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

    params
}

fn write_der_credential(
    port: &mut Box<dyn SerialPort>,
    tag: u32,
    kind: u32,
    cert: &Vec<u8>,
) -> Result<(), Error> {
    // Get the reader
    let mut reader = BufReader::new(port.try_clone()?);

    // Convert to string
    let cert = hex::encode(cert);

    log::info!("{}", cert);

    // Setup to write the ca cert
    if let Err(e) = port.write_fmt(format_args!(
        "credentials set {} {} {}\r\n",
        &tag,
        &kind,
        &cert.len()
    )) {
        return Err(Error::CustomError(format!(
            "Unable to send setup command. Error: {}",
            e
        )));
    }

    log::info!("credentials set {} {} {}", &tag, &kind, &cert.len());

    // Write the raw bytes
    if let Err(e) = port.write_fmt(format_args!("{}\r\n", cert)) {
        return Err(Error::CustomError(format!(
            "Unable to send bytes. Error: {}",
            e
        )));
    }

    // Flush output
    let _ = port.flush();

    // Get the current timestamp
    let now = std::time::Instant::now();

    // Wait for "OK" response
    loop {
        if now.elapsed().as_secs() > 5 {
            return Err(Error::TimeoutError);
        }

        let mut line = String::new();
        if reader.read_line(&mut line).is_ok()
            && line.contains(&format!("Setting pyrinas/cred/{}/{} saved", tag, kind))
        {
            break;
        }
    }

    Ok(())
}

fn write_der_credentials(
    port: &mut Box<dyn SerialPort>,
    cert: &CertEntry<Vec<u8>>,
) -> Result<(), Error> {
    // First the CA cert
    if let Some(c) = &cert.ca_cert {
        write_der_credential(port, cert.tag, 1, &c)?;

        // Delay
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Then the device cert
    if let Some(c) = &cert.pub_key {
        write_der_credential(port, cert.tag, 2, &c)?;

        // Delay
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Then the private key
    if let Some(c) = &cert.private_key {
        write_der_credential(port, cert.tag, 3, &c)?;

        // Delay
        thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn write_credentials(
    port: &mut Box<dyn SerialPort>,
    cert: &CertEntry<String>,
) -> Result<(), Error> {
    // Get the reader
    let mut reader = BufReader::new(port.try_clone()?);

    // Disable modem
    if let Err(e) = port.write_fmt(format_args!("{}AT+CFUN=4\r\n", AT_PREFIX)) {
        return Err(Error::CustomError(format!(
            "Unable to disable modem. Error: {}",
            e
        )));
    }

    // Flush output
    port.flush()?;

    // Get the current timestamp
    let now = std::time::Instant::now();

    // Wait for "OK" response
    loop {
        if now.elapsed().as_secs() > 5 {
            return Err(Error::TimeoutError);
        }

        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() && line.contains("OK") {
            break;
        }
    }

    // Set the ca certificate..
    // AT%CMNG=0,16842753,0,""
    if let Some(ca_cert) = &cert.ca_cert {
        // Get command payload
        let payload = format!("AT%CMNG=0,{},0,\"{}\"", &cert.tag, &ca_cert);

        // Activate raw mode
        if let Err(e) = port.write_fmt(format_args!("{}raw {}\r\n", AT_PREFIX, payload.len())) {
            return Err(Error::CustomError(format!(
                "Unable to write CA cert. Error: {}",
                e
            )));
        }

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }

        // Write payload
        if let Err(e) = port.write_fmt(format_args!("{}", payload)) {
            return Err(Error::CustomError(format!(
                "Unable to write CA cert. Error: {}",
                e
            )));
        }

        println!("Write ca cert");

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }

            if line.contains("ERROR") {
                return Err(Error::CustomError(format!("Unable to write CA cert.")));
            }
        }

        println!("Write ca cert complete");

        // Delay
        thread::sleep(std::time::Duration::from_secs(2));
    }

    // Write the public key
    if let Some(pub_key) = &cert.pub_key {
        let payload = format!("{}AT%CMNG=0,{},1,\"{}\"", AT_PREFIX, &cert.tag, pub_key);

        // AT%CMNG=0,16842753,1,""
        if let Err(e) = port.write_fmt(format_args!("{}raw {}\r\n", AT_PREFIX, payload.len())) {
            return Err(Error::CustomError(format!(
                "Unable to write public key. Error: {}",
                e
            )));
        }

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }

        // Write payload
        if let Err(e) = port.write_fmt(format_args!("{}", payload)) {
            return Err(Error::CustomError(format!(
                "Unable to write public key. Error: {}",
                e
            )));
        }

        println!("Write pub key");

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }

            if line.contains("ERROR") {
                return Err(Error::CustomError(format!("Unable to write private key.")));
            }
        }

        println!("Write pub key complete");

        // Delay
        thread::sleep(std::time::Duration::from_secs(2));
    }

    // AT%CMNG=0,16842753,2,""
    if let Some(private_key) = &cert.private_key {
        let payload = format!("{}AT%CMNG=0,{},2,\"{}\"", AT_PREFIX, cert.tag, private_key);

        if let Err(e) = port.write_fmt(format_args!("{}raw {}\r\n", AT_PREFIX, payload.len())) {
            return Err(Error::CustomError(format!(
                "Unable to write private key. Error: {}",
                e
            )));
        }

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }

        // Write payload
        if let Err(e) = port.write_fmt(format_args!("{}", payload)) {
            return Err(Error::CustomError(format!(
                "Unable to write private key. Error: {}",
                e
            )));
        }

        println!("Write private key");

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }

            if line.contains("ERROR") {
                return Err(Error::CustomError(format!("Unable to write public key.")));
            }
        }

        println!("Write private key complete");
    }

    Ok(())
}

/// Function used to process all incoming certification generation commands
pub fn process(config: &crate::Config, c: &CertCmd) -> Result<(), Error> {
    match &c.subcmd {
        CertSubcommand::Ca => {
            generate_ca_cert(&config.cert)?;
        }
        CertSubcommand::Server => {
            generate_server_cert(&config.cert)?;
        }
        CertSubcommand::Device(cmd) => {
            let id = match cmd.id.clone() {
                Some(id) => id,
                None => {
                    // Open port
                    let mut port = serialport::new(&cmd.port, 115_200)
                        .timeout(std::time::Duration::from_millis(10))
                        .open()?;

                    let mut reader = BufReader::new(port.try_clone()?);

                    // issue AT command to get IMEI
                    port.write_fmt(format_args!("hwinfo devid\r\n"))?;

                    // Get the current timestamp
                    let now = std::time::Instant::now();

                    loop {
                        if now.elapsed().as_secs() > 5 {
                            return Err(Error::CustomError(String::from(
                                "Timeout communicating with device.",
                            )));
                        }

                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() {
                            // See if the line contains the start dialog
                            if line.contains("ID: ") {
                                break line
                                    .strip_prefix("ID: 0x")
                                    .expect("Should have had a value!")
                                    .trim_end()
                                    .to_string();
                            }
                        }
                    }
                }
            };

            // Generate cert
            let certs = match generate_device_cert(&config.cert, &id) {
                Ok(c) => c,
                Err(_e) => {
                    // Get path
                    let path = get_device_cert_path(&config.cert, &id)?;

                    println!("Cert loaded from {}.", path);

                    // Read from file
                    let file = File::open(path)?;
                    let reader = BufReader::new(file);

                    // Convert to DeviceCert
                    serde_json::from_reader(reader)?
                }
            };

            // if the provision flag is set provision it
            if cmd.provision {
                // Open port
                let mut port = serialport::new(&cmd.port, 115_200)
                    .timeout(std::time::Duration::from_millis(10))
                    .open()?;

                // confirm provision
                if prompt_default("Ready to provision to device. Continue?", false)? {
                    if cmd.der {
                        let config_path = crate::config::get_config_path()?
                            .to_string_lossy()
                            .to_string();

                        // Get the paths
                        let ca_path =
                            format!("{}/certs/{}/ca/ca.der", config_path, config.cert.domain);

                        let public_key_path = format!(
                            "{}/certs/{}/{}/{}.crt.der",
                            config_path, config.cert.domain, &id, &id
                        );

                        let private_key_path = format!(
                            "{}/certs/{}/{}/{}.key.der",
                            config_path, config.cert.domain, &id, &id
                        );

                        let e = CertEntry {
                            tag: cmd.tag.unwrap_or(DEFAULT_PYRINAS_SECURITY_TAG),
                            ca_cert: Some(fs::read(ca_path)?),
                            private_key: Some(fs::read(private_key_path)?),
                            pub_key: Some(fs::read(public_key_path)?),
                        };

                        // Convert to DER format
                        // let ca_cert = pem::parse(certs.ca_cert)?;
                        // e.ca_cert = Some(ca_cert.contents);

                        // let private_key = pem::parse(certs.private_key)?;
                        // e.private_key = Some(private_key.contents);

                        // let client_cert = pem::parse(certs.client_cert)?;
                        // e.pub_key = Some(client_cert.contents);

                        write_der_credentials(&mut port, &e)?;
                    } else {
                        // Device default cert
                        write_credentials(
                            &mut port,
                            &CertEntry {
                                tag: cmd.tag.unwrap_or(DEFAULT_PYRINAS_SECURITY_TAG),
                                ca_cert: Some(certs.ca_cert),
                                private_key: Some(certs.private_key),
                                pub_key: Some(certs.client_cert),
                            },
                        )?;
                    }

                    // Other certs as necessary
                    if let Some(alts) = &config.alts {
                        for entry in alts {
                            if cmd.der {
                                let mut e = CertEntry {
                                    tag: entry.tag,
                                    ca_cert: None,
                                    private_key: None,
                                    pub_key: None,
                                };

                                // Convert to DER format
                                if let Some(ca) = &entry.ca_cert {
                                    let ca_cert = pem::parse(ca)?;
                                    e.ca_cert = Some(ca_cert.contents);
                                }

                                if let Some(pk) = &entry.private_key {
                                    let private_key = pem::parse(pk)?;
                                    e.private_key = Some(private_key.contents);
                                }

                                if let Some(pub_key) = &entry.pub_key {
                                    let client_cert = pem::parse(pub_key)?;
                                    e.pub_key = Some(client_cert.contents);
                                }

                                write_der_credentials(&mut port, &e)?;
                            } else {
                                write_credentials(&mut port, entry)?;
                            }
                        }
                    }

                    // Reboot
                    if let Err(e) = port.write_fmt(format_args!("kernel reboot cold\r\n")) {
                        return Err(Error::CustomError(format!(
                            "Unable to reboot device. Error: {}",
                            e
                        )));
                    }

                    // Flush output
                    let _ = port.flush();

                    println!("Provisioning complete!");
                }
            }
        }
    };

    Ok(())
}

pub fn generate_ca_cert(config: &crate::CertConfig) -> Result<(), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Get the path
    let ca_der_path = format!("{}/certs/{}/ca/ca.der", config_path, config.domain);
    let ca_private_der_path = format!("{}/certs/{}/ca/ca.key.der", config_path, config.domain);
    let ca_pem_path = format!("{}/certs/{}/ca/ca.pem", config_path, config.domain);

    // Check if CA exits
    if std::path::Path::new(&ca_der_path).exists() {
        return Err(Error::AlreadyExists {
            name: "ca".to_string(),
        });
    }
    // CA cert params
    let mut params: CertificateParams = get_default_params(config);

    // This can sign things!
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));

    // Set the key usage
    params.key_usages = vec![KeyUsagePurpose::CrlSign, KeyUsagePurpose::KeyCertSign];

    // Set this to 10 years instead of default 4
    params.not_after = params.not_before.checked_add((10 * 365).days()).unwrap();

    // Make sure folder exists
    std::fs::create_dir_all(format!("{}/certs/{}/ca", config_path, config.domain))?;

    // Create ca
    let ca_cert = Certificate::from_params(params).unwrap();

    fs::write(ca_der_path, &ca_cert.serialize_der().unwrap())?;
    fs::write(ca_private_der_path, &ca_cert.serialize_private_key_der())?;
    fs::write(ca_pem_path, &ca_cert.serialize_pem().unwrap())?;

    println!("Exported CA to {}", config_path);

    Ok(())
}

fn write_device_json(
    config: &CertConfig,
    name: &str,
    cert: &Certificate,
    ca_cert: &Certificate,
    ca_der: &[u8],
) -> Result<crate::device::DeviceCert, Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Make sure there's a directory
    std::fs::create_dir_all(format!("{}/certs/{}/{}/", config_path, config.domain, name))?;

    // Serialize output
    let cert_pem = cert.serialize_pem_with_signer(ca_cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();

    // Der
    let cert_der = pem::parse(&cert_pem).unwrap().contents;
    let key_der = pem::parse(&key_pem).unwrap().contents;

    fs::write(
        format!(
            "{}/certs/{}/{}/{}.crt.der",
            config_path, config.domain, name, name
        ),
        &cert_der,
    )?;

    fs::write(
        format!(
            "{}/certs/{}/{}/{}.key.der",
            config_path, config.domain, name, name
        ),
        &key_der,
    )?;

    // Get CA cert to pem but don't keep re-signing it..
    let p = Pem {
        tag: "CERTIFICATE".to_string(),
        contents: ca_der.to_vec(),
    };

    let ca_pem = pem::encode(&p);

    // Export as JSON
    let json_device_cert = crate::device::DeviceCert {
        private_key: key_pem,
        client_cert: cert_pem,
        ca_cert: ca_pem,
        client_id: name.to_string(),
    };

    let json_output = serde_json::to_string(&json_device_cert)?;

    // Write JSON
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.json",
            config_path, config.domain, name, name
        ),
        &json_output.as_bytes(),
    )?;

    Ok(json_device_cert)
}

pub fn write_keypair_pem(
    config: &CertConfig,
    name: &str,
    cert: &Certificate,
    ca_cert: &Certificate,
) -> Result<(), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Serialize output
    let cert_pem = cert.serialize_pem_with_signer(ca_cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();

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
    name: &str,
    cert: &Certificate,
    ca_cert: &Certificate,
    ca_der: &[u8],
) -> Result<(), Error> {
    // Config path
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Path to pfx
    let ca_pfx_path = format!(
        "{}/certs/{}/{}/{}.pfx",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&ca_pfx_path).exists() {
        return Err(Error::AlreadyExists {
            name: name.to_string(),
        });
    }

    // Create directory if not already
    std::fs::create_dir_all(format!("{}/certs/{}/{}/", config_path, config.domain, name))?;

    let cert_der = cert.serialize_der_with_signer(ca_cert)?;
    let key_der = cert.serialize_private_key_der();

    // Serialize ca_der as bytes without re-signing..

    // Generate pfx file!
    let ca_pfx = PFX::new(&cert_der, &key_der, Some(ca_der), &config.pfx_pass, name)
        .ok_or(Error::PfxGen)?
        .to_der()
        .to_vec();

    // Write it
    fs::write(ca_pfx_path, ca_pfx)?;

    Ok(())
}

pub fn get_ca_cert(config: &crate::CertConfig) -> Result<(Certificate, Vec<u8>), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

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
    )?;

    // Return the cert or error
    Ok((Certificate::from_params(ca_cert_params)?, ca_cert_der))
}

pub fn generate_server_cert(config: &crate::CertConfig) -> Result<(), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();
    let name = "server".to_string();

    let server_cert_path = format!(
        "{}/certs/{}/{}/{}.pfx",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&server_cert_path).exists() {
        return Err(Error::AlreadyExists { name });
    }

    // Get CA Cert
    let (ca_cert, ca_der) = get_ca_cert(config)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Set the key usage
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    // Set the ext key useage
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    // Set the alt name
    params.subject_alt_names = vec![SanType::DnsName(config.domain.clone())];

    // Make the cert
    let cert = Certificate::from_params(params)?;

    // Write cert to file(s)
    write_keypair_pem(config, &name, &cert, &ca_cert)?;

    // Write pfx
    write_pfx(config, &name, &cert, &ca_cert, &ca_der)?;

    println!("Exported server .pfx to {}", config_path);

    Ok(())
}

pub fn get_device_cert_path(config: &crate::CertConfig, name: &str) -> Result<String, Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    Ok(format!(
        "{}/certs/{}/{}/{}.json",
        config_path, config.domain, name, name
    ))
}

pub fn generate_device_cert(
    config: &crate::CertConfig,
    name: &str,
) -> Result<crate::device::DeviceCert, Error> {
    let device_cert_path = get_device_cert_path(config, name)?;

    // Check if it exists
    if std::path::Path::new(&device_cert_path).exists() {
        return Err(Error::AlreadyExists {
            name: name.to_string(),
        });
    }

    // Get CA Cert
    let (ca_cert, ca_der) = get_ca_cert(config)?;

    // Cert params
    let mut params: CertificateParams = get_default_params(config);

    // Set the key usage
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];

    // Set the ext key useage
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
    // write_keypair_pem(&config, &name, &cert, &ca_cert)?;

    // Write nRF Connect Desktop compatable JSON for cert install
    let certs = write_device_json(config, name, &cert, &ca_cert, &ca_der)?;

    println!("Exported cert for {} to {}", name, device_cert_path);

    Ok(certs)
}
