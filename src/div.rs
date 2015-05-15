pub fn div_up_usize(p: usize, q: usize) -> usize {
    div_up(p as u32, q as u32) as usize
}

pub fn div_up(p: u32, q: u32) -> u32 {
    if p % q == 0 { p/q }
    else { 1 + p/q }
}
