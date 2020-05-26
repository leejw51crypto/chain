use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Quote;

pub static OID_EXTENSION_ATTESTATION_REPORT: &[u64] = &[2, 16, 840, 1, 113_730, 1, 13];

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum EnclaveQuoteStatus {
    Ok,
    SignatureInvalid,
    GroupRevoked,
    SignatureRevoked,
    KeyRevoked,
    SigrlVersionMismatch,
    GroupOutOfDate,
    ConfigurationNeeded,
    SwHardeningNeeded,
    ConfigurationAndSwHardeningNeeded,
}

impl FromStr for EnclaveQuoteStatus {
    type Err = EnclaveQuoteStatusParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OK" => Ok(Self::Ok),
            "SIGNATURE_INVALID" => Ok(Self::SignatureInvalid),
            "GROUP_REVOKED" => Ok(Self::GroupRevoked),
            "SIGNATURE_REVOKED" => Ok(Self::SignatureRevoked),
            "KEY_REVOKED" => Ok(Self::KeyRevoked),
            "SIGRL_VERSION_MISMATCH" => Ok(Self::SigrlVersionMismatch),
            "GROUP_OUT_OF_DATE" => Ok(Self::GroupOutOfDate),
            "CONFIGURATION_NEEDED" => Ok(Self::ConfigurationNeeded),
            "SW_HARDENING_NEEDED" => Ok(Self::SwHardeningNeeded),
            "CONFIGURATION_AND_SW_HARDENING_NEEDED" => Ok(Self::ConfigurationAndSwHardeningNeeded),
            _ => Err(EnclaveQuoteStatusParsingError::InvalidStatus(s.to_owned())),
        }
    }
}

#[derive(Debug, Error)]
pub enum EnclaveQuoteStatusParsingError {
    #[error("Invalid enclave quote status: {0}")]
    InvalidStatus(String),
}

/// Attestation verification report body returned by IAS to SP
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationReportBody {
    pub id: String,
    pub timestamp: String,
    pub version: u8,
    pub isv_enclave_quote_status: String,
    pub isv_enclave_quote_body: String,
    pub revocation_reason: Option<u64>,
    pub pse_manifest_status: Option<String>,
    pub pse_manifest_hash: Option<String>,
    pub platform_info_blob: Option<String>,
    pub nonce: Option<String>,
    pub epid_pseudonym: Option<String>,
    pub advisory_url: Option<String>,
    pub advisory_ids: Option<Vec<String>>,
}

impl AttestationReportBody {
    /// Returns quote in attestation report body
    pub fn get_quote(&self) -> Result<Quote, QuoteParsingError> {
        let quote_bytes = base64::decode(&self.isv_enclave_quote_body)?;
        Quote::try_copy_from(&quote_bytes).ok_or_else(|| QuoteParsingError::InvalidQuoteStructure)
    }
}

#[derive(Debug, Error)]
pub enum QuoteParsingError {
    #[error("Unable to decode base64 encoded quote: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("Invalid quote structure")]
    InvalidQuoteStructure,
}

/// Attestation verification report (containing report body, signature and signing certificate)
#[derive(Debug, Serialize, Deserialize)]
pub struct AttestationReport {
    /// Report body (This can be converted into `AttestationReportBody` using `serde_json::from_slice`)
    pub body: Vec<u8>,
    /// Report signature
    pub signature: Vec<u8>,
    /// Report signing certificate
    pub signing_cert: Vec<u8>,
}
