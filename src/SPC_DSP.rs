use std::ptr;

use state::sample_t as sample_t;
use state::NULL_U8 as NULL_U8;
use registers::GlobalRegisters;
use registers::VoiceRegisters;
use registers::EnvMode;
use sizes::Sizes;
use state::State;
use voice::Voice;

use config::*;

use macros;

pub static counter_mask: [u32; 32] =
[
	rate!(   2,2), rate!(2048,4), rate!(1536,3),
	rate!(1280,5), rate!(1024,4), rate!( 768,3),
	rate!( 640,5), rate!( 512,4), rate!( 384,3),
	rate!( 320,5), rate!( 256,4), rate!( 192,3),
	rate!( 160,5), rate!( 128,4), rate!(  96,3),
	rate!(  80,5), rate!(  64,4), rate!(  48,3),
	rate!(  40,5), rate!(  32,4), rate!(  24,3),
	rate!(  20,5), rate!(  16,4), rate!(  12,3),
	rate!(  10,5), rate!(   8,4), rate!(   6,3),
	rate!(   5,5), rate!(   4,4), rate!(   3,3),
	               rate!(   2,4),
	               rate!(   1,4)
];

// holds the state
pub struct SPC_DSP {
    m:State<'static>,
}

pub trait Emulator<'a, 'b:'a> {
    fn new() -> SPC_DSP; 
    fn init(&mut self, ram_64K: &Vec<u8>);

    fn load(&mut self, regs: [u8; Sizes::REGISTER_COUNT as usize]);

    // Runs DSP for specified number of clocks (~1024000 per second). Every 32 clocks
    // a pair of samples is to be generated
    fn run(&mut self, clock_count: i64);
}

impl<'a, 'b:'a> Emulator<'a, 'b> for SPC_DSP {

    fn new() -> Self {
        SPC_DSP {
            m: State::new(),
        } 
    }

    fn init(&mut self, ram_64K: &Vec<u8>) {
        self.m.set_ram(ram_64K); 
        self.m.mute_voices(0);
        self.m.disable_surround(false);
        self.m.set_output(0 as *mut sample_t, 0i64);
        self.m.reset();

        if NDEBUG {
            assert_eq!(0x8000 as i16, -0x8000);
            assert!( (-1 >> 1) == -1 );
            let mut i:i16;
            i = 0x8000; clamp16!(i); assert!(i == 0x7FFF);
            i = -0x8001; clamp16!(i); assert!(i == -0x8000);
        }

        //SPC_DSP has a verify byte order; but i will forgo this for now
    }

    fn load(&mut self, regs: [u8; Sizes::REGISTER_COUNT as usize]) {
        self.m.regs = regs;

        let mut i:i64;
        //be careful here
        for i in (0..Sizes::VOICE_COUNT).rev() {
            self.m.voices[i].brr_offset = 1;
            self.m.voices[i].buf_pos = 0;
        }
        self.m.new_kon = self.m.regs[reg!(kon)] as i64;
        let mask = self.m.mute_mask;
        self.m.mute_voices(mask);
        self.m.soft_reset_common();
    }

    fn run(&mut self, clock_count: i64) {
        let new_phase: i64 = self.m.get_phase() + clock_count;
        let count: i64 = new_phase >> 5;
        self.m.set_phase((new_phase & 31)); //raises can't mutably borrow immutable field
        if count == 0  {
            return; 
        }
        
        let dir: Vec<u8> = self.m.ram[(self.m.regs[reg!(dir)] * 0x100)..(0xFFFF)]; 
        //let dir: [u8; (Sizes::RAM_SIZE - (self.m.regs[reg!(dir)] * 0x100i64))] = self.m.ram[(self.m.regs[reg!(dir)] * 0x100)..(Sizes::RAM_SIZE)];
        let slow_gaussian:i64 = ((self.m.regs[reg!(pmon)] >> 1) | self.m.regs[reg!(non)]) as i64; 
        let noise_rate:i64 = (self.m.regs[reg!(flg)] & 0x1F) as i64;

        //global volume
        let mvoll:i8 = self.m.regs[reg!(mvoll)] as i8;
        let mvolr:i8 = self.m.regs[reg!(mvolr)] as i8;

        if ((mvoll * mvolr) as i64) < self.m.surround_threshold {
            mvoll = -mvoll;
        }

        loop {
            // KON/KOFF reading
            self.m.every_other_sample ^= 1;
            if self.m.every_other_sample != 0i64 {
                self.m.new_kon &= !self.m.kon;
                self.m.kon = self.m.new_kon;
                self.m.t_koff = self.m.regs[reg!(koff)] as i64;
            }

            self.m.run_counter( 1i64 );
            self.m.run_counter( 2i64 );
            self.m.run_counter( 3i64 );

            // Noise
            if read_counter!(noise_rate, self.m) == 0 {
                let feedback: i64 = (self.m.noise << 13) ^ (self.m.noise << 14);
                self.m.noise = (feedback & 0x4000 ) ^ (self.m.noise >> 1);
            }
             
            let pmon_input = 0;
            let main_out_l = 0;
            let main_out_r = 0;
            let echo_out_l = 0;
            let echo_out_r = 0;
            let v:Voice = self.m.voices[0];
            let vbit = 1;

            loop {
                macro_rules! sample_ptr {
                    ( $i:expr ) => {
                        dir[(self.m.regs[vreg!(srcn)] * 4 + $i * 2) as usize];
                    }
                }

                let brr_header: i64 = self.m.ram[v.brr_addr as usize] as i64;
                let kon_delay: i64 = v.kon_delay; 

                // Pitch
                let mut pitch: i64 = (self.m.regs[vreg!(pitchl)].to_le() & 0x3FFF) as i64;
                if (self.m.regs[reg!(pmon)] & vbit) != 0 {
                    pitch += ((pmon_input >> 5) * pitch) >> 10;

                    // KON phases
                    if --kon_delay >= 0 {
                        v.kon_delay = kon_delay;
                        if kon_delay == 4 {
                            v.brr_addr =  sample_ptr!(0) as i64;
                        }


                    }
                }


            }

            
        }
        
        
    }


}



