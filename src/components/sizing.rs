use super::ComponentContext;

///Trait for layouts that determine the offset and allotted sizes of its children
pub trait Layout: std::fmt::Debug {

    ///Given a list of children size requests calculate the size request for the total layout
    fn size(&self, ctx: &mut ComponentContext, children: Vec<SizeInfo>) -> SizeInfo;

    ///Given an allotted size and the list of chlidren size requests (which may respect the size request),
    ///calculate the actual offsets and allotted sizes for its children
    fn build(&self, ctx: &mut ComponentContext, size: (u32, u32), children: Vec<SizeInfo>) -> Vec<((i32, i32), (u32, u32))>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizeInfo {
    min_width: u32,
    min_height: u32,
    max_width: u32,
    max_height: u32
}

impl SizeInfo {
    pub fn min_width(&self) -> u32 {self.min_width}
    pub fn min_height(&self) -> u32 {self.min_height}
    pub fn max_width(&self) -> u32 {self.max_width}
    pub fn max_height(&self) -> u32 {self.max_height}

    pub fn new(min_width: u32, min_height: u32, max_width: u32, max_height: u32) -> Self {
        if min_width > max_width {panic!("Min Width was Greater Than Max Width");}
        if min_height > max_height {panic!("Min Height was Greater Than Max Height");}
        SizeInfo{min_width, min_height, max_width, max_height}
    }

    pub fn fixed(size: (u32, u32)) -> Self {
        SizeInfo {
            min_width: size.0,
            min_height: size.1,
            max_width: size.0,
            max_height: size.1
        }
    }

    pub fn get(&self, size: (u32, u32)) -> (u32, u32) {
        (
            self.max_width.min(self.min_width.max(size.0)),
            self.max_height.min(self.min_height.max(size.1))
        )
    }

    pub fn add(&self, w: u32, h: u32) -> SizeInfo {
        self.add_width(w).add_height(h)
    }

    pub fn add_width(&self, w: u32) -> SizeInfo {
        SizeInfo::new(self.min_width.saturating_add(w), self.min_height, self.max_width.saturating_add(w), self.max_height)
    }

    pub fn add_height(&self, h: u32) -> SizeInfo {
        SizeInfo::new(self.min_width, self.min_height.saturating_add(h), self.max_width, self.max_height.saturating_add(h))
    }
}
