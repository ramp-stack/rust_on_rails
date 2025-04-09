use super::ComponentContext;

pub use crate::canvas::{MouseState, KeyboardEvent, KeyboardState, NamedKey, Key, SmolStr};

use downcast_rs::{DowncastSync, impl_downcast};

use std::fmt::Debug;

pub trait Events: Debug {
    fn on_event(&mut self, _ctx: &mut ComponentContext, _event: &mut dyn Event) -> bool {true}
}

pub trait Event: Debug + DowncastSync {
    ///Function for event to decide on weather to pass the event to a child, Event can also be modified for the child
    fn pass(self: Box<Self>, _ctx: &mut ComponentContext, children: Vec<((i32, i32), (u32, u32))>) -> Vec<Option<Box<dyn Event>>>;
}
impl_downcast!(sync Event);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub position: Option<(u32, u32)>,
    pub state: MouseState,
}

impl Event for MouseEvent {
    fn pass(self: Box<Self>, _ctx: &mut ComponentContext, children: Vec<((i32, i32), (u32, u32))>) -> Vec<Option<Box<dyn Event>>> {
        let mut passed = false;
        children.into_iter().rev().map(|(offset, size)| {//Reverse to click on the top most element
            let position = self.position.and_then(|position| (!passed).then(|| (
                (position.0 as i32) > offset.0 &&
                 (position.0 as i32) < offset.0+size.0 as i32 &&
                 (position.1 as i32) > offset.1 &&
                 (position.1 as i32) < offset.1+size.1 as i32
                ).then(|| {
                    passed = true;
                    ((position.0 as i32 - offset.0) as u32, (position.1 as i32 - offset.1) as u32)
            })).flatten());
            Some(Box::new(MouseEvent{position, state: self.state}) as Box<dyn Event>)
        }).collect::<Vec<_>>().into_iter().rev().collect()
    }
}

impl Event for KeyboardEvent {
    fn pass(self: Box<Self>, _ctx: &mut ComponentContext, children: Vec<((i32, i32), (u32, u32))>) -> Vec<Option<Box<dyn Event>>> {
        children.into_iter().map(|_| Some(self.clone() as Box<dyn Event>)).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TickEvent;
impl Event for TickEvent {
    fn pass(self: Box<Self>, _ctx: &mut ComponentContext, children: Vec<((i32, i32), (u32, u32))>) -> Vec<Option<Box<dyn Event>>> {
        children.into_iter().map(|_| Some(Box::new(*self) as Box<dyn Event>)).collect()
    }
}
