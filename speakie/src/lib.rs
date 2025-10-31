#![no_std]

/// A source of LPC-10 encoded data.
pub struct BitStream<T: AsRef<[u8]>> {
    buf: T,
    bit_addr: usize,
}

/// The main decoder object.
pub struct Speakie {
    last_params: Params,
    new_params: Params,
    // Result of interpolation
    params: Params,
    interp_minor: usize,
    interp_major: i32,
    period_counter: u8,
    rand: u16,
    x: [i16; 11],
}

#[derive(Clone, Copy, Default, Debug)]
struct Params {
    energy: u16,
    period: u8,
    k: [i16; 10],
    is_stop: bool,
}

impl<T: AsRef<[u8]>> BitStream<T> {
    pub fn new(buf: T) -> Self {
        let bit_addr = 0;
        Self { buf, bit_addr }
    }

    fn get_bits(&mut self, len: usize) -> usize {
        let buf = self.buf.as_ref();
        let bit = self.bit_addr % 8;
        let byte_addr = self.bit_addr / 8;
        self.bit_addr += len;
        let mut word = (buf[byte_addr].reverse_bits() as u16) << 8;
        if bit + len > 8 {
            word |= buf[byte_addr + 1].reverse_bits() as u16;
        }
        ((word << bit) >> (16 - len)) as usize
    }
}

const ENERGY: [u16; 0x10] = [
    0, 52, 87, 123, 174, 246, 348, 491, 694, 981, 1385, 1957, 2764, 3904, 5514, 7789,
];
const PERIOD: [u8; 0x40] = [
    0, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
    38, 39, 40, 41, 42, 44, 46, 48, 50, 52, 53, 56, 58, 60, 62, 65, 68, 70, 72, 76, 78, 80, 84, 86,
    91, 94, 98, 101, 105, 109, 114, 118, 122, 127, 132, 137, 142, 148, 153, 159,
];
const K1: [i16; 0x20] = [
    -501, -498, -497, -495, -493, -491, -488, -482, -478, -474, -469, -464, -459, -452, -445, -437,
    -412, -380, -339, -288, -227, -158, -81, -1, 80, 157, 226, 287, 337, 379, 411, 436,
];
const K2: [i16; 0x20] = [
    -328, -303, -274, -244, -211, -175, -138, -99, -59, -18, 24, 64, 105, 143, 180, 215, 248, 278,
    306, 331, 354, 374, 392, 408, 422, 435, 445, 455, 463, 470, 476, 506,
];
const K3: [i16; 0x10] = [
    -441, -387, -333, -279, -225, -171, -117, -63, -9, 45, 98, 152, 206, 260, 314, 368,
];
const K4: [i16; 0x10] = [
    -328, -273, -217, -161, -106, -50, 5, 61, 116, 172, 228, 283, 339, 394, 450, 506,
];
const K5: [i16; 0x10] = [
    -328, -282, -235, -189, -142, -96, -50, -3, 43, 90, 136, 182, 229, 275, 322, 368,
];
const K6: [i16; 0x10] = [
    -256, -212, -168, -123, -79, -35, 10, 54, 98, 143, 187, 232, 276, 320, 365, 409,
];
const K7: [i16; 0x10] = [
    -308, -260, -212, -164, -117, -69, -21, 27, 75, 122, 170, 218, 266, 314, 361, 409,
];
const K8: [i16; 0x08] = [-256, -161, -66, 29, 124, 219, 314, 409];
const K9: [i16; 0x08] = [-256, -176, -96, -15, 65, 146, 226, 307];
const K10: [i16; 0x08] = [-205, -132, -59, 14, 87, 160, 234, 307];

const CHIRP: [u8; 52] = [
    0x00, 0x03, 0x0f, 0x28, 0x4c, 0x6c, 0x71, 0x50, 0x25, 0x26, 0x4c, 0x44, 0x1a, 0x32, 0x3b, 0x13,
    0x37, 0x1a, 0x25, 0x1f, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

impl Speakie {
    /// Create a new decoder.
    pub fn new() -> Self {
        Self {
            last_params: Params::default(),
            new_params: Params::default(),
            params: Params::default(),
            interp_major: 0,
            interp_minor: 0,
            period_counter: 0,
            rand: 1,
            x: [0; 11],
        }
    }

    /// Process one frame.
    ///
    /// This method should be called at the start of each frame, which is 200 samples
    /// at the nominal speech rate.
    ///
    /// Returns true on "stop" frame.
    pub fn process_frame(&mut self, bs: &mut BitStream<impl AsRef<[u8]>>) -> bool {
        self.last_params = self.new_params;
        self.new_params.read(bs);
        if self.last_params.inhibit_interp(&self.new_params) {
            self.last_params = self.new_params;
        }
        self.interp_major = 0;
        self.interp_minor = 0;
        self.new_params.is_stop
    }

    /// Get one sample.
    pub fn get_sample(&mut self) -> i16 {
        if self.interp_minor == 0 {
            self.interp_major = (self.interp_major + 1).min(8);
            self.params = self
                .last_params
                .interpolate(&self.new_params, self.interp_major);
            //println!("last_params = {:?}", self.last_params);
            //println!("new_params = {:?}", self.new_params);
            //println!("interp result = {:?} {}", self.params, self.interp_major);
        }
        self.interp_minor += 1;
        // TODO: make rate adjustable
        if self.interp_minor == 25 {
            self.interp_minor = 0;
        }
        let u10;
        if self.params.is_voiced() {
            let chirp = CHIRP
                .get(self.period_counter as usize)
                .cloned()
                .unwrap_or_default() as i8;
            u10 = (((chirp as i32) * (self.params.energy as i32)) >> 6) as i16;
            self.period_counter += 1;
            if self.period_counter >= self.params.period {
                self.period_counter = 0;
            }
        } else {
            self.rand = (self.rand >> 1) ^ if (self.rand & 1) != 0 { 0xb800 } else { 0 };
            u10 = if (self.rand & 1) != 0 {
                self.params.energy as i16
            } else {
                -(self.params.energy as i16)
            };
        }
        let mut u = u10;
        for i in (0..10).rev() {
            u -= ((self.params.k[i] as i32 * self.x[i] as i32) >> 9) as i16;
            self.x[i + 1] = self.x[i] + ((self.params.k[i] as i32 * u as i32) >> 9) as i16;
        }
        // TODO: maybe change this
        u = u.clamp(-16384, 16383);
        self.x[0] = u;
        u
    }
}

impl Params {
    fn read(&mut self, bs: &mut BitStream<impl AsRef<[u8]>>) {
        let energy = bs.get_bits(4);
        if energy == 0 {
            self.energy = 0;
        } else if energy == 0xf {
            self.energy = 0;
            self.is_stop = true;
        } else {
            self.energy = ENERGY[energy] as u16;
            let repeat = bs.get_bits(1);
            self.period = PERIOD[bs.get_bits(6)];
            if repeat == 0 {
                self.k[0] = K1[bs.get_bits(5)];
                self.k[1] = K2[bs.get_bits(5)];
                self.k[2] = K3[bs.get_bits(4)];
                self.k[3] = K4[bs.get_bits(4)];
                if self.period != 0 {
                    self.k[4] = K5[bs.get_bits(4)];
                    self.k[5] = K6[bs.get_bits(4)];
                    self.k[6] = K7[bs.get_bits(4)];
                    self.k[7] = K8[bs.get_bits(3)];
                    self.k[8] = K9[bs.get_bits(3)];
                    self.k[9] = K10[bs.get_bits(3)];
                } else {
                    self.k[4..].fill(0);
                }
            }
        }
    }

    fn interpolate(&self, new_params: &Self, t: i32) -> Self {
        fn lerp(x0: i32, x1: i32, t: i32) -> i32 {
            (x0 * 8 + (x1 - x0) * t) / 8
        }

        Params {
            energy: lerp(self.energy as i32, new_params.energy as i32, t) as u16,
            period: lerp(self.period as i32, new_params.period as i32, t) as u8,
            k: core::array::from_fn(|i| lerp(self.k[i] as i32, new_params.k[i] as i32, t) as i16),
            is_stop: false,
        }
    }

    fn is_voiced(&self) -> bool {
        self.period != 0
    }

    fn inhibit_interp(&self, new_params: &Self) -> bool {
        self.is_voiced() != new_params.is_voiced()
            || (self.energy == 0 && new_params.energy != 0)
            || (!self.is_voiced() && new_params.energy == 0)
    }
}
