use crate::TelegramTarget;
use aes::Aes256;
use ctr::cipher::{KeyIvInit, StreamCipher};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const HANDSHAKE_LEN: usize = 64;
const PREKEY_START: usize = 8;
const IV_END: usize = 56;
const PROTOCOL_END: usize = 60;
const DC_END: usize = 62;

type Aes256Ctr = ctr::Ctr128BE<Aes256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProtocol {
    Abridged,
    Intermediate,
    PaddedIntermediate,
}

#[derive(Debug, ZeroizeOnDrop)]
pub struct ClientHandshake {
    #[zeroize(skip)]
    pub target: TelegramTarget,
    #[zeroize(skip)]
    pub protocol: TransportProtocol,
    prekey_iv: [u8; 48],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("invalid client handshake")]
    InvalidHandshake,
    #[error("unsupported Telegram data center")]
    UnsupportedDataCenter,
}

pub struct BridgeCrypto {
    client_decrypt: Aes256Ctr,
    client_encrypt: Aes256Ctr,
    telegram_encrypt: Aes256Ctr,
    telegram_decrypt: Aes256Ctr,
}

pub struct OutboundCrypto {
    client_decrypt: Aes256Ctr,
    telegram_encrypt: Aes256Ctr,
}

pub struct InboundCrypto {
    client_encrypt: Aes256Ctr,
    telegram_decrypt: Aes256Ctr,
}

pub struct MessageSplitter {
    decrypt: Aes256Ctr,
    protocol: TransportProtocol,
    cipher_buffer: Vec<u8>,
    header: Zeroizing<Vec<u8>>,
    expected_packet_length: Option<usize>,
    received_packet_bytes: usize,
}

impl Drop for MessageSplitter {
    fn drop(&mut self) {
        self.cipher_buffer.zeroize();
    }
}

impl MessageSplitter {
    pub fn from_relay(
        relay_init: &[u8; HANDSHAKE_LEN],
        protocol: TransportProtocol,
    ) -> Result<Self, ProtocolError> {
        let mut decrypt = Aes256Ctr::new_from_slices(&relay_init[8..40], &relay_init[40..56])
            .map_err(|_| ProtocolError::InvalidHandshake)?;
        let mut skip = [0u8; HANDSHAKE_LEN];
        decrypt.apply_keystream(&mut skip);
        Ok(Self {
            decrypt,
            protocol,
            cipher_buffer: Vec::new(),
            header: Zeroizing::new(Vec::with_capacity(4)),
            expected_packet_length: None,
            received_packet_bytes: 0,
        })
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, ProtocolError> {
        const MAX_BUFFER_BYTES: usize = 16 * 1024 * 1024;
        let mut plain = Zeroizing::new(chunk.to_vec());
        self.decrypt.apply_keystream(plain.as_mut());
        let mut parts = Vec::new();
        for (cipher_byte, plain_byte) in chunk.iter().copied().zip(plain.iter().copied()) {
            self.cipher_buffer.push(cipher_byte);
            self.received_packet_bytes += 1;

            if self.expected_packet_length.is_none() {
                self.header.push(plain_byte);
                self.expected_packet_length = packet_length(self.protocol, &self.header)?;
                if self
                    .expected_packet_length
                    .is_some_and(|length| length > MAX_BUFFER_BYTES)
                {
                    return Err(ProtocolError::InvalidHandshake);
                }
            }

            let Some(expected) = self.expected_packet_length else {
                continue;
            };
            if self.received_packet_bytes > expected {
                return Err(ProtocolError::InvalidHandshake);
            }
            if self.received_packet_bytes == expected {
                parts.push(std::mem::take(&mut self.cipher_buffer));
                self.header.zeroize();
                self.header.clear();
                self.expected_packet_length = None;
                self.received_packet_bytes = 0;
            }
        }
        Ok(parts)
    }
}

fn packet_length(
    protocol: TransportProtocol,
    header: &[u8],
) -> Result<Option<usize>, ProtocolError> {
    let (header_length, payload_length) = match protocol {
        TransportProtocol::Abridged => {
            let Some(first) = header.first().copied() else {
                return Ok(None);
            };
            if matches!(first, 0x7f | 0xff) {
                if header.len() < 4 {
                    return Ok(None);
                }
                (
                    4usize,
                    u32::from_le_bytes([header[1], header[2], header[3], 0]) as usize * 4,
                )
            } else {
                (1usize, usize::from(first & 0x7f) * 4)
            }
        }
        TransportProtocol::Intermediate | TransportProtocol::PaddedIntermediate => {
            if header.len() < 4 {
                return Ok(None);
            }
            let raw = u32::from_le_bytes(
                header[..4]
                    .try_into()
                    .map_err(|_| ProtocolError::InvalidHandshake)?,
            );
            (4usize, (raw & 0x7fff_ffff) as usize)
        }
    };
    if payload_length == 0 {
        return Err(ProtocolError::InvalidHandshake);
    }
    Ok(Some(header_length.saturating_add(payload_length)))
}

impl BridgeCrypto {
    pub fn from_client(
        client: &ClientHandshake,
        secret: &[u8; 16],
        relay_random: [u8; HANDSHAKE_LEN],
    ) -> Result<(Self, [u8; HANDSHAKE_LEN]), ProtocolError> {
        let client_decrypt_key = Sha256::new()
            .chain_update(&client.prekey_iv[..32])
            .chain_update(secret)
            .finalize();
        let mut client_decrypt =
            Aes256Ctr::new_from_slices(&client_decrypt_key, &client.prekey_iv[32..])
                .map_err(|_| ProtocolError::InvalidHandshake)?;

        let mut reversed_client = client.prekey_iv;
        reversed_client.reverse();
        let client_encrypt_key = Sha256::new()
            .chain_update(&reversed_client[..32])
            .chain_update(secret)
            .finalize();
        let client_encrypt =
            Aes256Ctr::new_from_slices(&client_encrypt_key, &reversed_client[32..])
                .map_err(|_| ProtocolError::InvalidHandshake)?;

        let protocol_tag = match client.protocol {
            TransportProtocol::Abridged => [0xef; 4],
            TransportProtocol::Intermediate => [0xee; 4],
            TransportProtocol::PaddedIntermediate => [0xdd; 4],
        };
        let dc_index = if client.target.media {
            -(client.target.dc as i16)
        } else {
            client.target.dc as i16
        };
        let mut relay_tail = [0u8; 8];
        relay_tail[..4].copy_from_slice(&protocol_tag);
        relay_tail[4..6].copy_from_slice(&dc_index.to_le_bytes());
        relay_tail[6..].copy_from_slice(&relay_random[62..64]);

        let mut relay_stream =
            Aes256Ctr::new_from_slices(&relay_random[8..40], &relay_random[40..56])
                .map_err(|_| ProtocolError::InvalidHandshake)?;
        let mut relay_keystream = [0u8; HANDSHAKE_LEN];
        relay_stream.apply_keystream(&mut relay_keystream);
        let mut relay_init = relay_random;
        for (index, byte) in relay_tail.iter().enumerate() {
            relay_init[56 + index] = byte ^ relay_keystream[56 + index];
        }

        let mut telegram_encrypt =
            Aes256Ctr::new_from_slices(&relay_init[8..40], &relay_init[40..56])
                .map_err(|_| ProtocolError::InvalidHandshake)?;
        let mut reversed_relay = [0u8; 48];
        reversed_relay.copy_from_slice(&relay_init[8..56]);
        reversed_relay.reverse();
        let telegram_decrypt =
            Aes256Ctr::new_from_slices(&reversed_relay[..32], &reversed_relay[32..])
                .map_err(|_| ProtocolError::InvalidHandshake)?;

        let mut skip = [0u8; HANDSHAKE_LEN];
        client_decrypt.apply_keystream(&mut skip);
        telegram_encrypt.apply_keystream(&mut skip);

        Ok((
            Self {
                client_decrypt,
                client_encrypt,
                telegram_encrypt,
                telegram_decrypt,
            },
            relay_init,
        ))
    }

    pub fn client_to_upstream(&mut self, chunk: &[u8]) -> Vec<u8> {
        let mut output = chunk.to_vec();
        self.client_decrypt.apply_keystream(&mut output);
        self.telegram_encrypt.apply_keystream(&mut output);
        output
    }

    pub fn upstream_to_client(&mut self, chunk: &[u8]) -> Vec<u8> {
        let mut output = chunk.to_vec();
        self.telegram_decrypt.apply_keystream(&mut output);
        self.client_encrypt.apply_keystream(&mut output);
        output
    }

    pub fn split(self) -> (OutboundCrypto, InboundCrypto) {
        (
            OutboundCrypto {
                client_decrypt: self.client_decrypt,
                telegram_encrypt: self.telegram_encrypt,
            },
            InboundCrypto {
                client_encrypt: self.client_encrypt,
                telegram_decrypt: self.telegram_decrypt,
            },
        )
    }
}

impl OutboundCrypto {
    pub fn transform(&mut self, chunk: &[u8]) -> Vec<u8> {
        let mut output = chunk.to_vec();
        self.client_decrypt.apply_keystream(&mut output);
        self.telegram_encrypt.apply_keystream(&mut output);
        output
    }
}

impl InboundCrypto {
    pub fn transform(&mut self, chunk: &[u8]) -> Vec<u8> {
        let mut output = chunk.to_vec();
        self.telegram_decrypt.apply_keystream(&mut output);
        self.client_encrypt.apply_keystream(&mut output);
        output
    }
}

pub fn decode_client_handshake(
    handshake: &[u8; HANDSHAKE_LEN],
    secret: &[u8; 16],
) -> Result<ClientHandshake, ProtocolError> {
    let mut prekey_iv = [0u8; 48];
    prekey_iv.copy_from_slice(&handshake[PREKEY_START..IV_END]);

    let key = Sha256::new()
        .chain_update(&prekey_iv[..32])
        .chain_update(secret)
        .finalize();
    let mut decrypted = Zeroizing::new(*handshake);
    let mut cipher = Aes256Ctr::new_from_slices(&key, &prekey_iv[32..])
        .map_err(|_| ProtocolError::InvalidHandshake)?;
    cipher.apply_keystream(decrypted.as_mut());

    let protocol = match &decrypted[IV_END..PROTOCOL_END] {
        [0xef, 0xef, 0xef, 0xef] => TransportProtocol::Abridged,
        [0xee, 0xee, 0xee, 0xee] => TransportProtocol::Intermediate,
        [0xdd, 0xdd, 0xdd, 0xdd] => TransportProtocol::PaddedIntermediate,
        _ => return Err(ProtocolError::InvalidHandshake),
    };
    let dc_index = i16::from_le_bytes(
        decrypted[PROTOCOL_END..DC_END]
            .try_into()
            .map_err(|_| ProtocolError::InvalidHandshake)?,
    );
    let media = dc_index < 0;
    let dc = dc_index.unsigned_abs();
    if !matches!(dc, 1..=5 | 203) {
        return Err(ProtocolError::UnsupportedDataCenter);
    }

    Ok(ClientHandshake {
        target: TelegramTarget { dc, media },
        protocol,
        prekey_iv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::Aes256;
    use ctr::cipher::{KeyIvInit, StreamCipher};
    use sha2::{Digest, Sha256};

    type Aes256Ctr = ctr::Ctr128BE<Aes256>;

    fn client_fixture(secret: &[u8; 16], protocol_tag: [u8; 4], dc_index: i16) -> [u8; 64] {
        let mut plain = [0u8; 64];
        for (index, byte) in plain.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_add(1);
        }
        plain[56..60].copy_from_slice(&protocol_tag);
        plain[60..62].copy_from_slice(&dc_index.to_le_bytes());

        let key = Sha256::new()
            .chain_update(&plain[8..40])
            .chain_update(secret)
            .finalize();
        let mut cipher = Aes256Ctr::new_from_slices(&key, &plain[40..56]).unwrap();
        let mut encrypted = plain;
        cipher.apply_keystream(&mut encrypted);

        let mut wire = plain;
        wire[56..64].copy_from_slice(&encrypted[56..64]);
        wire
    }

    #[test]
    fn decodes_main_and_media_dc_from_obfuscated_handshake() {
        let secret = [0x42; 16];
        let normal =
            decode_client_handshake(&client_fixture(&secret, [0xee; 4], 2), &secret).unwrap();
        assert_eq!(
            normal.target,
            TelegramTarget {
                dc: 2,
                media: false
            }
        );
        assert_eq!(normal.protocol, TransportProtocol::Intermediate);

        let media =
            decode_client_handshake(&client_fixture(&secret, [0xdd; 4], -4), &secret).unwrap();
        assert_eq!(media.target, TelegramTarget { dc: 4, media: true });
        assert_eq!(media.protocol, TransportProtocol::PaddedIntermediate);
    }

    #[test]
    fn rejects_unknown_protocol_and_dc() {
        let secret = [7u8; 16];
        assert!(matches!(
            decode_client_handshake(&client_fixture(&secret, [0x11; 4], 2), &secret),
            Err(ProtocolError::InvalidHandshake)
        ));
        assert!(matches!(
            decode_client_handshake(&client_fixture(&secret, [0xee; 4], 9), &secret),
            Err(ProtocolError::UnsupportedDataCenter)
        ));
    }

    #[test]
    fn bridge_reencrypts_both_directions_without_changing_plaintext() {
        let secret = [0x51; 16];
        let wire = client_fixture(&secret, [0xee; 4], 3);
        let client = decode_client_handshake(&wire, &secret).unwrap();

        let mut relay_random = [0u8; 64];
        for (index, byte) in relay_random.iter_mut().enumerate() {
            *byte = (200u8).wrapping_add(index as u8);
        }
        let (mut bridge, relay_init) =
            BridgeCrypto::from_client(&client, &secret, relay_random).unwrap();

        let client_key = Sha256::new()
            .chain_update(&wire[8..40])
            .chain_update(secret)
            .finalize();
        let mut client_sender = Aes256Ctr::new_from_slices(&client_key, &wire[40..56]).unwrap();
        let mut skipped = [0u8; 64];
        client_sender.apply_keystream(&mut skipped);

        let relay_key = &relay_init[8..40];
        let relay_iv = &relay_init[40..56];
        let mut telegram_receiver = Aes256Ctr::new_from_slices(relay_key, relay_iv).unwrap();
        telegram_receiver.apply_keystream(&mut skipped);

        let plaintext_up = b"outbound MTProto packet".to_vec();
        let mut client_ciphertext = plaintext_up.clone();
        client_sender.apply_keystream(&mut client_ciphertext);
        let mut telegram_ciphertext = bridge.client_to_upstream(&client_ciphertext);
        telegram_receiver.apply_keystream(&mut telegram_ciphertext);
        assert_eq!(telegram_ciphertext, plaintext_up);

        let reversed_relay: Vec<u8> = relay_init[8..56].iter().rev().copied().collect();
        let mut telegram_sender =
            Aes256Ctr::new_from_slices(&reversed_relay[..32], &reversed_relay[32..]).unwrap();

        let reversed_client: Vec<u8> = wire[8..56].iter().rev().copied().collect();
        let client_reverse_key = Sha256::new()
            .chain_update(&reversed_client[..32])
            .chain_update(secret)
            .finalize();
        let mut client_receiver =
            Aes256Ctr::new_from_slices(&client_reverse_key, &reversed_client[32..]).unwrap();

        let plaintext_down = b"inbound MTProto packet".to_vec();
        let mut telegram_response = plaintext_down.clone();
        telegram_sender.apply_keystream(&mut telegram_response);
        let mut client_response = bridge.upstream_to_client(&telegram_response);
        client_receiver.apply_keystream(&mut client_response);
        assert_eq!(client_response, plaintext_down);
    }

    #[test]
    fn splitter_preserves_intermediate_packet_boundaries_across_chunks() {
        let secret = [0x62; 16];
        let wire = client_fixture(&secret, [0xee; 4], 2);
        let client = decode_client_handshake(&wire, &secret).unwrap();
        let relay_random = [0x91u8; 64];
        let (_, relay_init) = BridgeCrypto::from_client(&client, &secret, relay_random).unwrap();
        let mut splitter =
            MessageSplitter::from_relay(&relay_init, TransportProtocol::Intermediate).unwrap();

        let mut plaintext = Vec::new();
        plaintext.extend_from_slice(&3u32.to_le_bytes());
        plaintext.extend_from_slice(b"one");
        plaintext.extend_from_slice(&6u32.to_le_bytes());
        plaintext.extend_from_slice(b"second");

        let mut sender =
            Aes256Ctr::new_from_slices(&relay_init[8..40], &relay_init[40..56]).unwrap();
        let mut skip = [0u8; 64];
        sender.apply_keystream(&mut skip);
        let mut encrypted = plaintext;
        sender.apply_keystream(&mut encrypted);

        assert!(splitter.push(&encrypted[..5]).unwrap().is_empty());
        let first = splitter.push(&encrypted[5..9]).unwrap();
        assert_eq!(first, vec![encrypted[..7].to_vec()]);
        let second = splitter.push(&encrypted[9..]).unwrap();
        assert_eq!(second, vec![encrypted[7..].to_vec()]);
    }

    #[test]
    fn splitter_rejects_unbounded_packet_size() {
        let secret = [0x33; 16];
        let wire = client_fixture(&secret, [0xee; 4], 2);
        let client = decode_client_handshake(&wire, &secret).unwrap();
        let (_, relay_init) = BridgeCrypto::from_client(&client, &secret, [0x72; 64]).unwrap();
        let mut splitter =
            MessageSplitter::from_relay(&relay_init, TransportProtocol::Intermediate).unwrap();

        let mut invalid = (32u32 * 1024 * 1024).to_le_bytes().to_vec();
        let mut sender =
            Aes256Ctr::new_from_slices(&relay_init[8..40], &relay_init[40..56]).unwrap();
        let mut skip = [0u8; 64];
        sender.apply_keystream(&mut skip);
        sender.apply_keystream(&mut invalid);

        assert_eq!(
            splitter.push(&invalid),
            Err(ProtocolError::InvalidHandshake)
        );
    }

    #[test]
    fn matches_known_answer_vectors_from_audited_upstream_revision() {
        let secret = hex::decode("00112233445566778899aabbccddeeff").unwrap();
        let secret: [u8; 16] = secret.try_into().unwrap();
        let wire: [u8; 64] = hex::decode(concat!(
            "0102030405060708090a0b0c0d0e0f101112131415161718",
            "191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f30",
            "3132333435363738dc5d7521f8ec0285"
        ))
        .unwrap()
        .try_into()
        .unwrap();
        let expected_relay: [u8; 64] = hex::decode(concat!(
            "c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedf",
            "e0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7",
            "f8f9fafbfcfdfeff7afb10b25ecef95d"
        ))
        .unwrap()
        .try_into()
        .unwrap();
        let mut relay_random: [u8; 64] = std::array::from_fn(|index| (200 + index) as u8);
        relay_random[62..64].copy_from_slice(b"12");

        let client = decode_client_handshake(&wire, &secret).unwrap();
        let (mut bridge, relay) =
            BridgeCrypto::from_client(&client, &secret, relay_random).unwrap();
        assert_eq!(relay, expected_relay);

        let client_key = Sha256::new()
            .chain_update(&wire[8..40])
            .chain_update(secret)
            .finalize();
        let mut client_sender = Aes256Ctr::new_from_slices(&client_key, &wire[40..56]).unwrap();
        let mut skipped = [0u8; 64];
        client_sender.apply_keystream(&mut skipped);
        let mut client_ciphertext = b"upstream-vector".to_vec();
        client_sender.apply_keystream(&mut client_ciphertext);
        assert_eq!(
            hex::encode(bridge.client_to_upstream(&client_ciphertext)),
            "28fcbbf986a8487c4e6a5c8e35b727"
        );

        let relay_reverse: Vec<u8> = relay[8..56].iter().rev().copied().collect();
        let mut telegram_sender =
            Aes256Ctr::new_from_slices(&relay_reverse[..32], &relay_reverse[32..]).unwrap();
        let mut upstream = b"downstream-vector".to_vec();
        telegram_sender.apply_keystream(&mut upstream);
        assert_eq!(
            hex::encode(bridge.upstream_to_client(&upstream)),
            "262aad5853006fb183cc4b95e4d4e72f5f"
        );
    }
}
