//! Internal SHA-256 helpers used by Worktree filesystem boundaries.

use haze_sync_common::ContentHash;

/// Compute the canonical content hash for an in-memory byte slice.
pub(crate) fn content_hash_for_bytes(bytes: &[u8]) -> ContentHash {
    let mut hasher = Sha256State::new();
    hasher.update(bytes);
    ContentHash::from_bytes(hasher.finalize())
}

#[derive(Debug, Clone)]
struct Sha256State {
    state: [u32; 8],
    message_len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha256State {
    const INITIAL_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            state: Self::INITIAL_STATE,
            message_len: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.message_len = self.message_len.wrapping_add(input.len() as u64);

        while !input.is_empty() {
            let available = self.buffer.len() - self.buffer_len;
            let copy_len = available.min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + copy_len]
                .copy_from_slice(&input[..copy_len]);
            self.buffer_len += copy_len;
            input = &input[copy_len..];

            if self.buffer_len == self.buffer.len() {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.message_len.wrapping_mul(8);
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            for index in self.buffer_len..self.buffer.len() {
                self.buffer[index] = 0;
            }
            let block = self.buffer;
            self.process_block(&block);
            self.buffer = [0; 64];
            self.buffer_len = 0;
        }

        for index in self.buffer_len..56 {
            self.buffer[index] = 0;
        }
        self.buffer[56..].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buffer;
        self.process_block(&block);

        let mut output = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut schedule = [0_u32; 64];
        for (index, word) in schedule.iter_mut().enumerate().take(16) {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        for index in 16..64 {
            schedule[index] = small_sigma_1(schedule[index - 2])
                .wrapping_add(schedule[index - 7])
                .wrapping_add(small_sigma_0(schedule[index - 15]))
                .wrapping_add(schedule[index - 16]);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for (index, word) in schedule.iter().enumerate() {
            let temp1 = h
                .wrapping_add(big_sigma_1(e))
                .wrapping_add(ch(e, f, g))
                .wrapping_add(Self::ROUND_CONSTANTS[index])
                .wrapping_add(*word);
            let temp2 = big_sigma_0(a).wrapping_add(maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

const fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

const fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

const fn big_sigma_0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

const fn big_sigma_1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

const fn small_sigma_0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

const fn small_sigma_1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_sha256_test_vector() {
        assert_eq!(
            content_hash_for_bytes(b"abc").to_string(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
