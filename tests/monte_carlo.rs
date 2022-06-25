use fchashmap::FcHashMap;
use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::collections::HashMap;

struct MonteCarlo {
    fc_hashmap: FcHashMap<u32, u32, MAP_SIZE>,
    std_hashmap: HashMap<u32, u32>,
}

const MAP_SIZE: usize = 16384;
const SEED: u64 = 1234567890987654321;

impl MonteCarlo {
    fn new() -> Self {
        Self {
            fc_hashmap: FcHashMap::<u32, u32, MAP_SIZE>::new(),
            std_hashmap: HashMap::<u32, u32>::new(),
        }
    }

    fn insert(&mut self, rng: &mut XorShiftRng) {
        let key = rng.next_u32();
        let value = rng.next_u32();
        let r_fc = self.fc_hashmap.insert(key, value);
        let r_std = self.std_hashmap.insert(key, value);

        match r_fc {
            Ok(r_v) => {
                if r_v != r_std {
                    println!(
                        "Error 1, len {}, key {}, value {}, r_v{:?}, r_std {:?}",
                        self.fc_hashmap.len(),
                        key,
                        value,
                        r_v,
                        r_std
                    );
                };
                assert_eq!(r_v, r_std);
            }
            Err(e) => {
                println!(
                    "Error 2, len {}, key {}, value {}, e{:?}, r_std {:?}",
                    self.fc_hashmap.len(),
                    key,
                    value,
                    e,
                    r_std
                );
                assert!(false);
            }
        }
    }

    fn remove(&mut self, rng: &mut XorShiftRng) {
        let key = rng.next_u32();
        let _ = rng.next_u32();
        let r_fc = self.fc_hashmap.remove(&key);
        let r_std = self.std_hashmap.remove(&key);

        if r_fc != r_std {
            println!(
                "Error 3, len {}, key {}, r_fc{:?}, r_std {:?}",
                self.fc_hashmap.len(),
                key,
                r_fc,
                r_std
            );
        };
        assert_eq!(r_fc, r_std);
    }

    fn get(&mut self, rng: &mut XorShiftRng) {
        let key = rng.next_u32();
        let _ = rng.next_u32();
        let r_fc = self.fc_hashmap.get(&key);
        let r_std = self.std_hashmap.get(&key);

        if r_fc != r_std {
            println!(
                "Error 4, len {}, key {}, r_fc{:?}, r_std {:?}",
                self.fc_hashmap.len(),
                key,
                r_fc,
                r_std
            );
        };
        assert_eq!(r_fc, r_std);
    }

    fn test_1(&mut self) {
        // These are positive tests, we write data into the maps and try to find it later on
        let mut rng = XorShiftRng::seed_from_u64(SEED);
        loop {
            if self.fc_hashmap.len() >= MAP_SIZE {
                break;
            }
            self.insert(&mut rng);
        }

        let mut rng = XorShiftRng::seed_from_u64(SEED);
        for _ in 0..MAP_SIZE {
            self.get(&mut rng);
        }

        if self.fc_hashmap.len() != self.std_hashmap.len() {
            println!("Error 5");
        }

        let mut rng = XorShiftRng::seed_from_u64(SEED);
        loop {
            if self.fc_hashmap.len() == 0 {
                break;
            }
            self.remove(&mut rng);
        }

        if self.fc_hashmap.len() != self.std_hashmap.len() {
            println!("Error 5");
        };
        assert_eq!(self.fc_hashmap.len(), self.std_hashmap.len());
    }

    fn test_2(&mut self) {
        // These are positive and negative tests
        // First, we fill the map at 50%
        let mut rng = XorShiftRng::seed_from_u64(SEED);
        loop {
            if self.fc_hashmap.len() >= MAP_SIZE / 2 {
                break;
            }
            self.insert(&mut rng);
        }

        // Every second acces is a faulty one
        let mut rng = XorShiftRng::seed_from_u64(SEED);
        for _ in 0..MAP_SIZE {
            let key = rng.next_u32();
            let r_fc = self.fc_hashmap.get(&key);
            let r_std = self.std_hashmap.get(&key);

            if r_fc != r_std {
                println!(
                    "Error 6, len {}, key {}, r_fc{:?}, r_std {:?}",
                    self.fc_hashmap.len(),
                    key,
                    r_fc,
                    r_std
                );
            };
            assert_eq!(r_fc, r_std);
        }
    }
}

#[test]
fn monte_carlo() {
    let mut m = MonteCarlo::new();
    m.test_1();
    m.test_2();
}
