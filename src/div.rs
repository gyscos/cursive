pub fn div_up_usize(p: usize, q: usize) -> usize {
    if p % q == 0 { p/q }
    else { 1 + p/q }
}

pub fn div_up(p: u32, q: u32) -> u32 {
    if p % q == 0 { p/q }
    else { 1 + p/q }
}
