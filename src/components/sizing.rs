use std::cmp::Ordering;
use std::ops::{Add, Sub};

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct MinSize(pub u32);

impl MinSize {
    pub fn get(&self, s: u32) -> u32 {s.max(self.0)}
}

impl Add<u32> for MinSize {
    type Output = Self;

    fn add(self, other: u32) -> Self {
        MinSize(self.0.checked_add(other).unwrap_or(u32::MAX))
    }
}

impl Add for MinSize {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        MinSize(self.0.checked_add(other.0).unwrap_or(u32::MAX))
    }
}

impl Sub for MinSize {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        MinSize(self.0.saturating_sub(other.0))
    }
}

impl PartialEq<MaxSize> for MinSize {
    fn eq(&self, other: &MaxSize) -> bool {self.0.eq(&other.0)}
}

impl PartialOrd<MaxSize> for MinSize {
    fn partial_cmp(&self, other: &MaxSize) -> Option<Ordering> {Some(self.0.cmp(&other.0))}
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct MaxSize(pub u32);

impl MaxSize {
    pub const MAX: Self = MaxSize(u32::MAX);
    pub fn get(&self, s: u32) -> u32 {s.min(self.0)}
}

//  impl PartialOrd for MaxSize {
//      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {Some(self.cmp(other))}
//  }

//  impl Ord for MaxSize {
//      fn cmp(&self, other: &Self) -> Ordering {self.0.cmp(other.0)}
//  }

impl Add<u32> for MaxSize {
    type Output = Self;
    fn add(self, other: u32) -> Self {
        MaxSize(self.0.checked_add(other).unwrap_or(u32::MAX))
    }
}

impl Add for MaxSize {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        MaxSize(self.0.checked_add(other.0).unwrap_or(u32::MAX))
    }
}

impl Sub for MaxSize {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        MaxSize(self.0.saturating_sub(other.0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizeInfo {
    min_width: MinSize,
    min_height: MinSize,
    max_width: MaxSize,
    max_height: MaxSize
}

impl SizeInfo {
    pub fn min_width(&self) -> MinSize {self.min_width}
    pub fn min_height(&self) -> MinSize {self.min_height}
    pub fn max_width(&self) -> MaxSize {self.max_width}
    pub fn max_height(&self) -> MaxSize {self.max_height}

    pub fn new(min_width: MinSize, min_height: MinSize, max_width: MaxSize, max_height: MaxSize) -> Self {
        if min_width > max_width {panic!("Min Width was Greater Than Max Width");}
        if min_height > max_height {panic!("Min Height was Greater Than Max Height");}
        SizeInfo{min_width, min_height, max_width, max_height}
    }

    pub fn fixed(size: (u32, u32)) -> Self {
        SizeInfo {
            min_width: MinSize(size.0),
            min_height: MinSize(size.1),
            max_width: MaxSize(size.0),
            max_height: MaxSize(size.1)
        }
    }

    pub fn get(&self, size: (u32, u32)) -> (u32, u32) {
        (
            self.max_width.get(self.min_width.get(size.0)),
            self.max_height.get(self.min_height.get(size.1))
        )
    }
}
