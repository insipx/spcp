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
    (mvoll) => (GlobalRegisters::r_mvoll as usize);
    (mvolr) => (GlobalRegisters::r_mvolr as usize);
    (evoll) => (GlobalRegisters::r_evoll as usize);
    (evolr) => (GlobalRegisters::r_evolr as usize);
    (kon)   => (GlobalRegisters::r_kon   as usize);
    (koff)  => (GlobalRegisters::r_koff  as usize);
    (flg)   => (GlobalRegisters::r_flg   as usize);
    (endx)  => (GlobalRegisters::r_endx  as usize);
    (efb)   => (GlobalRegisters::r_efb   as usize);
    (pmon)  => (GlobalRegisters::r_pmon  as usize);
    (non)   => (GlobalRegisters::r_non   as usize);
    (eon)   => (GlobalRegisters::r_eon   as usize);
    (dir)   => (GlobalRegisters::r_dir   as usize);
    (esa)   => (GlobalRegisters::r_esa   as usize);
    (edl)   => (GlobalRegisters::r_edl   as usize);
    (fir)   => (GlobalRegisters::r_fir   as usize);
}


/*
macro_rules! reg {
    (mvoll) => (unsafe{*(REGS + GlobalRegisters::r_mvoll as usize)});
    (mvolr) => (unsafe{*REGS[GlobalRegisters::r_mvolr as usize]});
    (evoll) => (unsafe{*REGS[GlobalRegisters::r_evoll as usize]});
    (evolr) => (unsafe{*REGS[GlobalRegisters::r_evolr as usize]});
    (kon)   => (unsafe{*(REGS.offset(GlobalRegisters::r_kon   as isize))});
    (koff)  => (unsafe{*REGS[GlobalRegisters::r_koff  as usize]});
    (flg)   => (unsafe{*REGS[GlobalRegisters::r_flg   as usize]});
    (endx)  => (unsafe{*REGS[GlobalRegisters::r_endx  as usize]});
    (efb)   => (unsafe{*REGS[GlobalRegisters::r_efb   as usize]});
    (pmon)  => (unsafe{*REGS[GlobalRegisters::r_pmon  as usize]});
    (non)   => (unsafe{*REGS[GlobalRegisters::r_non   as usize]});
    (eon)   => (unsafe{*REGS[GlobalRegisters::r_eon   as usize]});
    (dir)   => (unsafe{*REGS[GlobalRegisters::r_dir   as usize]});
    (esa)   => (unsafe{*REGS[GlobalRegisters::r_esa   as usize]});
    (edl)   => (unsafe{*REGS[GlobalRegisters::r_edl   as usize]});
    (fir)   => (unsafe{*REGS[GlobalRegisters::r_fir   as usize]});
}

*/
