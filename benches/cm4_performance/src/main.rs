#![no_std]
#![no_main]

use core::{panic::PanicInfo};

use cortex_m::{peripheral::{DWT, Peripherals}};
use cortex_m_rt::entry;
use stm32f3xx_hal::{pac, prelude::*};


//use fchashmap::FcHashMap;
use heapless::FnvIndexMap;
use rand_xorshift::XorShiftRng;
use rand_core::{RngCore, SeedableRng};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

struct Measure {
    //map: FcHashMap::<u32, u32, MAP_SIZE>,
    map: FnvIndexMap::<u32, u32, MAP_SIZE>,
    measures: [[u32; MAP_SIZE]; 3],
}

const MAP_SIZE: usize = 512;
const SEED_1: u64 = 1234567890987654321;
const SEED_2: u64 = 08154711;


impl Measure {
    fn new() -> Self {
        Self {
            //map: FcHashMap::<u32, u32, MAP_SIZE>::new(),
            map: FnvIndexMap::<u32, u32, MAP_SIZE>::new(),
            measures: [[0; MAP_SIZE]; 3],
        }
    }

    fn insert(&mut self, key: u32, value: u32) {
        let before= DWT::get_cycle_count();
        let _ = self.map.insert(key, value);
        let after = DWT::get_cycle_count();
        self.measures[0][self.map.len() - 1] = after.wrapping_sub(before);

    }

    fn remove(&mut self, key: &u32) {
        let before= DWT::get_cycle_count();
        let _ = self.map.remove(key);
        let after = DWT::get_cycle_count();
        self.measures[2][self.map.len()] = after.wrapping_sub(before);
    }

    fn get(&mut self, key: &u32) {
        let before= DWT::get_cycle_count();
        let _ = self.map.get(&key);
        let after = DWT::get_cycle_count();
        self.measures[1][self.map.len() - 1] = after.wrapping_sub(before);
    }

    fn measure(&mut self) {
        let mut rng1 = XorShiftRng::seed_from_u64(SEED_1);
        loop {
            let key = rng1.next_u32();
            let value = rng1.next_u32();
            let _ = self.insert(key, value);
            if self.map.len() >= MAP_SIZE {
                break;
            }
        }

        let mut rng1 = XorShiftRng::seed_from_u64(SEED_1);
        loop {
            let key = rng1.next_u32();
            let _ = rng1.next_u32();
            self.get(&key);
            self.remove(&key);
            if self.map.len() <= MAP_SIZE / 2 {
                break;
            }
        }

        let mut rng2 = XorShiftRng::seed_from_u64(SEED_2);
        loop {
            let key = rng2.next_u32();
            let value = rng2.next_u32();
            let _ = self.insert(key, value);
            if self.map.len() >= MAP_SIZE {
                break;
            }
        }

        loop {
            let key = rng1.next_u32();
            let _ = rng1.next_u32();
            self.get(&key);
            self.remove(&key);
            if self.map.len() <= MAP_SIZE / 2  {
                break;
            }
        }

        loop {
            let key = rng2.next_u32();
            let value = rng2.next_u32();
            let _ = self.insert(key, value);
            if self.map.len() >= MAP_SIZE {
                break;
            }
        }

        let mut rng2 = XorShiftRng::seed_from_u64(SEED_2);
        loop {
            let key = rng2.next_u32();
            let _ = rng2.next_u32();
            self.get(&key);
            self.remove(&key);
            if self.map.len() == 0 {
                break;
            }
        }
        self.measures[0] = self.measures[0];
    } 
}

#[entry]
fn main () -> ! {

    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let _clocks = rcc
        .cfgr
        .use_hse(8.mhz())       // esternal (quartz) oscillator
        .sysclk(72.mhz())      
        .freeze(&mut flash.acr);

    let mut cp = Peripherals::take().unwrap();
    cp.DWT.enable_cycle_counter();
    let mut m = Measure::new();
    m.measure();

    loop {}
}
