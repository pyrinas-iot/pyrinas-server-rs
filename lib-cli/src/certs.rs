use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::{X509NameBuilder, X509Ref, X509Req, X509ReqBuilder, X509};

use promptly::prompt_default;
use serialport::{self, SerialPort};
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader},
    str, thread, time,
};
use thiserror::Error;

use crate::{config, ota, CertCmd, CertConfig, CertEntry, CertSubcommand};

/// Default serial port for MAC
pub const DEFAULT_MAC_PORT: &str = "/dev/tty.SLAB_USBtoUART";

/// Default security tag for Pyrinas
pub const DEFAULT_PYRINAS_SECURITY_TAG: u32 = 1234;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{source}")]
    FileError {
        #[from]
        source: io::Error,
    },

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

    #[error("err: {0}")]
    CustomError(String),
}

fn mk_ca_cert(
    config: &crate::CertConfig,
) -> Result<(X509, PKey<Private>, Rsa<Private>), ErrorStack> {
    let rsa = Rsa::generate(2048)?;
    let key_pair = PKey::from_rsa(rsa.clone())?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", &config.country)?;
    x509_name.append_entry_by_text("ST", &config.state)?;
    x509_name.append_entry_by_text("O", &config.organization)?;
    x509_name.append_entry_by_text("CN", &config.domain)?;
    let x509_name = x509_name.build();

    let mut cert_builder = X509::builder()?;
    cert_builder.set_version(2)?;
    let serial_number = {
        let mut serial = BigNum::new()?;
        serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
        serial.to_asn1_integer()?
    };
    cert_builder.set_serial_number(&serial_number)?;
    cert_builder.set_subject_name(&x509_name)?;
    cert_builder.set_issuer_name(&x509_name)?;
    cert_builder.set_pubkey(&key_pair)?;
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    let not_after = Asn1Time::days_from_now(365 * 5)?;
    cert_builder.set_not_after(&not_after)?;

    cert_builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
    cert_builder.append_extension(
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()?,
    )?;

    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(None, None))?;
    cert_builder.append_extension(subject_key_identifier)?;

    cert_builder.sign(&key_pair, MessageDigest::sha256())?;
    let cert = cert_builder.build();

    Ok((cert, key_pair, rsa))
}

/// Make a X509 request with the given private key
fn mk_request(config: &crate::CertConfig, key_pair: &PKey<Private>) -> Result<X509Req, ErrorStack> {
    let mut req_builder = X509ReqBuilder::new()?;
    req_builder.set_pubkey(key_pair)?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", &config.country)?;
    x509_name.append_entry_by_text("ST", &config.state)?;
    x509_name.append_entry_by_text("O", &config.organization)?;
    x509_name.append_entry_by_text("CN", &config.domain)?;
    let x509_name = x509_name.build();
    req_builder.set_subject_name(&x509_name)?;

    req_builder.sign(key_pair, MessageDigest::sha256())?;
    let req = req_builder.build();
    Ok(req)
}

/// Make a certificate and private key signed by the given CA cert and private key
fn mk_ca_signed_cert(
    config: &crate::CertConfig,
    ca_cert: &X509Ref,
    ca_key_pair: &PKeyRef<Private>,
) -> Result<(X509, PKey<Private>, Rsa<Private>), ErrorStack> {
    let rsa = Rsa::generate(2048)?;
    let key_pair = PKey::from_rsa(rsa.clone())?;

    let req = mk_request(config, &key_pair)?;

    let mut cert_builder = X509::builder()?;
    cert_builder.set_version(2)?;
    let serial_number = {
        let mut serial = BigNum::new()?;
        serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
        serial.to_asn1_integer()?
    };
    cert_builder.set_serial_number(&serial_number)?;
    cert_builder.set_subject_name(req.subject_name())?;
    cert_builder.set_issuer_name(ca_cert.subject_name())?;
    cert_builder.set_pubkey(&key_pair)?;
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    let not_after = Asn1Time::days_from_now(365 * 5)?;
    cert_builder.set_not_after(&not_after)?;

    cert_builder.append_extension(BasicConstraints::new().build()?)?;

    cert_builder.append_extension(
        KeyUsage::new()
            .critical()
            .non_repudiation()
            .digital_signature()
            .key_encipherment()
            .build()?,
    )?;

    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(subject_key_identifier)?;

    let auth_key_identifier = AuthorityKeyIdentifier::new()
        .keyid(false)
        .issuer(false)
        .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(auth_key_identifier)?;

    let subject_alt_name = SubjectAlternativeName::new()
        .dns(&config.domain)
        .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
    cert_builder.append_extension(subject_alt_name)?;

    cert_builder.sign(ca_key_pair, MessageDigest::sha256())?;
    let cert = cert_builder.build();

    Ok((cert, key_pair, rsa))
}

fn write_credential(port: &mut Box<dyn SerialPort>, cert: &CertEntry) -> Result<(), Error> {
    // Get the reader
    let mut reader = BufReader::new(port.try_clone()?);

    // Set the ca certificate..
    // AT%CMNG=0,16842753,0,""
    if let Some(ca_cert) = &cert.ca_cert {
        if let Err(e) = port.write_fmt(format_args!(
            "AT%CMNG=0,{},0,\"{}\"\r\n",
            &cert.tag, &ca_cert
        )) {
            return Err(Error::CustomError(format!(
                "Unable to write CA cert. Error: {}",
                e
            )));
        }

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }

        // Delay
        thread::sleep(time::Duration::from_secs(2));
    }

    // Write the public key
    if let Some(pub_key) = &cert.pub_key {
        // AT%CMNG=0,16842753,1,""
        if let Err(e) = port.write_fmt(format_args!(
            "AT%CMNG=0,{},1,\"{}\"\r\n",
            &cert.tag, pub_key
        )) {
            return Err(Error::CustomError(format!(
                "Unable to write client cert. Error: {}",
                e
            )));
        }

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }

        // Delay
        thread::sleep(time::Duration::from_secs(2));
    }

    // AT%CMNG=0,16842753,2,""
    if let Some(private_key) = &cert.private_key {
        if let Err(e) = port.write_fmt(format_args!(
            "AT%CMNG=0,{},2,\"{}\"\r\n",
            cert.tag, private_key
        )) {
            //
            return Err(Error::CustomError(format!(
                " Unable to write private key. Error: {}",
                e
            )));
        }

        // Flush output
        let _ = port.flush();

        // Wait for "OK" response
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("OK") {
                break;
            }
        }
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
                    let mut port = match serialport::new(&cmd.port, 115_200)
                        .timeout(time::Duration::from_millis(10))
                        .open()
                    {
                        Ok(p) => p,
                        Err(_) => {
                            eprintln!(
                                "Port {} is not connected! Check your connections and try again.",
                                &cmd.port
                            );
                            return Ok(());
                        }
                    };

                    let mut reader = BufReader::new(port.try_clone()?);

                    // issue AT command to get IMEI
                    port.write_fmt(format_args!("AT+CGSN=1?\r\n"))?;

                    // Get the current timestamp
                    let now = time::Instant::now();

                    loop {
                        if now.elapsed().as_secs() > 5 {
                            return Err(Error::CustomError(String::from(
                                "Timeout communicating with device.",
                            )));
                        }

                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() {
                            // See if the line contains the start dialog
                            if line.contains("+CGSN: ") {
                                break line
                                    .strip_prefix("+CGSN: ")
                                    .expect("Should have had a value!")
                                    .trim_end()
                                    .trim_matches('\"')
                                    .to_string();
                            } else if line.contains("at_host: Error") {
                                return Err(Error::CustomError(String::from(
                                    "AT error communicating with device.",
                                )));
                            }
                        }
                    }
                }
            };

            // Generate cert
            let certs = match generate_device_cert(&config.cert, &id) {
                Ok(c) => c,
                Err(_e) => {
                    println!("Cert for {} already generated!", &id);

                    // Get path
                    let path = get_device_cert_path(&config.cert, &id)?;

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
                    .timeout(time::Duration::from_millis(10))
                    .open()?;

                // confirm provision
                if prompt_default("Ready to provision to device. Continue?", false)? {
                    // Device default cert
                    write_credential(
                        &mut port,
                        &CertEntry {
                            tag: cmd.tag.unwrap_or(DEFAULT_PYRINAS_SECURITY_TAG),
                            ca_cert: Some(certs.ca_cert),
                            private_key: Some(certs.private_key),
                            pub_key: Some(certs.public_key),
                        },
                    )?;

                    // Other certs as necessary
                    if let Some(alts) = &config.alts {
                        for entry in alts {
                            write_credential(&mut port, entry)?;
                        }
                    }

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
    let ca_private_key_path = format!("{}/certs/{}/ca/ca.key", config_path, config.domain);
    let ca_crt_path = format!("{}/certs/{}/ca/ca.crt", config_path, config.domain);

    // Check if CA exits
    if std::path::Path::new(&ca_crt_path).exists() {
        return Err(Error::AlreadyExists {
            name: "ca".to_string(),
        });
    }
    // Create CA cert
    let (crt, _, rsa_key_pair) = mk_ca_cert(config).unwrap();

    // Make sure folder exists
    std::fs::create_dir_all(format!("{}/certs/{}/ca", config_path, config.domain))?;

    // Write ca
    fs::write(
        ca_private_key_path,
        &rsa_key_pair.private_key_to_pem().unwrap(),
    )?;
    fs::write(ca_crt_path, &crt.to_pem().unwrap())?;

    println!("Exported CA to {}/certs/{}/ca", config_path, config.domain);

    Ok(())
}

fn write_device_json(
    config: &CertConfig,
    name: &str,
    cert: &Rsa<Private>,
    ca_cert: &X509,
) -> Result<crate::device::DeviceCert, Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Get string pem for each
    let private_key = String::from_utf8(cert.private_key_to_pem().unwrap()).unwrap();
    let public_key = String::from_utf8(cert.public_key_to_pem().unwrap()).unwrap();
    let ca_cert = String::from_utf8(ca_cert.to_pem().unwrap()).unwrap();

    // Export as JSON
    let json_device_cert = crate::device::DeviceCert {
        private_key,
        public_key,
        ca_cert,
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

    Ok(json_device_cert)
}

pub fn write_keypair_pem(
    config: &CertConfig,
    name: &str,
    cert: &PKey<Private>,
) -> Result<(), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Create directory if not already
    std::fs::create_dir_all(format!("{}/certs/{}/{}/", config_path, config.domain, name))?;

    // Write to files
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.pem",
            config_path, config.domain, name, name
        ),
        &cert.public_key_to_pem().unwrap(),
    )?;
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.key",
            config_path, config.domain, name, name
        ),
        &cert.private_key_to_pem_pkcs8().unwrap(),
    )?;

    Ok(())
}

pub fn get_ca_cert(config: &crate::CertConfig) -> Result<(X509, PKey<Private>), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();

    // Load CA
    let path = format!("{}/certs/{}/ca/ca.crt", config_path, config.domain);
    let ca_cert_pem = match fs::read(path.clone()) {
        Ok(d) => X509::from_pem(&d).unwrap(),
        Err(_) => panic!("{} not found. Generate CA first!", path),
    };

    // Load CA key pair
    let path = format!("{}/certs/{}/ca/ca.key", config_path, config.domain);
    let ca_key_pair = match fs::read(path.clone()) {
        Ok(d) => PKey::private_key_from_pem(&d).unwrap(),
        Err(_) => panic!("{} not found. Generate CA first!", path),
    };

    // Return the cert or error
    Ok((ca_cert_pem, ca_key_pair))
}

// TODO:
pub fn generate_server_cert(config: &crate::CertConfig) -> Result<(), Error> {
    let config_path = crate::config::get_config_path()?
        .to_string_lossy()
        .to_string();
    let name = "server".to_string();

    let server_cert_path = format!(
        "{}/certs/{}/{}/{}.pem",
        config_path, config.domain, name, name
    );

    // Check if it exists
    if std::path::Path::new(&server_cert_path).exists() {
        return Err(Error::AlreadyExists { name });
    }

    // Get CA cert from file
    let (ca_cert, ca_key_pair) = get_ca_cert(config).unwrap();

    // Generate keypair
    let (_crt, _, rsa_key_pair) = mk_ca_signed_cert(config, &ca_cert, &ca_key_pair).unwrap();

    // Make sure folder exists
    std::fs::create_dir_all(format!("{}/certs/{}/{}", config_path, config.domain, name))?;

    // Write to files
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.pem",
            config_path, config.domain, name, name
        ),
        &rsa_key_pair.public_key_to_pem().unwrap(),
    )?;
    fs::write(
        format!(
            "{}/certs/{}/{}/{}.key",
            config_path, config.domain, name, name
        ),
        &rsa_key_pair.private_key_to_pem().unwrap(),
    )?;

    println!(
        "Exported server cert to {}/certs/{}/{}",
        config_path, config.domain, name
    );

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

    // Get CA cert from file
    let (ca_cert, ca_key_pair) = get_ca_cert(config).unwrap();

    // Generate keypair
    let (_, _, rsa_key_pair) = mk_ca_signed_cert(config, &ca_cert, &ca_key_pair).unwrap();

    // Write nRF Connect Desktop compatable JSON for cert install
    let certs = write_device_json(config, name, &rsa_key_pair, &ca_cert)?;

    println!("Exported cert for {} to {}", name, device_cert_path);

    Ok(certs)
}
