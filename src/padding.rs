use std::fmt::Debug;

#[derive(Clone, Copy)]
pub struct Padding<const N: usize> {
    _padding: [u8; N],
}
impl<const N: usize> Default for Padding<N> {
    fn default() -> Self {
        Self{ _padding: [0u8; N] }
    }
}
impl <const N: usize> PartialEq for Padding<N> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl <const N: usize> Eq for Padding<N> {}
impl <const N: usize> Debug for Padding<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{N} bytes")
    }
}
