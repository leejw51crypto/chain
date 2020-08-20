use std::{
    convert::TryInto,
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use parity_scale_codec::{Decode, Encode};

use chain_core::tx::{data::input::TxoSize, TransactionId, TxEnclaveAux};
use enclave_protocol::{
    EnclaveRequest, EnclaveResponse, EncryptionRequest, EncryptionResponse, QueryEncryptRequest,
};
use enclave_utils::SealedData;

// read length, read binary
pub fn read_binary_buffer(this_stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut response_len = [0u8; 4];
    let response_len: usize = u32::from_le_bytes(response_len)
        .try_into()
        .map_err(|_| "Response length exceeds `usize` bounds".to_owned())?;
    if response_len == 0 {
        return Err("Unexpected response from chain-abci".to_owned());
    }
    let mut result_buf = vec![0u8; response_len];
    this_stream
        .read(&mut result_buf)
        .map_err(|err| format!("Error while reading response from chain-abci: {}", err))?;
    Ok(result_buf)
}
#[allow(clippy::boxed_local)]
pub fn handle_encryption_request(
    encryption_request: Box<EncryptionRequest>,
    request_len: usize,
    chain_data_stream: Arc<Mutex<TcpStream>>,
) -> Result<EncryptionResponse, String> {
    let request = construct_request(&*encryption_request, request_len);

    match request {
        None => Err("Failed to seal request data".to_owned()),
        Some(request) => {
            // Prepare enclave request
            let enclave_request = EnclaveRequest::EncryptTx(Box::new(request)).encode();

            let mut chain_data_stream = chain_data_stream.lock().unwrap();

            // Send request to chain-abci
            chain_data_stream
                .write_all(&enclave_request)
                .map_err(|err| format!("Error while writing request to chain-abci: {}", err))?;

            // Read reponse length from chain-abci (little endian u32 bytes)
            let mut response_len = [0u8; 4];
            chain_data_stream.read(&mut response_len).map_err(|err| {
                format!(
                    "Error while reading reponse length from chain-abci: {}",
                    err
                )
            })?;

            let response_len: usize = u32::from_le_bytes(response_len)
                .try_into()
                .map_err(|_| "Response length exceeds `usize` bounds".to_owned())?;
            if response_len == 0 {
                return Err("Unexpected response from chain-abci".to_owned());
            }
            // Read result from chain-abci
            let mut result_buf = vec![0u8; response_len];
            chain_data_stream
                .read(&mut result_buf)
                .map_err(|err| format!("Error while reading response from chain-abci: {}", err))?;

            match EnclaveResponse::decode(&mut result_buf.as_ref()) {
                Ok(EnclaveResponse::EncryptTx(enclave_response)) => {
                    let encryption_response = match enclave_response {
                        Ok(payload) => {
                            let tx = match *encryption_request {
                                EncryptionRequest::TransferTx(tx, _) => {
                                    let inputs = tx.inputs;
                                    let no_of_outputs = tx.outputs.len() as TxoSize;
                                    TxEnclaveAux::TransferTx {
                                        inputs,
                                        no_of_outputs,
                                        payload,
                                    }
                                }
                                EncryptionRequest::DepositStake(tx, _) => {
                                    TxEnclaveAux::DepositStakeTx { tx, payload }
                                }
                                EncryptionRequest::WithdrawStake(tx, witness) => {
                                    let no_of_outputs = tx.outputs.len() as TxoSize;
                                    TxEnclaveAux::WithdrawUnbondedStakeTx {
                                        no_of_outputs,
                                        witness,
                                        payload,
                                    }
                                }
                            };

                            EncryptionResponse { resp: Ok(tx) }
                        }
                        Err(e) => EncryptionResponse { resp: Err(e) },
                    };

                    Ok(encryption_response)
                }
                Ok(_) => Err("Unexpected response from chain-abci".to_owned()),
                Err(err) => Err(format!(
                    "Error while decoding response from chain-abci: {}",
                    err
                )),
            }
        }
    }
}

fn construct_request(req: &EncryptionRequest, req_len: usize) -> Option<QueryEncryptRequest> {
    let (txid, sealed, tx_inputs, tx_size, op_sig) = match req {
        // TODO: are the size estimates ok?
        EncryptionRequest::TransferTx(tx, _) => {
            let txid = tx.id();
            let sealed = SealedData::seal(&req.encode(), txid).ok();
            let tx_inputs = Some(tx.inputs.clone());
            (
                txid,
                sealed,
                tx_inputs,
                req_len + 34 * tx.inputs.len() + 74,
                None,
            )
        }
        EncryptionRequest::DepositStake(tx, _) => {
            let txid = tx.id();
            let sealed = SealedData::seal(&req.encode(), txid).ok();
            let tx_inputs = Some(tx.inputs.clone());
            (
                txid,
                sealed,
                tx_inputs,
                req_len + 34 * tx.inputs.len() + 74,
                None,
            )
        }
        EncryptionRequest::WithdrawStake(tx, witness) => {
            let txid = tx.id();
            let sealed = SealedData::seal(&req.encode(), txid).ok();
            (txid, sealed, None, req_len + 73, Some(witness.clone()))
        }
    };
    sealed.map(|sealed_enc_request| QueryEncryptRequest {
        txid,
        sealed_enc_request,
        tx_inputs,
        // TODO: checks, but this should fit, as all things are bounded more like u16::max
        tx_size: tx_size as u32,
        op_sig,
    })
}
