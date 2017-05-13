use std::ptr;

use state::sample_t as sample_t;
use state::NULL_U8 as NULL_U8;
use registers::GlobalRegisters;
use registers::VoiceRegisters;
use registers::EnvMode;
use registers::interleved_gauss;
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
    
    fn exit_env();

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
        let break_outer: bool = true;
        let new_phase: i64 = self.m.get_phase() + clock_count;
        let count: i64 = new_phase >> 5;
        self.m.set_phase((new_phase & 31)); //raises can't mutably borrow immutable field
        if count == 0  {
            return; 
        }
        
        let dir: Vec<u8> = self.m.ram[((self.m.regs[reg!(dir)] * 0x100) as usize)..(0xFFFF as usize)].to_vec(); 
        //let dir: [u8; (Sizes::RAM_SIZE - (self.m.regs[reg!(dir)] * 0x100i64))] = self.m.ram[(self.m.regs[reg!(dir)] * 0x100)..(Sizes::RAM_SIZE)];
        let slow_gaussian:i64 = ((self.m.regs[reg!(pmon)] >> 1) | self.m.regs[reg!(non)]) as i64; 
        let noise_rate:i64 = (self.m.regs[reg!(flg)] & 0x1F) as i64;

        // Global volume
        let mut mvoll:i8 = self.m.regs[reg!(mvoll)] as i8;
        let mvolr:i8 = self.m.regs[reg!(mvolr)] as i8;

        if ((mvoll * mvolr) as i64) < self.m.surround_threshold {
            mvoll = -mvoll;
        }

            'outer: loop {
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
                 
                let mut pmon_input = 0;
                let mut main_out_l = 0;
                let mut main_out_r = 0;
                let mut echo_out_l = 0;
                let mut echo_out_r = 0;
                let mut voice_indx = 0;
                let mut v:Voice = self.m.voices[voice_indx];
                let mut vreg_start:usize = 0;
                let mut v_regs = &mut self.m.regs[vreg_start..Sizes::REGISTER_COUNT];
                let mut vbit: i64 = 1;

                'inner: loop {
                    macro_rules! sample_ptr {
                        ( $i:expr ) => {
                            dir[(v_regs[vreg!(srcn)] * 4 + $i * 2) as usize];
                        }
                    }

                    let mut brr_header: i64 = self.m.ram[v.brr_addr as usize] as i64;
                    let kon_delay: i64 = v.kon_delay; 

                    // Pitch
                    let mut pitch: i64 = (v_regs[vreg!(pitchl)].to_le() & 0x3FFF) as i64;
                    if ((self.m.regs[reg!(pmon)] as i64) & vbit) != 0 {
                        pitch += ((pmon_input >> 5) * pitch) >> 10;
                    }

                    // KON phases
                    if --kon_delay >= 0 {
                        v.kon_delay = kon_delay;
                        if kon_delay == 4 {
                            v.brr_addr   =  sample_ptr!(0) as i64;
                            v.brr_offset = 1;
                            v.buf_pos    = 0;
                            brr_header   = 0;
                        }

                        // Envelope is never run during KON
                        v.env        = 0;
                        v.hidden_env = 0;

                        // Disable BRR decoding until last three samples
                        if (kon_delay & 3) != 0 {
                            v.interp_pos = 0x4000; 
                        } else {
                            v.interp_pos = 0; 
                        }
                        pitch = 0;
                    }
                    let mut env: i64 = v.env;
                    
                    //Gaussian interpolation
                    {
                        let mut output: i64 = 0;
                        v_regs[vreg!(envx)] = (env >> 4) as u8;
                        if env != 0 {
                            // Make pointers into gaussian based on fractional position between
                            // samples
                            let mut offset: isize = (v.interp_pos >> (3 & 0x1FE)) as isize;
                            let fwd: *const i64 = (&(interleved_gauss[0] as i64) as *const i64).wrapping_offset(offset);
                            let rev: *const i64 = (&(interleved_gauss[0] as i64) as *const i64).wrapping_offset(510).wrapping_offset(-offset);
                            
                            let _in: *const i64 = &v.buf[(v.buf_pos) + (v.interp_pos >> 12) as usize];
                            
                            if (slow_gaussian & vbit) == 0 { //99%
                                // Faster approximation when exact sample value isn't necessary for
                                // pitch mod 
                                output = 
                                    unsafe{
                                        (*fwd * *_in +
                                          *fwd.wrapping_offset(1) * *_in.wrapping_offset(1) +
                                          *rev.wrapping_offset(1) * *_in.wrapping_offset(2) +
                                          *rev.wrapping_offset(1) * *_in.wrapping_offset(3)) >> 11};
                                output = (output * env) >> 11;
                            }else {
                                output = self.m.noise *2;
                                if ((self.m.regs[reg!(non)] as i64) & vbit) == 0 {
                                    output = unsafe{(*fwd.wrapping_offset(0) * *_in.wrapping_offset(0))}  >> 11;
                                    output += unsafe{(*fwd.wrapping_offset(1) * *_in.wrapping_offset(1))} >> 11;
                                    output += unsafe{(*rev.wrapping_offset(1) * *_in.wrapping_offset(2))} >> 11;
                                    output = output;
                                    output += unsafe{(*rev.wrapping_offset(0) * *_in.wrapping_offset(3))} >> 11;
                                    //clamp16!(output as i16);
                                    output &= !1;
                                }
                                output = (output * env) >> 11 & !1;
                            }
                            // Output
                            
                            let l: i64 = output * v.volume[0];
                            let r: i64 = output * v.volume[1];

                            main_out_l += l;
                            main_out_r += r;

                            if ((self.m.regs[reg!(eon)] as i64) & vbit) != 0 {
                                echo_out_l += l;
                                echo_out_r += r;
                            }
                        }
                        pmon_input = output;
                        v_regs[vreg!(outx)] = (output >> 8) as u8;
                    }

                    // Soft reset or end of sample
                    if ((self.m.regs[reg!(flg)]) & 0x80 != 0) || ((brr_header & 3) == 1) {
                        v.env_mode = EnvMode::env_release;
                        env        = 0;
                    }

                    if self.m.every_other_sample != 0  {
                        // KOFF
                        if (self.m.t_koff & vbit) != 0 {
                            v.env_mode = EnvMode::env_release;
                        }

                        // KON
                        if (self.m.kon & vbit) != 0 {
                            v.kon_delay = 5;
                            v.env_mode = EnvMode::env_attack;
                            self.m.regs[reg!(endx)] &= !vbit as u8;
                        }
                    }

                    // Envelope
                    if v.kon_delay == 0 {
                        if v.env_mode == EnvMode::env_release { // 97%
                            env -= 0x8;
                            v.env = env;
                                if env <= 0 {
                                    v.env = 0;
                                    // TODO: NO BRR decoding for you! (remove goto)
                                }
                        } else { // 3%
                            let rate: i64;
                            let adsr0: i64 = v_regs[vreg!(adsr0)] as i64;
                            let mut env_data: i64 = v_regs[vreg!(adsr1)] as i64;
                            if adsr0 >= 0x80 /* 97% ADSR  */ {
                                if  v.env_mode > EnvMode::env_decay {
                                    env -= 1;
                                    env -= env >> 8;
                                    rate = env_data & 0x1F;

                                    // optimized handling
                                    v.hidden_env = env;
                                    if read_counter!(rate, self.m) != 0 {
                                        //goto exit_env; // TODO 
                                    }
                                    v.env = env;
                                    //goto exit_env; // TODO
                                } else if v.env_mode == EnvMode::env_decay {
                                    env -= 1;
                                    env -= env >> 8;
                                    rate = (adsr0 >> 3 & 0x0E) + 0x10;
                                } else /* env_attack */ {
                                    rate = (adsr0 & 0x0F) * 2 + 1;
                                    if rate < 31 { env += 0x20;}
                                    else {env+= 0x400;}
                                }
                            } else /* GAIN */ {
                                let mut mode: i64;
                                env_data = v_regs[vreg!(gain)] as i64;
                                mode = env_data >> 5;
                                if mode < 4  /* direct */ {
                                    env = env_data * 0x10;
                                    rate = 31;
                                } else {
                                    rate = env_data & 0x1F;
                                    if mode == 4 /* 4: linear decrease */ {
                                        env -= 0x20; 
                                    } else if mode < 6 /* 5: exponential decrease */ {
                                        env -=1;
                                        env -= env >> 8;
                                    } else /* 6,7: linear increase */ {
                                        env += 0x20;
                                        if  (mode > 6) && (v.hidden_env as usize) >= 0x600 {
                                            env += 0x8 - 0x20; //7: two-slope linear increase 
                                        }
                                    }
                                }
                            }
                            // Sustain level
                            if ((env >> 8) == (env_data >> 5)) && (v.env_mode == EnvMode::env_decay) {
                                v.env_mode = EnvMode::env_sustain; 
                            }
                            v.hidden_env = env;

                            //unsigned cast because linear decrease going negative also triggers
                            //this
                            if (env as usize )> 0x7FF {
                                if env < 0 { env = 0; }
                                else { env = 0x7FF; }
                                if v.env_mode == EnvMode::env_attack {
                                    v.env_mode = EnvMode::env_decay;
                                }
                            }

                            if read_counter!(rate, self.m) == 0 {
                                v.env = env;  // nothing else is controlled by the counter
                            }
                        }
                    }
                //exit_env
                // skip_brr
                vbit <<= 1;
                vreg_start += 0x10;
                let v_regs = &mut self.m.regs[vreg_start..Sizes::REGISTER_COUNT];
                voice_indx+=1;
                v = self.m.voices[voice_indx];
                if vbit >= 0x100 {break 'inner; }
                } // inner loop
            } // outer loop
        }
    fn exit_env() {
     
    }
}



