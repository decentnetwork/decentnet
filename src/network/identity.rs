use libp2p::{
    identity::{self, ed25519},
    noise::{AuthenticKeypair, Keypair, NoiseError, X25519Spec},
    rendezvous::client::Behaviour as RendezvousClientBehaviour,
    rendezvous::server::Behaviour as RendezvousServerBehaviour,
    rendezvous::server::Config as RendezvousServerConfig,
};

use super::{IdentityImpl, NetworkId, RendezvousBehaviour};

// Used to provide Unique Node Id across Network.
// Services can use this to identify users or
// they can use their own store to provide unique names across their service.
#[derive(Clone)]
pub struct Network {
    keypair: identity::Keypair,
}

impl<'a> IdentityImpl<'a> for Network {
    type PublicKey = identity::PublicKey;
    fn id(&self) -> NetworkId {
        NetworkId::from_public_key(&self.keypair.public())
    }

    fn public_key(&self) -> Self::PublicKey {
        self.keypair.public()
    }

    fn gen_random_id() -> Self {
        let private_key = ed25519::SecretKey::generate();
        let keypair = ed25519::Keypair::from(private_key);
        let keypair = identity::Keypair::Ed25519(keypair);
        Self { keypair }
    }

    fn gen_random_id_with_private() -> (ed25519::SecretKey, Self) {
        let private_key = ed25519::SecretKey::generate();
        let keypair = ed25519::Keypair::from(private_key.clone());
        let keypair = identity::Keypair::Ed25519(keypair);
        (private_key, Self { keypair })
    }

    fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
        let private_key = ed25519::SecretKey::from_bytes(bytes).unwrap();
        let keypair = ed25519::Keypair::from(private_key);
        let keypair = identity::Keypair::Ed25519(keypair);
        Self { keypair }
    }

    fn auth_key_pair(&self) -> Result<AuthenticKeypair<X25519Spec>, NoiseError> {
        Keypair::<X25519Spec>::new().into_authentic(&self.keypair)
    }
}

pub trait RendezvousBehaviourImpl {
    fn rendezvous_behaviour(&self, is_server: bool) -> RendezvousBehaviour;
}

impl RendezvousBehaviourImpl for Network {
    fn rendezvous_behaviour(&self, is_server: bool) -> RendezvousBehaviour {
        if is_server {
            RendezvousBehaviour::Server(RendezvousServerBehaviour::new(
                RendezvousServerConfig::default(),
            ))
        } else {
            RendezvousBehaviour::Client(RendezvousClientBehaviour::new(self.keypair.clone()))
        }
    }
}

//This is the struct that will be used to represent a software client in the network
pub struct Peer {
    // keypair: identity::Keypair,
}

// Used to get Network Strength, Metrics, Update System, Relay(Only) Mode.
// impl Network {
//     pub fn id(&self) -> String {
//         NetworkId::from_public_key(self.public()).to_base58()
//     }

//     pub fn public(&self) -> identity::PublicKey {
//         self.keypair.public()
//     }

//     fn gen_random_id() -> Self {
//         let private_key = ed25519::SecretKey::generate();
//         let keypair = ed25519::Keypair::from(private_key.clone());
//         let keypair = identity::Keypair::Ed25519(keypair.clone());
//         Network { keypair }
//     }

//     fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
//         let private_key = ed25519::SecretKey::from_bytes(bytes).unwrap();
//         let keypair = ed25519::Keypair::from(private_key);
//         let keypair = identity::Keypair::Ed25519(keypair);
//         Network { keypair }
//     }
// }

// This is representation for websites and apps, typical ZeroNet Style.
// Implement a Initial App Update Service.
// struct Service {
//     keypair: identity::Keypair,
// }

// impl Identity for Service {
//     fn id(&self) -> String {
//         NetworkId::from_public_key(self.keypair.public()).to_base58()
//     }

//     fn gen_random() -> Self {
//         let private_key = ed25519::SecretKey::generate();
//         let keypair = ed25519::Keypair::from(private_key.clone());
//         let keypair = identity::Keypair::Ed25519(keypair.clone());
//         Service { keypair }
//     }

//     fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
//         let private_key = ed25519::SecretKey::from_bytes(bytes).unwrap();
//         let keypair = ed25519::Keypair::from(private_key);
//         let keypair = identity::Keypair::Ed25519(keypair);
//         Service { keypair }
//     }
// }
