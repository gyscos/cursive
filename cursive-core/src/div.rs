use num::Num;

/// Integer division that rounds up.
pub fn div_up<T>(p: T, q: T) -> T
where
    T: Num + Clone,
{
    let d = p.clone() / q.clone();

    if p % q == T::zero() {
        d
    } else {
        T::one() + d
    }
}
