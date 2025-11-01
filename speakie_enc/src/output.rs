use ndarray::Array1;

#[derive(Default)]
pub struct Output {
    buf: Vec<u8>,
    bit_pos: usize,
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

impl Output {
    fn bit(&mut self, bit: u8) {
        //println!("writing bit {bit}");
        if self.bit_pos == 0 {
            self.buf.push(0);
        }
        *self.buf.last_mut().unwrap() |= bit << self.bit_pos;
        self.bit_pos = (self.bit_pos + 1) % 8;
    }

    pub fn pack(&mut self, val: u8, len: u32) {
        //println!("packing {val}, {len} bits");
        for i in 0..len {
            self.bit((val >> (len - 1 - i)) & 1);
        }
    }

    pub fn reap(self) -> Vec<u8> {
        self.buf
    }

    pub fn quantized<T: Into<i32> + Copy>(&mut self, table: &[T], x: T) {
        let code = quantize(table, x);
        self.pack(code, table.len().trailing_zeros());
    }

    pub fn frame(&mut self, energy: f64, pitch: f64, ks: Array1<f64>) {
        let energy_code = quantize(&ENERGY, energy.round().min(5514.0) as u16);
        self.pack(energy_code, 4);
        if energy_code > 0 {
            self.pack(0, 1); // repeat
            self.quantized(&PERIOD, pitch.round() as u8);
            self.quantized(&K1, (ks[0] * 512.0).round() as i16);
            self.quantized(&K2, (ks[1] * 512.0).round() as i16);
            self.quantized(&K3, (ks[2] * 512.0).round() as i16);
            self.quantized(&K4, (ks[3] * 512.0).round() as i16);
            if pitch != 0.0 {
                self.quantized(&K5, (ks[4] * 512.0).round() as i16);
                self.quantized(&K6, (ks[5] * 512.0).round() as i16);
                self.quantized(&K7, (ks[6] * 512.0).round() as i16);
                self.quantized(&K8, (ks[7] * 512.0).round() as i16);
                self.quantized(&K9, (ks[8] * 512.0).round() as i16);
                self.quantized(&K10, (ks[9] * 512.0).round() as i16);
            }
        }
    }
}

fn quantize<T: Into<i32> + Copy>(table: &[T], x: T) -> u8 {
    let mut best = 0;
    let mut best_err = i32::MAX;
    for (i, val) in table.iter().enumerate() {
        let err = (Into::<i32>::into(*val) - Into::<i32>::into(x)).pow(2);
        if err < best_err {
            best = i;
            best_err = err;
        }
    }
    best as u8
}
