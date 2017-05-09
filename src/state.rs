use macros;

use std::ops::Add;
use std::ptr;

use registers::VoiceRegisters;
use registers::GlobalRegisters;
use sizes::Sizes;
use voice::Voice;
use config::*;

// Keeps track of the state of the Emulator
// the Virtual CPU + RAM 

// Forseable problems:
//  I highly doubt any of the pointer arithmetic is correct

pub type sample_t = i16;
pub const NULL_U8: *mut u8 = 0 as *mut u8;

pub struct State<'a> {
    pub regs: [u8; Sizes::REGISTER_COUNT as usize],
    echo_hist: Option<[[&'a mut i64; 2]; (Sizes::ECHO_HIST_SIZE * 2) as usize]>,
    /*echo_hist_pos: [&'a mut i64; 2], //&echo hist[0 to 7]*/ //ignoring this for now
    every_other_sample: i64,
    kon: i64,
    noise: i64,
    echo_offset: i64,
    echo_length: i64,
    phase: i64,
    counters: [usize; 4],
    pub new_kon: i64,
    t_koff: i64,
    pub voices: [Voice; Sizes::VOICE_COUNT as usize],
    counter_select: [usize; 32],
    pub ram: [u8; 0xFFFF], // 64K shared RAM between DSP and SMP
    pub mute_mask: i64,
    surround_threshold: i64,
    out: Option<*mut sample_t>,
    out_end: Option<*mut sample_t>,
    out_begin: Option<*mut sample_t>,
    extra: [sample_t; Sizes::EXTRA_SIZE as usize],
}

//functions that directly modify the state
impl State<'static> {
    
    pub fn new() -> State<'static> {

        State {
            regs: [0; Sizes::REGISTER_COUNT as usize],
            echo_hist: None,
            every_other_sample: 0,
            kon: 0,
            noise: 0,
            echo_offset: 0,
            echo_length: 0,
            phase: 0,
            counters: [0; 4],
            new_kon: 0,
            t_koff: 0,
            voices: [Voice::new(); Sizes::VOICE_COUNT as usize],
            counter_select: [0;32],
            ram: [0u8; 0xFFFF], // 64K shared RAM between DSP and SMP
            mute_mask: 0,
            surround_threshold: 0,
            out: None,
            out_end: None,
            out_begin: None,
            extra: [0; Sizes::EXTRA_SIZE as usize],
        } 
    }
    
    pub fn set_ram(&mut self, ram_64K: [u8;0xFFFF]) {
        self.ram = ram_64K; 
    }

    pub fn extra(&self) -> [sample_t; 16] {
        return self.extra;
    }
    
    pub fn get_phase(&self) -> i64 {
        return self.phase;
    }

    pub fn set_phase(&mut self, new_phase:i64) {
        self.phase = new_phase; 
    }

    pub fn out_pos(&self) -> *const sample_t {
        return self.out.unwrap();
    }

    pub fn sample_count(&self) -> *const sample_t {
        return self.out.unwrap().wrapping_offset(-(self.out_begin.unwrap() as isize));
    }

    pub fn read(&self, addr: i64) -> u8 {
        assert!(addr < Sizes::REGISTER_COUNT as i64);
        return self.regs[addr as usize];
    }

    pub fn set_output<'a>(&mut self, out: *mut sample_t, out_size: i64) {
        assert_eq!((out_size & 1), 0, "Out size is not even!: {}", out_size);
        
        if out.is_null() {
            self.out_begin = Some(out);
            self.out = Some(out);
            self.out_end = Some(out.wrapping_offset(out_size as isize));
        }

        let out: *mut sample_t = &mut self.extra[0];
        let out_size = Sizes::EXTRA_SIZE as i64;
        self.out_begin = Some(out);
        self.out = Some(out);
        self.out_end = Some(out.wrapping_offset(out_size as isize));
    }

    pub fn write(&mut self, addr: i64, data: i64) {
        assert!(addr < Sizes::REGISTER_COUNT as i64);
        self.regs[addr as usize] = data as u8;
        let low: i64 = addr & 0x0F;

        //voice volumes
        if low < 0x2 {
            self.update_voice_vol(low ^ addr);
        } else if low == 0xC {
            if addr == GlobalRegisters::r_kon as i64 {
                self.new_kon = data;
            }

            // always cleared, regardless of data written
            if addr == GlobalRegisters::r_endx as i64 {
                self.regs[GlobalRegisters::r_endx as usize] = 0;
            }
        }
    }

    pub fn init_counter(&mut self) {
        self.counters[0] = 1;
        self.counters[1] = 0;
        self.counters[2] = (!0) << 5; // FFFFFFE0 ie: 4 bytes, last 5 bits 0
        self.counters[3] = 0x0B;

        let mut n = 2;

        for i in 0..32 {
            self.counter_select[i] = n as usize;
            //TODO: Make sure this is OK
            n -= 1;
            if n == 0 {
                n = 3;
            }
        }
        self.counter_select[0] = 0;
        self.counter_select[30] = 2;
    }

    pub fn run_counter(&mut self, i: i64) {
        let mut n = self.counters[i as usize];

        //TODO make sure this is OK
        //probably not going to work
        if (n & 7) == 0 {
            n.wrapping_sub((6 - i) as usize);
        }
        n.wrapping_sub(1);

        self.counters[i as usize] = n;
    }

    pub fn soft_reset_common(&mut self) {
        // require (m.ram)
        self.noise = 0x4000;
        /* *self.echo_hist_pos      = self.echo_hist; //TODO not sure if right */
         // ignoring this until further notice
        self.every_other_sample = 1;
        self.echo_offset = 0;
        self.phase = 0;

        self.init_counter();
    }

    //resets DSP to power-on state
    // Emulation
    pub fn reset(&self) {
        unimplemented!(); 
    }

    //Emulates pressing reset switch on SNES
    pub fn soft_reset(&self){
        unimplemented!(); 
    }

    // don't need this?
    /* fn write_outline(addr: i64, data: i64); */

    //TODO: no way will this work, using it as a basis
    pub fn update_voice_vol(&mut self, addr: i64) {
        let mut l: i64 = self.regs[(addr + VoiceRegisters::v_voll as i64) as usize] as i64;
        let mut r: i64 = self.regs[(addr + VoiceRegisters::v_volr as i64) as usize] as i64;
        if l * r < self.surround_threshold {
            //signs differ, so negate those that are negative
            l ^= l >> 7;
            r ^= r >> 7;
        }
        let v = &mut self.voices[(addr >> 4) as usize];
        let enabled: i64 = v.enabled;
        v.volume[0] = (l as i64) & enabled;
        v.volume[1] = (r as i64) & enabled;
    }

    pub fn disable_surround(&mut self, disable: bool) {
        if disable {
            self.surround_threshold = 0;
        } else {
            self.surround_threshold = -0x4000;
        }
    }

    pub fn mute_voices(&mut self, mask: i64) {
        self.mute_mask = mask;
        for i in 0..Sizes::VOICE_COUNT {
            self.voices[i].enabled = (mask >> i & 1) - 1; 
            self.update_voice_vol((i * 0x10) as i64);
        }
    }
}
