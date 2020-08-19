mod handler;

pub use rs_libc::alloc::*;

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
};

use parity_scale_codec::{Decode, Encode};
use rustls::{NoClientAuth, ServerConfig, ServerSession, StreamOwned};
use thread_pool::ThreadPool;

use enclave_protocol::{
    DecryptionRequest, TxQueryInitRequest, TxQueryInitResponse, ENCRYPTION_REQUEST_SIZE,
};
use ra_enclave::DEFAULT_EXPIRATION_SECS;
use ra_enclave::{EnclaveRaConfig, EnclaveRaContext};

use self::handler::{
    get_random_challenge, handle_decryption_request, handle_encryption_request,
    verify_decryption_request,
};
use chrono::Duration;

pub fn entry(cert_expiration: Option<Duration>) -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    log::info!("Connecting to chain-abci data");
    let chain_data_stream = Arc::new(Mutex::new(TcpStream::connect("chain-abci-data")?));
    let stream_to_txvalidation =
        Arc::new(Mutex::new(TcpStream::connect("stream_to_txvalidation")?));

    // FIXME: connect to tx-validation (mutually attested TLS)
    let num_threads = 4;

    // use the smaller Duration as certificate validity, so that we can check the certification is expired or not correctly
    let default_expiration_time = Duration::seconds(DEFAULT_EXPIRATION_SECS);
    let certificate_validity = match cert_expiration {
        None => default_expiration_time,
        Some(s) => s.min(default_expiration_time),
    };
    let config = EnclaveRaConfig {
        sp_addr: "ra-sp-server".to_string(),
        certificate_validity_secs: certificate_validity.num_seconds() as u32,
        certificate_expiration_time: cert_expiration,
    };

    let context = Arc::new(
        EnclaveRaContext::new(&config).expect("Unable to create new remote attestation context"),
    );

    log::info!("Successfully created remote attestation certificate!");
    log::info!("Starting TLS Server");

    let listener = TcpListener::bind("tx-query")?;

    let (thread_pool_sender, thread_pool) = ThreadPool::fixed_size(num_threads);

    for stream in listener.incoming() {
        let context = context.clone();
        let chain_data_stream = chain_data_stream.clone();
        let stream_to_txvalidation = stream_to_txvalidation.clone();

        thread_pool_sender
            .send(move || {
                let certificate = context
                    .get_certificate()
                    .expect("Unable to create remote attestation certificate");
                let mut tls_server_config = ServerConfig::new(NoClientAuth::new());
                certificate
                    .configure_server_config(&mut tls_server_config)
                    .expect("Unable to create TLS server config");
                tls_server_config.versions = vec![rustls::ProtocolVersion::TLSv1_3];

                let tls_server_config = Arc::new(tls_server_config);

                let tls_session = ServerSession::new(&tls_server_config);
                let stream = StreamOwned::new(tls_session, stream.unwrap());

                handle_connection(stream, chain_data_stream, stream_to_txvalidation);
            })
            .expect("Unable to send tasks to thread pool");
    }

    thread_pool.shutdown();
    Ok(())
}

fn handle_connection<T: Read + Write>(
    mut stream: T,
    chain_data_stream: Arc<Mutex<TcpStream>>,
    stream_to_txvalidation: Arc<Mutex<TcpStream>>,
) {
    let mut bytes = vec![0u8; ENCRYPTION_REQUEST_SIZE];

    // read user's request
    match stream.read(&mut bytes) {
        Ok(len) => {
            match TxQueryInitRequest::decode(&mut &bytes.as_slice()[0..len]) {
                Ok(TxQueryInitRequest::Encrypt(request)) => {
                    let response = handle_encryption_request(request, len, chain_data_stream);
                    // encrypt directly
                    // let response = handle_encryption_request(request, len, stream_to_txvalidation);

                    let response = match response {
                        Ok(response) => response,
                        Err(message) => {
                            log::error!("Error while handling encryption request: {}", message);
                            return;
                        }
                    };

                    if let Err(err) = stream.write_all(&response.encode()) {
                        log::error!(
                            "Error while writing encryption response back to TLS stream: {}",
                            err
                        );
                    }
                }
                Ok(TxQueryInitRequest::DecryptChallenge) => {
                    let challenge = get_random_challenge();

                    if let Err(err) =
                        stream.write_all(&TxQueryInitResponse::DecryptChallenge(challenge).encode())
                    {
                        log::error!("Unable to write random challenge to TLS stream: {}", err);
                        return;
                    }

                    match stream.read(&mut bytes) {
                        Ok(len) => {
                            match DecryptionRequest::decode(&mut &bytes.as_slice()[0..len]) {
                                Ok(decryption_request) => {
                                    if !verify_decryption_request(&decryption_request, challenge) {
                                        log::error!("Decryption request is invalid");
                                        return;
                                    }

                                    match handle_decryption_request(
                                        &decryption_request,
                                        chain_data_stream,
                                    ) {
                                        Ok(decryption_response) => {
                                            if let Err(err) =
                                                stream.write_all(&decryption_response.encode())
                                            {
                                                log::error!("Error while writing decryption response back to TLS stream: {}", err);
                                            }
                                        }
                                        Err(err) => log::error!(
                                            "Error while handling decryption request: {}",
                                            err
                                        ),
                                    }
                                }
                                Err(err) => {
                                    log::error!("Unable to decode decryption request: {}", err)
                                }
                            }
                        }
                        Err(err) => {
                            log::error!(
                                "Unable to read challenge response from TLS stream: {}",
                                err
                            );
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error while decoding tx-query init request: {}", err);
                }
            };
        }
        Err(err) => log::error!("Error while reading bytes from TLS stream: {}", err),
    }
}
