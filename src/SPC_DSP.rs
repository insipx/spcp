use std::ptr;

use state::sample_t as sample_t;
use state::NULL_U8 as NULL_U8;
use registers::GlobalRegisters;
use registers::EnvMode;
use sizes::Sizes;
use state::State;
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
struct SPC_DSP {
   m:State<'static>,
}

pub trait Emulator<'a, 'b:'a> {
   
    fn init(&mut self, ram_64K: &'b mut u8);

    fn load(&mut self, regs: [u8; Sizes::REGISTER_COUNT as usize]);

    // Runs DSP for specified number of clocks (~1024000 per second). Every 32 clocks
    // a pair of samples is to be generated
    fn run(clock_count: isize);
}

impl<'a, 'b:'a> Emulator<'a, 'b> for SPC_DSP {
  
    fn init(&mut self, ram_64K: &'b mut u8) {
        self.m.set_ram(ram_64K); 
        self.m.mute_voices(0);
        self.m.disable_surround(false);
        self.m.set_output(0 as *mut sample_t, 0isize);
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

        let mut i:isize;
        //be careful here
        for i in (0..Sizes::VOICE_COUNT).rev() {
            self.m.voices[i].brr_offset = 1;
            self.m.voices[i].buf_pos = 0;
        }
        self.m.new_kon = self.m.regs[reg!(kon)] as isize;
        let mask = self.m.mute_mask;
        self.m.mute_voices(mask);
        self.m.soft_reset_common();
    }

    fn run(clock_count: isize) {
        unimplemented!(); 
    }
}



