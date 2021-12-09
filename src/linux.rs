use std::process::Child;

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[repr(i32)]
pub enum Signal {
    INT = 2,
    _KILL = 9,
    _TERM = 15,
}

pub fn signal(process: &Child, signal: Signal) {
    unsafe {
        let ret_code = kill(process.id() as i32, signal as i32);
        if ret_code != 0 {
            panic!("call to kill syscall failed with retcode: {}", ret_code);
        }
    }
}