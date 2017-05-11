use registers::EnvMode;
use sizes::Sizes;
use config::*;

#[derive(Debug, Copy, Clone)]
pub struct Voice {
    // decoded samples. should be twice the size to simplify wrap handling
    buf: [i64; (Sizes::BRR_BUF_SIZE * 2) as usize],
    pub buf_pos: usize, // place in buffer where next samples will be decoded
    interp_pos: i64, // relative fractional position in sample (0x1000 = 1.0)
    pub brr_addr: i64, // address of current BRR block
    pub brr_offset: i64, // current decoding offset in BRR block
    pub kon_delay: i64, // KON delay/current setup phase
    env_mode: EnvMode,
    env: i64, // current envelope level
    hidden_env: i64, // used by GAIN mode 7, obscure quirk
    pub volume: [i64; 2], // copy of volume from DSP registers, with surround disabled
    pub enabled: i64, // -1 if enabled, 0 if muted
                    //TODO: Consider changing enabled to bool
}

impl Voice {
    pub fn new() -> Voice {
        Voice {
            buf: [0i64; ((Sizes::BRR_BUF_SIZE * 2) as usize)],
            buf_pos: 0,
            interp_pos: 0,
            brr_addr: 0,
            brr_offset: 0,
            kon_delay: 0,
            env_mode: EnvMode::env_release,
            env: 0,
            hidden_env: 0,
            volume: [064; 2],
            enabled: 0
        }  
    }
}

