use registers::EnvMode;
use sizes::Sizes;
use config::*;

#[derive(Debug, Copy, Clone)]
pub struct Voice {
    // decoded samples. should be twice the size to simplify wrap handling
    buf: [isize; (Sizes::BRR_BUF_SIZE * 2) as usize],
    pub buf_pos: usize, // place in buffer where next samples will be decoded
    interp_pos: isize, // relative fractional positoin in sample (0x1000 = 1.0)
    brr_addr: isize, // address of current BRR block
    pub brr_offset: isize, // current decoding offset in BRR block
    kon_delay: isize, // KON delay/current setup phase
    env_mode: EnvMode,
    env: isize, // current envelope level
    hidden_env: isize, // used by GAIN mode 7, obscure quirk
    pub volume: [isize; 2], // copy of volume from DSP registers, with surround disabled
    pub enabled: isize, // -1 if enabled, 0 if muted
                    //TODO: Consider changing enabled to bool
}

impl Voice {
    pub fn new() -> Voice {
        Voice {
            buf: [0isize; ((Sizes::BRR_BUF_SIZE * 2) as usize)],
            buf_pos: 0,
            interp_pos: 0,
            brr_addr: 0,
            brr_offset: 0,
            kon_delay: 0,
            env_mode: EnvMode::env_release,
            env: 0,
            hidden_env: 0,
            volume: [0isize; 2],
            enabled: 0
        }  
    }
}

