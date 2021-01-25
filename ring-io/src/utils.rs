use std::io;

pub fn resultify(x: i32) -> io::Result<u32> {
    if x >= 0 {
        Ok(x as u32)
    } else {
        Err(io::Error::from_raw_os_error(-x))
    }
}
