//! In order to support a variety of key exchange protocols [Ockam][main-ockam-crate-link] crate uses an abstract Key Exchange trait.
//!
//! This crate provides an implementation of Key Exchange using [Noise][noise-protocol-framework] protocol with XX pattern.
//! [noise-protocol-framework]: http://www.noiseprotocol.org/noise.html
//!
//! The main [Ockam][main-ockam-crate-link] has optional dependency on this crate.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

mod error;

pub use error::*;

/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE_U32: u32 = 32;
/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE_USIZE: usize = 32;

/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE_U32: u32 = 16;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE_USIZE: usize = 16;

/// Vault with XX required functionality
pub trait XXVault:
    EphemeralSecretsStore + AsymmetricVault + SymmetricVault + Send + Sync + 'static
{
}

impl<D> XXVault for D where
    D: SecretsStore + AsymmetricVault + SymmetricVault + Send + Sync + 'static
{
}

/// Vault with required functionalities after XX key exchange
pub trait XXInitializedVault:
    EphemeralSecretsStore + SymmetricVault + Send + Sync + 'static
{
}

impl<D> XXInitializedVault for D where
    D: EphemeralSecretsStore + SymmetricVault + Send + Sync + 'static
{
}

mod initiator;
mod state;
pub use initiator::*;
mod responder;
pub use responder::*;
mod new_key_exchanger;
pub use new_key_exchanger::*;
use ockam_vault::{AsymmetricVault, EphemeralSecretsStore, SecretsStore, SymmetricVault};

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Result;
    use ockam_core::{KeyExchanger, NewKeyExchanger};
    use ockam_node::Context;
    use ockam_vault::{EphemeralSecretsStore, Vault};

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn full_flow__correct_credential__keys_should_match(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let key_exchanger = XXNewKeyExchanger::new(vault.clone());

        let mut initiator = key_exchanger.initiator(None).await.unwrap();
        let mut responder = key_exchanger.responder(None).await.unwrap();

        loop {
            if !initiator.is_complete().await.unwrap() {
                let m = initiator.generate_request(&[]).await.unwrap();
                let _ = responder.handle_response(&m).await.unwrap();
            }

            if !responder.is_complete().await.unwrap() {
                let m = responder.generate_request(&[]).await.unwrap();
                let _ = initiator.handle_response(&m).await.unwrap();
            }

            if initiator.is_complete().await.unwrap() && responder.is_complete().await.unwrap() {
                break;
            }
        }

        let initiator = initiator.finalize().await.unwrap();
        let responder = responder.finalize().await.unwrap();

        assert_eq!(initiator.h(), responder.h());

        let s1 = vault
            .get_ephemeral_secret(initiator.encrypt_key(), "encrypt key")
            .await
            .unwrap();
        let s2 = vault
            .get_ephemeral_secret(responder.decrypt_key(), "decrypt key")
            .await
            .unwrap();

        assert_eq!(s1, s2);

        let s1 = vault
            .get_ephemeral_secret(initiator.decrypt_key(), "decrypt key")
            .await
            .unwrap();
        let s2 = vault
            .get_ephemeral_secret(responder.encrypt_key(), "encrypt key")
            .await
            .unwrap();

        assert_eq!(s1, s2);

        ctx.stop().await
    }
}
