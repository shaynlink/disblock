use std::sync::Arc;

use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("bot token encryption failed")]
    Encrypt,
    #[error("bot token decryption failed: wrong key or tampered data")]
    Decrypt,
}

/// Encrypts bot tokens at rest with AES-256-GCM.
///
/// The bot id is bound as associated data, so a ciphertext copied onto another
/// bot row does not decrypt.
#[derive(Clone)]
pub struct TokenCipher {
    cipher: Arc<Aes256Gcm>,
}

pub struct Sealed {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl TokenCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: Arc::new(Aes256Gcm::new(key.into())),
        }
    }

    pub fn seal(&self, bot_id: Uuid, token: &str) -> Result<Sealed, CryptoError> {
        let nonce = Nonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: token.as_bytes(),
                    aad: bot_id.as_bytes(),
                },
            )
            .map_err(|_| CryptoError::Encrypt)?;
        Ok(Sealed {
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    pub fn open(
        &self,
        bot_id: Uuid,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<String, CryptoError> {
        let nonce = Nonce::try_from(nonce).map_err(|_| CryptoError::Decrypt)?;
        let plaintext = self
            .cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad: bot_id.as_bytes(),
                },
            )
            .map_err(|_| CryptoError::Decrypt)?;
        String::from_utf8(plaintext).map_err(|_| CryptoError::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_binds_the_bot_id() {
        let cipher = TokenCipher::new(&[7; 32]);
        let bot = Uuid::new_v4();
        let sealed = cipher.seal(bot, "secret-token").expect("seal");
        assert_eq!(
            cipher
                .open(bot, &sealed.nonce, &sealed.ciphertext)
                .expect("open"),
            "secret-token"
        );
        assert!(
            cipher
                .open(Uuid::new_v4(), &sealed.nonce, &sealed.ciphertext)
                .is_err()
        );
    }
}
