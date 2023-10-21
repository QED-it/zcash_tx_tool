use std::convert::TryFrom;
use std::error::Error;
use std::io;
use orchard::{Bundle};
use orchard::bundle::Authorized;
use orchard::primitives::redpallas::SpendAuth;
use zcash_note_encryption::ShieldedOutput;
use zebra_chain::amount::Amount;
use zebra_chain::orchard::{Action, AuthorizedAction, Flags, Nullifier, ShieldedData, ValueCommitment, WrappedNoteKey};
use zebra_chain::orchard::keys::EphemeralPublicKey;
use zebra_chain::orchard::tree::Root;
use zebra_chain::primitives::{Halo2Proof, reddsa};
use zebra_chain::primitives::reddsa::Signature;
use zebra_chain::serialization::AtLeastOne;
use zebra_chain::orchard::EncryptedNote;



pub fn convert_orchard_bundle_to_shielded_data(bundle: Bundle<Authorized, Amount>) -> Result<ShieldedData, io::Error> {
    Ok(ShieldedData {
        flags: Flags::from_bits(bundle.flags().to_byte())?,
        value_balance: *bundle.value_balance(),
        shared_anchor: Root::try_from(bundle.anchor().to_bytes())?, // TODO change pallas point visibility in Orchard to avoid serialization loop
        proof: Halo2Proof(bundle.authorization().proof().as_ref().to_vec()),
        actions:bundle.actions().iter().map(|orchard_action| convert_orchard_action_to_shielded_data_action(orchard_action)).collect::<Result<AtLeastOne<AuthorizedAction>, Error>>()?,
        binding_sig: Signature::from(bundle.authorization().binding_signature().into()), // TODO same, change visibility in Orchard
    })
}

fn convert_orchard_action_to_shielded_data_action(action: &orchard::Action<orchard::primitives::redpallas::Signature<SpendAuth>>) -> Result<AuthorizedAction, Box<dyn Error>> {
    Ok(AuthorizedAction {
        action: Action {
            cv: ValueCommitment::try_from(action.cv_net().to_bytes())?,
            nullifier: Nullifier::try_from(action.nullifier().to_bytes())?,
            rk: reddsa::VerificationKeyBytes::from(action.rk().into()),
            cm_x: Default::default(),
            ephemeral_key: EphemeralPublicKey::try_from(action.ephemeral_key().0)?,
            enc_ciphertext: EncryptedNote(*action.enc_ciphertext()),
            out_ciphertext: WrappedNoteKey::from(action.encrypted_note().out_ciphertext),
        },
        spend_auth_sig: action.authorization().0
    })
}