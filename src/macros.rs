use state::State;
use SPC_DSP::counter_mask;

macro_rules! clamp16 {
    ( $io:expr ) => {
        {
            if ($io as i16) != $io {
                $io = ($io >> 31) ^ 0x7FFF; 
            }
        }
    };
}

macro_rules! read_counter {
    ( $rate:expr, $state:expr) => {
        (*$state.counter_select[$rate] & counter_mask![$rate])
    }
}

//TODO some tricks because you can't use if-else in static invocation
//will eventually be added in Rust
//but for now hacky implementation
macro_rules! rate {
   ( $rate:expr, $div:expr ) => {
        (
            ($rate >= $div) as i32 * ($rate / $div * 8 - 1) +
            ($rate <  $div) as i32 * ($rate - 1)
        ) as u32
   }
}

macro_rules! reg {
    (mvoll) => (m.regs[GlobalRegisters::r_mvoll as usize]);
    (mvolr) => (m.regs[GlobalRegisters::r_mvolr as usize]);
    (evoll) => (m.regs[GlobalRegisters::r_evoll as usize]);
    (evolr) => (m.regs[GlobalRegisters::r_evolr as usize]);
    (kon)   => (m.regs[GlobalRegisters::r_kon   as usize]);
    (koff)  => (m.regs[GlobalRegisters::r_koff  as usize]);
    (flg)   => (m.regs[GlobalRegisters::r_flg   as usize]);
    (endx)  => (m.regs[GlobalRegisters::r_endx  as usize]);
    (efb)   => (m.regs[GlobalRegisters::r_efb   as usize]);
    (pmon)  => (m.regs[GlobalRegisters::r_pmon  as usize]);
    (non)   => (m.regs[GlobalRegisters::r_non   as usize]);
    (eon)   => (m.regs[GlobalRegisters::r_eon   as usize]);
    (dir)   => (m.regs[GlobalRegisters::r_dir   as usize]);
    (esa)   => (m.regs[GlobalRegisters::r_esa   as usize]);
    (edl)   => (m.regs[GlobalRegisters::r_edl   as usize]);
    (fir)   => (m.regs[GlobalRegisters::r_fir   as usize]);
}

